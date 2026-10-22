use std::{
    env, io,
    path::PathBuf,
    process::{Command, Output},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use serde::Deserialize;
use serde_json::Value;
use thiserror::Error;
use url::Url;

use crate::{
    diagnostics::Diagnostics,
    model::{Repository, StagedAttachment},
};

const ATTACHMENT_ENDPOINT: &str = "https://uploads.github.com/user-attachments/assets";
const AUTH_HELP: &str =
    "GitHub CLI not found or unauthenticated. Please run gh auth login in your terminal.";

mod native;

#[derive(Debug, Error)]
pub enum GithubError {
    #[error("{AUTH_HELP}")]
    CliUnavailable,
    #[error("{AUTH_HELP}")]
    Unauthenticated,
    #[error("Could not load repositories: {0}")]
    RepositoryList(String),
    #[error("Upload failed: {0}")]
    Upload(String),
    #[error("Upload canceled. You can retry this attachment.")]
    Canceled,
}

#[derive(Clone)]
pub struct GithubSession {
    pub token: String,
    pub repositories: Vec<Repository>,
}

pub fn load_session(owner: &str, diagnostics: &Diagnostics) -> Result<GithubSession, GithubError> {
    let token = read_token(diagnostics)?;
    let repositories = read_repositories(owner, diagnostics)?;
    Ok(GithubSession {
        token,
        repositories,
    })
}

/// Reads the active GitHub CLI token without requesting any repositories.
pub fn load_token(diagnostics: &Diagnostics) -> Result<String, GithubError> {
    read_token(diagnostics)
}

/// Resolves one remembered repository without loading the owner's full list.
pub fn load_repository_id(
    owner: &str,
    repository: &str,
    diagnostics: &Diagnostics,
) -> Result<u64, GithubError> {
    let route = format!("repos/{owner}/{repository}");
    diagnostics.info("Resolving the remembered repository through GitHub CLI.");
    let output = run_gh(
        ["api", "--method", "GET", &route, "--jq", ".id"],
        diagnostics,
    )
    .map_err(|error| match error.kind() {
        io::ErrorKind::NotFound => GithubError::CliUnavailable,
        _ => GithubError::RepositoryList(error.to_string()),
    })?;
    if !output.status.success() {
        log_gh_failure("GitHub CLI repository request", &output, diagnostics);
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(GithubError::RepositoryList(if detail.is_empty() {
            "GitHub CLI returned an error".into()
        } else {
            detail
        }));
    }
    String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse()
        .map_err(|_| GithubError::RepositoryList("GitHub returned an invalid repository ID".into()))
}

/// Shared by a whole sequential batch. Cancel drops the active HTTP request and
/// prevents subsequent requests; already accepted remote assets cannot be undone.
#[derive(Clone, Default)]
pub struct UploadControl {
    canceled: Arc<AtomicBool>,
}

impl UploadControl {
    pub fn cancel(&self) {
        self.canceled.store(true, Ordering::Release);
    }

    pub fn is_canceled(&self) -> bool {
        self.canceled.load(Ordering::Acquire)
    }
}

#[derive(Clone, Copy)]
struct UploadTimeouts {
    total: Duration,
    idle: Duration,
}

impl Default for UploadTimeouts {
    fn default() -> Self {
        Self {
            total: Duration::from_secs(300),
            idle: Duration::from_secs(90),
        }
    }
}

pub fn upload(
    token: &str,
    repository_id: u64,
    attachment: &StagedAttachment,
    diagnostics: &Diagnostics,
    control: &UploadControl,
    progress: impl Fn(u64, u64) + Send + Sync + 'static,
) -> Result<String, GithubError> {
    if control.is_canceled() {
        return Err(GithubError::Canceled);
    }
    diagnostics.info("Streaming an attachment to GitHub.");
    let result = upload_to(
        ATTACHMENT_ENDPOINT,
        token,
        repository_id,
        attachment,
        control,
        Arc::new(progress),
        UploadTimeouts::default(),
    );
    if result.is_ok() {
        diagnostics.info("GitHub accepted the attachment.");
    }
    result
}

fn upload_to(
    endpoint: &str,
    token: &str,
    repository_id: u64,
    attachment: &StagedAttachment,
    control: &UploadControl,
    progress: Arc<dyn Fn(u64, u64) + Send + Sync>,
    timeouts: UploadTimeouts,
) -> Result<String, GithubError> {
    native::upload(
        endpoint,
        token,
        repository_id,
        attachment,
        control,
        progress,
        timeouts,
    )
}

fn validate_attachment_url(url: &str) -> Result<(), GithubError> {
    let valid = Url::parse(url).is_ok_and(|parsed| {
        parsed.scheme() == "https"
            && parsed.host_str() == Some("github.com")
            && parsed.port_or_known_default() == Some(443)
            && parsed.username().is_empty()
            && parsed.password().is_none()
            && parsed
                .path()
                .strip_prefix("/user-attachments/assets/")
                .is_some_and(|id| !id.is_empty())
    });
    if valid {
        Ok(())
    } else {
        Err(GithubError::Upload(
            "GitHub returned an unexpected attachment URL. The upload may have completed.".into(),
        ))
    }
}

fn upload_http_error(status: u16, body: &str, retry_after: Option<&str>) -> String {
    let hint = match status {
        401 => "GitHub authentication expired. Run gh auth login and reopen Gatto.",
        403 | 404 => {
            "GitHub denied the upload. Check repository write access, token permissions, and any organization SSO requirement."
        }
        413 => {
            "GitHub rejected the file size. For videos over 10 MB, verify the repository's paid plan and your upload eligibility, or disable the 100 MB preference and compress below 10 MB."
        }
        415 | 422 => {
            "GitHub rejected this attachment. Check its format or export the video as H.264 MP4."
        }
        429 => "GitHub is rate limiting uploads. Wait before retrying.",
        500..=599 => "GitHub's upload service is temporarily unavailable. Try again later.",
        _ => "GitHub rejected the upload. Check repository access and try again.",
    };
    let detail = serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|value| {
            value
                .get("message")
                .or_else(|| value.get("error"))
                .and_then(Value::as_str)
                .map(|value| value.chars().take(300).collect::<String>())
        })
        .unwrap_or_default();
    let retry = retry_after
        .and_then(|value| value.parse::<u64>().ok())
        .map(|seconds| format!(" Retry after {seconds} seconds."))
        .unwrap_or_default();
    format!(
        "HTTP {status}: {hint}{retry}{}",
        if detail.is_empty() {
            String::new()
        } else {
            format!(" ({detail})")
        }
    )
}

fn read_token(diagnostics: &Diagnostics) -> Result<String, GithubError> {
    diagnostics.info("Checking GitHub CLI authentication.");
    let output =
        run_gh(["auth", "token", "--hostname", "github.com"], diagnostics).map_err(|error| {
            match error.kind() {
                io::ErrorKind::NotFound => GithubError::CliUnavailable,
                _ => GithubError::Unauthenticated,
            }
        })?;
    if !output.status.success() {
        log_gh_failure("GitHub CLI authentication", &output, diagnostics);
        return Err(GithubError::Unauthenticated);
    }
    let token = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if token.is_empty() {
        return Err(GithubError::Unauthenticated);
    }
    Ok(token)
}

fn read_repositories(
    owner: &str,
    diagnostics: &Diagnostics,
) -> Result<Vec<Repository>, GithubError> {
    read_repositories_with(owner, diagnostics, |args| run_gh(args, diagnostics))
}

// RepositoryOwner covers both personal accounts and organizations. Restricting
// ownerAffiliations avoids including repositories a person only collaborates on.
const REPOSITORIES_QUERY: &str = r#"
query($owner: String!, $endCursor: String) {
  repositoryOwner(login: $owner) {
    repositories(first: 100, after: $endCursor, ownerAffiliations: [OWNER]) {
      nodes { databaseId name }
      pageInfo { hasNextPage endCursor }
    }
  }
}
"#;

fn read_repositories_with(
    owner: &str,
    diagnostics: &Diagnostics,
    request: impl FnOnce([&str; 9]) -> io::Result<Output>,
) -> Result<Vec<Repository>, GithubError> {
    let owner_field = format!("owner={owner}");
    let query_field = format!("query={REPOSITORIES_QUERY}");
    diagnostics.info("Requesting the account or organization repository list through GitHub CLI.");
    let output = request([
        "api",
        "graphql",
        "--paginate",
        "-f",
        &query_field,
        "-f",
        &owner_field,
        "--jq",
        ".data.repositoryOwner.repositories.nodes[] | [.databaseId, .name] | @tsv",
    ])
    .map_err(|error| match error.kind() {
        io::ErrorKind::NotFound => GithubError::CliUnavailable,
        _ => GithubError::RepositoryList(error.to_string()),
    })?;

    if !output.status.success() {
        log_gh_failure("GitHub CLI repository request", &output, diagnostics);
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(GithubError::RepositoryList(if detail.is_empty() {
            "GitHub CLI returned an error".into()
        } else {
            detail
        }));
    }

    let mut repositories = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let (id, name) = line.split_once('\t')?;
            Some(Repository {
                id: id.parse().ok()?,
                name: name.to_owned(),
            })
        })
        .collect::<Vec<_>>();
    repositories.sort_by_key(|repository| repository.name.to_ascii_lowercase());

    if repositories.is_empty() {
        return Err(GithubError::RepositoryList(format!(
            "No accessible repositories were found for {owner}"
        )));
    }
    diagnostics.info(format!(
        "GitHub CLI returned {} repositories.",
        repositories.len()
    ));
    Ok(repositories)
}

fn run_gh<const N: usize>(args: [&str; N], diagnostics: &Diagnostics) -> io::Result<Output> {
    let Some(path) = github_cli_path() else {
        diagnostics.error(
            "GitHub CLI was not found. Checked GH_PATH, PATH, /opt/homebrew/bin/gh, and /usr/local/bin/gh.",
        );
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "GitHub CLI not found",
        ));
    };
    diagnostics.info(format!("Using GitHub CLI at {}.", path.display()));
    let result = Command::new(&path)
        .args(args)
        .env("GH_PROMPT_DISABLED", "1")
        .output();
    if let Err(error) = &result {
        diagnostics.error(format!(
            "Could not run GitHub CLI at {}: {error}",
            path.display()
        ));
    }
    result
}

fn github_cli_path() -> Option<PathBuf> {
    env::var_os("GH_PATH")
        .map(PathBuf::from)
        .filter(|path| path.is_file())
        .or_else(|| {
            env::var_os("PATH").and_then(|paths| {
                env::split_paths(&paths)
                    .map(|directory| directory.join("gh"))
                    .find(|path| path.is_file())
            })
        })
        .or_else(|| {
            ["/opt/homebrew/bin/gh", "/usr/local/bin/gh"]
                .into_iter()
                .map(PathBuf::from)
                .find(|path| path.is_file())
        })
}

fn log_gh_failure(operation: &str, output: &Output, diagnostics: &Diagnostics) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let detail = stderr
        .lines()
        .next()
        .unwrap_or("No diagnostic message was returned.");
    diagnostics.error(format!(
        "{operation} exited with {}: {detail}",
        output
            .status
            .code()
            .map_or_else(|| "a signal".to_owned(), |code| code.to_string())
    ));
}

#[derive(Deserialize)]
struct AttachmentResponse {
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    href: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read as _;
    use std::os::unix::process::ExitStatusExt;
    use std::time::Instant;

    fn output(code: i32, stdout: &str, stderr: &str) -> Output {
        Output {
            status: std::process::ExitStatus::from_raw(code << 8),
            stdout: stdout.as_bytes().to_vec(),
            stderr: stderr.as_bytes().to_vec(),
        }
    }

    struct Request {
        headers: String,
        body: Vec<u8>,
    }

    fn receive_request(socket: &mut std::net::TcpStream) -> Request {
        socket
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut bytes = Vec::new();
        while !bytes.ends_with(b"\r\n\r\n") {
            let mut byte = [0];
            socket.read_exact(&mut byte).unwrap();
            bytes.push(byte[0]);
        }
        let headers = String::from_utf8(bytes).unwrap();
        let len: usize = headers
            .lines()
            .find_map(|line| {
                let (key, value) = line.split_once(':')?;
                key.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse().unwrap())
            })
            .unwrap();
        let mut body = vec![0; len];
        socket.read_exact(&mut body).unwrap();
        Request { headers, body }
    }

    fn server(status: u16, body: &str) -> (String, std::thread::JoinHandle<Request>) {
        use std::io::Write as _;
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!(
            "http://{}/user-attachments/assets",
            listener.local_addr().unwrap()
        );
        let body = body.to_owned();
        let worker = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let request = receive_request(&mut socket);
            write!(socket, "HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nRetry-After: 7\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
            request
        });
        (url, worker)
    }

    fn video_attachment() -> StagedAttachment {
        StagedAttachment::from_path(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample.mp4"),
            Default::default(),
        )
        .unwrap()
    }

    fn run_upload(
        endpoint: &str,
        item: &StagedAttachment,
        control: &UploadControl,
        progress: Arc<dyn Fn(u64, u64) + Send + Sync>,
        timeout: Duration,
    ) -> Result<String, GithubError> {
        upload_to(
            endpoint,
            "test-token",
            42,
            item,
            control,
            progress,
            UploadTimeouts {
                total: timeout,
                ..Default::default()
            },
        )
    }

    #[test]
    fn streams_original_video_with_length_metadata_and_progress_then_reopens_for_retry() {
        let item = video_attachment();
        let progress = Arc::new(std::sync::Mutex::new(Vec::new()));
        for _ in 0..2 {
            let (endpoint, worker) = server(
                201,
                r#"{"url":"https://github.com/user-attachments/assets/test-video"}"#,
            );
            let capture = progress.clone();
            let url = run_upload(
                &endpoint,
                &item,
                &UploadControl::default(),
                Arc::new(move |sent, total| capture.lock().unwrap().push((sent, total))),
                Duration::from_secs(5),
            )
            .unwrap();
            assert_eq!(url, "https://github.com/user-attachments/assets/test-video");
            let request = worker.join().unwrap();
            assert!(request.headers.starts_with("POST /user-attachments/assets?name=sample.mp4&content_type=video%2Fmp4&repository_id=42 "));
            assert!(
                request
                    .headers
                    .to_ascii_lowercase()
                    .contains("content-type: video/mp4")
            );
            assert_eq!(request.body, include_bytes!("../tests/fixtures/sample.mp4"));
        }
        let values = progress.lock().unwrap();
        assert_eq!(values.first(), Some(&(0, item.len())));
        assert_eq!(values.last(), Some(&(item.len(), item.len())));
        assert!(values.iter().all(|(sent, total)| sent <= total));
    }

    #[test]
    fn maps_server_failures_and_rejects_unsafe_or_missing_urls() {
        let item = video_attachment();
        for (status, body, message) in [
            (401, "{}", "authentication"),
            (404, "{}", "write access"),
            (413, "{}", "10 MB"),
            (415, "{}", "H.264"),
            (429, "{}", "Retry after 7 seconds"),
            (503, "{}", "temporarily unavailable"),
            (201, "not json", "unreadable"),
            (201, "{}", "no attachment URL"),
            (
                201,
                r#"{"url":"https://evil.example/user-attachments/assets/a"}"#,
                "unexpected",
            ),
            (
                201,
                r#"{"url":"http://github.com/user-attachments/assets/a"}"#,
                "unexpected",
            ),
        ] {
            let (endpoint, worker) = server(status, body);
            let error = run_upload(
                &endpoint,
                &item,
                &UploadControl::default(),
                Arc::new(|_, _| {}),
                Duration::from_secs(5),
            )
            .unwrap_err();
            assert!(error.to_string().contains(message), "{error}");
            worker.join().unwrap();
        }
    }

    #[test]
    fn native_transport_rejects_redirects_challenges_large_responses_and_post_replay() {
        use std::io::Write as _;
        for mode in ["redirect", "challenge", "oversized", "disconnect"] {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let endpoint = format!("http://{}/", listener.local_addr().unwrap());
            let destination = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            destination.set_nonblocking(true).unwrap();
            let location = format!("http://{}/stolen", destination.local_addr().unwrap());
            let worker = std::thread::spawn(move || {
                let (mut socket, _) = listener.accept().unwrap();
                let request = receive_request(&mut socket);
                assert!(
                    request
                        .headers
                        .to_ascii_lowercase()
                        .contains("authorization: bearer test-token")
                );
                let response = match mode {
                    "redirect" => format!("HTTP/1.1 307 Redirect\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"),
                    "challenge" => "HTTP/1.1 401 Unauthorized\r\nWWW-Authenticate: Basic realm=\"unexpected\"\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".into(),
                    "oversized" => format!("HTTP/1.1 201 Created\r\nContent-Length: 70000\r\nConnection: close\r\n\r\n{}", "x".repeat(70000)),
                    _ => String::new(),
                };
                let _ = socket.write_all(response.as_bytes());
                drop(socket);
                std::thread::sleep(Duration::from_millis(100));
                listener.set_nonblocking(true).unwrap();
                assert!(
                    matches!(listener.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock),
                    "POST must not be automatically repeated"
                );
            });
            let error = run_upload(
                &endpoint,
                &video_attachment(),
                &UploadControl::default(),
                Arc::new(|_, _| {}),
                Duration::from_secs(5),
            )
            .unwrap_err();
            if mode == "oversized" {
                assert!(error.to_string().contains("oversized"), "{error}");
            }
            if mode == "challenge" {
                assert!(error.to_string().contains("authentication"), "{error}");
            }
            assert!(
                matches!(destination.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock),
                "Redirect target must never receive the bearer token"
            );
            worker.join().unwrap();
        }
    }

    #[test]
    fn cancellation_interrupts_waiting_for_github_and_timeout_is_actionable() {
        for mode in ["cancel", "timeout", "idle"] {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let endpoint = format!("http://{}/", listener.local_addr().unwrap());
            let (received, ready) = std::sync::mpsc::channel();
            let (release, hold) = std::sync::mpsc::channel::<()>();
            let server = std::thread::spawn(move || {
                let (mut socket, _) = listener.accept().unwrap();
                receive_request(&mut socket);
                received.send(()).unwrap();
                let _ = hold.recv_timeout(Duration::from_secs(5));
            });
            let control = UploadControl::default();
            let job_control = control.clone();
            let job = std::thread::spawn(move || {
                let timeouts = UploadTimeouts {
                    total: if mode == "timeout" {
                        Duration::from_millis(200)
                    } else {
                        Duration::from_secs(30)
                    },
                    idle: if mode == "idle" {
                        Duration::from_millis(200)
                    } else {
                        Duration::from_secs(90)
                    },
                };
                upload_to(
                    &endpoint,
                    "test-token",
                    42,
                    &video_attachment(),
                    &job_control,
                    Arc::new(|_, _| {}),
                    timeouts,
                )
            });
            ready.recv_timeout(Duration::from_secs(5)).unwrap();
            let started = Instant::now();
            if mode == "cancel" {
                control.cancel();
            }
            let error = job.join().unwrap().unwrap_err();
            assert!(started.elapsed() < Duration::from_secs(2));
            assert!(error.to_string().contains(match mode {
                "cancel" => "canceled",
                "idle" => "No upload activity",
                _ => "timed out",
            }));
            release.send(()).unwrap();
            server.join().unwrap();
        }
    }

    #[test]
    fn lists_personal_and_organization_repositories_across_pages() {
        for owner in ["octocat", "acme-labs"] {
            let repositories = read_repositories_with(owner, &Diagnostics::new(), |args| {
                assert_eq!(&args[..4], &["api", "graphql", "--paginate", "-f"]);
                assert_eq!(args[6], format!("owner={owner}"));
                assert!(args[4].contains("repositoryOwner(login: $owner)"));
                assert!(args[4].contains("ownerAffiliations: [OWNER]"));
                assert!(args[4].contains("after: $endCursor"));
                assert!(args[4].contains("pageInfo { hasNextPage endCursor }"));
                assert!(args[8].contains(".databaseId"));
                // gh emits the selected rows from every page into one stream.
                Ok(output(0, "42\tzebra\n9\tAlpha\n103\tprivate-repo\n", ""))
            })
            .unwrap();
            assert_eq!(
                repositories
                    .iter()
                    .map(|repo| (repo.id, repo.name.as_str()))
                    .collect::<Vec<_>>(),
                [(9, "Alpha"), (103, "private-repo"), (42, "zebra")]
            );
        }
    }

    #[test]
    fn reports_inaccessible_owners_and_empty_repository_lists() {
        let diagnostics = Diagnostics::new();
        let error = read_repositories_with("missing", &diagnostics, |_| {
            Ok(output(1, "", "Could not resolve to a RepositoryOwner"))
        })
        .unwrap_err();
        assert!(error.to_string().contains("Could not resolve"));
        let error =
            read_repositories_with("empty", &diagnostics, |_| Ok(output(0, "", ""))).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("No accessible repositories were found for empty")
        );
    }
}

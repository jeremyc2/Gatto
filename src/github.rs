use std::{
    env, io,
    path::PathBuf,
    process::{Command, Output},
    time::Duration,
};

use reqwest::{StatusCode, blocking::Client};
use serde::Deserialize;
use serde_json::Value;
use thiserror::Error;
use url::Url;

use crate::{
    diagnostics::Diagnostics,
    model::{Repository, StagedImage},
};

const ATTACHMENT_ENDPOINT: &str = "https://uploads.github.com/user-attachments/assets";
const AUTH_HELP: &str =
    "GitHub CLI not found or unauthenticated. Please run gh auth login in your terminal.";

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
}

#[derive(Clone)]
pub struct GithubSession {
    pub token: String,
    pub repositories: Vec<Repository>,
}

pub fn load_session(
    organization: &str,
    diagnostics: &Diagnostics,
) -> Result<GithubSession, GithubError> {
    let token = read_token(diagnostics)?;
    let repositories = read_repositories(organization, diagnostics)?;
    Ok(GithubSession {
        token,
        repositories,
    })
}

/// Reads the active GitHub CLI token without requesting any repositories.
pub fn load_token(diagnostics: &Diagnostics) -> Result<String, GithubError> {
    read_token(diagnostics)
}

/// Resolves one remembered repository without loading the organization's full list.
pub fn load_repository_id(
    organization: &str,
    repository: &str,
    diagnostics: &Diagnostics,
) -> Result<u64, GithubError> {
    let route = format!("repos/{organization}/{repository}");
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

pub fn upload(
    token: &str,
    repository_id: u64,
    image: &StagedImage,
    diagnostics: &Diagnostics,
) -> Result<String, GithubError> {
    diagnostics.info("Sending image data to GitHub's attachment service.");
    let client = Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(90))
        .build()
        .map_err(|error| GithubError::Upload(error.to_string()))?;
    let repository_id = repository_id.to_string();

    let response = client
        .post(ATTACHMENT_ENDPOINT)
        .bearer_auth(token)
        .header("Accept", "application/json")
        .header("Content-Type", &image.mime_type)
        .header("User-Agent", concat!("gatto/", env!("CARGO_PKG_VERSION")))
        .query(&[
            ("name", image.name.as_str()),
            ("content_type", image.mime_type.as_str()),
            ("repository_id", repository_id.as_str()),
        ])
        .body(image.bytes.as_ref().clone())
        .send()
        .map_err(|error| GithubError::Upload(error.to_string()))?;

    let status = response.status();
    let body = response
        .text()
        .map_err(|error| GithubError::Upload(error.to_string()))?;
    if !status.is_success() {
        diagnostics.error(format!("GitHub attachment service returned HTTP {status}."));
        return Err(GithubError::Upload(api_error(status, &body)));
    }

    let uploaded: AttachmentResponse = serde_json::from_str(&body)
        .map_err(|error| GithubError::Upload(format!("Invalid GitHub response: {error}")))?;
    let attachment_url = uploaded
        .url
        .or(uploaded.href)
        .ok_or_else(|| GithubError::Upload("GitHub did not return an attachment URL".into()))?;
    let parsed = Url::parse(&attachment_url)
        .map_err(|_| GithubError::Upload("GitHub returned an invalid attachment URL".into()))?;

    if parsed.scheme() != "https"
        || parsed.host_str() != Some("github.com")
        || !parsed.path().starts_with("/user-attachments/assets/")
    {
        return Err(GithubError::Upload(
            "GitHub returned an unexpected attachment URL".into(),
        ));
    }

    diagnostics.info("GitHub attachment service accepted the image.");
    Ok(attachment_url)
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
    organization: &str,
    diagnostics: &Diagnostics,
) -> Result<Vec<Repository>, GithubError> {
    let route = format!("orgs/{organization}/repos?per_page=100&type=all");
    diagnostics.info("Requesting the organization repository list through GitHub CLI.");
    let output = run_gh(
        [
            "api",
            "--paginate",
            "--method",
            "GET",
            &route,
            "--jq",
            ".[] | [.id, .name] | @tsv",
        ],
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
            "No accessible repositories were found for {organization}"
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

fn api_error(status: StatusCode, body: &str) -> String {
    let parsed = serde_json::from_str::<Value>(body).ok();
    let message = parsed
        .as_ref()
        .and_then(|value| value.get("message"))
        .and_then(Value::as_str)
        .or_else(|| {
            parsed
                .as_ref()
                .and_then(|value| value.get("error"))
                .and_then(Value::as_str)
        })
        .unwrap_or_else(|| body.trim());

    if message.is_empty() {
        format!("GitHub returned HTTP {status}")
    } else {
        format!("GitHub returned HTTP {status}: {message}")
    }
}

#[derive(Deserialize)]
struct AttachmentResponse {
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    href: Option<String>,
}

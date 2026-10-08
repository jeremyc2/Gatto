use std::{io, process::Command, time::Duration};

use reqwest::{StatusCode, blocking::Client};
use serde::Deserialize;
use serde_json::Value;
use thiserror::Error;
use url::Url;

use crate::model::{Repository, StagedImage};

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

pub fn load_session(organization: &str) -> Result<GithubSession, GithubError> {
    let token = read_token()?;
    let repositories = read_repositories(organization)?;
    Ok(GithubSession {
        token,
        repositories,
    })
}

pub fn upload(token: &str, repository_id: u64, image: &StagedImage) -> Result<String, GithubError> {
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
        .header(
            "User-Agent",
            concat!("gpui-github-image-upload/", env!("CARGO_PKG_VERSION")),
        )
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

    Ok(attachment_url)
}

fn read_token() -> Result<String, GithubError> {
    let output = Command::new("gh")
        .args(["auth", "token", "--hostname", "github.com"])
        .output()
        .map_err(|error| match error.kind() {
            io::ErrorKind::NotFound => GithubError::CliUnavailable,
            _ => GithubError::Unauthenticated,
        })?;
    if !output.status.success() {
        return Err(GithubError::Unauthenticated);
    }
    let token = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if token.is_empty() {
        return Err(GithubError::Unauthenticated);
    }
    Ok(token)
}

fn read_repositories(organization: &str) -> Result<Vec<Repository>, GithubError> {
    let route = format!("orgs/{organization}/repos?per_page=100&type=all");
    let output = Command::new("gh")
        .args([
            "api",
            "--paginate",
            "--method",
            "GET",
            &route,
            "--jq",
            ".[] | [.id, .name] | @tsv",
        ])
        .output()
        .map_err(|error| match error.kind() {
            io::ErrorKind::NotFound => GithubError::CliUnavailable,
            _ => GithubError::RepositoryList(error.to_string()),
        })?;

    if !output.status.success() {
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
    Ok(repositories)
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

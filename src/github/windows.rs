//! Windows HTTP adapter. Upload bodies are single-use streams and redirects
//! and retries are disabled so the bearer token and body stay on one request.
use super::*;
use futures_util::stream;
use reqwest::{Client, header};
use std::{io::Read as _, sync::Mutex, time::Instant};

const BUFFER_SIZE: usize = 65_536;
const RESPONSE_LIMIT: usize = 65_536;

pub(super) fn upload(
    endpoint: &str,
    token: &str,
    repository_id: u64,
    attachment: &StagedAttachment,
    control: &UploadControl,
    progress: Arc<dyn Fn(u64, u64) + Send + Sync>,
    timeouts: UploadTimeouts,
) -> Result<String, GithubError> {
    if control.is_canceled() {
        return Err(GithubError::Canceled);
    }
    if token.bytes().any(|byte| byte.is_ascii_control()) {
        return Err(GithubError::Unauthenticated);
    }
    let mut url = Url::parse(endpoint).map_err(|_| upload_error("Invalid upload endpoint."))?;
    url.query_pairs_mut()
        .append_pair("name", &attachment.name)
        .append_pair("content_type", &attachment.mime_type)
        .append_pair("repository_id", &repository_id.to_string());
    let reader = attachment
        .reader()
        .map_err(|error| upload_error(error.to_string()))?;
    let len = attachment.len();
    progress(0, len);

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| upload_error("Could not initialize the upload worker."))?;
    runtime.block_on(async {
        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .build()
            .map_err(|_| upload_error("Could not initialize the secure upload connection."))?;
        let activity = Arc::new(Mutex::new(Instant::now()));
        let body_activity = activity.clone();
        let body = stream::try_unfold((reader, 0), move |(mut reader, sent)| {
            let progress = progress.clone();
            let activity = body_activity.clone();
            async move {
                if sent == len {
                    return Ok(None);
                }
                let mut bytes = vec![0; (len - sent).min(BUFFER_SIZE as u64) as usize];
                let count = loop {
                    match reader.read(&mut bytes) {
                        Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                        result => break result?,
                    }
                };
                if count == 0 {
                    return Err(io::Error::from(io::ErrorKind::UnexpectedEof));
                }
                bytes.truncate(count);
                let sent = sent + count as u64;
                *activity.lock().unwrap() = Instant::now();
                progress(sent, len);
                Ok(Some((bytes, (reader, sent))))
            }
        });
        let request = client
            .post(url)
            .bearer_auth(token)
            .header(header::ACCEPT, "application/json")
            .header(header::CONTENT_TYPE, attachment.mime_type.as_str())
            .header(header::CONTENT_LENGTH, len)
            .header(header::USER_AGENT, concat!("gatto/", env!("CARGO_PKG_VERSION")))
            .body(reqwest::Body::wrap_stream(body));
        let receive = async {
            let mut response = request.send().await.map_err(transport_error)?;
            let status = response.status().as_u16();
            let retry_after = response.headers().get(header::RETRY_AFTER)
                .and_then(|value| value.to_str().ok()).map(str::to_owned);
            *activity.lock().unwrap() = Instant::now();
            let mut body = Vec::new();
            while let Some(bytes) = response.chunk().await.map_err(transport_error)? {
                if bytes.len() > RESPONSE_LIMIT - body.len() {
                    return Err(upload_error("GitHub returned an oversized response."));
                }
                body.extend_from_slice(&bytes);
                *activity.lock().unwrap() = Instant::now();
            }
            let body = String::from_utf8_lossy(&body);
            if !(200..300).contains(&status) {
                return Err(upload_error(upload_http_error(status, &body, retry_after.as_deref())));
            }
            let uploaded: AttachmentResponse = serde_json::from_str(&body).map_err(|_|
                upload_error("GitHub returned an unreadable upload response. The attachment may have uploaded; retrying can create another copy."))?;
            let url = uploaded.url.or(uploaded.href).ok_or_else(||
                upload_error("GitHub returned no attachment URL. The attachment may have uploaded; retrying can create another copy."))?;
            validate_attachment_url(&url)?;
            Ok(url)
        };
        let watch = async {
            loop {
                if control.is_canceled() {
                    return Err(GithubError::Canceled);
                }
                if activity.lock().unwrap().elapsed() >= timeouts.idle {
                    return Err(upload_error("No upload activity. Check your connection and retry; GitHub may already have received the file."));
                }
                tokio::time::sleep(Duration::from_millis(40)).await;
            }
        };
        tokio::select! {
            biased;
            result = watch => result,
            result = tokio::time::timeout(timeouts.total, receive) => result.map_err(|_|
                upload_error("The upload timed out. GitHub may already have received the file."))?,
        }
    })
}

fn upload_error(message: impl Into<String>) -> GithubError {
    GithubError::Upload(message.into())
}

fn transport_error(error: reqwest::Error) -> GithubError {
    // Reqwest errors may carry URLs containing attachment and repository metadata.
    if error.is_body() {
        upload_error("Could not read or send the attachment. Add the file again and retry.")
    } else {
        upload_error(
            "Could not complete the secure upload. Check your network, VPN, or proxy and retry.",
        )
    }
}

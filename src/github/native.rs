//! The synchronous upload worker owns its reader until Foundation has finished
//! every callback. Native transport uses a 64 KiB bound stream and serial queue.
use super::*;
use std::{
    ffi::{CStr, CString, c_char, c_void},
    sync::Mutex,
};

#[repr(C)]
#[derive(Default)]
struct Response {
    status: u16,
    error: i32,
    body: *mut u8,
    body_length: usize,
    retry_after: *mut c_char,
}

unsafe extern "C" {
    fn GattoUpload(
        url: *const c_char,
        token: *const c_char,
        mime: *const c_char,
        agent: *const c_char,
        length: u64,
        total: f64,
        idle: f64,
        context: *mut c_void,
        read: extern "C" fn(*mut c_void, *mut u8, usize) -> isize,
        canceled: extern "C" fn(*mut c_void) -> bool,
        progress: extern "C" fn(*mut c_void, u64, u64),
        result: *mut Response,
    );
    fn GattoUploadResultFree(result: *mut Response);
}

impl Drop for Response {
    fn drop(&mut self) {
        // SAFETY: The native producer allocates both buffers and relinquishes
        // them to this response. This is their only release, after copying.
        unsafe { GattoUploadResultFree(self) }
    }
}

struct Callbacks {
    reader: Mutex<Box<dyn std::io::Read + Send>>,
    control: UploadControl,
    progress: Arc<dyn Fn(u64, u64) + Send + Sync>,
    callback_failed: AtomicBool,
}

extern "C" fn read(context: *mut c_void, bytes: *mut u8, capacity: usize) -> isize {
    // SAFETY: GattoUpload holds the context/buffer alive for this call. The
    // producer calls read synchronously; the mutex also protects its ownership.
    let context = unsafe { &*(context.cast::<Callbacks>()) };
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let bytes = unsafe { std::slice::from_raw_parts_mut(bytes, capacity) };
        let mut reader = context.reader.lock().map_err(|_| ())?;
        loop {
            match reader.read(bytes) {
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                result => return result.map(|count| count as isize).map_err(|_| ()),
            }
        }
    }));
    result.ok().and_then(Result::ok).unwrap_or(-1)
}

extern "C" fn canceled(context: *mut c_void) -> bool {
    // SAFETY: Native invalidation completes before the context is dropped.
    let context = unsafe { &*(context.cast::<Callbacks>()) };
    context.control.is_canceled() || context.callback_failed.load(Ordering::Acquire)
}

extern "C" fn progress(context: *mut c_void, sent: u64, total: u64) {
    // SAFETY: This immutable context and its progress closure are Send + Sync;
    // Foundation's serial delegate queue is joined before returning to Rust.
    let context = unsafe { &*(context.cast::<Callbacks>()) };
    if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        (context.progress)(sent, total)
    }))
    .is_err()
    {
        context.callback_failed.store(true, Ordering::Release);
    }
}

pub(super) fn upload(
    endpoint: &str,
    token: &str,
    repository_id: u64,
    attachment: &StagedAttachment,
    control: &UploadControl,
    progress_callback: Arc<dyn Fn(u64, u64) + Send + Sync>,
    timeouts: UploadTimeouts,
) -> Result<String, GithubError> {
    if control.is_canceled() {
        return Err(GithubError::Canceled);
    }
    if token.bytes().any(|byte| byte.is_ascii_control()) {
        return Err(GithubError::Unauthenticated);
    }
    let mut url =
        Url::parse(endpoint).map_err(|_| GithubError::Upload("Invalid upload endpoint".into()))?;
    url.query_pairs_mut()
        .append_pair("name", &attachment.name)
        .append_pair("content_type", &attachment.mime_type)
        .append_pair("repository_id", &repository_id.to_string());
    let string = |value: &str| {
        CString::new(value).map_err(|_| GithubError::Upload("Invalid upload metadata".into()))
    };
    let url = string(url.as_str())?;
    let token = string(token)?;
    let mime = string(&attachment.mime_type)?;
    let agent = string(concat!("gatto/", env!("CARGO_PKG_VERSION")))?;
    let reader = attachment
        .reader()
        .map_err(|error| GithubError::Upload(error.to_string()))?;
    progress_callback(0, attachment.len());
    let mut context = Callbacks {
        reader: Mutex::new(reader),
        control: control.clone(),
        progress: progress_callback,
        callback_failed: AtomicBool::new(false),
    };
    let mut response = Response::default();
    // SAFETY: All C strings and the callback state remain alive throughout this
    // synchronous call. Upload.m waits for delegate invalidation before return.
    unsafe {
        GattoUpload(
            url.as_ptr(),
            token.as_ptr(),
            mime.as_ptr(),
            agent.as_ptr(),
            attachment.len(),
            timeouts.total.as_secs_f64(),
            timeouts.idle.as_secs_f64(),
            (&mut context as *mut Callbacks).cast(),
            read,
            canceled,
            progress,
            &mut response,
        )
    };
    let message = match response.error {
        0 => None,
        1 => Some(
            "Could not connect securely to GitHub. Check your network, VPN, or proxy and retry.",
        ),
        2 => Some(
            "The upload timed out. Check your connection and retry. If GitHub already received the file, retrying can create another copy.",
        ),
        3 => Some("Could not read or send the attachment. Add the file again and retry."),
        4 => return Err(GithubError::Canceled),
        5 => Some(
            "No upload activity for 90 seconds. Check your connection and retry; GitHub may already have received the file.",
        ),
        6 => Some("GitHub returned an oversized response. Check GitHub's status and retry."),
        8 => Some("Could not initialize the secure upload connection."),
        9 => Some(
            "GitHub authentication expired or an unexpected authentication challenge was rejected. Run gh auth login and reopen Gatto.",
        ),
        _ => Some(
            "The connection to GitHub was interrupted. Check your connection and retry; the upload may already have completed.",
        ),
    };
    if let Some(message) = message {
        return Err(GithubError::Upload(message.into()));
    }
    // SAFETY: Both buffers are owned by response until its Drop after parsing;
    // native code caps the body at 64 KiB and reports the actual allocation.
    let bytes = if response.body_length == 0 {
        &[][..]
    } else {
        unsafe { std::slice::from_raw_parts(response.body, response.body_length) }
    };
    let body = String::from_utf8_lossy(bytes);
    let retry_after = if response.retry_after.is_null() {
        None
    } else {
        Some(unsafe { CStr::from_ptr(response.retry_after) }.to_string_lossy())
    };
    if !(200..300).contains(&response.status) {
        return Err(GithubError::Upload(upload_http_error(
            response.status,
            &body,
            retry_after.as_deref(),
        )));
    }
    let uploaded: AttachmentResponse = serde_json::from_str(&body).map_err(|_|
        GithubError::Upload("GitHub returned an unreadable upload response. The attachment may have uploaded; retrying can create another copy.".into()))?;
    let url = uploaded.url.or(uploaded.href).ok_or_else(|| GithubError::Upload("GitHub returned no attachment URL. The attachment may have uploaded; retrying can create another copy.".into()))?;
    validate_attachment_url(&url)?;
    Ok(url)
}

//! Foundation transports bytes; Rust owns the body, policy, deadlines and state.
//! No callback borrows a worker stack or owns a manually allocated C buffer.
use super::*;
use block2::DynBlock;
use objc2::{
    AnyThread as _, DefinedClass, define_class, msg_send, rc::Retained, runtime::ProtocolObject,
};
use objc2_foundation::*;
use std::{
    cell::Cell,
    ptr::NonNull,
    sync::{Condvar, Mutex, MutexGuard},
    time::Instant,
};

const BUFFER_SIZE: usize = 65_536;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Failure {
    Connection,
    Timeout,
    Source,
    Canceled,
    Idle,
    Oversized,
    Interrupted,
    Setup,
    Authentication,
}
impl Failure {
    fn error(self) -> GithubError {
        let message = match self {
            Self::Connection => {
                "Could not connect securely to GitHub. Check your network, VPN, or proxy and retry."
            }
            Self::Timeout => {
                "The upload timed out. Check your connection and retry. If GitHub already received the file, retrying can create another copy."
            }
            Self::Source => "Could not read or send the attachment. Add the file again and retry.",
            Self::Canceled => return GithubError::Canceled,
            Self::Idle => {
                "No upload activity for 90 seconds. Check your connection and retry; GitHub may already have received the file."
            }
            Self::Oversized => {
                "GitHub returned an oversized response. Check GitHub's status and retry."
            }
            Self::Interrupted => {
                "The connection to GitHub was interrupted. Check your connection and retry; the upload may already have completed."
            }
            Self::Setup => "Could not initialize the secure upload connection.",
            Self::Authentication => {
                "GitHub authentication expired or an unexpected authentication challenge was rejected. Run gh auth login and reopen Gatto."
            }
        };
        GithubError::Upload(message.into())
    }
    #[allow(non_upper_case_globals)] // Match the SDK constants without duplicating error codes.
    fn from_native(error: &NSError) -> Self {
        match error.code() {
            NSURLErrorTimedOut => Self::Timeout,
            NSURLErrorUserAuthenticationRequired | NSURLErrorUserCancelledAuthentication => {
                Self::Authentication
            }
            NSURLErrorCannotFindHost
            | NSURLErrorCannotConnectToHost
            | NSURLErrorDNSLookupFailed
            | NSURLErrorSecureConnectionFailed
            | NSURLErrorServerCertificateUntrusted
            | NSURLErrorServerCertificateHasBadDate
            | NSURLErrorServerCertificateHasUnknownRoot
            | NSURLErrorServerCertificateNotYetValid => Self::Connection,
            _ => Self::Interrupted,
        }
    }
}

struct State {
    status: u16,
    failure: Option<Failure>,
    body: Vec<u8>,
    retry_after: Option<String>,
    done: bool,
    invalidated: bool,
    activity: Instant,
    last_progress: Option<Instant>,
}
struct Shared {
    state: Mutex<State>,
    invalidation: Condvar,
    progress: Arc<dyn Fn(u64, u64) + Send + Sync>,
    length: u64,
}
impl Shared {
    fn state(&self) -> MutexGuard<'_, State> {
        self.state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }
    fn fail(&self, failure: Failure) {
        self.state().failure.get_or_insert(failure);
    }
    fn challenge(
        &self,
        challenge: &NSURLAuthenticationChallenge,
        completion: &DynBlock<dyn Fn(NSURLSessionAuthChallengeDisposition, *mut NSURLCredential)>,
    ) {
        // SAFETY: This Foundation constant is available on every supported OS.
        let trust = challenge
            .protectionSpace()
            .authenticationMethod()
            .isEqualToString(unsafe { NSURLAuthenticationMethodServerTrust });
        if !trust {
            self.fail(Failure::Authentication);
        }
        completion.call((
            if trust {
                NSURLSessionAuthChallengeDisposition::PerformDefaultHandling
            } else {
                NSURLSessionAuthChallengeDisposition::CancelAuthenticationChallenge
            },
            std::ptr::null_mut(),
        ));
    }
}

/// Exclusive, unopened endpoint of a bound stream. Only take() exposes it, once.
struct InputOnce(Option<Retained<NSInputStream>>);
// SAFETY: The creator relinquishes this unopened, unscheduled stream. A Mutex
// serializes the sole take, which hands it directly to Foundation's body-stream
// completion. Rust never reads, opens, schedules or concurrently accesses it.
// This is an ownership transfer, not a claim that NSInputStream is thread-safe.
unsafe impl Send for InputOnce {}
impl InputOnce {
    fn take(&mut self) -> Option<Retained<NSInputStream>> {
        self.0.take()
    }
}
struct UploadIvars {
    input: Mutex<InputOnce>,
    shared: Arc<Shared>,
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements; all delegate state is
    // owned and synchronized. No borrowed Rust pointers cross this boundary.
    #[unsafe(super(NSObject))]
    #[ivars = UploadIvars]
    #[name = "GattoUploadDelegate"]
    struct UploadDelegate;
    // SAFETY: NSObjectProtocol imposes no additional requirements.
    unsafe impl NSObjectProtocol for UploadDelegate {}
    // SAFETY: Signatures match Foundation. Each completion runs exactly once,
    // synchronously; no block or borrowed argument is kept after a callback.
    unsafe impl NSURLSessionDelegate for UploadDelegate {
        #[unsafe(method(URLSession:didBecomeInvalidWithError:))]
        fn invalid(&self, _session: &NSURLSession, _error: Option<&NSError>) {
            let shared = &self.ivars().shared;
            shared.state().invalidated = true;
            shared.invalidation.notify_all();
        }
        #[unsafe(method(URLSession:didReceiveChallenge:completionHandler:))]
        unsafe fn challenge(
            &self,
            _session: &NSURLSession,
            challenge: &NSURLAuthenticationChallenge,
            completion: &DynBlock<
                dyn Fn(NSURLSessionAuthChallengeDisposition, *mut NSURLCredential),
            >,
        ) {
            self.ivars().shared.challenge(challenge, completion);
        }
    }
    // SAFETY: Same synchronous completion and ownership contract as above.
    unsafe impl NSURLSessionTaskDelegate for UploadDelegate {
        #[unsafe(method(URLSession:task:willPerformHTTPRedirection:newRequest:completionHandler:))]
        unsafe fn redirect(
            &self,
            _session: &NSURLSession,
            _task: &NSURLSessionTask,
            _response: &NSHTTPURLResponse,
            _request: &NSURLRequest,
            completion: &DynBlock<dyn Fn(*mut NSURLRequest)>,
        ) {
            // Never forward the bearer token to another endpoint.
            completion.call((std::ptr::null_mut(),));
        }
        #[unsafe(method(URLSession:task:didReceiveChallenge:completionHandler:))]
        unsafe fn task_challenge(
            &self,
            _session: &NSURLSession,
            _task: &NSURLSessionTask,
            challenge: &NSURLAuthenticationChallenge,
            completion: &DynBlock<
                dyn Fn(NSURLSessionAuthChallengeDisposition, *mut NSURLCredential),
            >,
        ) {
            self.ivars().shared.challenge(challenge, completion);
        }
        #[unsafe(method(URLSession:task:needNewBodyStream:))]
        unsafe fn stream(
            &self,
            _session: &NSURLSession,
            _task: &NSURLSessionTask,
            completion: &DynBlock<dyn Fn(*mut NSInputStream)>,
        ) {
            let input = self
                .ivars()
                .input
                .lock()
                .unwrap_or_else(|poison| poison.into_inner())
                .take();
            if input.is_none() {
                self.ivars().shared.fail(Failure::Interrupted);
            }
            let pointer = input.as_ref().map_or(std::ptr::null_mut(), |input| {
                Retained::as_ptr(input).cast_mut()
            });
            completion.call((pointer,)); // The completion retains the initial stream.
        }
        #[unsafe(method(URLSession:task:needNewBodyStreamFromOffset:completionHandler:))]
        unsafe fn offset_stream(
            &self,
            _session: &NSURLSession,
            _task: &NSURLSessionTask,
            _offset: i64,
            completion: &DynBlock<dyn Fn(*mut NSInputStream)>,
        ) {
            self.ivars().shared.fail(Failure::Interrupted);
            completion.call((std::ptr::null_mut(),)); // Never replay a POST.
        }
        #[unsafe(method(URLSession:task:didSendBodyData:totalBytesSent:totalBytesExpectedToSend:))]
        fn progress(
            &self,
            _session: &NSURLSession,
            _task: &NSURLSessionTask,
            _bytes: i64,
            sent: i64,
            _expected: i64,
        ) {
            let shared = &self.ivars().shared;
            let now = Instant::now();
            let report = {
                let mut state = shared.state();
                state.activity = now;
                let report = sent >= 0
                    && (state
                        .last_progress
                        .is_none_or(|last| now.duration_since(last) >= Duration::from_millis(100))
                        || sent as u64 >= shared.length);
                if report {
                    state.last_progress = Some(now);
                }
                report
            };
            if report
                && std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    (shared.progress)((sent as u64).min(shared.length), shared.length)
                }))
                .is_err()
            {
                shared.fail(Failure::Canceled); // Never unwind through Objective-C.
            }
        }
        #[unsafe(method(URLSession:task:didCompleteWithError:))]
        fn completed(
            &self,
            _session: &NSURLSession,
            _task: &NSURLSessionTask,
            error: Option<&NSError>,
        ) {
            let mut state = self.ivars().shared.state();
            if let Some(error) = error {
                state
                    .failure
                    .get_or_insert_with(|| Failure::from_native(error));
            }
            state.done = true;
        }
    }
    // SAFETY: Borrowed response/data is converted to owned Rust values within
    // each callback. Body size is checked before copying native bytes.
    unsafe impl NSURLSessionDataDelegate for UploadDelegate {
        #[unsafe(method(URLSession:dataTask:didReceiveResponse:completionHandler:))]
        unsafe fn response(
            &self,
            _session: &NSURLSession,
            _task: &NSURLSessionDataTask,
            response: &NSURLResponse,
            completion: &DynBlock<dyn Fn(NSURLSessionResponseDisposition)>,
        ) {
            let Some(http) = response.downcast_ref::<NSHTTPURLResponse>() else {
                self.ivars().shared.fail(Failure::Interrupted);
                completion.call((NSURLSessionResponseDisposition::Cancel,));
                return;
            };
            {
                let mut state = self.ivars().shared.state();
                state.status = http.statusCode() as u16;
                state.retry_after = http
                    .valueForHTTPHeaderField(&NSString::from_str("Retry-After"))
                    .map(|s| s.to_string());
                state.activity = Instant::now();
            }
            completion.call((NSURLSessionResponseDisposition::Allow,));
        }
        #[unsafe(method(URLSession:dataTask:didReceiveData:))]
        fn data(&self, _session: &NSURLSession, task: &NSURLSessionDataTask, data: &NSData) {
            let oversized = {
                let mut state = self.ivars().shared.state();
                state.activity = Instant::now();
                if data.len() > BUFFER_SIZE - state.body.len() {
                    state.failure.get_or_insert(Failure::Oversized);
                    true
                } else {
                    state.body.extend(data.to_vec());
                    false
                }
            };
            if oversized {
                task.cancel();
            }
        }
    }
);

struct OutputIvars {
    ready: Cell<bool>,
    failed: Cell<bool>,
}
define_class!(
    // SAFETY: This delegate and its stream stay on the worker's run loop.
    // Cell deliberately makes the type neither Send nor Sync.
    #[unsafe(super(NSObject))]
    #[ivars = OutputIvars]
    #[name = "GattoUploadOutput"]
    struct OutputDelegate;
    // SAFETY: NSObjectProtocol has no extra requirements.
    unsafe impl NSObjectProtocol for OutputDelegate {}
    // SAFETY: Foundation invokes this signature on the scheduled worker loop.
    unsafe impl NSStreamDelegate for OutputDelegate {
        #[unsafe(method(stream:handleEvent:))]
        fn event(&self, _stream: &NSStream, event: NSStreamEvent) {
            if event.contains(NSStreamEvent::HasSpaceAvailable) {
                self.ivars().ready.set(true);
            }
            if event.contains(NSStreamEvent::ErrorOccurred) {
                self.ivars().failed.set(true);
            }
        }
    }
);

/// Waits for session invalidation on success, error, or Rust unwinding. Owners
/// remain alive throughout invalidation; callbacks hold their own Arc as well.
struct Transport {
    output: Retained<NSOutputStream>,
    writer: Retained<OutputDelegate>,
    run_loop: Retained<NSRunLoop>,
    session: Retained<NSURLSession>,
    shared: Arc<Shared>,
}
impl Transport {
    fn write(&self, bytes: &[u8]) -> Option<usize> {
        let pointer = NonNull::new(bytes.as_ptr().cast_mut())?;
        // SAFETY: The slice covers exactly this length and stays borrowed for
        // the synchronous write. Foundation reads and does not retain it.
        let count = unsafe { self.output.write_maxLength(pointer, bytes.len()) };
        usize::try_from(count)
            .ok()
            .filter(|count| *count <= bytes.len())
    }
    fn pump(&self) {
        // SAFETY: Exported Foundation run-loop mode exists on supported OSes.
        let mode = unsafe { NSDefaultRunLoopMode };
        // Drain temporary Foundation objects each turn rather than retaining
        // hundreds of polling dates for the lifetime of a slow upload.
        let active = objc2::rc::autoreleasepool(|_| {
            self.run_loop
                .runMode_beforeDate(mode, &NSDate::dateWithTimeIntervalSinceNow(0.01))
        });
        if !active {
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}
impl Drop for Transport {
    fn drop(&mut self) {
        self.output.close();
        // SAFETY: Unschedule on the same worker loop before releasing the owned
        // delegate. No callback can refer to it after this synchronous cleanup.
        unsafe {
            self.output
                .removeFromRunLoop_forMode(&self.run_loop, NSDefaultRunLoopMode);
            self.output.setDelegate(None);
        }
        self.session.invalidateAndCancel();
        let mut state = self.shared.state();
        while !state.invalidated {
            state = self
                .shared
                .invalidation
                .wait(state)
                .unwrap_or_else(|poison| poison.into_inner());
        }
    }
}

fn send(
    endpoint: &str,
    token: &str,
    mime: &str,
    reader: &mut dyn io::Read,
    control: &UploadControl,
    shared: Arc<Shared>,
    timeouts: UploadTimeouts,
) -> Result<(), Failure> {
    let url = NSURL::URLWithString(&NSString::from_str(endpoint)).ok_or(Failure::Setup)?;
    let mut input = None;
    let mut output = None;
    NSStream::getBoundStreamsWithBufferSize_inputStream_outputStream(
        BUFFER_SIZE,
        Some(&mut input),
        Some(&mut output),
    );
    let input = input.ok_or(Failure::Setup)?;
    let output = output.ok_or(Failure::Setup)?;
    let delegate = UploadDelegate::alloc().set_ivars(UploadIvars {
        input: Mutex::new(InputOnce(Some(input))),
        shared: shared.clone(),
    });
    // SAFETY: NSObject initializer preserves the initialized owned ivars.
    let delegate: Retained<UploadDelegate> = unsafe { msg_send![super(delegate), init] };
    let configuration = NSURLSessionConfiguration::ephemeralSessionConfiguration();
    configuration.setURLCredentialStorage(None);
    configuration.setHTTPCookieStorage(None);
    configuration.setURLCache(None);
    configuration.setHTTPShouldSetCookies(false);
    configuration.setWaitsForConnectivity(false);
    configuration.setTimeoutIntervalForResource(timeouts.total.as_secs_f64());
    configuration.setTimeoutIntervalForRequest(timeouts.total.max(timeouts.idle).as_secs_f64());
    let queue = NSOperationQueue::new();
    queue.setMaxConcurrentOperationCount(1);
    // SAFETY: The delegate implements the required typed protocols and owns all
    // callback state. The session retains it through its invalidation callback.
    let session = unsafe {
        NSURLSession::sessionWithConfiguration_delegate_delegateQueue(
            &configuration,
            Some(ProtocolObject::from_ref(&*delegate)),
            Some(&queue),
        )
    };
    let request = NSMutableURLRequest::requestWithURL(&url);
    request.setHTTPMethod(&NSString::from_str("POST"));
    request.setHTTPShouldHandleCookies(false);
    request.setCachePolicy(NSURLRequestCachePolicy::ReloadIgnoringLocalCacheData);
    for (name, value) in [
        ("Authorization", format!("Bearer {token}")),
        ("Accept", "application/json".into()),
        ("Content-Type", mime.into()),
        ("Content-Length", shared.length.to_string()),
        (
            "User-Agent",
            concat!("gatto/", env!("CARGO_PKG_VERSION")).into(),
        ),
    ] {
        request.setValue_forHTTPHeaderField(
            Some(&NSString::from_str(&value)),
            &NSString::from_str(name),
        );
    }
    let task = session.uploadTaskWithStreamedRequest(&request);
    let writer = OutputDelegate::alloc().set_ivars(OutputIvars {
        ready: Cell::new(false),
        failed: Cell::new(false),
    });
    // SAFETY: NSObject initializer preserves the initialized worker-local ivars.
    let writer: Retained<OutputDelegate> = unsafe { msg_send![super(writer), init] };
    let run_loop = NSRunLoop::currentRunLoop();
    // SAFETY: Keep writer alive in Transport, schedule and unschedule the stream
    // on this same worker loop, and disconnect the delegate before dropping it.
    unsafe {
        output.setDelegate(Some(ProtocolObject::from_ref(&*writer)));
        output.scheduleInRunLoop_forMode(&run_loop, NSDefaultRunLoopMode);
    }
    let transport = Transport {
        output,
        writer,
        run_loop,
        session,
        shared: shared.clone(),
    };
    transport.output.open();
    task.resume();
    let mut buffer = [0; BUFFER_SIZE];
    let (mut pending, mut offset, mut produced, mut closed) = (0, 0, 0u64, false);
    let start = Instant::now();
    loop {
        let (done, mut failure, activity) = {
            let state = shared.state();
            (state.done, state.failure, state.activity)
        };
        if done {
            break;
        } // A received success wins a simultaneous cancellation.
        let now = Instant::now();
        if failure.is_none() && control.is_canceled() {
            failure = Some(Failure::Canceled);
        }
        if failure.is_none() && now.duration_since(start) >= timeouts.total {
            failure = Some(Failure::Timeout);
        }
        if failure.is_none() && now.saturating_duration_since(activity) >= timeouts.idle {
            failure = Some(Failure::Idle);
        }
        if failure.is_none() && transport.writer.ivars().failed.get() {
            failure = Some(Failure::Source);
        }
        if let Some(failure) = failure {
            {
                let mut state = shared.state();
                if !state.done {
                    state.failure.get_or_insert(failure);
                }
            }
            task.cancel();
        } else if !closed && transport.writer.ivars().ready.get() {
            if offset == pending {
                match read(reader, &mut buffer) {
                    Ok(0) if produced == shared.length => {
                        transport.output.close();
                        closed = true;
                    }
                    Ok(count) if count > 0 && count as u64 <= shared.length - produced => {
                        pending = count;
                        offset = 0;
                        produced += count as u64;
                    }
                    _ => {
                        shared.fail(Failure::Source);
                        task.cancel();
                    }
                }
            }
            if !closed && offset < pending {
                transport.writer.ivars().ready.set(false);
                match transport.write(&buffer[offset..pending]) {
                    Some(count) => offset += count,
                    None => transport.writer.ivars().failed.set(true),
                }
                if offset == pending && produced == shared.length {
                    if read(reader, &mut [0]).ok() != Some(0) {
                        shared.fail(Failure::Source);
                        task.cancel();
                    }
                    transport.output.close();
                    closed = true;
                }
            }
        }
        transport.pump();
    }
    drop(transport); // Includes invalidation before inspecting the final state.
    shared.state().failure.map_or(Ok(()), Err)
}
fn read(reader: &mut dyn io::Read, bytes: &mut [u8]) -> io::Result<usize> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        loop {
            match reader.read(bytes) {
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Ok(count) if count > bytes.len() => {
                    return Err(io::Error::other("Invalid attachment read length"));
                }
                result => return result,
            }
        }
    }))
    .unwrap_or_else(|_| Err(io::Error::other("Attachment reader failed")))
}

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
    let mut url =
        Url::parse(endpoint).map_err(|_| GithubError::Upload("Invalid upload endpoint".into()))?;
    url.query_pairs_mut()
        .append_pair("name", &attachment.name)
        .append_pair("content_type", &attachment.mime_type)
        .append_pair("repository_id", &repository_id.to_string());
    if [&attachment.mime_type, url.as_str(), token]
        .iter()
        .any(|value| value.contains('\0'))
    {
        return Err(GithubError::Upload("Invalid upload metadata".into()));
    }
    let mut reader = attachment
        .reader()
        .map_err(|error| GithubError::Upload(error.to_string()))?;
    progress(0, attachment.len());
    let shared = Arc::new(Shared {
        state: Mutex::new(State {
            status: 0,
            failure: None,
            body: Vec::new(),
            retry_after: None,
            done: false,
            invalidated: false,
            activity: Instant::now(),
            last_progress: None,
        }),
        invalidation: Condvar::new(),
        progress,
        length: attachment.len(),
    });
    objc2::rc::autoreleasepool(|_| {
        send(
            url.as_str(),
            token,
            &attachment.mime_type,
            &mut *reader,
            control,
            shared.clone(),
            timeouts,
        )
    })
    .map_err(Failure::error)?;
    let state = shared.state();
    let body = String::from_utf8_lossy(&state.body);
    if !(200..300).contains(&state.status) {
        return Err(GithubError::Upload(upload_http_error(
            state.status,
            &body,
            state.retry_after.as_deref(),
        )));
    }
    let uploaded: AttachmentResponse = serde_json::from_str(&body).map_err(|_|
        GithubError::Upload("GitHub returned an unreadable upload response. The attachment may have uploaded; retrying can create another copy.".into()))?;
    let url = uploaded.url.or(uploaded.href).ok_or_else(|| GithubError::Upload("GitHub returned no attachment URL. The attachment may have uploaded; retrying can create another copy.".into()))?;
    validate_attachment_url(&url)?;
    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_panics_and_invalid_lengths_become_errors_and_interrupted_reads_retry() {
        struct Source(u8);
        impl io::Read for Source {
            fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
                match self.0 {
                    0 => panic!("simulated reader panic"),
                    1 => Ok(bytes.len() + 1),
                    2 => {
                        self.0 = 3;
                        Err(io::ErrorKind::Interrupted.into())
                    }
                    _ => Ok(0),
                }
            }
        }
        assert!(read(&mut Source(0), &mut [0; 4]).is_err());
        assert!(read(&mut Source(1), &mut [0; 4]).is_err());
        assert_eq!(read(&mut Source(2), &mut [0; 4]).unwrap(), 0);
    }
}

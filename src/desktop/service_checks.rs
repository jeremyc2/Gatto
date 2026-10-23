//! Main-thread tests and disposable Finder receiver; excluded from the app.
use super::*;
use objc2::{DefinedClass, runtime::ProtocolObject};
use std::cell::RefCell;

struct ReceiverState {
    selections: RefCell<Vec<Vec<String>>>,
    output: Option<String>,
}
define_class!(
    // SAFETY: NSObject subclass, confined to AppKit's main thread.
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = ReceiverState]
    #[name = "GattoTestReceiver"]
    struct Receiver;
    // SAFETY: No extra requirements.
    unsafe impl NSObjectProtocol for Receiver {}
    // SAFETY: AppKit's typed delegate signature; no borrowed value is retained.
    unsafe impl NSApplicationDelegate for Receiver {
        #[unsafe(method(application:openURLs:))]
        fn opened(&self, _app: &NSApplication, urls: &NSArray<NSURL>) {
            let selection = urls
                .iter()
                .filter_map(|url| url.path().map(|path| path.to_string()))
                .collect();
            let mut selections = self.ivars().selections.borrow_mut();
            selections.push(selection);
            if let Some(output) = &self.ivars().output {
                let record = serde_json::json!({"selections": &*selections});
                let temporary = format!("{output}.tmp");
                if std::fs::write(&temporary, record.to_string()).is_ok() {
                    let _ = std::fs::rename(temporary, output);
                }
            }
        }
    }
);
fn receiver(mtm: MainThreadMarker, output: Option<String>) -> Retained<Receiver> {
    let receiver = Receiver::alloc(mtm).set_ivars(ReceiverState {
        selections: RefCell::new(Vec::new()),
        output,
    });
    // SAFETY: NSObject initializer preserves initialized test ivars.
    unsafe { msg_send![super(receiver), init] }
}

pub(crate) fn run_receiver() -> bool {
    use objc2_foundation::NSBundle;
    let output = NSBundle::mainBundle()
        .objectForInfoDictionaryKey(&NSString::from_str("GattoTestOutput"))
        .and_then(|value| value.downcast::<NSString>().ok())
        .map(|s| s.to_string());
    let Some(output) = output else {
        return false;
    };
    let mtm = MainThreadMarker::new().expect("AppKit main thread");
    let app = NSApplication::sharedApplication(mtm);
    let receiver = receiver(mtm, Some(output));
    app.setDelegate(Some(ProtocolObject::from_ref(&*receiver)));
    app.setActivationPolicy(objc2_app_kit::NSApplicationActivationPolicy::Accessory);
    install(mtm);
    app.run();
    true
}

pub(crate) fn verify() -> anyhow::Result<()> {
    let mtm = MainThreadMarker::new().expect("AppKit main thread");
    let app = NSApplication::sharedApplication(mtm);
    let receiver = receiver(mtm, None);
    app.setDelegate(Some(ProtocolObject::from_ref(&*receiver)));
    install(mtm);
    let board = NSPasteboard::pasteboardWithUniqueName();
    let paths = ["/tmp/猫 #?%\n$(touch nope).png", "/tmp/video clip.mov"];
    let urls = paths.map(|path| NSURL::fileURLWithPath(&NSString::from_str(path)));
    let write = |urls: &[&NSURL]| {
        board.clearContents();
        assert!(
            board.writeObjects(&NSArray::from_slice(
                &urls
                    .iter()
                    .map(|url| ProtocolObject::from_ref(*url))
                    .collect::<Vec<_>>()
            ))
        );
    };
    let invoke = || -> Result<(), String> {
        let provider = PROVIDER.with(|cell| cell.get().unwrap().clone());
        let mut error: *mut NSString = std::ptr::null_mut();
        // Exercise the registered selector's autoreleasing error slot too.
        // SAFETY: The registered Services selector has this exact signature.
        let _: () = unsafe {
            msg_send![&*provider, sendToGatto: &*board, userData: Option::<&NSString>::None, error: &mut error]
        };
        // SAFETY: The callback returns a live autoreleased NSString or null;
        // copy it before this invocation's autorelease pool can drain.
        if error.is_null() {
            Ok(())
        } else {
            Err(unsafe { &*error }.to_string())
        }
    };
    write(&[&urls[0], &urls[1]]);
    invoke().unwrap();
    assert_eq!(receiver.ivars().selections.borrow().last().unwrap(), &paths);
    let fixture = tempfile::NamedTempFile::new()?;
    let file = NSURL::fileURLWithPath(&NSString::from_str(fixture.path().to_str().unwrap()));
    let reference = file.fileReferenceURL().unwrap();
    assert!(reference.isFileReferenceURL());
    write(&[&reference, &urls[1]]);
    invoke().unwrap();
    assert_eq!(
        receiver.ivars().selections.borrow().last().unwrap(),
        &[
            reference.filePathURL().unwrap().path().unwrap().to_string(),
            paths[1].into()
        ]
    );
    let stale = NSURL::URLWithString(&reference.absoluteString().unwrap()).unwrap();
    drop(fixture);
    write(&[&stale]);
    let count = receiver.ivars().selections.borrow().len();
    assert!(invoke().is_err());
    assert_eq!(receiver.ivars().selections.borrow().len(), count);
    board.clearContents();
    let strings = paths.map(NSString::from_str);
    // SAFETY: Legacy Finder format is a property-list array of path strings.
    #[allow(deprecated)]
    unsafe {
        assert!(board.setPropertyList_forType(
            &NSArray::from_retained_slice(&strings),
            objc2_app_kit::NSFilenamesPboardType
        ));
    }
    invoke().unwrap();
    assert_eq!(receiver.ivars().selections.borrow().last().unwrap(), &paths);
    let web = NSURL::URLWithString(&NSString::from_str("https://example.com/a.png")).unwrap();
    write(&[&web]);
    let count = receiver.ivars().selections.borrow().len();
    assert!(invoke().is_err());
    assert_eq!(receiver.ivars().selections.borrow().len(), count);
    // The binding omits the deprecated releaseGlobally selector.
    // SAFETY: This no-argument selector releases this unique test pasteboard.
    let _: () = unsafe { msg_send![&*board, releaseGlobally] };
    // SAFETY: Tests disconnect the process-lifetime provider before exiting.
    unsafe {
        app.setServicesProvider(None);
    }
    app.setDelegate(None);
    Ok(())
}

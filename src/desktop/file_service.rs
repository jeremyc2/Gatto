//! The Services selector has one native out-parameter; file selection and error
//! handling otherwise use owned Rust/Objective-C values, never callback contexts.
use objc2::{
    ClassType, MainThreadMarker, MainThreadOnly, define_class, msg_send, rc::Retained, sel,
};
use objc2_app_kit::{
    NSApplication, NSApplicationDelegate, NSPasteboard, NSPasteboardURLReadingFileURLsOnlyKey,
};
use objc2_foundation::{
    NSArray, NSDictionary, NSNumber, NSObject, NSObjectProtocol, NSString, NSURL,
};
use std::cell::OnceCell;

define_class!(
    // SAFETY: NSObject has no subclassing requirements. There are no Rust ivars
    // or borrowed callback pointers, and service dispatch is main-thread-only.
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[name = "GattoFileService"]
    struct FileService;

    // SAFETY: NSObjectProtocol has no extra requirements.
    unsafe impl NSObjectProtocol for FileService {}

    impl FileService {
        // SAFETY: This is the AppKit Services signature. AppKit supplies a live
        // pasteboard, optional user data, and a nullable autoreleasing error slot.
        #[unsafe(method(sendToGatto:userData:error:))]
        fn send(&self, pasteboard: &NSPasteboard, _user_data: Option<&NSString>, error: *mut *mut NSString) {
            if let Err(message) = deliver(pasteboard, self.mtm())
                && !error.is_null() {
                    // SAFETY: Services owns this slot for the invocation. Transfer
                    // an autoreleased string, not a pointer to a Rust local owner.
                    unsafe { *error = Retained::autorelease_ptr(NSString::from_str(message)); }
            }
        }
    }
);

thread_local! {
    static PROVIDER: OnceCell<Retained<FileService>> = const { OnceCell::new() };
}

pub(super) fn install(main_thread: MainThreadMarker) {
    PROVIDER.with(|provider| {
        let provider = provider.get_or_init(|| {
            // SAFETY: NSObject's initializer has this signature. The returned
            // object is retained in this main-thread cell for the process lifetime.
            unsafe { msg_send![FileService::alloc(main_thread), init] }
        });
        // SAFETY: FileService implements the selector declared by Info.plist;
        // the main-thread cell keeps it alive independently of AppKit retention.
        unsafe {
            NSApplication::sharedApplication(main_thread).setServicesProvider(Some(provider));
        }
    });
}

fn selection(pasteboard: &NSPasteboard) -> Vec<Retained<NSURL>> {
    // SAFETY: URL is an NSPasteboardReading class. The documented option takes
    // an NSNumber boolean. Validate returned dynamic objects before using them.
    let objects = unsafe {
        pasteboard.readObjectsForClasses_options(
            &NSArray::from_slice(&[NSURL::class()]),
            Some(&NSDictionary::from_slices(
                &[NSPasteboardURLReadingFileURLsOnlyKey],
                &[&*NSNumber::new_bool(true) as &objc2::runtime::AnyObject],
            )),
        )
    };
    let urls = objects
        .map(|objects| {
            objects
                .iter()
                .filter_map(|object| object.downcast::<NSURL>().ok())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if !urls.is_empty() {
        return urls;
    }
    // SAFETY: This exported constant is available on the minimum supported SDK.
    // Finder still supplies this legacy format; retain compatibility.
    #[allow(deprecated)]
    let paths = pasteboard.propertyListForType(unsafe { objc2_app_kit::NSFilenamesPboardType });
    let Some(paths) = paths.and_then(|paths| paths.downcast::<NSArray>().ok()) else {
        return Vec::new();
    };
    paths
        .iter()
        .filter_map(|path| path.downcast::<NSString>().ok())
        .filter(|path| std::path::Path::new(&path.to_string()).is_absolute())
        .map(|path| NSURL::fileURLWithPath(&path))
        .collect()
}

fn deliver(pasteboard: &NSPasteboard, main_thread: MainThreadMarker) -> Result<(), &'static str> {
    let urls = selection(pasteboard);
    let app = NSApplication::sharedApplication(main_thread);
    let delegate = app
        .delegate()
        .filter(|delegate| delegate.respondsToSelector(sel!(application:openURLs:)));
    let Some(delegate) = delegate.filter(|_| !urls.is_empty()) else {
        return Err("Select local files in Finder and try Send to Gatto again.");
    };
    let paths = urls.iter().map(|url| url.filePathURL().ok_or("A selected file could not be located. Select it again in Finder and retry Send to Gatto.")).collect::<Result<Vec<_>, _>>()?;
    // The delegate only queues paths; do not borrow GPUI or import synchronously.
    delegate.application_openURLs(&app, &NSArray::from_retained_slice(&paths));
    Ok(())
}

#[cfg(feature = "walkthrough-screenshots")]
#[allow(dead_code)] // Compiled with test-support; called by platform-tests only.
#[path = "service_checks.rs"]
pub(super) mod checks;

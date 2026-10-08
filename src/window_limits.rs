use gpui_kit::Window;

/// GPUI exposes the minimum size directly. macOS maximum size support is set
/// through the underlying NSWindow until GPUI exposes the matching option.
#[cfg(target_os = "macos")]
pub fn set_maximum_content_size(window: &Window, width: f64, height: f64) {
    use objc2_app_kit::NSView;
    use objc2_foundation::NSSize;
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    let Ok(handle) = HasWindowHandle::window_handle(window) else {
        return;
    };
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return;
    };

    // GPUI's AppKit handle is a valid NSView for as long as the GPUI window is alive.
    let view = unsafe { &*(handle.ns_view.cast::<NSView>().as_ptr()) };
    if let Some(native_window) = view.window() {
        native_window.setContentMaxSize(NSSize::new(width, height));
    }
}

#[cfg(not(target_os = "macos"))]
pub fn set_maximum_content_size(_: &Window, _: f64, _: f64) {}

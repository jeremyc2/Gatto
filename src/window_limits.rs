use gpui_kit::Window;

/// Hides one native window without hiding the rest of the application.
///
/// GPUI currently exposes application-wide hiding, but not a window-scoped
/// equivalent. The main Gatto window uses this when its close button is
/// pressed so utility windows can keep their own independent lifecycle.
#[cfg(target_os = "macos")]
pub fn hide(window: &Window) {
    use objc2_app_kit::NSView;
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
        native_window.orderOut(None);
    }
}

#[cfg(not(target_os = "macos"))]
pub fn hide(_: &Window) {}

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

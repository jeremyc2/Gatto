use gpui_kit::{App, Window};

/// Counts minimized and occluded windows as open. GPUI visibility tracks
/// occlusion and can
/// lag behind AppKit, so it is not the right signal for Dock reopening.
#[cfg(target_os = "macos")]
pub fn is_presented(window: &Window) -> bool {
    use objc2_app_kit::NSView;
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    let Ok(handle) = HasWindowHandle::window_handle(window) else {
        return false;
    };
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return false;
    };
    // The native content view remains valid while the GPUI window is alive.
    let view = unsafe { &*(handle.ns_view.cast::<NSView>().as_ptr()) };
    view.window()
        .is_some_and(|window| window.isVisible() || window.isMiniaturized())
}

#[cfg(not(target_os = "macos"))]
pub fn is_presented(window: &Window) -> bool {
    window.is_visible()
}

/// Runs the native close request outside GPUI's current update. AppKit calls
/// back into GPUI synchronously, so invoking performClose during an update
/// would re-borrow the application and panic.
#[cfg(target_os = "macos")]
pub fn request_close(window: &Window, cx: &App) {
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
        cx.foreground_executor()
            .spawn(async move {
                native_window.performClose(None);
            })
            .detach();
    }
}

#[cfg(not(target_os = "macos"))]
pub fn request_close(window: &mut Window, _: &App) {
    window.remove_window();
}

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

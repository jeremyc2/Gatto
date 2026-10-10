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

/// Asks AppKit to close the native window so GPUI's existing close handler can
/// apply the same hide-and-clean-up behavior as the title-bar close button.
#[cfg(target_os = "macos")]
pub fn request_close(window: &Window) {
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
        native_window.performClose(None);
    }
}

#[cfg(not(target_os = "macos"))]
pub fn request_close(_: &Window) {}

/// Intercepts a native close request and keeps the GPUI window alive.
///
/// Retaining utility windows avoids AppKit/GPUI teardown differences between
/// `cargo run` and a signed app bundle. Opening the utility again simply
/// activates the existing hidden window.
pub fn hide_instead_of_close(window: &Window) -> bool {
    hide(window);
    false
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

#[cfg(test)]
mod tests {
    use gpui_kit::{
        AppContext as _, Context, IntoElement, Render, TestAppContext, VisualTestContext, Window,
        div,
    };

    use super::hide_instead_of_close;

    struct UtilityWindow;

    impl Render for UtilityWindow {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
        }
    }

    #[gpui_kit::test]
    fn utility_window_close_requests_are_intercepted(cx: &mut TestAppContext) {
        let handle = cx.add_window(|window, cx| {
            window.on_window_should_close(cx, |window, _| hide_instead_of_close(window));
            UtilityWindow
        });
        let mut window = VisualTestContext::from_window(handle.into(), cx);

        assert!(!window.simulate_close());
        assert!(cx.update_window(handle.into(), |_, _, _| ()).is_ok());
    }
}

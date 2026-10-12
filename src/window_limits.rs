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

/// Avoids GPUI/AppKit window teardown, which can abort Touch Bar Macs with
/// `_NSTouchBarFinderObservation` removing an unregistered `nextResponder`
/// observer (https://github.com/longbridge/gpui-kit/issues/3192).
/// Callers retain one handle per window type and reset transient state on close.
#[cfg(target_os = "macos")]
pub fn close(window: &Window, cx: &mut App) -> bool {
    use objc2_app_kit::NSView;
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    if let Ok(handle) = HasWindowHandle::window_handle(window)
        && let RawWindowHandle::AppKit(handle) = handle.as_raw()
    {
        // The view and its window are valid throughout this close callback.
        let view = unsafe { &*(handle.ns_view.cast::<NSView>().as_ptr()) };
        if let Some(native_window) = view.window() {
            native_window.orderOut(None);
        }
    }
    // Query other windows after the close callback releases GPUI's current
    // window borrow. Hidden retained windows must not keep the Dock visible.
    cx.defer(|cx| {
        let any_presented = cx.windows().into_iter().any(|handle| {
            handle
                .update(cx, |_, window, _| is_presented(window))
                .unwrap_or(false)
        });
        if !any_presented {
            set_dock_visible(false);
        }
    });
    false
}

#[cfg(not(target_os = "macos"))]
pub fn close(_: &Window, _: &mut App) -> bool {
    true
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

/// Shows the Dock icon while the window is open; the menu bar item always stays.
#[cfg(target_os = "macos")]
pub fn set_dock_visible(visible: bool) {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};

    if let Some(main_thread) = MainThreadMarker::new() {
        let policy = if visible {
            NSApplicationActivationPolicy::Regular
        } else {
            NSApplicationActivationPolicy::Accessory
        };
        NSApplication::sharedApplication(main_thread).setActivationPolicy(policy);
    }
    // Switching back from Accessory resets the Dock tile, so the icon is set again.
    if visible {
        set_dock_icon();
    }
}

/// Sets the Dock icon explicitly when running outside an app bundle, which has no icon of its own.
#[cfg(target_os = "macos")]
fn set_dock_icon() {
    use objc2::{AnyThread as _, MainThreadMarker};
    use objc2_app_kit::{NSApplication, NSImage};
    use objc2_foundation::NSData;

    static ICON: &[u8] = include_bytes!("../packaging/AppIcon.icns");

    let bundled = std::env::current_exe()
        .is_ok_and(|path| path.to_string_lossy().contains(".app/Contents/MacOS/"));
    if bundled {
        return;
    }

    if let Some(main_thread) = MainThreadMarker::new() {
        let data = NSData::with_bytes(ICON);
        if let Some(image) = NSImage::initWithData(NSImage::alloc(), &data) {
            unsafe {
                NSApplication::sharedApplication(main_thread).setApplicationIconImage(Some(&image))
            };
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn set_dock_visible(_: bool) {}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use gpui_kit::{
        AppContext as _, Context, IntoElement, Render, TestAppContext, VisualTestContext, Window,
        div,
    };

    struct UtilityWindow;

    impl Render for UtilityWindow {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
        }
    }

    // Tests GPUI retention and independent close callbacks; the native Touch
    // Bar exception still requires testing a bundled app on affected hardware.
    #[gpui_kit::test]
    fn repeated_closes_retain_the_window_and_leave_its_sibling_alive(cx: &mut TestAppContext) {
        let sibling = cx.add_window(|_, _| UtilityWindow);
        let handle = cx.add_window(|window, cx| {
            window.on_window_should_close(cx, |window, cx| super::close(window, cx));
            UtilityWindow
        });
        let mut window = VisualTestContext::from_window(handle.into(), cx);
        for _ in 0..10 {
            assert!(!window.simulate_close());
            window.update(|_, _| ());
        }
        assert!(cx.update_window(sibling.into(), |_, _, _| ()).is_ok());
        cx.update(|cx| assert_eq!(cx.windows().len(), 2));
    }
}

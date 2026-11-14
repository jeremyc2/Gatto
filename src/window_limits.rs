use gpui_kit::{App, Window};

/// This is the single conversion from GPUI's borrowed native handle. Retaining
/// the window lets deferred main-thread work outlive the borrowed content view.
#[cfg(target_os = "macos")]
fn native_window(window: &Window) -> Option<objc2::rc::Retained<objc2_app_kit::NSWindow>> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    let _main_thread = objc2::MainThreadMarker::new()?;
    let handle = HasWindowHandle::window_handle(window).ok()?;
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return None;
    };
    // SAFETY: HasWindowHandle guarantees a live NSView for this Window borrow.
    // AppKit access is on the checked main thread, and window() retains its result.
    let view = unsafe { &*handle.ns_view.cast::<objc2_app_kit::NSView>().as_ptr() };
    view.window()
}

/// Installs native window styling once and restores it after fullscreen transitions.
#[cfg(target_os = "macos")]
pub fn style_window<V: 'static>(
    view: &gpui_kit::Entity<V>,
    window: &mut Window,
    full_size_content: bool,
    cx: &mut App,
) {
    use gpui_kit::component::ActiveTheme as _;
    use objc2::MainThreadOnly as _;
    use objc2_app_kit::{NSBox, NSBoxType, NSTitlePosition, NSTitlebarSeparatorStyle};

    let Some(main_thread) = objc2::MainThreadMarker::new() else {
        return;
    };
    let Some(native_window) = native_window(window) else {
        return;
    };
    let Some(content) = native_window.contentView() else {
        return;
    };
    let border = (!full_size_content).then(|| {
        let border = NSBox::initWithFrame(NSBox::alloc(main_thread), content.bounds());
        border.setBoxType(NSBoxType::Custom);
        border.setTitlePosition(NSTitlePosition::NoTitle);
        border.setBorderWidth(0.);
        border.setCornerRadius(0.);
        content.addSubview(&border);
        border
    });

    let update = move |window: &Window, cx: &App| {
        if let Some(border) = &border {
            border.setHidden(window.is_fullscreen());
        }
        if window.is_fullscreen() {
            return;
        }
        native_window.setBackgroundColor(Some(&native_color(cx.theme().sidebar)));
        native_window.setTitlebarAppearsTransparent(true);
        let Some(border) = &border else {
            native_window.setTitlebarSeparatorStyle(NSTitlebarSeparatorStyle::None);
            return;
        };
        border.setFillColor(&native_color(cx.theme().sidebar_border));
        let bounds = content.bounds();
        let y = if content.isFlipped() {
            bounds.origin.y
        } else {
            bounds.origin.y + bounds.size.height - 1.
        };
        border.setFrame(objc2_foundation::NSRect::new(
            objc2_foundation::NSPoint::new(bounds.origin.x, y),
            objc2_foundation::NSSize::new(bounds.size.width, 1.),
        ));
    };
    update(window, cx);
    view.update(cx, |_, cx| {
        cx.observe_window_bounds(window, move |_, window, cx| update(window, cx))
            .detach();
    });
}

#[cfg(target_os = "macos")]
fn native_color(color: gpui_kit::Hsla) -> objc2::rc::Retained<objc2_app_kit::NSColor> {
    let color = color.to_rgb();
    objc2_app_kit::NSColor::colorWithSRGBRed_green_blue_alpha(
        color.r.into(),
        color.g.into(),
        color.b.into(),
        color.a.into(),
    )
}

#[cfg(not(target_os = "macos"))]
pub fn style_window<V: 'static>(_: &gpui_kit::Entity<V>, _: &mut Window, _: bool, _: &mut App) {}

/// Counts minimized and occluded windows as open. GPUI visibility tracks
/// occlusion and can
/// lag behind AppKit, so it is not the right signal for Dock reopening.
#[cfg(target_os = "macos")]
pub fn is_presented(window: &Window) -> bool {
    native_window(window).is_some_and(|window| window.isVisible() || window.isMiniaturized())
}

#[cfg(not(target_os = "macos"))]
pub fn is_presented(window: &Window) -> bool {
    window.is_visible()
}

/// Avoids GPUI/AppKit window teardown, which can abort Touch Bar Macs with
/// `_NSTouchBarFinderObservation` removing an unregistered `nextResponder`
/// observer (https://github.com/longbridge/gpui-kit/issues/3192).
/// Decision and removal criteria: docs/adr/0001-retain-macos-windows-to-avoid-touch-bar-crashes.md.
/// Callers retain one handle per window type and reset transient state on close.
#[cfg(target_os = "macos")]
pub fn close(window: &Window) -> bool {
    if let Some(native_window) = native_window(window) {
        native_window.orderOut(None);
    }
    false
}

#[cfg(not(target_os = "macos"))]
pub fn close(_: &Window) -> bool {
    true
}

/// Runs the native close request outside GPUI's current update. AppKit calls
/// back into GPUI synchronously, so invoking performClose during an update
/// would re-borrow the application and panic.
#[cfg(target_os = "macos")]
pub fn request_close(window: &Window, cx: &App) {
    if let Some(native_window) = native_window(window) {
        cx.foreground_executor()
            .spawn(async move {
                native_window.performClose(None);
            })
            .detach();
    }
}

#[cfg(windows)]
fn native_window(window: &Window) -> Option<windows::Win32::Foundation::HWND> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows::Win32::Foundation::HWND;

    let RawWindowHandle::Win32(handle) = HasWindowHandle::window_handle(window).ok()?.as_raw()
    else {
        return None;
    };
    Some(HWND(handle.hwnd.get() as _))
}

#[cfg(windows)]
pub fn request_close(window: &Window, _: &App) {
    use windows::Win32::{
        Foundation::{LPARAM, WPARAM},
        UI::WindowsAndMessaging::{PostMessageW, WM_CLOSE},
    };

    if let Some(hwnd) = native_window(window) {
        // SAFETY: The window owns the HWND. WM_CLOSE carries no pointers and
        // lets GPUI's normal close callback validate and flush pending work.
        let _ = unsafe { PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)) };
    }
}

/// Opens the native system menu for the uploader's custom caption.
#[cfg(windows)]
pub fn show_system_menu(window: &Window, position: gpui_kit::Point<gpui_kit::Pixels>, cx: &App) {
    use windows::Win32::{
        Foundation::{LPARAM, POINT, WPARAM},
        Graphics::Gdi::ClientToScreen,
        UI::WindowsAndMessaging::{
            EnableMenuItem, GetSystemMenu, IsZoomed, MF_BYCOMMAND, MF_ENABLED, MF_GRAYED,
            PostMessageW, SC_MAXIMIZE, SC_MOVE, SC_RESTORE, SC_SIZE, TPM_RETURNCMD,
            TPM_RIGHTBUTTON, TrackPopupMenu, WM_SYSCOMMAND,
        },
    };

    let Some(hwnd) = native_window(window) else {
        return;
    };
    let scale = window.scale_factor();
    let mut anchor = POINT {
        x: (position.x.as_f32() * scale).round() as i32,
        y: (position.y.as_f32() * scale).round() as i32,
    };
    // SAFETY: Window supplies a live HWND and the output point is stack-owned.
    if !unsafe { ClientToScreen(hwnd, &mut anchor) }.as_bool() {
        return;
    }

    // The native menu runs a modal message loop. Let the GPUI event update
    // finish before that loop starts dispatching messages back to the app.
    cx.foreground_executor()
        .spawn(async move {
            // SAFETY: The foreground executor runs on the owning UI thread.
            // GetSystemMenu can fail if the window has closed, and Windows owns
            // the returned menu for the lifetime of the window.
            unsafe {
                let menu = GetSystemMenu(hwnd, false);
                if menu.is_invalid() {
                    return;
                }
                let maximized = IsZoomed(hwnd).as_bool();
                for (command, enabled) in [
                    (SC_RESTORE, maximized),
                    (SC_MOVE, !maximized),
                    (SC_SIZE, !maximized),
                    (SC_MAXIMIZE, !maximized),
                ] {
                    let state = if enabled { MF_ENABLED } else { MF_GRAYED };
                    let _ = EnableMenuItem(menu, command, MF_BYCOMMAND | state);
                }
                let command = TrackPopupMenu(
                    menu,
                    TPM_RETURNCMD | TPM_RIGHTBUTTON,
                    anchor.x,
                    anchor.y,
                    None,
                    hwnd,
                    None,
                );
                if command.0 != 0 {
                    let _ =
                        PostMessageW(Some(hwnd), WM_SYSCOMMAND, WPARAM(command.0 as _), LPARAM(0));
                }
            }
        })
        .detach();
}

#[cfg(not(any(target_os = "macos", windows)))]
pub fn request_close(window: &mut Window, _: &App) {
    window.remove_window();
}

/// GPUI exposes the minimum size directly. macOS maximum size support is set
/// through the underlying NSWindow until GPUI exposes the matching option.
#[cfg(target_os = "macos")]
pub fn set_maximum_content_size(window: &Window, width: f64, height: f64) {
    if let Some(native_window) = native_window(window) {
        native_window.setContentMaxSize(objc2_foundation::NSSize::new(width, height));
    }
}

#[cfg(not(target_os = "macos"))]
pub fn set_maximum_content_size(_: &Window, _: f64, _: f64) {}

#[cfg(target_os = "macos")]
pub fn set_minimum_content_size(window: &Window, width: f64, height: f64) {
    if let Some(native_window) = native_window(window) {
        native_window.setContentMinSize(objc2_foundation::NSSize::new(width, height));
    }
}

#[cfg(not(target_os = "macos"))]
pub fn set_minimum_content_size(_: &Window, _: f64, _: f64) {}

/// Login launches stay out of the Dock until a window is opened. The Dock tile
/// then stays available for reopening the app and choosing recent projects.
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
            window.on_window_should_close(cx, |window, _| super::close(window));
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

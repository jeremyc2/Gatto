mod app;
mod custom_url;
mod diagnostics;
mod github;
mod image_preview;
mod log_viewer;
mod menu_bar;
mod model;
mod settings;
mod theme;
mod window_limits;

use std::{cell::RefCell, rc::Rc};

use gpui_kit::{
    AnyWindowHandle, App, AppContext as _, Entity, Focusable as _, WindowBounds, WindowOptions, px,
    size,
};

use crate::{
    app::UploaderApp,
    custom_url::CustomUrlAction,
    diagnostics::Diagnostics,
    menu_bar::{MenuAction, MenuBar, MenuBarController},
};

fn main() {
    let application = gpui_kit::application()
        .with_assets(gpui_kit::assets::AllAssets)
        .with_quit_mode(gpui_kit::QuitMode::Explicit);
    let reopen_window = Rc::new(RefCell::new(None::<AnyWindowHandle>));
    let reopen_window_callback = reopen_window.clone();
    application.on_reopen(move |cx| {
        let Some(window_handle) = *reopen_window_callback.borrow() else {
            return;
        };
        set_dock_visible(true);
        cx.activate(true);
        let _ = window_handle.update(cx, |_, window, _| {
            window.activate_window();
        });
    });
    let (custom_url_sender, custom_url_events) = async_channel::unbounded();
    application.on_open_urls(move |urls| {
        for url in urls {
            let _ = custom_url_sender.try_send(url);
        }
    });

    application.run(move |cx| {
        cx.set_app_identity("com.jeremy-chandler.gatto", "Gatto");
        gpui_kit::init(cx);
        theme::force_dark_appearance();
        theme::apply_dark_theme(cx);
        app::init_keybindings(cx);
        image_preview::init_keybindings(cx);
        set_dock_visible(true);
        let diagnostics = Diagnostics::new();
        diagnostics.info("Application started.");
        let menu_bar_controller = MenuBarController::default();

        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(620.), px(780.)), cx)),
            window_min_size: Some(size(px(520.), px(600.))),
            ..Default::default()
        };

        let (window_handle, view) = gpui_kit::open_window(window_options, cx, |window, cx| {
            window.set_window_title("Gatto");
            window_limits::set_maximum_content_size(window, 920., 1040.);

            let diagnostics = diagnostics.clone();
            let menu_bar_controller = menu_bar_controller.clone();
            let view = cx.new(|cx| UploaderApp::new(window, diagnostics, menu_bar_controller, cx));
            let close_view = view.clone();
            window.on_window_should_close(cx, move |window, cx| {
                close_view.update(cx, |this, cx| {
                    this.clear_staged_images(
                        "Closed the main window; discarded the staged image.",
                        cx,
                    );
                });
                window_limits::hide(window);
                set_dock_visible(false);
                false
            });
            let focus = view.read(cx).focus_handle(cx);
            window.focus(&focus, cx);
            view
        })
        .expect("Could not open the Gatto window");
        *reopen_window.borrow_mut() = Some(window_handle);

        let custom_url_window = window_handle;
        let custom_url_view = view.clone();
        cx.spawn(async move |cx| {
            while let Ok(url) = custom_url_events.recv().await {
                cx.update(|cx| match CustomUrlAction::parse(&url) {
                    Ok(action) => {
                        perform_action(action.into(), custom_url_window, &custom_url_view, cx)
                    }
                    Err(error) => {
                        show_main_window(custom_url_window, cx);
                        let _ = custom_url_window.update(cx, |_, window, cx| {
                            custom_url_view.update(cx, |this, cx| {
                                this.report_custom_url_error(error.to_string(), window, cx);
                            });
                        });
                    }
                });
            }
        })
        .detach();

        let paste_actions_visible = view.read(cx).has_pinned_repository();
        let (menu_bar, menu_events) = match MenuBar::install(paste_actions_visible) {
            Ok(menu_bar) => menu_bar,
            Err(error) => {
                let _ = window_handle.update(cx, |_, window, cx| {
                    view.update(cx, |this, cx| {
                        this.report_menu_bar_error(
                            format!("Menu bar setup failed: {error}"),
                            window,
                            cx,
                        );
                    });
                });
                // The window remains usable if macOS refuses to create a status item.
                return;
            }
        };
        menu_bar_controller.attach(menu_bar);

        cx.spawn(async move |cx| {
            // Keep the native status item alive for the entire application lifetime.
            while let Ok(event) = menu_events.recv().await {
                let Some(action) = menu_bar_controller.action_for(&event) else {
                    continue;
                };
                if action == MenuAction::Quit {
                    cx.update(|cx| cx.quit());
                    break;
                }
                cx.update(|cx| perform_action(action, window_handle, &view, cx));
            }
        })
        .detach();
    });
}

impl From<CustomUrlAction> for MenuAction {
    fn from(action: CustomUrlAction) -> Self {
        match action {
            CustomUrlAction::Open => Self::Open,
            CustomUrlAction::Preview => Self::PreviewFromClipboard,
            CustomUrlAction::QuickCopy => Self::QuickCopy,
            CustomUrlAction::Settings => Self::Settings,
        }
    }
}

fn perform_action(
    action: MenuAction,
    window_handle: AnyWindowHandle,
    view: &Entity<UploaderApp>,
    cx: &mut App,
) {
    match action {
        MenuAction::Open => show_main_window(window_handle, cx),
        MenuAction::PreviewFromClipboard => {
            show_main_window(window_handle, cx);
            let _ = window_handle.update(cx, |_, window, cx| {
                window.dispatch_action(Box::new(app::PreviewFromClipboard), cx);
            });
        }
        MenuAction::QuickCopy => {
            set_dock_visible(false);
            let _ = window_handle.update(cx, |_, window, cx| {
                window_limits::hide(window);
                window.dispatch_action(Box::new(app::QuickCopy), cx);
            });
        }
        MenuAction::Settings => {
            show_main_window(window_handle, cx);
            view.update(cx, |this, cx| this.show_settings(cx));
        }
        MenuAction::Quit => cx.quit(),
    }
}

fn show_main_window(window_handle: AnyWindowHandle, cx: &mut App) {
    set_dock_visible(true);
    cx.activate(true);
    let _ = window_handle.update(cx, |_, window, _| {
        window.activate_window();
    });
}

/// Shows the Dock icon while the window is open; the menu bar item always stays.
#[cfg(target_os = "macos")]
fn set_dock_visible(visible: bool) {
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
fn set_dock_visible(_: bool) {}

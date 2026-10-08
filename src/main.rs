mod app;
mod diagnostics;
mod github;
mod log_viewer;
mod menu_bar;
mod model;
mod settings;
mod window_limits;

use gpui_kit::component::{Theme, ThemeMode};
use gpui_kit::{AppContext as _, Focusable as _, Hsla, WindowBounds, WindowOptions, px, rgb, size};

use crate::{
    app::UploaderApp,
    diagnostics::Diagnostics,
    menu_bar::{MenuAction, MenuBar, MenuBarController},
};

fn main() {
    let application = gpui_kit::application().with_assets(gpui_kit::assets::Assets);

    application.run(|cx| {
        gpui_kit::init(cx);
        apply_dark_theme(cx);
        app::init_keybindings(cx);
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
            window.on_window_should_close(cx, |_, cx| {
                cx.hide();
                set_dock_visible(false);
                false
            });

            let diagnostics = diagnostics.clone();
            let menu_bar_controller = menu_bar_controller.clone();
            let view = cx.new(|cx| UploaderApp::new(window, diagnostics, menu_bar_controller, cx));
            let focus = view.read(cx).focus_handle(cx);
            window.focus(&focus, cx);
            view
        })
        .expect("Could not open the Gatto window");

        let quick_paste_visible = view.read(cx).has_pinned_repository();
        let (menu_bar, menu_events) = match MenuBar::install(quick_paste_visible) {
            Ok(menu_bar) => menu_bar,
            Err(error) => {
                view.update(cx, |this, cx| {
                    this.report_menu_bar_error(format!("Menu bar setup failed: {error}"), cx);
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

                match action {
                    MenuAction::Open => {
                        cx.update(|cx| {
                            set_dock_visible(true);
                            cx.activate(true);
                            let _ = window_handle.update(cx, |_, window, _| {
                                window.activate_window();
                            });
                        });
                    }
                    MenuAction::QuickPaste => {
                        cx.update(|cx| {
                            set_dock_visible(true);
                            cx.activate(true);
                            let _ = window_handle.update(cx, |_, window, cx| {
                                window.activate_window();
                                window.dispatch_action(Box::new(app::QuickPaste), cx);
                            });
                        });
                    }
                    MenuAction::Settings => {
                        let view = view.clone();
                        cx.update(|cx| {
                            set_dock_visible(true);
                            cx.activate(true);
                            view.update(cx, |this, cx| this.show_settings(cx));
                            let _ = window_handle.update(cx, |_, window, _| {
                                window.activate_window();
                            });
                        });
                    }
                    MenuAction::Quit => {
                        cx.update(|cx| cx.quit());
                        break;
                    }
                }
            }
        })
        .detach();
    });
}

fn color(hex: u32) -> Hsla {
    rgb(hex).into()
}

/// The app is dark-only; there is no light theme to switch to.
fn apply_dark_theme(cx: &mut gpui_kit::App) {
    force_dark_appearance();
    Theme::change(ThemeMode::Dark, None, cx);
    Theme::update(cx, |theme| {
        theme.background = color(0x0e1017);
        theme.border = color(0x2b2f45);
        theme.primary = color(0x6c63ff);
        theme.primary_hover = color(0x8279ff);
        theme.primary_active = color(0x574edb);
        theme.primary_foreground = color(0xffffff);
        theme.button_primary = color(0x6c63ff);
        theme.button_primary_hover = color(0x8279ff);
        theme.button_primary_active = color(0x574edb);
        theme.button_primary_foreground = color(0xffffff);
        theme.ring = color(0x8b7bff);
        theme.link = color(0x7fb2ff);
        theme.success = color(0x3ddc97);
        theme.warning = color(0xffb454);
        theme.danger = color(0xff6b81);
    });
}

#[cfg(target_os = "macos")]
fn force_dark_appearance() {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSAppearance, NSAppearanceNameDarkAqua, NSApplication};

    if let Some(main_thread) = MainThreadMarker::new() {
        let appearance = unsafe { NSAppearance::appearanceNamed(NSAppearanceNameDarkAqua) };
        NSApplication::sharedApplication(main_thread).setAppearance(appearance.as_deref());
    }
}

#[cfg(not(target_os = "macos"))]
fn force_dark_appearance() {}

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

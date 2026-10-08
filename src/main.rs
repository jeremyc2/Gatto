mod app;
mod github;
mod menu_bar;
mod model;
mod settings;

use gpui_kit::{AppContext as _, Focusable as _, WindowBounds, WindowOptions, px, size};

use crate::{
    app::UploaderApp,
    menu_bar::{MenuAction, MenuBar},
};

fn main() {
    let application = gpui_kit::application().with_assets(gpui_kit::assets::Assets);

    application.run(|cx| {
        gpui_kit::init(cx);
        app::init_keybindings(cx);
        configure_as_menu_bar_app();

        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(620.), px(780.)), cx)),
            ..Default::default()
        };

        let (window_handle, view) = gpui_kit::open_window(window_options, cx, |window, cx| {
            window.set_window_title("GitHub Image Upload");
            window.on_window_should_close(cx, |_, cx| {
                cx.hide();
                false
            });

            let view = cx.new(|cx| UploaderApp::new(window, cx));
            let focus = view.read(cx).focus_handle(cx);
            window.focus(&focus, cx);
            view
        })
        .expect("Could not open the GitHub Image Upload window");

        let (menu_bar, menu_events) = match MenuBar::install() {
            Ok(menu_bar) => menu_bar,
            Err(error) => {
                view.update(cx, |this, cx| {
                    this.report_menu_bar_error(format!("Menu bar setup failed: {error}"), cx);
                });
                // The window remains usable if macOS refuses to create a status item.
                return;
            }
        };

        cx.spawn(async move |cx| {
            // Keep the native status item alive for the entire application lifetime.
            let menu_bar = menu_bar;
            while let Ok(event) = menu_events.recv().await {
                let Some(action) = menu_bar.action_for(&event) else {
                    continue;
                };

                match action {
                    MenuAction::Open => {
                        cx.update(|cx| {
                            cx.activate(true);
                            let _ = window_handle.update(cx, |_, window, _| {
                                window.activate_window();
                            });
                        });
                    }
                    MenuAction::Settings => {
                        let view = view.clone();
                        cx.update(|cx| {
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

#[cfg(target_os = "macos")]
fn configure_as_menu_bar_app() {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};

    if let Some(main_thread) = MainThreadMarker::new() {
        let application = NSApplication::sharedApplication(main_thread);
        application.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
    }
}

#[cfg(not(target_os = "macos"))]
fn configure_as_menu_bar_app() {}

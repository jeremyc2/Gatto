mod annotation;
mod app;
mod application_menu;
mod compression;
mod custom_url;
mod desktop;
mod diagnostics;
mod github;
mod global_shortcut;
mod image_composer;
mod image_preview;
#[cfg(target_os = "macos")]
mod installer_cleanup;
mod log_viewer;
mod menu_bar;
mod model;
mod open_request;
mod quick_copy;
mod rooftops;
mod rooftops_score;
mod settings;
mod theme;
mod updater;
mod video;
mod window_limits;

use std::{cell::RefCell, rc::Rc};

use window_limits::set_dock_visible;

use gpui_kit::{
    AnyWindowHandle, App, AppContext as _, Entity, Focusable as _, WindowBounds, WindowOptions, px,
    size,
};

use crate::{
    app::UploaderApp,
    custom_url::CustomUrlAction,
    diagnostics::Diagnostics,
    global_shortcut::GlobalShortcutController,
    menu_bar::{MenuAction, MenuBar, MenuBarController},
};

fn main() {
    let login_launch = std::env::args_os()
        .skip(1)
        .any(|argument| argument == settings::LOGIN_ARGUMENT);
    let application = gpui_kit::application()
        .with_assets(gpui_kit::assets::AllAssets)
        .with_quit_mode(gpui_kit::QuitMode::Explicit);
    let main_window = Rc::new(RefCell::new(None::<MainWindow>));
    let reopen_main = main_window.clone();
    application.on_reopen(move |cx| {
        let controller = reopen_main.borrow().clone();
        if let Some(controller) = controller {
            controller.reopen(cx);
        }
    });
    let (open_sender, open_events) = async_channel::unbounded();
    application.on_open_urls(move |urls| {
        let _ = open_sender.try_send(urls);
    });

    application.run(move |cx| {
        cx.set_app_identity("com.jeremy-chandler.gatto", "Gatto");
        gpui_kit::init(cx);
        theme::force_dark_appearance();
        theme::apply_dark_theme(cx);
        app::init_keybindings(cx);
        image_preview::init_keybindings(cx);
        application_menu::init(cx);
        set_dock_visible(!login_launch);
        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() {
                set_dock_visible(false);
            }
        })
        .detach();
        let diagnostics = Diagnostics::new();
        diagnostics.info(if login_launch {
            "Started in the background at login."
        } else {
            "Application started."
        });
        if let Err(error) = settings::AppSettings::refresh_start_at_login() {
            diagnostics.error(format!("Could not refresh Start at Login: {error:#}"));
        }
        let startup_settings = settings::AppSettings::load().unwrap_or_else(|error| {
            diagnostics.error(format!("Could not load startup preferences: {error:#}"));
            settings::AppSettings::default()
        });
        let menu_bar_controller = MenuBarController::default();
        let shortcut_menu = menu_bar_controller.clone();
        let shortcut_diagnostics = diagnostics.clone();
        let global_shortcut_controller = GlobalShortcutController::new(move |shortcut| {
            if let Err(error) = shortcut_menu.set_quick_copy_shortcut(shortcut) {
                shortcut_diagnostics.error(format!("Could not refresh menu shortcuts: {error:#}"));
            }
        });
        let shortcut_events = match global_shortcut_controller.install() {
            Ok(events) => Some(events),
            Err(error) => {
                diagnostics.error(format!("Global shortcut setup failed: {error}"));
                None
            }
        };

        if startup_settings.global_quick_copy_shortcut_enabled
            && let Err(error) = global_shortcut_controller
                .set_shortcut(Some(&startup_settings.global_quick_copy_shortcut))
        {
            diagnostics.error(format!(
                "Could not restore the global Quick Copy shortcut: {error:#}"
            ));
        }

        let controller = MainWindow::new(
            diagnostics.clone(),
            menu_bar_controller.clone(),
            global_shortcut_controller.clone(),
        );
        let update_main = controller.clone();
        cx.on_action(move |_: &application_menu::InstallUpdate, cx| {
            let main = update_main.clone();
            let generation = updater::Updater::shared(cx).read(cx).generation();
            cx.defer(move |cx| {
                if updater::Updater::shared(cx).read(cx).generation() == generation {
                    perform_action(MenuAction::Update, &main, cx);
                }
            });
        });
        let check_main = controller.clone();
        cx.on_action(move |_: &application_menu::CheckForUpdates, cx| {
            let main = check_main.clone();
            cx.defer(move |cx| {
                if !updater::Updater::shared(cx).read(cx).check_for_updates() {
                    let (handle, _) = main.show(cx);
                    let _ = handle.update(cx, |_, window, cx| {
                        drop(window.prompt(
                            gpui_kit::PromptLevel::Info,
                            "Update checking is unavailable",
                            Some("Use a packaged release of Gatto with its bundled updater. If this is a packaged release, see Application logs for details."),
                            &["OK"],
                            cx,
                        ));
                    });
                }
            });
        });
        let preferences_main = controller.clone();
        cx.on_action(move |_: &application_menu::OpenPreferences, cx| {
            // Keyboard dispatch can still borrow the focused window.
            let main = preferences_main.clone();
            cx.defer(move |cx| perform_action(MenuAction::Settings, &main, cx));
        });
        let install_main = controller.clone();
        cx.on_action(move |_: &application_menu::InstallFinderQuickAction, cx| {
            let main = install_main.clone();
            cx.defer(move |cx| {
                if let Some(path) = open_request::quick_action_path()
                    && let Ok(url) = url::Url::from_file_path(path)
                {
                    cx.open_url(url.as_str());
                } else {
                    handle_open_request(open_request::OpenRequest::Invalid(
                        "The Finder Quick Action is missing. Reinstall a packaged release of Gatto.".into()
                    ), &main, cx);
                }
            });
        });
        controller.start(login_launch, cx);
        let composition_main = controller.clone();
        cx.on_action(move |_: &application_menu::CreateComposition, cx| {
            let main = composition_main.clone();
            cx.defer(move |cx| main.show_composer(cx));
        });
        *main_window.borrow_mut() = Some(controller.clone());
        let open_main = controller.clone();
        cx.spawn(async move |cx| {
            while let Ok(urls) = open_events.recv().await {
                cx.update(|cx| {
                    for request in open_request::OpenRequest::from_urls(urls) {
                        handle_open_request(request, &open_main, cx);
                    }
                });
            }
        })
        .detach();

        if let Some(shortcut_events) = shortcut_events {
            let shortcut_main = controller.clone();
            let shortcut_controller = global_shortcut_controller.clone();
            cx.spawn(async move |cx| {
                while let Ok(id) = shortcut_events.recv().await {
                    if shortcut_controller.is_registered_event(id) {
                        cx.update(|cx| perform_action(MenuAction::QuickCopy, &shortcut_main, cx));
                    }
                }
            })
            .detach();
        }

        let paste_actions_visible = !startup_settings.pinned_repositories.is_empty();
        let (menu_bar, menu_events) = match MenuBar::install(
            paste_actions_visible,
            global_shortcut_controller.registered_shortcut().as_deref(),
        ) {
            Ok(menu_bar) => menu_bar,
            Err(error) => {
                diagnostics.error(format!("Menu bar setup failed: {error}"));
                if !login_launch {
                    let (handle, view) = controller.show(cx);
                    let _ = handle.update(cx, |_, window, cx| {
                        view.update(cx, |this, cx| {
                            this.report_menu_bar_error(
                                format!("Menu bar setup failed: {error}"),
                                window,
                                cx,
                            );
                        });
                    });
                }
                return;
            }
        };
        menu_bar_controller.attach(menu_bar);
        updater::Updater::start(cx, diagnostics.clone(), menu_bar_controller.clone(), startup_settings.release_channel);
        #[cfg(target_os = "macos")]
        installer_cleanup::start(diagnostics.clone());

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
                cx.update(|cx| perform_action(action, &controller, cx));
            }
        })
        .detach();
    });
}

fn handle_open_request(request: open_request::OpenRequest, main: &MainWindow, cx: &mut App) {
    use open_request::OpenRequest;
    match request {
        OpenRequest::Action(action) => perform_action(action.into(), main, cx),
        request => {
            let (handle, view) = main.show(cx);
            let _ = handle.update(cx, |_, window, cx| {
                view.update(cx, |this, cx| match request {
                    OpenRequest::Files(paths) => this.stage_opened_files(paths, window, cx),
                    OpenRequest::Invalid(error) => this.report_open_error(error, window, cx),
                    OpenRequest::Action(_) => unreachable!(),
                });
            });
        }
    }
}

impl From<CustomUrlAction> for MenuAction {
    fn from(action: CustomUrlAction) -> Self {
        match action {
            CustomUrlAction::Open => Self::Open,
            CustomUrlAction::Preview => Self::StageFromClipboard,
            CustomUrlAction::QuickCopy => Self::QuickCopy,
            CustomUrlAction::Settings => Self::Settings,
        }
    }
}

#[derive(Clone)]
struct MainWindow(Rc<RefCell<MainWindowState>>);

struct MainWindowState {
    current: Option<(AnyWindowHandle, Entity<UploaderApp>)>,
    composer: Option<(AnyWindowHandle, Entity<image_composer::ImageComposer>)>,
    diagnostics: Diagnostics,
    menu_bar: MenuBarController,
    shortcut: GlobalShortcutController,
    quick_copy: quick_copy::QuickCopy,
}

impl MainWindow {
    fn new(
        diagnostics: Diagnostics,
        menu_bar: MenuBarController,
        shortcut: GlobalShortcutController,
    ) -> Self {
        Self(Rc::new(RefCell::new(MainWindowState {
            current: None,
            composer: None,
            quick_copy: quick_copy::QuickCopy::new(diagnostics.clone()),
            diagnostics,
            menu_bar,
            shortcut,
        })))
    }

    fn start(&self, login_launch: bool, cx: &mut App) {
        if !login_launch {
            self.show(cx);
        }
    }

    fn show_composer(&self, cx: &mut App) {
        set_dock_visible(true);
        cx.activate(true);
        let existing = self.0.borrow().composer.clone();
        if let Some((handle, view)) = existing
            && handle
                .update(cx, |_, window, cx| {
                    if !window_limits::is_presented(window) {
                        window.focus(&view.read(cx).focus_handle(cx), cx);
                    }
                    window.activate_window();
                })
                .is_ok()
        {
            return;
        }
        let controller = self.clone();
        let result = gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(gpui_kit::Bounds::centered(
                    None,
                    size(
                        px(image_composer::WINDOW_WIDTH),
                        px(image_composer::WINDOW_HEIGHT),
                    ),
                    cx,
                ))),
                window_min_size: Some(size(
                    px(image_composer::WINDOW_WIDTH),
                    px(image_composer::WINDOW_HEIGHT),
                )),
                ..Default::default()
            },
            cx,
            move |window, cx| {
                window.set_window_title(&format!("{} — Gatto", image_composer::TITLE));
                let view = cx.new(|cx| image_composer::ImageComposer::new(window, cx));
                let close_view = view.downgrade();
                window.on_window_should_close(cx, move |window, cx| {
                    if close_view
                        .update(cx, |view, cx| view.should_close(window, cx))
                        .is_ok_and(|approved| !approved)
                    {
                        return false;
                    }
                    let closed = window_limits::close(window, cx);
                    if closed {
                        controller.0.borrow_mut().composer = None;
                    }
                    closed
                });
                window.focus(&view.read(cx).focus_handle(cx), cx);
                view
            },
        );
        let Ok((handle, view)) = result else {
            return;
        };
        let controller = self.clone();
        cx.subscribe(
            &view,
            move |view, event: &image_composer::ComparisonRequested, cx| {
                let (main_handle, main_view) = controller.show(cx);
                let result = main_handle
                    .update(cx, |_, window, cx| {
                        main_view.update(cx, |main, cx| {
                            main.begin_comparison(event.0.clone(), window, cx)
                        })
                    })
                    .unwrap_or(Err(
                        "Could not open the main window. Remove an image and add it again to retry.",
                    ));
                let _ = handle.update(cx, |_, window, cx| {
                    view.update(cx, |view, cx| view.requested(result, window, cx));
                    if result.is_err() {
                        window.activate_window();
                    }
                });
            },
        )
        .detach();
        self.0.borrow_mut().composer = Some((handle, view));
    }

    fn ensure(&self, cx: &mut App) -> (AnyWindowHandle, Entity<UploaderApp>) {
        let current = self.0.borrow().current.clone();
        if let Some((handle, view)) = current
            && cx.windows().contains(&handle)
        {
            return (handle, view);
        }
        let (diagnostics, menu_bar, shortcut) = {
            let state = self.0.borrow();
            (
                state.diagnostics.clone(),
                state.menu_bar.clone(),
                state.shortcut.clone(),
            )
        };
        let controller = self.clone();
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(620.), px(780.)), cx)),
            window_min_size: Some(size(px(520.), px(600.))),
            ..Default::default()
        };
        let result = gpui_kit::open_window(options, cx, move |window, cx| {
            window.set_window_title("Gatto");
            window_limits::set_maximum_content_size(window, 920., 1040.);
            let view = cx.new(|cx| UploaderApp::new(window, diagnostics, menu_bar, shortcut, cx));
            let close_view = view.downgrade();
            window.on_window_should_close(cx, move |window, cx| {
                let _ = close_view.update(cx, |this, cx| {
                    this.prepare_to_close(window, cx);
                    this.clear_staged_attachments(
                        "Closed the main window; discarded staged attachments.",
                        cx,
                    );
                });
                gpui_kit::component::WindowExt::close_dialog(window, cx);
                let should_close = window_limits::close(window, cx);
                if should_close {
                    controller.0.borrow_mut().current = None;
                }
                should_close
            });
            window.focus(&view.read(cx).focus_handle(cx), cx);
            view
        })
        .expect("Could not open the Gatto window");
        self.0.borrow_mut().current = Some(result.clone());
        result
    }

    fn show(&self, cx: &mut App) -> (AnyWindowHandle, Entity<UploaderApp>) {
        let (handle, view) = self.ensure(cx);
        set_dock_visible(true);
        cx.activate(true);
        let _ = handle.update(cx, |_, window, _| window.activate_window());
        (handle, view)
    }

    fn reopen(&self, cx: &mut App) {
        let presented: Vec<_> = cx
            .windows()
            .into_iter()
            .filter(|handle| {
                handle
                    .update(cx, |_, window, _| window_limits::is_presented(window))
                    .unwrap_or(false)
            })
            .collect();
        if !presented.is_empty() {
            set_dock_visible(true);
            cx.activate(true);
            let target = cx
                .active_window()
                .filter(|handle| presented.contains(handle))
                .unwrap_or(presented[0]);
            let _ = target.update(cx, |_, window, _| window.activate_window());
            return;
        }
        let (handle, view) = self.show(cx);
        let _ = handle.update(cx, |_, window, cx| {
            view.update(cx, |this, cx| this.show_main_page(window, cx));
        });
    }
}

fn perform_action(action: MenuAction, main: &MainWindow, cx: &mut App) {
    match action {
        MenuAction::Open => {
            main.show(cx);
        }
        MenuAction::StageFromClipboard => {
            let (handle, _) = main.show(cx);
            let _ = handle.update(cx, |_, window, cx| {
                window.dispatch_action(Box::new(app::StageFromClipboard), cx);
            });
        }
        MenuAction::QuickCopy => {
            let quick_copy = main.0.borrow().quick_copy.clone();
            quick_copy.start(cx);
        }
        MenuAction::Settings => {
            let (_, view) = main.show(cx);
            view.update(cx, |this, cx| this.show_settings(cx));
        }
        MenuAction::CreateComposition => main.show_composer(cx),
        MenuAction::Update => {
            let (handle, view) = main.show(cx);
            let quick_copy_busy = main.0.borrow().quick_copy.is_busy();
            let _ = handle.update(cx, |_, window, cx| {
                view.update(cx, |this, cx| {
                    this.install_update(quick_copy_busy, window, cx)
                });
            });
        }
        MenuAction::Quit => cx.quit(),
    }
}

#[cfg(test)]
mod startup_tests {
    use super::*;

    #[gpui_kit::test]
    fn login_launch_does_not_create_a_main_window(cx: &mut gpui_kit::TestAppContext) {
        cx.update(|cx| {
            gpui_kit::init(cx);
            let main = MainWindow::new(
                Diagnostics::new(),
                MenuBarController::default(),
                GlobalShortcutController::default(),
            );
            main.start(true, cx);
            assert!(
                cx.windows().is_empty(),
                "login must not create even a hidden window"
            );
            assert!(main.0.borrow().current.is_none());
        });
    }
}

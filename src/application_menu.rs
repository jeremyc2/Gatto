use gpui_kit::component::WindowExt as _;
use gpui_kit::{App, KeyBinding, Menu, MenuItem, Window, actions};

actions!(
    gatto,
    [
        CloseWindow,
        DismissWindow,
        Quit,
        InstallUpdate,
        CheckForUpdates,
        OpenPreferences,
        CreateComposition,
        HelpRepository,
        HelpWalkthrough,
        HelpSetup,
        HelpSupport
    ]
);

#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize, gpui_kit::Action)]
#[action(namespace = gatto, no_json)]
pub struct OpenRecentProject {
    pub path: std::path::PathBuf,
}

pub fn set_recent_projects(projects: Vec<(String, std::path::PathBuf)>, cx: &App) {
    cx.set_dock_menu(
        projects
            .into_iter()
            .take(5)
            .map(|(name, path)| MenuItem::action(name, OpenRecentProject { path }))
            .collect(),
    );
}

pub fn init(cx: &mut App) {
    cx.intercept_keystrokes(|event, window, cx| {
        if event.keystroke.key == "escape"
            && window.root::<gpui_kit::base::Root>().flatten().is_some()
            && window.has_active_dialog(cx)
        {
            window.dispatch_action(Box::new(gpui_kit::component::dialog::Cancel), cx);
            cx.stop_propagation();
        }
    })
    .detach();
    // Unscoped actions work in every window, including focused text fields.
    cx.bind_keys([
        KeyBinding::new("cmd-w", CloseWindow, None),
        KeyBinding::new("escape", DismissWindow, None),
        KeyBinding::new("cmd-q", Quit, None),
        KeyBinding::new("cmd-,", OpenPreferences, None),
    ]);
    cx.on_action(|_: &Quit, cx| request_quit(cx));
    cx.on_action(|_: &CloseWindow, cx| close_focused_window(false, cx));
    cx.on_action(|_: &DismissWindow, cx| close_focused_window(true, cx));
    cx.on_action(|_: &HelpRepository, cx| cx.open_url("https://github.com/jeremyc2/Gatto"));
    cx.on_action(|_: &HelpWalkthrough, cx| {
        cx.open_url("https://github.com/jeremyc2/Gatto/blob/main/docs/WALKTHROUGH.md")
    });
    cx.on_action(|_: &HelpSetup, cx| {
        cx.open_url("https://github.com/jeremyc2/Gatto/blob/main/docs/SETUP.md")
    });
    cx.on_action(|_: &HelpSupport, cx| {
        cx.open_url("https://github.com/jeremyc2/Gatto/blob/main/SUPPORT.md")
    });
    cx.set_menus([
        Menu::new("Gatto").items([
            MenuItem::action("App Preferences", OpenPreferences),
            MenuItem::separator(),
            MenuItem::action("Check for Updates…", CheckForUpdates),
            MenuItem::separator(),
            MenuItem::action("Quit Gatto", Quit),
        ]),
        Menu::new("File").items([MenuItem::action("Close Window", CloseWindow)]),
        Menu::new("Advanced").items([MenuItem::action(
            crate::image_composer::TITLE,
            CreateComposition,
        )]),
        Menu::new("Help").items([
            MenuItem::action("GitHub Repository", HelpRepository),
            MenuItem::action("Walkthrough", HelpWalkthrough),
            MenuItem::action("Setup", HelpSetup),
            MenuItem::action("Support", HelpSupport),
        ]),
    ]);
}

/// User-requested quits must not cut off a project write. The platform quit
/// hook remains a bounded fallback for shutdowns initiated outside these actions.
pub(crate) fn request_quit(cx: &mut App) {
    cx.defer(|cx| {
        let mut ready = true;
        for handle in cx.windows() {
            let _ = handle.update(cx, |_, window, cx| {
                if let Some(root) = window.root::<gpui_kit::base::Root>().flatten()
                    && let Ok(preview) = root
                        .read(cx)
                        .view()
                        .clone()
                        .downcast::<crate::image_preview::ImagePreview>()
                    && preview.read(cx).has_pending_edits()
                {
                    window.activate_window();
                    preview.update(cx, |preview, cx| preview.confirm_exit(window, cx));
                    ready = false;
                    return;
                }
                if let Some(root) = window.root::<gpui_kit::base::Root>().flatten()
                    && let Ok(view) = root
                        .read(cx)
                        .view()
                        .clone()
                        .downcast::<crate::app::UploaderApp>()
                    && !view.update(cx, |view, cx| view.prepare_to_close(window, cx))
                {
                    window.activate_window();
                    ready = false;
                }
            });
        }
        if ready {
            cx.quit();
        }
    });
}

pub(crate) fn is_extra_window(window: &Window, cx: &App) -> bool {
    window
        .root::<gpui_kit::base::Root>()
        .flatten()
        .is_some_and(|root| {
            root.read(cx).view().entity_type() != std::any::TypeId::of::<crate::app::UploaderApp>()
        })
}

fn close_focused_window(dismiss: bool, cx: &mut App) {
    if let Some(handle) = cx.active_window() {
        // Keyboard dispatch already borrows the focused window. Capture that
        // window before deferring so another window cannot become the target.
        cx.defer(move |cx| {
            let _ = handle.update(cx, |_, window, cx| {
                if dismiss
                    && window.root::<gpui_kit::base::Root>().flatten().is_some()
                    && window.has_active_dialog(cx)
                {
                    window.dispatch_action(Box::new(gpui_kit::component::dialog::Cancel), cx);
                    return;
                }
                if let Some(root) = window.root::<gpui_kit::base::Root>().flatten() {
                    let content = root.read(cx).view().clone();
                    if dismiss
                        && let Ok(composer) = content
                            .clone()
                            .downcast::<crate::image_composer::ImageComposer>()
                    {
                        composer.update(cx, |composer, cx| composer.dismiss(window, cx));
                        return;
                    }
                    if let Ok(preview) = content.downcast::<crate::image_preview::ImagePreview>() {
                        // Preview Escape never reaches the app-wide close fallback, even when blurred.
                        preview.update(cx, |preview, cx| {
                            if dismiss {
                                preview.dismiss_editor(&DismissWindow, window, cx);
                            } else {
                                preview.confirm_exit(window, cx);
                            }
                        });
                        return;
                    }
                }
                if dismiss && !is_extra_window(window, cx) {
                    // Key bindings run before raw key listeners. Route Escape
                    // to the main view even when no control has keyboard focus.
                    if let Some(root) = window.root::<gpui_kit::base::Root>().flatten() {
                        let content = root.read(cx).view().clone();
                        if let Ok(main) = content.downcast::<crate::app::UploaderApp>() {
                            main.update(cx, |main, cx| main.dismiss(window, cx));
                        }
                    }
                    return;
                }
                // Preserve each window's cleanup and Touch Bar workaround.
                crate::window_limits::request_close(window, cx);
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use gpui_kit::{
        AppContext as _, Context, Entity, Focusable as _, IntoElement, ParentElement as _, Render,
        TestAppContext, Window,
        component::input::{Input, InputState},
        div,
    };

    use super::*;

    struct PreferencesInput(Entity<InputState>);

    impl Render for PreferencesInput {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().child(Input::new(&self.0))
        }
    }

    fn assert_shortcut_dispatches<A: gpui_kit::Action>(cx: &mut TestAppContext, shortcut: &str) {
        let dispatched = Rc::new(Cell::new(false));
        let capture = dispatched.clone();
        cx.update(move |cx| {
            gpui_kit::init(cx);
            crate::app::init_keybindings(cx);
            crate::image_preview::init_keybindings(cx);
            init(cx);
            // Run the real handlers and observe the resolved action. Native
            // close and quit effects are verified with a bundled app on macOS.
            cx.observe_keystrokes(move |event, _, _| {
                if let Some(action) = &event.action {
                    capture.set(action.as_any().is::<A>());
                }
            })
            .detach();
        });
        let (_, window) = cx.add_window_view(|window, cx| {
            let input = cx.new(|cx| InputState::new(window, cx));
            window.focus(&input.read(cx).focus_handle(cx), cx);
            window.activate_window();
            PreferencesInput(input)
        });
        window.simulate_keystrokes(shortcut);
        assert!(
            dispatched.get(),
            "{shortcut} did not dispatch its app action"
        );
    }

    #[gpui_kit::test]
    fn command_w_dispatches_with_a_text_input_focused(cx: &mut TestAppContext) {
        assert_shortcut_dispatches::<CloseWindow>(cx, "cmd-w");
    }

    #[gpui_kit::test]
    fn command_q_dispatches_with_a_text_input_focused(cx: &mut TestAppContext) {
        assert_shortcut_dispatches::<Quit>(cx, "cmd-q");
    }

    #[gpui_kit::test]
    fn command_comma_dispatches_with_a_text_input_focused(cx: &mut TestAppContext) {
        // main.rs registers this process-wide action with the main-window
        // controller; this test supplies a receiver without opening native UI.
        cx.update(|cx| {
            cx.on_action(|_: &OpenPreferences, _| {});
        });
        assert_shortcut_dispatches::<OpenPreferences>(cx, "cmd-,");
    }

    fn observe_close(cx: &mut TestAppContext) -> Rc<Cell<bool>> {
        let closed = Rc::new(Cell::new(false));
        let capture = closed.clone();
        cx.update(move |cx| {
            gpui_kit::init(cx);
            crate::image_preview::init_keybindings(cx);
            init(cx);
            cx.observe_keystrokes(move |event, _, _| {
                if let Some(action) = &event.action {
                    capture.set(action.as_any().is::<DismissWindow>());
                }
            })
            .detach();
        });
        closed
    }

    #[gpui_kit::test]
    fn escape_cancels_a_modal_and_runs_its_hooks_in_an_extra_window(cx: &mut TestAppContext) {
        use gpui_kit::component::WindowExt as _;
        let canceled = Rc::new(Cell::new(false));
        let finished = Rc::new(Cell::new(false));
        cx.update(|cx| {
            gpui_kit::init(cx);
            init(cx);
        });
        let (_, cx) = cx.add_window_view(|window, cx| {
            window.activate_window();
            let input = cx.new(|cx| InputState::new(window, cx));
            let view = cx.new(|_| PreferencesInput(input));
            gpui_kit::base::Root::new(view, window, cx)
        });
        let cancel = canceled.clone();
        let close = finished.clone();
        cx.update(|window, cx| {
            let input = cx.new(|cx| InputState::new(window, cx));
            let focus = input.clone();
            window.open_alert_dialog(cx, move |alert, window, cx| {
                focus.update(cx, |input, cx| input.focus(window, cx));
                let cancel = cancel.clone();
                let close = close.clone();
                alert
                    .title("Test modal")
                    .child(Input::new(&input))
                    .on_cancel(move |_, _, _| {
                        cancel.set(true);
                        true
                    })
                    .on_close(move |_, _, _| close.set(true))
            });
            window.draw(cx).clear(cx);
        });
        cx.simulate_keystrokes("escape");
        cx.run_until_parked();
        cx.update(|window, cx| assert!(!window.has_active_dialog(cx)));
        assert!(canceled.get());
        assert!(finished.get());
    }

    #[gpui_kit::test]
    fn escape_closes_logs_without_a_control_focused(cx: &mut TestAppContext) {
        let closed = observe_close(cx);
        let (_, cx) = cx.add_window_view(|window, cx| {
            window.activate_window();
            let view = cx.new(|cx| {
                crate::log_viewer::LogViewer::new(crate::diagnostics::Diagnostics::new(), cx)
            });
            gpui_kit::base::Root::new(view, window, cx)
        });
        cx.update(|window, cx| assert!(is_extra_window(window, cx)));
        cx.simulate_keystrokes("escape");
        assert!(closed.get(), "Escape must close the focused Logs window");
        closed.set(false);
        cx.simulate_keystrokes("tab escape");
        assert!(
            closed.get(),
            "Escape must also work from a Logs toolbar control"
        );
    }

    #[gpui_kit::test]
    fn preview_escape_clears_focus_and_clean_command_w_skips_confirmation_without_focus(
        cx: &mut TestAppContext,
    ) {
        use gpui_kit::component::WindowExt as _;
        cx.update(|cx| {
            gpui_kit::init(cx);
            crate::image_preview::init_keybindings(cx);
            init(cx);
        });
        let (_, cx) = cx.add_window_view(|window, cx| {
            window.activate_window();
            let view = cx.new(|cx| crate::image_preview::ImagePreview::new(Vec::new(), 0, cx));
            window.focus(&view.read(cx).focus_handle(cx), cx);
            gpui_kit::base::Root::new(view, window, cx)
        });
        cx.simulate_keystrokes("escape escape");
        cx.update(|window, cx| {
            assert!(
                window.focused(cx).is_some(),
                "Keyboard handling returns to the artwork"
            );
            assert!(
                !window.has_active_dialog(cx),
                "Escape must never request preview exit"
            );
            window.blur(cx); // Exercise Command+W's app-wide fallback too.
        });
        cx.simulate_keystrokes("cmd-w");
        cx.run_until_parked();
        cx.update(|window, cx| {
            assert!(
                !window.has_active_dialog(cx),
                "Clean previews close without confirmation"
            );
        });
        cx.simulate_keystrokes("tab escape escape");
        cx.update(|window, cx| {
            assert!(
                window.focused(cx).is_some(),
                "Keyboard handling returns to the artwork"
            );
            assert!(!window.has_active_dialog(cx));
        });
    }

    #[gpui_kit::test]
    fn escape_closes_rooftops(cx: &mut TestAppContext) {
        let closed = observe_close(cx);
        let (_, cx) = cx.add_window_view(|window, cx| {
            let view = cx.new(|cx| crate::rooftops::Rooftops::new(window, cx));
            window.focus(&view.read(cx).focus_handle(cx), cx);
            window.activate_window();
            gpui_kit::base::Root::new(view, window, cx)
        });
        cx.update(|window, cx| assert!(is_extra_window(window, cx)));
        cx.simulate_keystrokes("escape");
        assert!(closed.get(), "Escape must close the focused game window");
    }
}

use gpui_kit::{App, KeyBinding, Menu, MenuItem, Window, actions};

actions!(
    gatto,
    [
        CloseWindow,
        DismissWindow,
        Quit,
        InstallUpdate,
        CheckForUpdates,
        OpenPreferences
    ]
);

pub fn init(cx: &mut App) {
    // Unscoped actions work in every window, including focused text fields.
    cx.bind_keys([
        KeyBinding::new("cmd-w", CloseWindow, None),
        KeyBinding::new("escape", DismissWindow, None),
        KeyBinding::new("cmd-q", Quit, None),
        KeyBinding::new("cmd-,", OpenPreferences, None),
    ]);
    cx.on_action(|_: &Quit, cx| cx.quit());
    cx.on_action(|_: &CloseWindow, cx| close_focused_window(false, cx));
    cx.on_action(|_: &DismissWindow, cx| close_focused_window(true, cx));
    cx.set_menus([
        Menu::new("Gatto").items([
            MenuItem::action("App Preferences", OpenPreferences),
            MenuItem::separator(),
            MenuItem::action("Check for Updates…", CheckForUpdates),
            MenuItem::separator(),
            MenuItem::action("Quit Gatto", Quit),
        ]),
        Menu::new("File").items([MenuItem::action("Close Window", CloseWindow)]),
    ]);
}

pub(crate) fn is_extra_window(window: &Window, cx: &App) -> bool {
    window
        .root::<gpui_kit::base::Root>()
        .flatten()
        .is_some_and(|root| {
            root.read(cx).view().entity_type() != std::any::TypeId::of::<crate::app::UploaderApp>()
        })
}

fn close_focused_window(extra_only: bool, cx: &mut App) {
    if let Some(handle) = cx.active_window() {
        // Keyboard dispatch already borrows the focused window. Capture that
        // window before deferring so another window cannot become the target.
        cx.defer(move |cx| {
            let _ = handle.update(cx, |_, window, cx| {
                if !extra_only || is_extra_window(window, cx) {
                    // Preserve each window's cleanup and Touch Bar workaround.
                    crate::window_limits::request_close(window, cx);
                }
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
    fn escape_closes_preview_after_keyboard_focus_is_cleared(cx: &mut TestAppContext) {
        let closed = observe_close(cx);
        let (_, cx) = cx.add_window_view(|window, cx| {
            window.activate_window();
            let view = cx.new(|cx| crate::image_preview::ImagePreview::new(Vec::new(), 0, cx));
            gpui_kit::base::Root::new(view, window, cx)
        });
        cx.update(|window, cx| {
            assert!(is_extra_window(window, cx));
            window.blur(cx);
        });
        cx.simulate_keystrokes("escape");
        assert!(closed.get(), "Escape must close the focused preview window");
        closed.set(false);
        cx.simulate_keystrokes("tab escape");
        assert!(
            closed.get(),
            "Escape must also work from a preview toolbar control"
        );
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

use gpui_kit::{App, KeyBinding, Menu, MenuItem, actions};

actions!(
    gatto,
    [
        CloseWindow,
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
        KeyBinding::new("cmd-q", Quit, None),
        KeyBinding::new("cmd-,", OpenPreferences, None),
    ]);
    cx.on_action(|_: &Quit, cx| cx.quit());
    cx.on_action(|_: &CloseWindow, cx| {
        if let Some(handle) = cx.active_window() {
            // Keyboard dispatch already borrows the focused window. Wait for
            // that update to finish before accessing it through its handle.
            cx.defer(move |cx| {
                let _ = handle.update(cx, |_, window, cx| {
                    // Preserve each window's cleanup and Touch Bar workaround.
                    crate::window_limits::request_close(window, cx);
                });
            });
        }
    });
    cx.set_menus([
        Menu::new("Gatto").items([
            MenuItem::action("Check for Updates…", CheckForUpdates),
            MenuItem::separator(),
            MenuItem::action("Quit Gatto", Quit),
        ]),
        Menu::new("File").items([MenuItem::action("Close Window", CloseWindow)]),
    ]);
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
}

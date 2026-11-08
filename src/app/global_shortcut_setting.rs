use super::*;

pub(super) fn modifier_count(modifiers: &gpui_kit::Modifiers) -> u8 {
    [
        modifiers.platform,
        modifiers.control,
        modifiers.alt,
        modifiers.shift,
    ]
    .into_iter()
    .filter(|pressed| *pressed)
    .count() as u8
}

impl UploaderApp {
    pub(super) fn update_recorded_shortcut(
        &mut self,
        shortcut: Option<String>,
    ) -> anyhow::Result<()> {
        let registration = self.global_shortcut.registered_shortcut();
        self.global_shortcut.set_shortcut(
            shortcut
                .as_deref()
                .filter(|_| self.settings.add_from_clipboard_available()),
        )?;
        let previous = (
            self.settings.global_add_from_clipboard_shortcut.clone(),
            self.settings.global_add_from_clipboard_shortcut_enabled,
        );
        self.settings.global_add_from_clipboard_shortcut = shortcut;
        self.settings.global_add_from_clipboard_shortcut_enabled =
            self.settings.global_add_from_clipboard_shortcut.is_some();
        if self.persist_preferences
            && let Err(error) = self.settings.save()
        {
            (
                self.settings.global_add_from_clipboard_shortcut,
                self.settings.global_add_from_clipboard_shortcut_enabled,
            ) = previous;
            if let Err(rollback) = self.global_shortcut.set_shortcut(registration.as_deref()) {
                self.diagnostics.error(format!(
                    "Could not restore shortcut registration: {rollback}"
                ));
            }
            return Err(error.context("Could not save shortcut preferences"));
        }
        Ok(())
    }

    pub(super) fn reset_global_shortcut(
        &mut self,
        _: &gpui_kit::ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Err(error) = self.update_recorded_shortcut(None) {
            window.push_notification(Notification::error(error.to_string()), cx);
            return;
        }
        self.recording_global_shortcut = false;
        self.shortcut_modifier_hint = false;
        self.sync_add_from_clipboard_enabled();
        cx.notify();
    }

    pub(super) fn render_global_shortcut(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let keys = self
            .settings
            .global_add_from_clipboard_shortcut
            .as_deref()
            .unwrap_or_default()
            .split('+')
            .filter(|key| !key.is_empty())
            .map(|key| {
                let key = match key {
                    "Command" => "cmd".into(),
                    "Control" => "ctrl".into(),
                    "Option" => "alt".into(),
                    _ => key.to_ascii_lowercase(),
                };
                Kbd::new(Keystroke {
                    key,
                    ..Default::default()
                })
                .outline()
            })
            .collect::<Vec<_>>();
        let row = h_flex()
            .id("global-shortcut-keys")
            .gap_3()
            .items_center()
            .child(
                Button::new("record-global-add-from-clipboard-shortcut")
                    .app_secondary()
                    .label(if self.recording_global_shortcut {
                        "Cancel"
                    } else {
                        "Record global hotkey"
                    })
                    .on_click(cx.listener(Self::start_global_shortcut_recording)),
            )
            .child(
                Button::new("reset-global-add-from-clipboard-shortcut")
                    .app_secondary()
                    .label("Reset")
                    .on_click(cx.listener(Self::reset_global_shortcut)),
            )
            .child(
                h_flex()
                    .id("recorded-shortcut-keys")
                    .h(theme::CONTROL_HEIGHT)
                    .gap_1()
                    .items_center()
                    .children(keys),
            );
        v_flex()
            .gap_2()
            .child(row)
            .when(
                self.recording_global_shortcut && !self.shortcut_modifier_hint,
                |row| {
                    row.child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("Press keys… Esc to cancel."),
                    )
                },
            )
            .when(self.shortcut_modifier_hint, |content| {
                content.child(
                    div()
                        .id("shortcut-modifier-hint")
                        .text_xs()
                        .text_color(cx.theme().danger)
                        .child("At least two modifiers are required."),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use gpui_kit::{Modifiers, PlatformInput, TestAppContext};

    #[gpui_kit::test]
    fn recording_validation_and_reset_do_not_require_an_enable_toggle(cx: &mut TestAppContext) {
        let directory = tempfile::tempdir().unwrap();
        let (view, cx) = super::super::tests::upload_window(cx);
        cx.update(|window, cx| {
            view.update(cx, |app, cx| {
                // No native binding while the selected destination is unavailable.
                // Native registration/replacement is covered by platform-tests.
                app.settings.project_directory_override = Some(directory.path().into());
                app.settings.add_from_clipboard_project = Some("Missing".into());
                app.show_settings(cx);
                assert!(app.settings.global_add_from_clipboard_shortcut.is_none());
                assert!(!app.shortcut_modifier_hint);
                app.start_global_shortcut_recording(&Default::default(), window, cx);
            });
            window.draw(cx).clear(cx);
        });
        cx.simulate_keystrokes("cmd-g");
        cx.update(|window, cx| {
            assert!(view.read(cx).shortcut_modifier_hint);
            assert!(view.read(cx).recording_global_shortcut);
            window.draw(cx).clear(cx);
            window.dispatch_event(
                PlatformInput::ModifiersChanged(gpui_kit::ModifiersChangedEvent {
                    modifiers: Modifiers {
                        platform: true,
                        shift: true,
                        ..Default::default()
                    },
                    ..Default::default()
                }),
                cx,
            );
            assert!(!view.read(cx).shortcut_modifier_hint);
        });
        cx.simulate_keystrokes("cmd-shift-u");
        cx.update(|window, cx| {
            view.update(cx, |app, cx| {
                assert!(!app.recording_global_shortcut);
                assert!(!app.shortcut_modifier_hint);
                assert_eq!(
                    app.settings.global_add_from_clipboard_shortcut.as_deref(),
                    Some("Command+Shift+U")
                );
                assert!(app.settings.global_add_from_clipboard_shortcut_enabled);
                app.start_global_shortcut_recording(&Default::default(), window, cx);
            });
            window.draw(cx).clear(cx);
        });
        cx.simulate_keystrokes("cmd-g escape");
        cx.update(|window, cx| {
            view.update(cx, |app, cx| {
                assert!(!app.shortcut_modifier_hint);
                assert_eq!(
                    app.settings.global_add_from_clipboard_shortcut.as_deref(),
                    Some("Command+Shift+U")
                );
                app.start_global_shortcut_recording(&Default::default(), window, cx);
                app.reset_global_shortcut(&Default::default(), window, cx);
                assert!(app.settings.global_add_from_clipboard_shortcut.is_none());
                assert!(!app.settings.global_add_from_clipboard_shortcut_enabled);
                assert!(!app.recording_global_shortcut);
                assert!(!app.shortcut_modifier_hint);
            });
        });
    }
}

use super::*;
use gpui_kit::component::menu::{PopupMenu, PopupMenuItem};

impl UploaderApp {
    pub(super) fn render_upload_button(
        &self,
        count: usize,
        enabled: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let owner = cx.entity().downgrade();
        let action = self.settings.upload_action;
        let menu = move |mut menu: PopupMenu, _: &mut Window, _: &mut Context<PopupMenu>| {
            for choice in [UploadAction::Upload, UploadAction::UploadAndCopy] {
                let view = owner.clone();
                menu = menu.item(
                    PopupMenuItem::new(choice.label(count))
                        .checked(choice == action)
                        .on_click(move |_, window, cx| {
                            let _ = view.update(cx, |this, cx| {
                                this.choose_upload_action(choice, window, cx);
                            });
                        }),
                );
            }
            menu
        };
        h_flex()
            .items_center()
            .child(
                Button::new("send-image")
                    .debug_selector(|| "send-image".into())
                    .app_primary()
                    .rounded_r_none()
                    .label(if self.uploading {
                        "Uploading…".into()
                    } else {
                        action.label(count)
                    })
                    .icon(AssetIconName::Upload)
                    .loading(self.uploading)
                    .disabled(!enabled)
                    .when(!enabled, |button| button.cursor_not_allowed())
                    .when(self.staged_attachments.is_empty(), |button| {
                        button.tooltip("Add an image or video first")
                    })
                    .on_click(cx.listener(|this, _, window, cx| this.send_attachments(window, cx))),
            )
            .child(
                Button::new("upload-options-chevron")
                    .debug_selector(|| "upload-options-chevron".into())
                    .app_primary()
                    .when(enabled, |button| button.bg(cx.theme().primary_active))
                    .rounded_l_none()
                    .min_w_0()
                    .px_0()
                    .w(px(32.))
                    .border_l_1()
                    .border_color(cx.theme().primary_foreground.opacity(0.3))
                    .icon(AssetIconName::ChevronDown)
                    .accessibility_label("Upload options")
                    .tooltip("Upload options")
                    .disabled(!enabled)
                    .when(!enabled, |button| button.cursor_not_allowed())
                    .dropdown_menu_with_anchor(gpui_kit::Anchor::BottomRight, menu),
            )
    }

    fn choose_upload_action(
        &mut self,
        action: UploadAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.uploading || self.import_busy() || self.loading_token || self.loading_repository {
            return;
        }
        let previous = self.settings.upload_action;
        self.settings.upload_action = action;
        if self.persist_preferences
            && let Err(error) = self.settings.save()
        {
            self.settings.upload_action = previous;
            self.warn(
                format!("Could not save the upload option: {error}"),
                window,
                cx,
            );
            return;
        }
        cx.notify();
        self.send_attachments(window, cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::test::TestWindowExt as _;
    use gpui_kit::{Modifiers, TestAppContext};

    // Fail normal upload validation before any network work, so pointer tests
    // can exercise the real actions with either remembered mode.
    fn window_with_oversized_attachment(
        cx: &mut TestAppContext,
    ) -> (Entity<UploaderApp>, &mut gpui_kit::VisualTestContext) {
        let (view, cx) = super::super::tests::upload_window(cx);
        cx.update(|window, cx| {
            view.update(cx, |app, cx| {
                app.load_state = LoadState::Ready;
                app.selected_repository = Some(Repository {
                    id: 42,
                    name: "images".into(),
                });
                app.token = Some("test-token".into());
                let mut attachment = StagedAttachment::from_bytes(
                    "image.png".into(),
                    include_bytes!("../../packaging/menu-bar-icon.png").to_vec(),
                    None,
                )
                .unwrap();
                if let Media::Image(image) = &mut attachment.media {
                    image.bytes = vec![0; crate::model::MAX_ATTACHMENT_BYTES as usize + 1].into();
                }
                app.staged_attachments = vec![attachment];
                cx.notify();
            });
            window.draw(cx).clear(cx);
        });
        (view, cx)
    }

    #[gpui_kit::test]
    fn main_button_runs_the_remembered_action_without_opening_options(cx: &mut TestAppContext) {
        let (view, cx) = window_with_oversized_attachment(cx);
        for action in [UploadAction::Upload, UploadAction::UploadAndCopy] {
            cx.update(|window, cx| {
                let before = view.update(cx, |app, cx| {
                    app.settings.upload_action = action;
                    cx.notify();
                    app.diagnostics.lines().len()
                });
                window.render_frame(cx);
                window.click("send-image", cx);
                assert!(window.try_find("popup-menu").is_none());
                let app = view.read(cx);
                assert_eq!(app.settings.upload_action, action);
                let messages = app.diagnostics.lines();
                assert!(messages.len() > before);
                assert!(
                    messages.last().unwrap().contains("image.png:"),
                    "main click must run upload validation"
                );
                assert!(!app.uploading);
            });
        }
    }

    #[gpui_kit::test]
    fn chevron_opens_choices_above_and_menu_selection_runs_and_remembers_the_action(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = window_with_oversized_attachment(cx);
        let button = cx.debug_bounds("send-image").unwrap();
        let click = cx.debug_bounds("upload-options-chevron").unwrap().center();
        cx.simulate_mouse_down(click, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(click, MouseButton::Left, Modifiers::default());
        cx.update(|window, cx| {
            window.render_frame(cx);
            let menu = window.find("popup-menu").bounds();
            assert!(
                menu.bottom() <= button.top(),
                "the menu must open above the button"
            );
            window.within("popup-menu").click(1usize, cx);
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            window.render_frame(cx);
            assert!(window.try_find("popup-menu").is_none());
            let app = view.read(cx);
            assert_eq!(app.settings.upload_action, UploadAction::UploadAndCopy);
            assert!(
                app.diagnostics
                    .lines()
                    .last()
                    .unwrap()
                    .contains("image.png:"),
                "menu choice must also run upload validation"
            );
            assert!(!app.uploading);
        });
    }
}

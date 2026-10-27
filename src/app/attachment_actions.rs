use super::*;
use gpui_kit::component::menu::PopupMenuItem;

impl UploaderApp {
    pub(super) fn attachment_options(
        &self,
        ids: Vec<u64>,
        individual: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let owner = cx.entity().downgrade();
        let can_annotate = individual
            && self
                .staged_attachments
                .iter()
                .any(|item| ids.contains(&item.id) && item.image().is_some());
        let can_convert = individual
            && self.staged_attachments.iter().any(|item| {
                item.id == ids[0]
                    && item.image().is_some_and(|image| !image.is_comparison)
                    && item.mime_type != "image/gif"
            });
        let id = if individual {
            format!("attachment-options-{}", ids[0])
        } else {
            "selected-attachment-options".into()
        };
        Button::new(id.clone())
            .debug_selector(move || id)
            .app_icon()
            .when(!individual, |button| button.small().w(px(28.)).h(px(28.)))
            .icon(AssetIconName::Ellipsis)
            .accessibility_label("Attachment options")
            .dropdown_menu(move |menu, _, _| {
                let mut menu = menu;
                if individual {
                    let view = owner.clone();
                    let selected = ids[0];
                    menu = menu.item(PopupMenuItem::new("Edit metadata").on_click(
                        move |_, window, cx| {
                            let _ = view.update(cx, |this, cx| {
                                this.open_description_dialog(selected, window, cx)
                            });
                        },
                    ));
                }
                if can_annotate {
                    let view = owner.clone();
                    let selected = ids.clone();
                    menu = menu.item(PopupMenuItem::new("Annotate").on_click(
                        move |_, window, cx| {
                            let _ = view.update(cx, |this, cx| {
                                this.annotation_selection = Some(selected.clone());
                                if let Some(id) = this
                                    .staged_attachments
                                    .iter()
                                    .find(|item| {
                                        selected.contains(&item.id) && item.image().is_some()
                                    })
                                    .map(|item| item.id)
                                {
                                    this.open_attachment_preview(id, window, cx);
                                }
                                this.annotation_selection = None;
                            });
                        },
                    ));
                }
                if can_convert {
                    let view = owner.clone();
                    let selected = ids[0];
                    menu = menu.item(
                        PopupMenuItem::new("Convert to side-by-side comparison").on_click(
                            move |_, _, cx| {
                                let _ = view.update(cx, |this, cx| {
                                    if !this.uploading
                                        && !this.import_busy()
                                        && let Some(source) = this
                                            .staged_attachments
                                            .iter()
                                            .find(|item| {
                                                item.id == selected
                                                    && item
                                                        .image()
                                                        .is_some_and(|image| !image.is_comparison)
                                            })
                                            .cloned()
                                    {
                                        cx.emit(ConvertAttachmentRequested(source));
                                    }
                                });
                            },
                        ),
                    );
                }
                let view = owner.clone();
                let selected = ids.clone();
                menu = menu.item(PopupMenuItem::new("Move to project").on_click(
                    move |_, window, cx| {
                        let _ = view.update(cx, |this, cx| {
                            this.open_move_dialog(selected.clone(), window, cx)
                        });
                    },
                ));
                let view = owner.clone();
                let selected = ids.clone();
                menu.item(PopupMenuItem::new("Remove").on_click(move |_, _, cx| {
                    let _ = view.update(cx, |this, cx| this.remove_attachments(&selected, cx));
                }))
            })
    }

    fn remove_attachments(&mut self, ids: &[u64], cx: &mut Context<Self>) {
        if self.uploading || self.import_busy() {
            return;
        }
        self.staged_attachments
            .retain(|item| !ids.contains(&item.id));
        self.selected_attachments.retain(|id| !ids.contains(id));
        self.sync_upload_result();
        self.save_project(cx);
        cx.notify();
    }

    pub(super) fn open_move_dialog(
        &mut self,
        ids: Vec<u64>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.uploading || self.import_busy() || self.project_state.saving {
            return;
        }
        let choices = self
            .project_state
            .folders
            .iter()
            .filter(|folder| self.project_state.folder.as_ref() != Some(*folder))
            .map(|folder| {
                let name = crate::projects::title(folder).unwrap_or_else(|_| {
                    folder
                        .file_name()
                        .unwrap_or(folder.as_os_str())
                        .to_string_lossy()
                        .into_owned()
                });
                (name, folder.clone())
            })
            .collect::<Vec<_>>();
        let names = choices
            .iter()
            .map(|(name, _)| name.clone())
            .collect::<Vec<_>>();
        let picker = (!choices.is_empty()).then(|| {
            cx.new(|cx| {
                ComboboxState::new(
                    SearchableVec::new(vec![SearchableGroup::new("Projects").items(names)]),
                    vec![],
                    window,
                    cx,
                )
                .searchable(true)
            })
        });
        let feedback = theme::DialogFeedback::default();
        let owner = cx.entity().downgrade();
        window.open_alert_dialog(cx, move |alert, _, cx| {
            use gpui_kit::component::dialog::{DialogAction, DialogClose, DialogFooter};
            let feedback = feedback.clone();
            let picker = picker.clone();
            let owner = owner.clone();
            let choices = choices.clone();
            let ids = ids.clone();
            let create_owner = owner.clone();
            let create_ids = ids.clone();
            alert
                .title("Move to project")
                .when_some(picker.as_ref(), |alert, picker| {
                    alert.child(
                        Combobox::new(picker)
                            .placeholder("Select a project")
                            .w_full(),
                    )
                })
                .children(feedback.element(cx))
                .footer(
                    DialogFooter::new()
                        .child(
                            div().w(px(88.)).h(px(32.)).child(
                                DialogClose::new().child(
                                    Button::new("cancel")
                                        .app_secondary()
                                        .label("Cancel")
                                        .w(px(88.))
                                        .h(px(32.)),
                                ),
                            ),
                        )
                        .child(
                            Button::new("create-move-destination")
                                .debug_selector(|| "create-move-destination".into())
                                .app_secondary()
                                .label("Create Project")
                                .h(px(32.))
                                .on_click(move |_, window, cx| {
                                    let _ = create_owner.update(cx, |this, cx| {
                                        if this.can_create_project() {
                                            window.close_dialog(cx);
                                            this.project_state.pending_move_to_new =
                                                Some(create_ids.clone());
                                            this.create_project(&CreateProject, window, cx);
                                        }
                                    });
                                }),
                        )
                        .when(picker.is_some(), |footer| {
                            footer.child(
                                div().w(px(88.)).h(px(32.)).child(
                                    DialogAction::new().child(
                                        Button::new("move-attachments")
                                            .app_primary()
                                            .label("Move")
                                            .w(px(88.))
                                            .h(px(32.)),
                                    ),
                                ),
                            )
                        }),
                )
                .on_ok(move |_, window, cx| {
                    let Some(picker) = &picker else {
                        return false;
                    };
                    let Some(name) = picker.read(cx).selected_values().into_iter().next() else {
                        feedback.show("Choose a project.", window);
                        return false;
                    };
                    let Some((_, target)) = choices.iter().find(|(label, _)| label == &name) else {
                        return false;
                    };
                    let _ = owner.update(cx, |this, cx| {
                        this.move_attachments_to_project(ids.clone(), target.clone(), window, cx)
                    });
                    true
                })
        });
    }
}

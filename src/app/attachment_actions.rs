use super::*;
use gpui_kit::component::menu::PopupMenuItem;

impl UploaderApp {
    pub(super) fn attachment_options(
        &self,
        attachment_id: u64,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let owner = cx.entity().downgrade();
        let saving = self.project_state.saving;
        let can_annotate = self
            .staged_attachments
            .iter()
            .any(|item| item.id == attachment_id && item.image().is_some());
        let can_convert = self.staged_attachments.iter().any(|item| {
            item.id == attachment_id
                && item.image().is_some_and(|image| !image.is_comparison)
                && item.mime_type != "image/gif"
        });
        let id = format!("attachment-options-{}", attachment_id);
        Button::new(id.clone())
            .debug_selector(move || id)
            .app_icon()
            .icon(AssetIconName::Ellipsis)
            .accessibility_label("Attachment options")
            .tooltip("Attachment options")
            .dropdown_menu(move |menu, _, _| {
                let mut menu = menu;
                {
                    let view = owner.clone();
                    let selected = attachment_id;
                    menu = menu.item(
                        PopupMenuItem::new("Edit metadata")
                            .icon(AssetIconName::Tags)
                            .on_click(move |_, window, cx| {
                                let _ = view.update(cx, |this, cx| {
                                    this.open_description_dialog(selected, window, cx)
                                });
                            }),
                    );
                }

                if can_annotate {
                    let view = owner.clone();
                    let selected = attachment_id;
                    menu = menu.item(
                        PopupMenuItem::new("Clone")
                            .icon(AssetIconName::ImagePlus)
                            .on_click(move |_, window, cx| {
                                let _ = view.update(cx, |this, cx| {
                                    if let Err(error) = this.clone_attachment(selected, window, cx)
                                    {
                                        this.diagnostics
                                            .error(format!("Image cloning failed: {error}"));
                                        this.warn(error.to_string(), window, cx);
                                    }
                                });
                            }),
                    );
                    let view = owner.clone();
                    let selected = vec![attachment_id];
                    menu = menu.item(
                        PopupMenuItem::new("Annotate")
                            .icon(AssetIconName::Paintbrush)
                            .on_click(move |_, window, cx| {
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
                            }),
                    );
                }
                if can_convert {
                    let view = owner.clone();
                    let selected = attachment_id;
                    menu = menu.item(
                        PopupMenuItem::element(|_, _| {
                            div()
                                .id("convert-comparison-label")
                                .debug_selector(|| "convert-comparison-label".into())
                                .min_w_0()
                                .max_w(px(180.))
                                .py_1()
                                .whitespace_normal()
                                .child("Convert to\nside-by-side comparison")
                        })
                        .icon(AssetIconName::Columns2)
                        .on_click(move |_, _, cx| {
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
                        }),
                    );
                }
                let view = owner.clone();
                let selected = vec![attachment_id];
                if !saving {
                    menu = menu.item(
                        PopupMenuItem::new("Move to project")
                            .icon(AssetIconName::FolderInput)
                            .on_click(move |_, window, cx| {
                                let _ = view.update(cx, |this, cx| {
                                    this.open_move_dialog(selected.clone(), window, cx)
                                });
                            }),
                    );
                }
                let view = owner.clone();
                let selected = vec![attachment_id];
                menu.item(
                    PopupMenuItem::new("Remove")
                        .icon(AssetIconName::Trash)
                        .on_click(move |_, _, cx| {
                            let _ =
                                view.update(cx, |this, cx| this.remove_attachments(&selected, cx));
                        }),
                )
            })
    }

    pub(super) fn selection_actions(
        &self,
        ids: Vec<u64>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let available = !ids.is_empty() && !self.uploading && !self.import_busy();
        let can_combine = available && self.selected_comparison_sources(&ids).is_some();
        let selected_copy = available
            .then(|| self.selected_upload_snippets(&ids))
            .flatten();
        h_flex()
            .items_center()
            .gap_2()
            .when(can_combine, |row| {
                let selected = ids.clone();
                row.child(
                    Button::new("combine-selected-attachments")
                        .debug_selector(|| "combine-selected-attachments".into())
                        .app_icon()
                        .icon(AssetIconName::Columns2)
                        .accessibility_label("Combine into side-by-side comparison")
                        .tooltip("Combine into side-by-side comparison")
                        .on_click(cx.listener(move |this, _, window, cx| {
                            if let Err(error) =
                                this.combine_selected_attachments(&selected, window, cx)
                            {
                                this.warn(error, window, cx);
                            }
                        })),
                )
            })
            .when_some(selected_copy, |row, snippet| {
                row.child(
                    Button::new("copy-selected-attachments")
                        .debug_selector(|| "copy-selected-attachments".into())
                        .app_icon()
                        .icon(AssetIconName::Copy)
                        .accessibility_label("Copy selected attachments")
                        .tooltip("Copy selected attachments")
                        .on_click(cx.listener(move |this, _, window, cx| {
                            cx.write_to_clipboard(ClipboardItem::new_string(snippet.clone()));
                            this.finish_copy(
                                "Selected attachments copied to the clipboard.",
                                window,
                                cx,
                            );
                        })),
                )
            })
            .when(available && !self.project_state.saving, |row| {
                let selected = ids.clone();
                row.child(
                    Button::new("move-selected-attachments")
                        .debug_selector(|| "move-selected-attachments".into())
                        .app_icon()
                        .icon(AssetIconName::FolderInput)
                        .accessibility_label("Move into project")
                        .tooltip("Move into project")
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.open_move_dialog(selected.clone(), window, cx);
                        })),
                )
            })
            .when(available, |row| {
                row.child(
                    Button::new("remove-selected-attachments")
                        .debug_selector(|| "remove-selected-attachments".into())
                        .app_icon()
                        .icon(AssetIconName::Trash)
                        .accessibility_label("Remove")
                        .tooltip("Remove")
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.remove_attachments(&ids, cx);
                        })),
                )
            })
    }

    fn selected_upload_snippets(&self, ids: &[u64]) -> Option<String> {
        if ids.is_empty() {
            return None;
        }
        let selected = self
            .ordered_attachments()
            .filter(|item| ids.contains(&item.id))
            .collect::<Vec<_>>();
        if selected.len() != ids.len() {
            return None;
        }
        let snippets = selected
            .into_iter()
            .map(|item| {
                item.upload_state
                    .uploaded_url()
                    .map(|url| item.snippet(url, self.settings.image_copy_format()))
            })
            .collect::<Option<Vec<_>>>()?;
        Some(snippets.join("\n\n"))
    }

    fn selected_comparison_sources(&self, ids: &[u64]) -> Option<[StagedAttachment; 2]> {
        if ids.len() != 2 || ids[0] == ids[1] {
            return None;
        }
        let mut selected = self
            .ordered_attachments()
            .filter(|item| ids.contains(&item.id));
        let sources = [selected.next()?, selected.next()?];
        sources
            .iter()
            .all(|item| {
                item.image().is_some_and(|image| !image.is_comparison)
                    && item.mime_type != "image/gif"
            })
            .then(|| sources.map(Clone::clone))
    }

    fn combine_selected_attachments(
        &mut self,
        ids: &[u64],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), &'static str> {
        let sources = self
            .selected_comparison_sources(ids)
            .ok_or("Select exactly two PNG, JPEG, or WebP images that aren't already side-by-side comparisons.")?;
        self.begin_comparison(sources, window, cx)
    }

    fn clone_attachment(
        &mut self,
        id: u64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> anyhow::Result<()> {
        anyhow::ensure!(
            !self.uploading && !self.import_busy(),
            "Finish the current upload or import before cloning an image."
        );
        let source = self
            .staged_attachments
            .iter()
            .find(|item| item.id == id)
            .ok_or_else(|| anyhow::anyhow!("That attachment is no longer staged."))?;
        let cloned = source.clone_image()?;
        self.append_staged_attachments(vec![cloned], window, cx);
        Ok(())
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
            .filter(|folder| self.project_state.storage_folder.as_ref() != Some(*folder))
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

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::TestAppContext;
    use gpui_kit::test::TestWindowExt as _;

    fn image() -> StagedAttachment {
        StagedAttachment::from_bytes(
            "photo.png".into(),
            include_bytes!("../../packaging/menu-bar-icon.png").to_vec(),
            None,
        )
        .unwrap()
    }

    fn solid_image(color: [u8; 4], format: image::ImageFormat) -> StagedAttachment {
        let pixels = image::RgbaImage::from_pixel(8, 8, image::Rgba(color));
        let mut bytes = std::io::Cursor::new(Vec::new());
        pixels.write_to(&mut bytes, format).unwrap();
        StagedAttachment::from_bytes("solid.png".into(), bytes.into_inner(), None).unwrap()
    }

    #[gpui_kit::test]
    fn selection_toolbar_combines_two_images_in_display_order(cx: &mut TestAppContext) {
        let (view, cx) = super::super::tests::upload_window(cx);
        let originals = cx.update(|window, cx| {
            let originals = view.update(cx, |app, cx| {
                app.append_staged_attachments(
                    vec![
                        solid_image([255, 0, 0, 255], image::ImageFormat::Png),
                        image(),
                        solid_image([0, 0, 255, 255], image::ImageFormat::Png),
                    ],
                    window,
                    cx,
                );
                let originals = app
                    .staged_attachments
                    .iter()
                    .map(|item| item.persistent_id)
                    .collect::<Vec<_>>();
                // Selection order must not change which image appears on the left.
                app.selected_attachments =
                    [app.staged_attachments[2].id, app.staged_attachments[0].id].into();
                cx.notify();
                originals
            });
            window.render_frame(cx);
            window.click("combine-selected-attachments", cx);
            assert!(view.read(cx).combining_comparison);
            originals
        });
        cx.run_until_parked();
        cx.update(|_, cx| {
            let app = view.read(cx);
            assert_eq!(app.staged_attachments.len(), 4);
            assert_eq!(
                app.staged_attachments[..3]
                    .iter()
                    .map(|item| item.persistent_id)
                    .collect::<Vec<_>>(),
                originals
            );
            let comparison = app.staged_attachments[3].image().unwrap();
            assert!(comparison.is_comparison);
            let document = comparison.annotation_document.as_ref().unwrap().document();
            for (panel, color) in document
                .comparison
                .as_ref()
                .unwrap()
                .panels
                .iter()
                .zip([[255, 0, 0, 255], [0, 0, 255, 255]])
            {
                let point = panel.center();
                assert_eq!(
                    document.base.get_pixel(point.x as u32, point.y as u32).0,
                    color
                );
            }
        });
    }

    #[gpui_kit::test]
    fn selection_toolbar_hides_unavailable_actions_and_removes_only_selected_items(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = super::super::tests::upload_window(cx);
        cx.update(|window, cx| {
            let ids = view.update(cx, |app, cx| {
                let video = StagedAttachment::from_path(
                    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample.mp4"),
                    Default::default(),
                )
                .unwrap();
                app.append_staged_attachments(
                    vec![
                        image(),
                        image(),
                        image(),
                        video,
                        solid_image([0, 0, 0, 255], image::ImageFormat::Gif),
                    ],
                    window,
                    cx,
                );
                app.staged_attachments
                    .iter()
                    .map(|item| item.id)
                    .collect::<Vec<_>>()
            });
            for (selection, can_combine) in [
                (vec![], false),
                (vec![ids[0]], false),
                (vec![ids[0], ids[1]], true),
                (vec![ids[0], ids[1], ids[2]], false),
                (vec![ids[0], ids[3]], false),
                (vec![ids[0], ids[4]], false),
            ] {
                let has_selection = !selection.is_empty();
                view.update(cx, |app, cx| {
                    app.selected_attachments = selection.into_iter().collect();
                    cx.notify();
                });
                window.render_frame(cx);
                assert_eq!(
                    window.try_find("combine-selected-attachments").is_some(),
                    can_combine
                );
                assert_eq!(
                    window.try_find("move-selected-attachments").is_some(),
                    has_selection
                );
                assert_eq!(
                    window.try_find("remove-selected-attachments").is_some(),
                    has_selection
                );
            }
            for uploading in [true, false] {
                view.update(cx, |app, cx| {
                    app.selected_attachments = ids[..2].iter().copied().collect();
                    app.uploading = uploading;
                    app.loading_attachment_paths = !uploading;
                    cx.notify();
                });
                window.render_frame(cx);
                for action in [
                    "combine-selected-attachments",
                    "move-selected-attachments",
                    "remove-selected-attachments",
                    "select-all-attachments",
                ] {
                    assert!(window.try_find(action).is_none());
                }
            }
            view.update(cx, |app, cx| {
                app.loading_attachment_paths = false;
                app.project_state.saving = true;
                cx.notify();
            });
            window.render_frame(cx);
            assert!(window.try_find("move-selected-attachments").is_none());
            assert!(window.try_find("combine-selected-attachments").is_some());
            view.update(cx, |app, cx| {
                app.project_state.saving = false;
                cx.notify();
            });
            window.render_frame(cx);
            window.click("remove-selected-attachments", cx);
            let app = view.read(cx);
            assert_eq!(
                app.staged_attachments
                    .iter()
                    .map(|item| item.id)
                    .collect::<Vec<_>>(),
                ids[2..]
            );
            assert!(app.selected_attachments.is_empty());
        });
    }

    #[gpui_kit::test]
    fn existing_comparisons_hide_combine_and_cannot_be_combined_again(cx: &mut TestAppContext) {
        let (view, cx) = super::super::tests::upload_window(cx);
        cx.update(|window, cx| {
            let ids = view.update(cx, |app, cx| {
                let mut images = vec![image(), image(), image(), image()];
                for index in [0, 3] {
                    if let Media::Image(image) = &mut images[index].media {
                        image.is_comparison = true;
                    }
                }
                app.append_staged_attachments(images, window, cx);
                app.staged_attachments
                    .iter()
                    .map(|item| item.id)
                    .collect::<Vec<_>>()
            });
            for selection in [[ids[0], ids[1]], [ids[2], ids[3]], [ids[0], ids[3]]] {
                view.update(cx, |app, cx| {
                    app.selected_attachments = selection.into();
                    cx.notify();
                });
                window.render_frame(cx);
                assert!(window.try_find("combine-selected-attachments").is_none());
                view.update(cx, |app, cx| {
                    assert!(
                        app.combine_selected_attachments(&selection, window, cx)
                            .is_err()
                    );
                    assert!(!app.combining_comparison);
                    assert_eq!(app.staged_attachments.len(), 4);
                });
            }
            view.update(cx, |app, cx| {
                app.selected_attachments = [ids[1], ids[2]].into();
                cx.notify();
            });
            window.render_frame(cx);
            assert!(window.try_find("combine-selected-attachments").is_some());
        });
    }

    #[gpui_kit::test]
    fn selected_copy_requires_only_uploaded_items_and_copies_only_the_selection(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = super::super::tests::upload_window(cx);
        cx.update(|window, cx| {
            let ids = view.update(cx, |app, cx| {
                app.settings.copy_images_as_markdown = true;
                app.append_staged_attachments(vec![image(), image(), image(), image()], window, cx);
                for (index, item) in app.staged_attachments.iter_mut().take(3).enumerate() {
                    item.title = format!("Image {index}");
                    item.upload_state = AttachmentUploadState::Uploaded {
                        url: format!("https://example.com/{index}"),
                    };
                }
                app.sync_upload_result();
                app.staged_attachments
                    .iter()
                    .map(|item| item.id)
                    .collect::<Vec<_>>()
            });
            for selection in [vec![], vec![ids[3]], vec![ids[0], ids[3]]] {
                view.update(cx, |app, cx| {
                    app.selected_attachments = selection.into_iter().collect();
                    cx.notify();
                });
                window.render_frame(cx);
                assert!(window.try_find("copy-selected-attachments").is_none());
            }
            view.update(cx, |app, cx| {
                app.selected_attachments = [ids[1], ids[0]].into();
                cx.notify();
            });
            window.render_frame(cx);
            window.click("copy-selected-attachments", cx);
            assert_eq!(
                cx.read_from_clipboard()
                    .and_then(|item| item.text())
                    .as_deref(),
                Some("![Image 0](https://example.com/0)\n\n![Image 1](https://example.com/1)")
            );
        });
    }

    #[gpui_kit::test]
    fn combining_selection_rejects_wrong_counts_unsupported_media_and_busy_work(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = super::super::tests::upload_window(cx);
        cx.update(|window, cx| {
            view.update(cx, |app, cx| {
                let video = StagedAttachment::from_path(
                    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample.mp4"),
                    Default::default(),
                )
                .unwrap();
                app.append_staged_attachments(
                    vec![
                        image(),
                        image(),
                        image(),
                        video,
                        solid_image([255, 0, 0, 255], image::ImageFormat::Gif),
                    ],
                    window,
                    cx,
                );
                let ids = app
                    .staged_attachments
                    .iter()
                    .map(|item| item.id)
                    .collect::<Vec<_>>();
                for selected in [
                    vec![],
                    vec![ids[0]],
                    vec![ids[0], ids[1], ids[2]],
                    vec![ids[0], ids[0]],
                    vec![ids[0], ids[3]],
                    vec![ids[0], ids[4]],
                    vec![ids[0], u64::MAX],
                ] {
                    assert!(
                        app.combine_selected_attachments(&selected, window, cx)
                            .is_err()
                    );
                    assert!(!app.combining_comparison);
                }
                app.uploading = true;
                assert!(
                    app.combine_selected_attachments(&ids[..2], window, cx)
                        .is_err()
                );
                app.uploading = false;
                app.loading_attachment_paths = true;
                assert!(
                    app.combine_selected_attachments(&ids[..2], window, cx)
                        .is_err()
                );
                app.loading_attachment_paths = false;
                assert_eq!(app.staged_attachments.len(), 5);
            })
        });
    }

    #[gpui_kit::test]
    fn image_options_clone_click_adds_another_attachment(cx: &mut TestAppContext) {
        let (view, cx) = super::super::tests::upload_window(cx);
        cx.update(|window, cx| {
            let id = view.update(cx, |app, cx| {
                app.append_staged_attachments(vec![image()], window, cx);
                app.staged_attachments[0].id
            });
            window.render_frame(cx);
            window.click(format!("attachment-options-{id}"), cx);
            window.within("popup-menu").click(1usize, cx);
            let app = view.read(cx);
            assert_eq!(app.staged_attachments.len(), 2);
            let cloned = &app.staged_attachments[1];
            assert_eq!(cloned.name, "photo 2.png");
            assert!(
                window
                    .try_find(format!("attachment-options-{}", cloned.id))
                    .is_some()
            );
        });
    }

    #[gpui_kit::test]
    fn comparison_menu_item_wraps_and_dispatches_conversion(cx: &mut TestAppContext) {
        let (view, cx) = super::super::tests::upload_window(cx);
        let requests = Rc::new(Cell::new(0));
        let captured = requests.clone();
        cx.update(|window, cx| {
            cx.subscribe(&view, move |_, _: &ConvertAttachmentRequested, _| {
                captured.set(captured.get() + 1);
            })
            .detach();
            let id = view.update(cx, |app, cx| {
                app.append_staged_attachments(vec![image()], window, cx);
                app.staged_attachments[0].id
            });
            window.render_frame(cx);
            window.click(format!("attachment-options-{id}"), cx);
            let mut menu = window.within("popup-menu");
            let edit = menu.find(0usize).bounds();
            let convert = menu.find(3usize).bounds();
            let next = menu.find(4usize).bounds();
            assert!(convert.size.height > edit.size.height);
            assert!(convert.bottom() <= next.top());
            menu.click(3usize, cx);
        });
        cx.run_until_parked();
        assert_eq!(requests.get(), 1);
    }

    #[gpui_kit::test]
    fn cloning_and_importing_images_share_incrementing_names(cx: &mut TestAppContext) {
        let (view, cx) = super::super::tests::upload_window(cx);
        cx.update(|window, cx| {
            view.update(cx, |app, cx| {
                app.append_staged_attachments(vec![image(), image()], window, cx);
                let id = app.staged_attachments[0].id;
                let second_id = app.staged_attachments[1].id;
                app.staged_attachments[0].upload_state = AttachmentUploadState::Uploaded {
                    url: "https://example.com/original".into(),
                };
                app.staged_attachments[0].description = "Keep this description".into();
                app.clone_attachment(id, window, cx).unwrap();
                let cloned_id = app.staged_attachments[2].id;
                app.clone_attachment(cloned_id, window, cx).unwrap();
                app.clone_attachment(second_id, window, cx).unwrap();
                app.append_staged_attachments(vec![image()], window, cx);
                assert_eq!(
                    app.staged_attachments
                        .iter()
                        .map(|item| item.name.as_str())
                        .collect::<Vec<_>>(),
                    [
                        "photo.png",
                        "photo 2.png",
                        "photo 3.png",
                        "photo 4.png",
                        "photo 5.png",
                        "photo 6.png"
                    ]
                );
                let original = &app.staged_attachments[0];
                let cloned = &app.staged_attachments[2];
                assert_ne!(original.id, cloned.id);
                assert_ne!(original.persistent_id, cloned.persistent_id);
                assert_eq!(cloned.title, "photo 3.png");
                assert_eq!(cloned.description, original.description);
                assert_eq!(cloned.upload_state, AttachmentUploadState::Preview);
                assert_eq!(
                    cloned.image().unwrap().bytes,
                    original.image().unwrap().bytes
                );
                assert!(original.upload_state.is_uploaded());
                assert_eq!(app.upload_result.as_ref().unwrap().attachments.len(), 1);
            });
        });
    }

    #[gpui_kit::test]
    fn cloning_is_blocked_while_busy_and_duplicate_videos_are_renamed_but_not_cloned(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = super::super::tests::upload_window(cx);
        let video = StagedAttachment::from_path(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample.mp4"),
            Default::default(),
        )
        .unwrap();
        cx.update(|window, cx| {
            view.update(cx, |app, cx| {
                app.append_staged_attachments(
                    vec![image(), video.clone(), video.clone()],
                    window,
                    cx,
                );
                app.append_staged_attachments(vec![video], window, cx);
                let id = app.staged_attachments[0].id;
                app.uploading = true;
                assert!(app.clone_attachment(id, window, cx).is_err());
                app.uploading = false;
                app.loading_attachment_paths = true;
                assert!(app.clone_attachment(id, window, cx).is_err());
                app.loading_attachment_paths = false;
                let video_id = app.staged_attachments[1].id;
                assert!(app.clone_attachment(video_id, window, cx).is_err());
                assert!(app.clone_attachment(u64::MAX, window, cx).is_err());
                assert_eq!(app.staged_attachments.len(), 4);
                assert_eq!(app.staged_attachments[1].name, "sample.mp4");
                assert_eq!(app.staged_attachments[2].name, "sample 2.mp4");
                assert_eq!(app.staged_attachments[2].title, "sample 2.mp4");
                assert_eq!(app.staged_attachments[3].name, "sample 3.mp4");
                assert_eq!(app.staged_attachments[3].title, "sample 3.mp4");
            });
        });
    }

    #[gpui_kit::test]
    fn cloned_annotations_are_independent_and_survive_project_reopen(cx: &mut TestAppContext) {
        let (view, cx) = super::super::tests::upload_window(cx);
        let directory = tempfile::tempdir().unwrap();
        let folder = directory.path().join("Clones");
        let (mut project, _) = crate::projects::open_scratchpad(folder.clone()).unwrap();
        cx.update(|window, cx| {
            view.update(cx, |app, cx| {
                let source = image();
                let data = source.image().unwrap();
                let mut retained = data
                    .annotation_session
                    .open(&data.bytes, data.preview.clone())
                    .unwrap();
                retained.document.marks.push(crate::annotation::Mark::new(
                    crate::annotation::Tool::Rectangle,
                    crate::annotation::Pos::new(4., 8.),
                    Default::default(),
                ));
                let saved = retained.document.edit_state();
                data.annotation_session
                    .retain(retained.document, retained.image, saved);
                let id = source.id;
                app.append_staged_attachments(vec![source], window, cx);
                app.clone_attachment(id, window, cx).unwrap();
                project.save(&app.staged_attachments).unwrap();
                let clone = app.staged_attachments[1].image().unwrap();
                let mut retained = clone
                    .annotation_session
                    .open(&clone.bytes, clone.preview.clone())
                    .unwrap();
                assert_eq!(retained.document.marks.len(), 1);
                retained.document.marks.clear();
                let saved = retained.document.edit_state();
                clone
                    .annotation_session
                    .retain(retained.document, retained.image, saved);
                let original = app.staged_attachments[0].image().unwrap();
                let retained = original
                    .annotation_session
                    .open(&original.bytes, original.preview.clone())
                    .unwrap();
                assert_eq!(
                    retained.document.marks.len(),
                    1,
                    "The clone must not share mutable editor state"
                );
                let saved = retained.document.edit_state();
                original
                    .annotation_session
                    .retain(retained.document, retained.image, saved);
            });
        });
        cx.run_until_parked();
        let (_, items) = crate::projects::open_scratchpad(folder).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name, "photo.png");
        assert_eq!(items[1].name, "photo 2.png");
        assert_ne!(items[0].persistent_id, items[1].persistent_id);
        let data = items[1].image().unwrap();
        let retained = data
            .annotation_session
            .open(&data.bytes, data.preview.clone())
            .unwrap();
        assert_eq!(retained.document.marks.len(), 1);
    }
}

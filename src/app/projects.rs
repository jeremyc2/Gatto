use super::*;
use crate::projects::{Project, ProjectSummary, WorkspaceMetadata};
use gpui_kit::component::{
    Collapsible,
    menu::{DropdownMenu as _, PopupMenuItem},
    sidebar::{Sidebar, SidebarItem, SidebarMenuItem},
};

enum ProjectWrite {
    Save {
        revision: u64,
        attachments: Vec<StagedAttachment>,
        metadata: WorkspaceMetadata,
    },
    Flush(async_channel::Sender<Result<(), String>>),
    SetTitle(String),
    ForgetAttachment(u64),
}

#[derive(Clone)]
struct ProjectRows {
    rows: Vec<(PathBuf, SidebarMenuItem)>,
    bounds: Rc<RefCell<HashMap<PathBuf, Bounds<gpui_kit::Pixels>>>>,
    active: Option<PathBuf>,
}
impl Collapsible for ProjectRows {
    fn is_collapsed(&self) -> bool {
        false
    }
    fn collapsed(self, _: bool) -> Self {
        self
    }
}
impl SidebarItem for ProjectRows {
    fn render(
        self,
        id: impl Into<gpui_kit::ElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        let id = id.into();
        self.bounds.borrow_mut().clear();
        v_flex()
            .gap_2()
            .children(
                self.rows
                    .into_iter()
                    .enumerate()
                    .map(|(index, (folder, item))| {
                        let layout = self.bounds.clone();
                        let selected = self.active.as_ref() == Some(&folder);
                        let can_drop = !selected;
                        div()
                            .id(format!("{id}-project-drop-{index}"))
                            .relative()
                            .w_full()
                            .rounded_lg()
                            .border_1()
                            .border_color(cx.theme().border.opacity(0.))
                            .when(can_drop, |row| {
                                row.drag_over::<attachment_drag::AttachmentDrag>(
                                    |style, _, _, cx| {
                                        style
                                            .border_color(cx.theme().primary)
                                            .bg(cx.theme().primary.opacity(0.12))
                                    },
                                )
                            })
                            .child(item.render(format!("{id}-project-item-{index}"), window, cx))
                            .child(
                                gpui_kit::canvas(
                                    move |bounds, _, _| {
                                        layout.borrow_mut().insert(folder, bounds);
                                    },
                                    |_, _, _, _| {},
                                )
                                .absolute()
                                .inset_0(),
                            )
                    }),
            )
    }
}

#[derive(Default)]
pub(super) struct ProjectState {
    pub folders: Vec<PathBuf>,
    pub folder: Option<PathBuf>,
    pub title: Option<String>,
    pub metadata: WorkspaceMetadata,
    pub storage_folder: Option<PathBuf>,
    scratchpad_folder: Option<PathBuf>,
    pending_destination: Option<PathBuf>,
    pub(super) pending_move_to_new: Option<Vec<u64>>,
    titles: HashMap<PathBuf, String>,
    summaries: HashMap<PathBuf, ProjectSummary>,
    search: Option<Entity<InputState>>,
    active: Option<async_channel::Sender<ProjectWrite>>,
    revision: u64,
    pub loading: bool,
    pub saving: bool,
    pub error: Option<String>,
}

impl UploaderApp {
    pub(super) fn initialize_workspaces(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("Search projects"));
        cx.subscribe(&search, |_, _, _: &InputEvent, cx| cx.notify())
            .detach();
        self.project_state.search = Some(search);
        self.refresh_project_names(cx);
        if self.persist_preferences {
            match crate::settings::scratchpad_directory() {
                Ok(path) => self.load_workspace(path, true, window, cx),
                Err(error) => self.project_state.error = Some(error.to_string()),
            }
        }
    }

    pub(super) fn refresh_project_names(&self, cx: &mut Context<Self>) {
        let folders = self.project_state.folders.clone();
        let directory = self
            .persist_preferences
            .then(|| self.settings.project_directory().ok())
            .flatten();
        let task = cx.background_spawn(async move {
            let paths = directory
                .as_deref()
                .map(|directory| crate::projects::project_folders(directory).unwrap_or_default())
                .unwrap_or(folders);
            paths
                .into_iter()
                .filter_map(|path| {
                    crate::projects::summary(&path)
                        .ok()
                        .map(|summary| (path, summary))
                })
                .collect::<HashMap<_, _>>()
        });
        cx.spawn(async move |this, cx| {
            let summaries = task.await;
            let _ =
                this.update(cx, |this, cx| {
                    this.project_state.folders = summaries.keys().cloned().collect();
                    this.project_state
                        .titles
                        .retain(|path, _| summaries.contains_key(path));
                    this.project_state
                        .summaries
                        .retain(|path, _| summaries.contains_key(path));
                    for (path, summary) in summaries {
                        if this.project_state.summaries.get(&path).is_none_or(|old| {
                            old.metadata.updated_at <= summary.metadata.updated_at
                        }) {
                            this.project_state
                                .titles
                                .insert(path.clone(), summary.title.clone());
                            this.project_state.summaries.insert(path, summary);
                        }
                    }
                    this.sort_projects(cx);
                    cx.notify();
                });
        })
        .detach();
    }

    fn sort_projects(&mut self, cx: &mut Context<Self>) {
        self.project_state.folders.sort_by(|a, b| {
            let updated = |path| {
                self.project_state
                    .summaries
                    .get(path)
                    .map_or(0, |s| s.metadata.updated_at)
            };
            updated(b).cmp(&updated(a)).then_with(|| a.cmp(b))
        });
        crate::application_menu::set_recent_projects(
            self.project_state
                .folders
                .iter()
                .take(5)
                .map(|path| (self.project_label(path), path.clone()))
                .collect(),
            cx,
        );
    }

    fn project_label(&self, folder: &std::path::Path) -> String {
        self.project_state
            .titles
            .get(folder)
            .cloned()
            .unwrap_or_else(|| {
                folder
                    .file_name()
                    .unwrap_or(folder.as_os_str())
                    .to_string_lossy()
                    .into_owned()
            })
    }

    pub(super) fn can_create_project(&self) -> bool {
        !self.settings_open && !self.creation_busy()
    }

    fn close_clean_preview(&mut self, cx: &mut Context<Self>) {
        if self.preview_open
            && let Some((handle, preview)) = self.preview_window.clone()
            && !preview.read(cx).has_pending_edits()
        {
            self.preview_open = false;
            let _ = handle.update(cx, |_, window, cx| {
                preview.update(cx, |preview, cx| preview.cancel_and_close(window, cx))
            });
        }
    }

    fn creation_busy(&self) -> bool {
        self.uploading || self.import_busy() || self.project_state.saving || self.preview_open
    }

    pub(super) fn create_project(
        &mut self,
        _: &CreateProject,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.close_clean_preview(cx);
        if !self.can_create_project() {
            self.pending_work_dialog(None, window, cx);
            return;
        }
        let input = cx.new(|cx| InputState::new(window, cx));
        let feedback = theme::DialogFeedback::default();
        let clear_feedback = feedback.clone();
        cx.subscribe_in(
            &input,
            window,
            move |_, _, event: &InputEvent, window, _| {
                if matches!(event, InputEvent::Change) {
                    clear_feedback.clear(window);
                }
            },
        )
        .detach();
        let description = cx.new(|cx| TextareaState::new(window, cx).rows(4));
        let clear_feedback = feedback.clone();
        cx.subscribe_in(
            &description,
            window,
            move |_, _, event: &InputEvent, window, _| {
                if matches!(event, InputEvent::Change) {
                    clear_feedback.clear(window);
                }
            },
        )
        .detach();
        let transfer = Rc::new(Cell::new(
            self.project_state.folder.is_none() && !self.staged_attachments.is_empty(),
        ));
        let offer_transfer = transfer.get() && self.project_state.pending_move_to_new.is_none();
        let cleanup_owner = cx.entity().downgrade();
        let focus = Rc::new(Cell::new(true));
        let owner = cx.entity().downgrade();
        window.open_alert_dialog(cx, move |alert, window, cx| {
            if focus.replace(false) {
                input.update(cx, |input, cx| input.focus(window, cx));
            }
            let feedback = feedback.clone();
            let input = input.clone();
            let description = description.clone();
            let transfer = transfer.clone();
            let owner = owner.clone();
            alert
                .width(px(440.))
                .title("Create project")
                .show_cancel(true)
                .ok_text("Create")
                .child(
                    v_flex()
                        .gap_3()
                        .children(feedback.element(cx))
                        .child(
                            v_flex()
                                .gap_1()
                                .child(div().text_sm().child("Project name"))
                                .child(
                                    div()
                                        .id("project-name-field")
                                        .debug_selector(|| "project-name-field".into())
                                        .child(
                                            Input::new(&input).w_full().aria_label("Project name"),
                                        ),
                                ),
                        )
                        .child(
                            v_flex()
                                .gap_1()
                                .child(div().text_sm().child("Description (optional)"))
                                .child(
                                    div()
                                        .id("project-description-field")
                                        .debug_selector(|| "project-description-field".into())
                                        .child(
                                            Textarea::new(&description)
                                                .aria_label("Project description"),
                                        ),
                                ),
                        )
                        .when(offer_transfer, |form| {
                            let transfer = transfer.clone();
                            form.child(
                                Checkbox::new("transfer-scratchpad")
                                    .debug_selector(|| "transfer-scratchpad".into())
                                    .label("Transfer Scratchpad attachments")
                                    .checked(transfer.get())
                                    .on_click(move |checked, _, _| transfer.set(*checked)),
                            )
                        }),
                )
                .footer(theme::confirmation_footer(
                    "confirm-project-name",
                    "Create",
                    104.,
                ))
                .on_ok(move |_, window, cx| {
                    let name = input.read(cx).value().to_string();
                    let description = description.read(cx).value().to_string();
                    match owner.update(cx, |this, cx| {
                        this.create_named_project(
                            &name,
                            description,
                            transfer.get(),
                            feedback.clone(),
                            window,
                            cx,
                        )
                    }) {
                        Ok(Ok(())) => false,
                        Ok(Err(error)) => {
                            feedback.show(error, window);
                            false
                        }
                        Err(_) => true,
                    }
                })
                .on_close({
                    let owner = cleanup_owner.clone();
                    move |_, _, cx| {
                        let _ = owner
                            .update(cx, |this, _| this.project_state.pending_move_to_new = None);
                    }
                })
        });
    }

    fn create_named_project(
        &mut self,
        name: &str,
        description: String,
        transfer: bool,
        feedback: theme::DialogFeedback,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> anyhow::Result<()> {
        anyhow::ensure!(!self.creation_busy(), "Finish the current work first.");
        let parent = self.settings.project_directory()?;
        let folder = crate::projects::project_folder(&parent, name)?;
        let name = crate::projects::distinct_name(
            name,
            &folder,
            &self.project_state.folders,
            &self.project_state.titles,
        )?;
        let moving = self.project_state.pending_move_to_new.clone();
        let recovering = self.project_state.error.is_some();
        let transferring = (transfer && self.project_state.folder.is_none()) || recovering;
        let attachments = if let Some(ids) = &moving {
            self.staged_attachments
                .iter()
                .filter(|item| ids.contains(&item.id))
                .cloned()
                .collect()
        } else if transferring {
            self.staged_attachments.clone()
        } else {
            Vec::new()
        };
        let mut metadata = if transferring || moving.is_some() {
            self.project_state.metadata.clone()
        } else {
            WorkspaceMetadata::default()
        };
        metadata.description = (!description.trim().is_empty()).then_some(description);
        metadata.created_at = crate::projects::timestamp();
        metadata.updated_at = metadata.created_at;
        let source = if (transferring || moving.is_some()) && !recovering {
            self.project_state.storage_folder.clone()
        } else {
            None
        };
        let remaining = if let Some(ids) = &moving {
            self.staged_attachments
                .iter()
                .filter(|item| !ids.contains(&item.id))
                .cloned()
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        self.project_state.loading = true;
        let task = cx.background_spawn(async move {
            std::fs::create_dir_all(&parent)?;
            let mut project = Project::create_in_parent(&parent, &name, &attachments)?;
            project.metadata = metadata;
            project.save(&attachments)?;
            if let Some(source) = source {
                let result =
                    Project::open(source).and_then(|(mut scratchpad, _)| scratchpad.save(&remaining));
                if let Err(error) = result {
                    if let Err(cleanup) = crate::projects::delete_project(&project.folder) {
                        anyhow::bail!("Could not transfer attachments: {error}. A copy remains at {}: {cleanup}", project.folder.display());
                    }
                    return Err(error);
                }
            }
            Ok::<_, anyhow::Error>((project, attachments))
        });
        cx.spawn_in(window, async move |this, window| {
            let result = task.await;
            let _ = this.update_in(window, |this, window, cx| {
                this.project_state.loading = false;
                match result {
                    Ok((project, attachments)) => {
                        this.project_state.pending_move_to_new = None;
                        if let Some(ids) = &moving {
                            this.remember_project(project.folder.clone());
                            this.project_state
                                .titles
                                .insert(project.folder, project.title);
                            this.staged_attachments
                                .retain(|item| !ids.contains(&item.id));
                            this.selected_attachments.retain(|id| !ids.contains(id));
                            for id in ids {
                                if let Some(writer) = &this.project_state.active {
                                    let _ = writer.try_send(ProjectWrite::ForgetAttachment(*id));
                                }
                            }
                            this.sync_upload_result();
                            this.save_project(cx);
                            this.refresh_project_names(cx);
                        } else {
                            this.install_workspace(project, attachments, false, window, cx);
                        }
                        window.close_dialog(cx);
                    }
                    Err(error) => feedback.show(error, window),
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
        Ok(())
    }

    pub(super) fn edit_project_metadata(
        &mut self,
        folder: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.project_navigation_busy() {
            self.pending_work_dialog(None, window, cx);
            return;
        }
        let name = self.project_label(&folder);
        let description = self
            .project_state
            .summaries
            .get(&folder)
            .and_then(|summary| summary.metadata.description.clone())
            .unwrap_or_default();
        let name_input = cx.new(|cx| InputState::new(window, cx).default_value(name));
        let description_input = cx.new(|cx| {
            TextareaState::new(window, cx)
                .rows(4)
                .default_value(description)
        });
        let feedback = theme::DialogFeedback::default();
        let clear = feedback.clone();
        cx.subscribe_in(
            &name_input,
            window,
            move |_, _, event: &InputEvent, window, _| {
                if matches!(event, InputEvent::Change) {
                    clear.clear(window);
                }
            },
        )
        .detach();
        let clear = feedback.clone();
        cx.subscribe_in(
            &description_input,
            window,
            move |_, _, event: &InputEvent, window, _| {
                if matches!(event, InputEvent::Change) {
                    clear.clear(window);
                }
            },
        )
        .detach();
        let owner = cx.entity().downgrade();
        window.open_alert_dialog(cx, move |alert, _window, cx| {
            let name_input = name_input.clone();
            let description_input = description_input.clone();
            let feedback = feedback.clone();
            let folder = folder.clone();
            let owner = owner.clone();
            alert
                .width(px(440.))
                .title("Edit metadata")
                .show_cancel(true)
                .ok_text("Save")
                .child(
                    v_flex()
                        .gap_3()
                        .children(feedback.element(cx))
                        .child(
                            v_flex()
                                .gap_1()
                                .child(div().text_sm().child("Project name"))
                                .child(
                                    div()
                                        .id("project-name-field")
                                        .debug_selector(|| "project-name-field".into())
                                        .child(
                                            Input::new(&name_input)
                                                .w_full()
                                                .aria_label("Project name"),
                                        ),
                                ),
                        )
                        .child(
                            v_flex()
                                .gap_1()
                                .child(div().text_sm().child("Description (optional)"))
                                .child(
                                    div()
                                        .id("project-description-field")
                                        .debug_selector(|| "project-description-field".into())
                                        .child(
                                            Textarea::new(&description_input)
                                                .aria_label("Project description"),
                                        ),
                                ),
                        ),
                )
                .footer(theme::confirmation_footer(
                    "save-project-metadata",
                    "Save",
                    104.,
                ))
                .on_ok(move |_, window, cx| {
                    let name = name_input.read(cx).value().to_string();
                    let description = description_input.read(cx).value().to_string();
                    let description = (!description.trim().is_empty()).then_some(description);
                    match owner.update(cx, |this, cx| {
                        let directory = this.settings.project_directory()?;
                        let folder = crate::projects::project_in_directory(&directory, &folder)?;
                        let name = crate::projects::edit_metadata(
                            &folder,
                            &name,
                            description.as_deref(),
                            &this.project_state.folders,
                            &this.project_state.titles,
                        )?;
                        this.project_state
                            .titles
                            .insert(folder.clone(), name.clone());
                        if this.project_state.folder.as_ref() == Some(&folder) {
                            this.project_state.title = Some(name.clone());
                            this.project_state.metadata.description = description;
                            if let Some(writer) = &this.project_state.active {
                                let _ = writer.try_send(ProjectWrite::SetTitle(name));
                            }
                            this.save_project(cx);
                        }
                        this.refresh_project_names(cx);
                        Ok::<_, anyhow::Error>(())
                    }) {
                        Ok(Ok(())) => true,
                        Ok(Err(error)) => {
                            feedback.show(error, window);
                            false
                        }
                        Err(_) => true,
                    }
                })
        });
    }

    pub(super) fn move_attachment_to_project(
        &mut self,
        id: u64,
        target: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.move_attachments_to_project(vec![id], target, window, cx);
    }

    pub(super) fn move_attachments_to_project(
        &mut self,
        ids: Vec<u64>,
        target: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(target) = self.resolve_project_folder(target, window, cx) else {
            return;
        };
        self.close_clean_preview(cx);
        if self.project_navigation_busy() || self.project_state.folder.as_ref() == Some(&target) {
            return;
        }
        let attachments = self
            .staged_attachments
            .iter()
            .filter(|item| ids.contains(&item.id))
            .cloned()
            .collect::<Vec<_>>();
        if attachments.is_empty() {
            return;
        }
        let remaining = self
            .staged_attachments
            .iter()
            .filter(|item| !ids.contains(&item.id))
            .cloned()
            .collect::<Vec<_>>();
        let source = self.project_state.storage_folder.clone();
        self.project_state.loading = true;
        let task = cx.background_spawn(async move {
            crate::projects::move_attachments(source.as_deref(), &target, &attachments, &remaining)
        });
        cx.spawn_in(window, async move |this, window| {
            let result = task.await;
            let _ = this.update_in(window, |this, window, cx| {
                this.project_state.loading = false;
                match result {
                    Ok(_) => {
                        this.staged_attachments
                            .retain(|item| !ids.contains(&item.id));
                        for id in &ids {
                            this.selected_attachments.remove(id);
                            if let Some(writer) = &this.project_state.active {
                                let _ = writer.try_send(ProjectWrite::ForgetAttachment(*id));
                            }
                        }
                        this.sync_upload_result();
                        this.save_project(cx);
                        this.refresh_project_names(cx);
                    }
                    Err(error) => {
                        window.push_notification(Notification::error(error.to_string()), cx)
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn project_navigation_busy(&self) -> bool {
        self.uploading
            || self.import_busy()
            || self.project_state.saving
            || self.project_state.error.is_some()
            || self.preview_open
    }

    pub(super) fn pending_work_dialog(
        &mut self,
        target: Option<Option<PathBuf>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let saving = self.project_state.saving
            || self.project_state.loading
            || self.project_state.error.is_some();
        let preview = self.preview_open;
        let owner = cx.entity().downgrade();
        window.open_alert_dialog(cx, move |alert, _, _| {
            let owner = owner.clone();
            let target = target.clone();
            alert
                .title("Work in progress")
                .description(if saving {
                    "Saving changes. Try again shortly."
                } else if preview {
                    "Discard unsaved edits?"
                } else {
                    "Cancel current work?"
                })
                .show_cancel(!saving)
                .cancel_text("Continue working")
                .ok_text(if saving { "OK" } else { "Cancel work" })
                .on_ok(move |_, window, cx| {
                    if !saving {
                        let _ = owner.update(cx, |this, cx| {
                            if let Some((handle, view)) = this.preview_window.clone() {
                                this.preview_open = false;
                                let _ = handle.update(cx, |_, window, cx| {
                                    view.update(cx, |view, cx| view.cancel_and_close(window, cx))
                                });
                            }
                            this.cancel_workspace_work(window, cx);
                            if let Some(target) = &target {
                                this.switch_after_save(target.clone(), window, cx);
                            }
                        });
                    }
                    true
                })
        });
    }

    fn switch_after_save(
        &mut self,
        target: Option<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let writer = self.project_state.active.clone();
        cx.spawn_in(window, async move |this, window| {
            let result = if let Some(writer) = writer {
                let (done, completed) = async_channel::bounded(1);
                if writer.send(ProjectWrite::Flush(done)).await.is_ok() {
                    completed
                        .recv()
                        .await
                        .unwrap_or_else(|_| Err("Saving stopped.".into()))
                } else {
                    Err("Saving stopped.".into())
                }
            } else {
                Ok(())
            };
            let _ = this.update_in(window, |this, window, cx| {
                match result {
                    Ok(()) => {
                        this.project_state.saving = false;
                        this.request_workspace(target, window, cx);
                    }
                    Err(error) => this.project_state.error = Some(error),
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn cancel_workspace_work(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.attachment_drag.is_some() {
            self.cancel_attachment_reorder(window, cx);
        }
        if let Some(control) = self.upload_control.take() {
            control.cancel();
        }
        if let Some(job) = self.compression.take() {
            job.control.cancel();
        }
        self.invalidate_project_callbacks();
        self.uploading = false;
        self.pending_import = None;
        self.loading_attachment_paths = false;
        self.combining_comparison = false;
        for item in &mut self.staged_attachments {
            if !item.upload_state.is_uploaded() {
                item.upload_state = AttachmentUploadState::Preview;
            }
        }
        self.sync_upload_result();
        self.save_project(cx);
    }

    fn scratchpad_path(&self) -> anyhow::Result<PathBuf> {
        self.project_state
            .scratchpad_folder
            .clone()
            .map(Ok)
            .unwrap_or_else(crate::settings::scratchpad_directory)
    }

    fn resolve_project_folder(
        &self,
        folder: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<PathBuf> {
        let result = self
            .settings
            .project_directory()
            .and_then(|directory| crate::projects::project_in_directory(&directory, &folder));
        match result {
            Ok(folder) => Some(folder),
            Err(error) => {
                window.push_notification(Notification::error(error.to_string()), cx);
                None
            }
        }
    }

    pub(crate) fn request_workspace(
        &mut self,
        target: Option<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if target.as_ref().is_some_and(|path| !path.exists()) {
            window.push_notification(Notification::error("This project has been deleted."), cx);
            self.refresh_project_names(cx);
            return;
        }
        let target = if let Some(folder) = target {
            let Some(folder) = self.resolve_project_folder(folder, window, cx) else {
                return;
            };
            Some(folder)
        } else {
            None
        };
        if target == self.project_state.folder {
            return;
        }
        if self.project_state.loading && self.project_state.storage_folder.is_none() {
            self.project_state.pending_destination = target;
            return;
        }
        self.close_clean_preview(cx);
        if self.project_navigation_busy() {
            self.pending_work_dialog(Some(target), window, cx);
            return;
        }
        match target {
            Some(path) => self.load_workspace(path, false, window, cx),
            None => match self.scratchpad_path() {
                Ok(path) => self.load_workspace(path, true, window, cx),
                Err(error) => window.push_notification(Notification::error(error.to_string()), cx),
            },
        }
    }

    fn load_workspace(
        &mut self,
        folder: PathBuf,
        scratchpad: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.project_state.loading = true;
        self.invalidate_project_callbacks();
        let folders = self.project_state.folders.clone();
        let known = self.project_state.titles.clone();
        let fallback_repository = self.settings.preferred_repository().map(|name| Repository {
            name: name.to_owned(),
            id: self.settings.last_repository_id.unwrap_or_default(),
        });
        let fallback_owner = self.settings.owner.clone();
        let missing_folder = folder.clone();
        let task = cx.background_spawn(async move {
            let initial = scratchpad && !folder.join("gatto-project.json").exists();
            let (mut project, attachments) = if scratchpad {
                crate::projects::open_scratchpad(folder)?
            } else {
                Project::open(folder)?
            };
            if initial {
                project.metadata.repository = fallback_repository;
                project.metadata.repository_owner = fallback_owner;
                project.save(&attachments)?;
            }
            if !scratchpad {
                crate::projects::distinct_name(&project.title, &project.folder, &folders, &known)?;
            }
            Ok::<_, anyhow::Error>((project, attachments))
        });
        cx.spawn_in(window, async move |this, window| {
            let result = task.await;
            let _ = this.update_in(window, |this, window, cx| {
                match result {
                    Ok((project, attachments)) => {
                        this.install_workspace(project, attachments, scratchpad, window, cx)
                    }
                    Err(error) => {
                        this.project_state.loading = false;
                        if !scratchpad && !missing_folder.exists() {
                            window.push_notification(
                                Notification::error("This project has been deleted."),
                                cx,
                            );
                            this.refresh_project_names(cx);
                        } else {
                            if this.project_state.storage_folder.is_none() {
                                this.project_state.error = Some(error.to_string());
                            }
                            window.push_notification(Notification::error(error.to_string()), cx);
                        }
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn install_workspace(
        &mut self,
        project: Project,
        attachments: Vec<StagedAttachment>,
        scratchpad: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.project_state.revision = self.project_state.revision.wrapping_add(1);
        if scratchpad {
            self.project_state.scratchpad_folder = Some(project.folder.clone());
        }
        self.project_state.folder = (!scratchpad).then(|| project.folder.clone());
        self.project_state.storage_folder = Some(project.folder.clone());
        self.project_state.title = (!scratchpad).then(|| project.title.clone());
        self.project_state.metadata = project.metadata.clone();
        self.project_state.loading = false;
        self.project_state.saving = false;
        self.project_state.error = None;
        if !scratchpad {
            self.remember_project(project.folder.clone());
        }
        self.staged_attachments = attachments;
        self.selected_attachments.clear();
        self.selected_repository = self
            .project_state
            .metadata
            .repository_for_owner(self.settings.owner.as_deref())
            .cloned();
        if let Some(repository) = &self.selected_repository {
            self.repositories
                .insert(repository.name.clone(), repository.clone());
        }
        self.refresh_repository_picker(window, cx);
        self.start_project_writer(project, cx);
        self.sync_upload_result();
        self.refresh_project_names(cx);
        if let Some(target) = self.project_state.pending_destination.take() {
            self.request_workspace(Some(target), window, cx);
        } else if !self.pending_open_files.is_empty() {
            let paths = std::mem::take(&mut self.pending_open_files);
            self.stage_opened_files(paths, window, cx);
        } else if let Some(images) = self.pending_comparison.take() {
            let _ = self.begin_comparison(images, window, cx);
        } else if self.pending_menu_paste {
            self.pending_menu_paste = false;
            self.on_paste(&PasteImage, window, cx);
        }
        cx.notify();
    }

    fn invalidate_project_callbacks(&mut self) {
        self.paste_generation = self.paste_generation.wrapping_add(1);
        self.upload_generation = self.upload_generation.wrapping_add(1);
        self.loading_token = false;
        self.loading_repository = false;
        self.pending_menu_paste = false;
    }

    fn start_project_writer(&mut self, mut project: Project, cx: &mut Context<Self>) {
        let (sender, receiver) = async_channel::unbounded::<ProjectWrite>();
        let (results, completed) = async_channel::unbounded();
        self.project_state.active = Some(sender);
        cx.background_spawn(async move {
            let mut last_error = None;
            while let Ok(command) = receiver.recv().await {
                match command {
                    ProjectWrite::Save {
                        revision,
                        attachments,
                        metadata,
                    } => {
                        project.metadata = metadata;
                        let result = project
                            .save(&attachments)
                            .map(|_| project.metadata.updated_at)
                            .map_err(|error| format!("{error:#}"));
                        last_error = result.as_ref().err().cloned();
                        let _ = results.send((revision, result)).await;
                    }
                    ProjectWrite::SetTitle(title) => project.title = title,
                    ProjectWrite::ForgetAttachment(id) => project.forget_attachment(id),
                    ProjectWrite::Flush(done) => {
                        let _ = done.send(last_error.clone().map_or(Ok(()), Err)).await;
                    }
                }
            }
        })
        .detach();
        cx.spawn(async move |this, cx| {
            while let Ok((revision, result)) = completed.recv().await {
                if this
                    .update(cx, |this, cx| {
                        if revision != this.project_state.revision {
                            return;
                        }
                        this.project_state.saving = false;
                        match result {
                            Ok(updated) => {
                                this.project_state.error = None;
                                this.project_state.metadata.updated_at = updated;
                                this.refresh_project_names(cx);
                            }
                            Err(error) => {
                                this.project_state.error =
                                    Some(format!("Could not save changes: {error}"))
                            }
                        }
                        cx.notify();
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
    }

    pub(super) fn save_project(&mut self, cx: &mut Context<Self>) {
        if self.attachment_drag.is_some() || self.project_state.loading {
            return;
        }
        let Some(writer) = &self.project_state.active else {
            return;
        };
        self.project_state.revision = self.project_state.revision.wrapping_add(1);
        self.project_state.saving = true;
        self.project_state.error = None;
        if writer
            .try_send(ProjectWrite::Save {
                revision: self.project_state.revision,
                attachments: self.staged_attachments.clone(),
                metadata: self.project_state.metadata.clone(),
            })
            .is_err()
        {
            self.project_state.saving = false;
            self.project_state.error = Some("Saving stopped. Keep this window open.".into());
        }
        cx.notify();
    }

    pub(super) fn register_project_shutdown(&self, cx: &mut Context<Self>) {
        cx.on_app_quit(|this, _| {
            let writer = this.project_state.active.clone();
            async move {
                if let Some(writer) = writer {
                    let (done, completed) = async_channel::bounded(1);
                    if writer.send(ProjectWrite::Flush(done)).await.is_ok() {
                        let _ = completed.recv().await;
                    }
                }
            }
        })
        .detach();
    }

    fn remember_project(&mut self, folder: PathBuf) {
        if !self.project_state.folders.contains(&folder) {
            self.project_state.folders.push(folder);
        }
    }

    fn remove_project(&mut self, folder: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        if self.project_navigation_busy() {
            self.pending_work_dialog(None, window, cx);
            return;
        }
        let Some(folder) = self.resolve_project_folder(folder, window, cx) else {
            return;
        };
        self.project_state.loading = true;
        let path = folder.clone();
        let task = cx.background_spawn(async move { crate::projects::delete_project(&path) });
        cx.spawn_in(window, async move |this, window| {
            let result = task.await;
            let _ = this.update_in(window, |this, window, cx| {
                this.project_state.loading = false;
                match result {
                    Ok(()) => {
                        this.project_state.folders.retain(|path| path != &folder);
                        this.project_state.titles.remove(&folder);
                        this.project_state.summaries.remove(&folder);
                        if this.project_state.folder.as_ref() == Some(&folder)
                            && let Ok(scratchpad) = this.scratchpad_path()
                        {
                            this.load_workspace(scratchpad, true, window, cx);
                        }
                        this.sort_projects(cx);
                    }
                    Err(error) => {
                        window.push_notification(Notification::error(error.to_string()), cx)
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    pub(super) fn delete_all_projects(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.project_navigation_busy() {
            self.pending_work_dialog(None, window, cx);
            return;
        }
        let directory = match self.settings.project_directory() {
            Ok(path) => path,
            Err(error) => {
                window.push_notification(Notification::error(error.to_string()), cx);
                return;
            }
        };
        self.project_state.loading = true;
        let task = cx.background_spawn(async move {
            let paths = match crate::projects::project_folders(&directory) {
                Ok(paths) => paths,
                Err(error) => return vec![(directory, Err(error))],
            };
            paths
                .into_iter()
                .map(|path| {
                    let result = crate::projects::delete_project(&path);
                    (path, result)
                })
                .collect::<Vec<_>>()
        });
        cx.spawn_in(window, async move |this, window| {
            let results = task.await;
            let _ = this.update_in(window, |this, window, cx| {
                this.project_state.loading = false;
                for (path, result) in results {
                    match result {
                        Ok(()) => {
                            this.project_state.folders.retain(|p| !p.starts_with(&path));
                            this.project_state.titles.remove(&path);
                            this.project_state.summaries.remove(&path);
                        }
                        Err(error) => window.push_notification(
                            Notification::error(format!("{}: {error}", path.display())),
                            cx,
                        ),
                    }
                }
                this.project_state.folders.retain(|path| path.exists());
                if this
                    .project_state
                    .folder
                    .as_ref()
                    .is_some_and(|path| !this.project_state.folders.contains(path))
                    && let Ok(path) = this.scratchpad_path()
                {
                    this.load_workspace(path, true, window, cx);
                }
                this.sort_projects(cx);
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    pub(super) fn begin_reset(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> anyhow::Result<()> {
        anyhow::ensure!(
            !self.project_navigation_busy(),
            "Finish the current work first."
        );
        let directory = self.settings.project_directory()?;
        let scratchpad = crate::settings::scratchpad_directory()?;
        self.project_state.active = None;
        self.project_state.loading = true;
        let task = cx.background_spawn(async move {
            for path in crate::projects::project_folders(&directory)? {
                crate::projects::delete_project(&path)?;
            }
            if scratchpad.exists() {
                crate::projects::delete_project(&scratchpad)?;
            }
            Ok::<_, anyhow::Error>(())
        });
        cx.spawn_in(window, async move |this, window| {
            let result = task.await;
            let _ = this.update_in(window, |this, window, cx| {
                this.project_state.loading = false;
                match result.and_then(|_| this.reset_app(window, cx)) {
                    Ok(()) => this.initialize_workspaces(window, cx),
                    Err(error) => {
                        this.project_state.error = Some(error.to_string());
                        window.push_notification(Notification::error(error.to_string()), cx);
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
        Ok(())
    }

    pub(super) fn open_project_info(&self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(path) = self.project_state.storage_folder.clone() else {
            return;
        };
        let fallback = self.project_state.metadata.clone();
        let title = self
            .project_state
            .title
            .clone()
            .unwrap_or_else(|| "Scratchpad".into());
        let task = cx.background_spawn(async move { crate::projects::details(&path) });
        cx.spawn_in(window, async move |_, window| {
            let summary = task.await;
            let _ = window.update(|window, cx| {
                let (metadata, bytes) = summary
                    .map(|s| (s.summary.metadata, Some(s.storage_size)))
                    .unwrap_or((fallback, None));
                let repository = metadata
                    .repository
                    .as_ref()
                    .map(|r| {
                        format!(
                            "{}/{}",
                            metadata.repository_owner.as_deref().unwrap_or(""),
                            r.name
                        )
                    })
                    .unwrap_or_else(|| "None selected".into());
                let rows = [
                    ("Name", title.clone()),
                    (
                        "Description",
                        metadata
                            .description
                            .clone()
                            .unwrap_or_else(|| "Not set".into()),
                    ),
                    (
                        "Total storage",
                        bytes
                            .map(format_project_size)
                            .unwrap_or_else(|| "Unavailable".into()),
                    ),
                    ("Repository", repository),
                    ("Created at", format_project_time(metadata.created_at)),
                    ("Updated at", format_project_time(metadata.updated_at)),
                ];
                window.open_alert_dialog(cx, move |alert, _, _| {
                    alert.title("Project information").ok_text("Close").child(
                        v_flex().gap_3().children(rows.iter().map(|(label, value)| {
                            h_flex()
                                .items_start()
                                .gap_4()
                                .text_sm()
                                .child(div().w(px(120.)).flex_none().child(*label))
                                .child(
                                    v_flex().flex_1().min_w_0().children(
                                        value
                                            .split('\n')
                                            .map(|line| div().min_h(px(18.)).child(line.to_owned()))
                                            .collect::<Vec<_>>(),
                                    ),
                                )
                        })),
                    )
                });
            });
        })
        .detach();
    }

    pub(super) fn toggle_project_sidebar(&mut self, cx: &mut Context<Self>) {
        self.settings.sidebar_collapsed = !self.settings.sidebar_collapsed;
        if self.persist_preferences {
            let _ = self.settings.save();
        }
        cx.notify();
    }

    pub(super) fn render_project_sidebar(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        if self.settings.sidebar_collapsed {
            return div().w(px(0.)).h_full().into_any_element();
        }
        let query = self
            .project_state
            .search
            .as_ref()
            .map(|s| s.read(cx).value().to_lowercase())
            .unwrap_or_default();
        Sidebar::new("projects-sidebar")
            .w(px(220.))
            .collapsible(false)
            .header(
                v_flex()
                    .w_full()
                    .gap_3()
                    .child(
                        h_flex()
                            .w_full()
                            .justify_between()
                            .items_center()
                            .child(div().text_sm().font_semibold().child("Projects"))
                            .child(
                                Button::new("hide-project-sidebar")
                                    .debug_selector(|| "hide-project-sidebar".into())
                                    .app_icon()
                                    .icon(AssetIconName::PanelLeftClose)
                                    .accessibility_label("Hide sidebar")
                                    .tooltip("Hide sidebar · ⌘B")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.toggle_project_sidebar(cx)
                                    })),
                            ),
                    )
                    .when_some(self.project_state.search.as_ref(), |header, search| {
                        header.child(Input::new(search).aria_label("Search projects"))
                    }),
            )
            .footer(
                v_flex()
                    .w_full()
                    .gap_2()
                    .child(
                        Button::new("scratchpad-workspace")
                            .debug_selector(|| "scratchpad-workspace".into())
                            .app_secondary()
                            .w_full()
                            .label("Scratchpad")
                            .icon(AssetIconName::NotebookPen)
                            .when(self.project_state.folder.is_none(), |button| {
                                button.bg(cx.theme().sidebar_accent)
                            })
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.request_workspace(None, window, cx)
                            })),
                    )
                    .child(
                        Button::new("create-project")
                            .debug_selector(|| "create-project".into())
                            .app_secondary()
                            .w_full()
                            .label("Create Project")
                            .icon(AssetIconName::FolderPlus)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.create_project(&CreateProject, window, cx)
                            })),
                    ),
            )
            .child(ProjectRows {
                rows: self
                    .project_state
                    .folders
                    .iter()
                    .enumerate()
                    .filter(|(_, path)| self.project_label(path).to_lowercase().contains(&query))
                    .map(|(index, folder)| {
                        let label = self.project_label(folder);
                        let path = folder.clone();
                        let rename_folder = folder.clone();
                        let delete_folder = folder.clone();
                        let rename_view = cx.entity().downgrade();
                        let delete_view = rename_view.clone();
                        let item = SidebarMenuItem::new(label.clone())
                            .accessibility_label(label)
                            .icon(AssetIconName::Folder)
                            .active(self.project_state.folder.as_ref() == Some(folder))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.request_workspace(Some(path.clone()), window, cx)
                            }))
                            .suffix(move |_, _| {
                                let rename_view = rename_view.clone();
                                let delete_view = delete_view.clone();
                                let rename_folder = rename_folder.clone();
                                let delete_folder = delete_folder.clone();
                                Button::new(format!("project-menu-{index}"))
                                    .debug_selector(move || format!("project-options-{index}"))
                                    .app_icon()
                                    .icon(AssetIconName::Ellipsis)
                                    .accessibility_label("Project options")
                                    .dropdown_menu(move |menu, _, _| {
                                        let view = rename_view.clone();
                                        let path = rename_folder.clone();
                                        let menu = menu.item(
                                            PopupMenuItem::new("Edit metadata").on_click(
                                                move |_, window, cx| {
                                                    let _ = view.update(cx, |this, cx| {
                                                        this.edit_project_metadata(
                                                            path.clone(),
                                                            window,
                                                            cx,
                                                        )
                                                    });
                                                },
                                            ),
                                        );
                                        let view = delete_view.clone();
                                        let path = delete_folder.clone();
                                        menu.item(PopupMenuItem::new("Delete").on_click(
                                            move |_, window, cx| {
                                                let _ = view.update(cx, |this, cx| {
                                                    this.remove_project(path.clone(), window, cx)
                                                });
                                            },
                                        ))
                                    })
                            });
                        (folder.clone(), item)
                    })
                    .collect(),
                bounds: self.project_bounds.clone(),
                active: self.project_state.folder.clone(),
            })
            .into_any_element()
    }
}

fn format_project_size(bytes: u64) -> String {
    if bytes < 1000 {
        format!("{bytes} bytes")
    } else if bytes < 1_000_000 {
        format!("{:.1} KB", bytes as f64 / 1000.)
    } else if bytes < 1_000_000_000 {
        format!("{:.1} MB", bytes as f64 / 1_000_000.)
    } else {
        format!("{:.1} GB", bytes as f64 / 1_000_000_000.)
    }
}

fn format_project_time(milliseconds: u64) -> String {
    crate::desktop::format_local_timestamp(milliseconds).unwrap_or_else(|| "Unknown".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::{Modifiers, TestAppContext, test::TestWindowExt};

    fn image() -> StagedAttachment {
        StagedAttachment::from_bytes(
            "review.png".into(),
            include_bytes!("../../packaging/menu-bar-icon.png").to_vec(),
            None,
        )
        .unwrap()
    }

    fn project(parent: &std::path::Path, name: &str, attachments: &[StagedAttachment]) -> Project {
        let directory = parent.join("projects");
        std::fs::create_dir_all(&directory).unwrap();
        Project::create_in_parent(&directory, name, attachments).unwrap()
    }

    fn scratchpad(
        view: &Entity<UploaderApp>,
        cx: &mut gpui_kit::VisualTestContext,
        parent: &std::path::Path,
    ) -> PathBuf {
        let path = parent.join("scratchpad");
        let (project, items) = crate::projects::open_scratchpad(path.clone()).unwrap();
        cx.update(|window, cx| {
            view.update(cx, |app, cx| {
                app.settings.project_directory_override = Some(parent.join("projects"));
                app.install_workspace(project, items, true, window, cx);
            })
        });
        cx.run_until_parked();
        path
    }

    #[gpui_kit::test]
    fn external_projects_cannot_be_opened_moved_to_or_deleted(cx: &mut TestAppContext) {
        let (view, cx) = super::super::tests::upload_window(cx);
        let directory = tempfile::tempdir().unwrap();
        scratchpad(&view, cx, directory.path());
        let inside = project(directory.path(), "Inside", &[]);
        let outside = Project::create_in_parent(directory.path(), "Outside", &[]).unwrap();
        cx.update(|window, cx| {
            view.update(cx, |app, cx| {
                app.append_staged_attachments(vec![image()], window, cx)
            });
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            view.update(cx, |app, cx| {
                let id = app.staged_attachments[0].id;
                app.request_workspace(Some(outside.folder.clone()), window, cx);
                assert!(!app.project_state.loading);
                assert!(app.project_state.folder.is_none());
                app.move_attachments_to_project(vec![id], outside.folder.clone(), window, cx);
                assert!(!app.project_state.loading);
                app.remove_project(outside.folder.clone(), window, cx);
                assert!(!app.project_state.loading);
                assert_eq!(app.staged_attachments.len(), 1);
                app.request_workspace(Some(inside.folder.clone()), window, cx);
            });
        });
        cx.run_until_parked();
        assert!(outside.folder.is_dir());
        assert!(Project::open(outside.folder).unwrap().1.is_empty());
        cx.update(|_, cx| {
            assert_eq!(
                view.read(cx).project_state.folder.as_ref(),
                Some(&inside.folder)
            )
        });
    }

    #[gpui_kit::test]
    fn switching_workspaces_releases_pending_upload_preparation(cx: &mut TestAppContext) {
        let (view, cx) = super::super::tests::upload_window(cx);
        let directory = tempfile::tempdir().unwrap();
        scratchpad(&view, cx, directory.path());
        let target = project(directory.path(), "Target", &[]);
        cx.update(|window, cx| {
            view.update(cx, |app, cx| {
                app.loading_token = true;
                app.loading_repository = true;
                app.request_workspace(Some(target.folder), window, cx);
            });
        });
        cx.run_until_parked();
        cx.update(|_, cx| {
            assert!(!view.read(cx).loading_token);
            assert!(!view.read(cx).loading_repository);
        });
    }

    #[gpui_kit::test]
    fn delayed_upload_preparation_cannot_affect_another_workspace(cx: &mut TestAppContext) {
        let (view, cx) = super::super::tests::upload_window(cx);
        let directory = tempfile::tempdir().unwrap();
        scratchpad(&view, cx, directory.path());
        let generation = cx.update(|_, cx| view.read(cx).upload_generation);
        let target = project(directory.path(), "Target", &[]);
        cx.update(|window, cx| {
            view.update(cx, |app, cx| {
                app.request_workspace(Some(target.folder), window, cx)
            });
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            view.update(cx, |app, cx| {
                // B has its own pending requests. A's late results must not
                // clear these flags, supply a token, or continue an upload.
                app.loading_token = true;
                app.loading_repository = true;
                app.accept_upload_token(generation, Ok("old-token".into()), window, cx);
                assert!(app.token.is_none());
                assert!(app.loading_token);
                app.accept_upload_repository(
                    generation,
                    "images",
                    Err(github::GithubError::Unauthenticated),
                    window,
                    cx,
                );
                assert!(app.loading_repository);
                assert!(!app.uploading);
            });
        });
    }

    #[gpui_kit::test]
    fn repository_loading_respects_the_workspaces_saved_owner(cx: &mut TestAppContext) {
        let (view, cx) = super::super::tests::upload_window(cx);
        cx.update(|window, cx| {
            view.update(cx, |app, cx| {
                app.project_state.metadata.repository_owner = Some("other-owner".into());
                app.project_state.metadata.repository = Some(Repository {
                    id: 10,
                    name: "images".into(),
                });
                let session = github::GithubSession {
                    token: "test-token".into(),
                    repositories: vec![Repository {
                        id: 20,
                        name: "images".into(),
                    }],
                };
                app.accept_repository_session(session.clone(), window, cx);
                assert!(app.selected_repository.is_none());
                assert_eq!(
                    app.project_state.metadata.repository.as_ref().unwrap().id,
                    10
                );
                app.project_state.metadata.repository_owner = app
                    .settings
                    .owner
                    .as_ref()
                    .map(|owner| owner.to_uppercase());
                app.accept_repository_session(session, window, cx);
                assert_eq!(app.selected_repository.as_ref().unwrap().id, 20);
            });
        });
    }

    #[gpui_kit::test]
    fn scratchpad_transfer_and_workspace_switching_preserve_separate_repositories(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = super::super::tests::upload_window(cx);
        let directory = tempfile::tempdir().unwrap();
        let scratch = scratchpad(&view, cx, directory.path());
        let item = image();
        let id = item.persistent_id;
        cx.update(|window, cx| {
            view.update(cx, |app, cx| {
                app.project_state.metadata.repository = Some(Repository {
                    id: 12,
                    name: "scratch-repo".into(),
                });
                app.project_state.metadata.repository_owner = app.settings.owner.clone();
                app.append_staged_attachments(vec![item], window, cx);
            })
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            view.update(cx, |app, cx| {
                app.create_named_project(
                    "Review",
                    "Notes for the review".into(),
                    true,
                    theme::DialogFeedback::default(),
                    window,
                    cx,
                )
                .unwrap()
            })
        });
        cx.run_until_parked();
        let folder = directory
            .path()
            .join("projects/Review")
            .canonicalize()
            .unwrap();
        assert!(Project::open(scratch.clone()).unwrap().1.is_empty());
        let (project, items) = Project::open(folder.clone()).unwrap();
        assert_eq!(items[0].persistent_id, id);
        assert_eq!(
            project.metadata.description.as_deref(),
            Some("Notes for the review")
        );
        cx.update(|window, cx| {
            view.update(cx, |app, cx| {
                assert_eq!(app.project_state.folder.as_ref(), Some(&folder));
                app.project_state.metadata.repository = Some(Repository {
                    id: 34,
                    name: "project-repo".into(),
                });
                app.save_project(cx);
                app.focus_handle.focus(window, cx);
            })
        });
        cx.run_until_parked();
        cx.update(|window, cx| view.update(cx, |app, cx| app.request_workspace(None, window, cx)));
        cx.run_until_parked();
        cx.update(|window, cx| {
            view.update(cx, |app, cx| {
                assert!(app.project_state.folder.is_none());
                assert!(app.staged_attachments.is_empty());
                assert_eq!(
                    app.selected_repository.as_ref().unwrap().name,
                    "scratch-repo"
                );
                app.request_workspace(None, window, cx);
                assert!(
                    !app.project_state.loading,
                    "The current workspace must not reload"
                );
                app.request_workspace(Some(folder.clone()), window, cx);
            })
        });
        cx.run_until_parked();
        cx.update(|_, cx| {
            view.update(cx, |app, _| {
                assert_eq!(app.staged_attachments[0].persistent_id, id);
                assert_eq!(
                    app.selected_repository.as_ref().unwrap().name,
                    "project-repo"
                );
            })
        });
    }

    #[gpui_kit::test]
    fn creating_without_transfer_keeps_scratchpad_and_creates_an_empty_project(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = super::super::tests::upload_window(cx);
        let directory = tempfile::tempdir().unwrap();
        let scratch = scratchpad(&view, cx, directory.path());
        cx.update(|window, cx| {
            view.update(cx, |app, cx| {
                app.append_staged_attachments(vec![image()], window, cx)
            })
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            view.update(cx, |app, cx| app.create_project(&CreateProject, window, cx));
            window.draw(cx).clear(cx);
        });
        assert!(
            !cx.did_prompt_for_paths(),
            "Creation uses the configured location"
        );
        let name = cx.debug_bounds("project-name-field").unwrap().center();
        cx.simulate_mouse_down(name, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(name, MouseButton::Left, Modifiers::default());
        cx.simulate_input("Empty");
        let description = cx
            .debug_bounds("project-description-field")
            .unwrap()
            .center();
        cx.simulate_mouse_down(description, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(description, MouseButton::Left, Modifiers::default());
        cx.simulate_input("Optional project notes");
        let transfer = cx.debug_bounds("transfer-scratchpad").unwrap().center();
        cx.simulate_mouse_down(transfer, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(transfer, MouseButton::Left, Modifiers::default());
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let create = cx.debug_bounds("confirm-project-name").unwrap().center();
        cx.simulate_mouse_down(create, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(create, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();
        assert_eq!(
            Project::open(directory.path().join("projects/Empty"))
                .unwrap()
                .0
                .metadata
                .description,
            Some("Optional project notes".into())
        );
        assert_eq!(Project::open(scratch).unwrap().1.len(), 1);
        assert!(
            Project::open(directory.path().join("projects/Empty"))
                .unwrap()
                .1
                .is_empty()
        );
    }

    #[gpui_kit::test]
    fn a_new_project_with_no_description_stores_null(cx: &mut TestAppContext) {
        let (view, cx) = super::super::tests::upload_window(cx);
        let directory = tempfile::tempdir().unwrap();
        scratchpad(&view, cx, directory.path());
        cx.update(|window, cx| {
            view.update(cx, |app, cx| {
                app.create_named_project(
                    "Undescribed",
                    String::new(),
                    false,
                    theme::DialogFeedback::default(),
                    window,
                    cx,
                )
                .unwrap()
            })
        });
        cx.run_until_parked();
        let project = Project::open(directory.path().join("projects/Undescribed"))
            .unwrap()
            .0;
        assert_eq!(project.metadata.description, None);
    }

    #[gpui_kit::test]
    fn sidebar_search_and_selection_controls_keep_footer_visible(cx: &mut TestAppContext) {
        let (view, cx) = super::super::tests::upload_window(cx);
        cx.update(|window, cx| {
            view.update(cx, |app, cx| {
                app.staged_attachments = vec![image(), image()];
                for (index, item) in app.staged_attachments.iter_mut().enumerate() {
                    item.id = index as u64 + 1;
                }
                let alpha = PathBuf::from("/Alpha");
                let beta = PathBuf::from("/Beta");
                app.project_state.folders = vec![alpha.clone(), beta.clone()];
                app.project_state.titles.insert(alpha, "Alpha".into());
                app.project_state.titles.insert(beta, "Beta".into());
                app.project_state
                    .search
                    .as_ref()
                    .unwrap()
                    .update(cx, |input, cx| input.set_value("alpha", window, cx));
                cx.notify();
            });
            window.draw(cx).clear(cx);
        });
        assert!(cx.debug_bounds("project-options-0").is_some());
        assert!(cx.debug_bounds("project-options-1").is_none());
        assert!(cx.debug_bounds("create-project").is_some());
        assert!(cx.debug_bounds("scratchpad-workspace").is_some());
        let checkbox = cx.debug_bounds("select-attachment-1").unwrap().center();
        cx.simulate_mouse_down(checkbox, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(checkbox, MouseButton::Left, Modifiers::default());
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            assert_eq!(view.read(cx).selected_attachments.len(), 1);
            assert!(view.read(cx).preview_window.is_none());
            assert!(view.read(cx).attachment_drag.is_none());
        });
        let all = cx.debug_bounds("select-all-attachments").unwrap().center();
        cx.simulate_mouse_down(all, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(all, MouseButton::Left, Modifiers::default());
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            assert_eq!(view.read(cx).selected_attachments.len(), 2);
        });
        let options = cx.debug_bounds("selected-attachment-options").unwrap();
        let selection = cx.debug_bounds("select-all-attachments").unwrap();
        assert_eq!(options.size.height, selection.size.height);
        assert_eq!(options.size.width, options.size.height);
        cx.update(|window, cx| {
            crate::app::init_keybindings(cx);
            view.update(cx, |app, cx| app.focus_handle.focus(window, cx));
        });
        cx.simulate_keystrokes("cmd-b");
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert!(cx.debug_bounds("create-project").is_none());
        assert!(cx.debug_bounds("show-project-sidebar").is_some());
    }

    #[gpui_kit::test]
    fn delete_all_preserves_scratchpad_and_unrelated_folders(cx: &mut TestAppContext) {
        let (view, cx) = super::super::tests::upload_window(cx);
        let directory = tempfile::tempdir().unwrap();
        let scratch = scratchpad(&view, cx, directory.path());
        let project = project(directory.path(), "Unlisted", &[image()]);
        std::fs::write(directory.path().join("keep.txt"), "keep").unwrap();
        cx.update(|window, cx| {
            view.update(cx, |app, cx| {
                app.delete_all_projects(window, cx);
            })
        });
        cx.run_until_parked();
        assert!(scratch.join("gatto-project.json").exists());
        assert!(!project.folder.exists());
        assert_eq!(
            std::fs::read_to_string(directory.path().join("keep.txt")).unwrap(),
            "keep"
        );
    }

    #[gpui_kit::test]
    fn sorting_uses_saved_update_times(cx: &mut TestAppContext) {
        let (view, cx) = super::super::tests::upload_window(cx);
        let directory = tempfile::tempdir().unwrap();
        let first = project(directory.path(), "First", &[]);
        let mut second = project(directory.path(), "Second", &[]);
        second.metadata.updated_at = first.metadata.updated_at + 1000;
        second.save(&[]).unwrap();
        cx.update(|_, cx| {
            view.update(cx, |app, cx| {
                app.project_state.folders = vec![first.folder.clone(), second.folder.clone()];
                app.refresh_project_names(cx);
            })
        });
        cx.run_until_parked();
        cx.update(|_, cx| {
            assert_eq!(
                view.read(cx).project_state.folders,
                vec![second.folder, first.folder]
            )
        });
    }

    #[gpui_kit::test]
    fn project_menu_edits_metadata_and_deletes_without_confirmation(cx: &mut TestAppContext) {
        let (view, cx) = super::super::tests::upload_window(cx);
        let directory = tempfile::tempdir().unwrap();
        scratchpad(&view, cx, directory.path());
        let project = project(directory.path(), "Original", &[]);
        let folder = project.folder;
        cx.update(|window, cx| {
            view.update(cx, |app, cx| {
                app.remember_project(folder.clone());
                app.refresh_project_names(cx);
            });
            window.draw(cx).clear(cx);
        });
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let menu = cx.debug_bounds("project-options-0").unwrap().center();
        cx.simulate_mouse_down(menu, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(menu, MouseButton::Left, Modifiers::default());
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.simulate_keystrokes("down enter");
        cx.run_until_parked();
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            assert!(window.has_active_dialog(cx));
        });
        let field = cx.debug_bounds("project-name-field").unwrap().center();
        cx.simulate_mouse_down(field, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(field, MouseButton::Left, Modifiers::default());
        cx.simulate_keystrokes("cmd-a");
        cx.simulate_input("Renamed");
        let description = cx
            .debug_bounds("project-description-field")
            .unwrap()
            .center();
        cx.simulate_mouse_down(description, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(description, MouseButton::Left, Modifiers::default());
        cx.simulate_input("First line\nSecond line");
        let save = cx.debug_bounds("save-project-metadata").unwrap().center();
        cx.simulate_mouse_down(save, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(save, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();
        assert_eq!(crate::projects::title(&folder).unwrap(), "Renamed");
        assert_eq!(
            crate::projects::summary(&folder)
                .unwrap()
                .metadata
                .description
                .as_deref(),
            Some("First line\nSecond line")
        );
        cx.update(|window, cx| {
            view.update(cx, |app, cx| app.remove_project(folder.clone(), window, cx))
        });
        cx.run_until_parked();
        assert!(!folder.exists());
        cx.update(|window, cx| assert!(!window.has_active_dialog(cx)));
    }

    #[gpui_kit::test]
    fn duplicate_name_feedback_stays_inside_the_modal_and_escape_dismisses_focused_fields(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = super::super::tests::upload_window(cx);
        let directory = tempfile::tempdir().unwrap();
        scratchpad(&view, cx, directory.path());
        let existing = project(directory.path(), "Existing", &[]);
        cx.update(|window, cx| {
            crate::application_menu::init(cx);
            view.update(cx, |app, cx| {
                app.remember_project(existing.folder.clone());
                app.create_project(&CreateProject, window, cx);
            });
            window.draw(cx).clear(cx);
        });
        let name = cx.debug_bounds("project-name-field").unwrap().center();
        cx.simulate_mouse_down(name, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(name, MouseButton::Left, Modifiers::default());
        cx.simulate_input("Existing");
        let confirm = cx.debug_bounds("confirm-project-name").unwrap().center();
        cx.simulate_mouse_down(confirm, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(confirm, MouseButton::Left, Modifiers::default());
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            assert!(window.has_active_dialog(cx));
        });
        let error = cx.debug_bounds("dialog-validation-error").unwrap();
        let description = cx.debug_bounds("project-description-field").unwrap();
        assert!(
            error.origin.y < description.origin.y,
            "Validation is part of the form"
        );
        cx.simulate_mouse_down(
            description.center(),
            MouseButton::Left,
            Modifiers::default(),
        );
        cx.simulate_mouse_up(
            description.center(),
            MouseButton::Left,
            Modifiers::default(),
        );
        cx.simulate_input("Notes");
        cx.simulate_keystrokes("escape");
        cx.run_until_parked();
        cx.update(|window, cx| {
            assert!(!window.has_active_dialog(cx));
            assert!(view.read(cx).project_state.folder.is_none());
            assert!(existing.folder.is_dir());
        });
    }

    #[gpui_kit::test]
    fn existing_folder_create_error_keeps_the_dialog_open(cx: &mut TestAppContext) {
        let (view, cx) = super::super::tests::upload_window(cx);
        let directory = tempfile::tempdir().unwrap();
        scratchpad(&view, cx, directory.path());
        std::fs::create_dir_all(directory.path().join("projects")).unwrap();
        let existing =
            Project::create_in_parent(directory.path().join("projects").as_path(), "Existing", &[])
                .unwrap();
        cx.update(|window, cx| {
            view.update(cx, |app, cx| app.create_project(&CreateProject, window, cx));
            window.draw(cx).clear(cx);
        });
        let name = cx.debug_bounds("project-name-field").unwrap().center();
        cx.simulate_mouse_down(name, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(name, MouseButton::Left, Modifiers::default());
        cx.simulate_input("Existing");
        let description = cx
            .debug_bounds("project-description-field")
            .unwrap()
            .center();
        cx.simulate_mouse_down(description, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(description, MouseButton::Left, Modifiers::default());
        cx.simulate_input("Keep these notes");
        let confirm = cx.debug_bounds("confirm-project-name").unwrap().center();
        cx.simulate_mouse_down(confirm, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(confirm, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            assert!(window.has_active_dialog(cx));
        });
        assert!(cx.debug_bounds("dialog-validation-error").is_some());
        assert_eq!(
            crate::projects::title(&existing.folder).unwrap(),
            "Existing"
        );
    }

    #[gpui_kit::test]
    fn closing_a_retained_clean_preview_does_not_block_workspace_switches(cx: &mut TestAppContext) {
        let (view, cx) = super::super::tests::upload_window(cx);
        cx.update(|_, cx| {
            let (handle, preview) = crate::theme::open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                        Default::default(),
                        size(px(640.), px(500.)),
                    ))),
                    ..Default::default()
                },
                cx,
                |_, cx| cx.new(|cx| ImagePreview::new(vec![], 0, cx)),
            )
            .unwrap();
            view.update(cx, |app, cx| {
                app.preview_window = Some((handle, preview));
                app.preview_open = true;
                assert!(app.project_navigation_busy());
                app.note_preview_closed(false, cx);
                assert!(
                    !app.project_navigation_busy(),
                    "A hidden retained window is not unfinished editing"
                );
            });
        });
    }

    #[gpui_kit::test]
    fn pending_upload_requires_a_choice_and_cancel_switches_after_saving(cx: &mut TestAppContext) {
        let (view, cx) = super::super::tests::upload_window(cx);
        let directory = tempfile::tempdir().unwrap();
        scratchpad(&view, cx, directory.path());
        let project = project(directory.path(), "Other", &[]);
        let folder = project.folder;
        cx.update(|window, cx| {
            view.update(cx, |app, cx| {
                app.uploading = true;
                app.staged_attachments = vec![image()];
                app.request_workspace(Some(folder.clone()), window, cx);
                assert!(app.project_state.folder.is_none());
                assert!(app.uploading);
            });
            window.draw(cx).clear(cx);
            assert!(window.has_active_dialog(cx));
        });
        cx.simulate_keystrokes("enter");
        cx.run_until_parked();
        cx.update(|_, cx| {
            view.update(cx, |app, _| {
                assert!(!app.uploading);
                assert_eq!(app.project_state.folder.as_ref(), Some(&folder));
            })
        });
    }
    #[gpui_kit::test]
    fn move_without_destinations_can_create_a_project_for_only_selected_attachments(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = super::super::tests::upload_window(cx);
        let directory = tempfile::tempdir().unwrap();
        let scratch = directory.path().join("scratchpad");
        let (project, _) = crate::projects::open_scratchpad(scratch.clone()).unwrap();
        let items: Vec<_> = (0..2)
            .map(|i| {
                StagedAttachment::from_bytes(
                    format!("image-{i}.png"),
                    include_bytes!("../../packaging/menu-bar-icon.png").to_vec(),
                    None,
                )
                .unwrap()
            })
            .collect();
        let selected = items[0].id;
        let moved_uuid = items[0].persistent_id;
        let remaining_uuid = items[1].persistent_id;
        cx.update(|window, cx| {
            view.update(cx, |app, cx| {
                app.project_state.folders.clear();
                app.settings.project_directory_override = Some(directory.path().join("projects"));
                app.install_workspace(project, items, true, window, cx);
                app.save_project(cx);
            })
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            view.update(cx, |app, cx| {
                app.open_move_dialog(vec![selected], window, cx)
            });
            window.draw(cx).clear(cx);
            assert!(window.try_find("move-attachments").is_none());
            window.click("create-move-destination", cx);
            window.draw(cx).clear(cx);
            assert!(window.try_find("transfer-scratchpad").is_none());
        });
        let name = cx.debug_bounds("project-name-field").unwrap().center();
        cx.simulate_mouse_down(name, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(name, MouseButton::Left, Modifiers::default());
        cx.simulate_input("Selected");
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            window.click("confirm-project-name", cx);
        });
        cx.run_until_parked();
        let (_, moved) =
            crate::projects::Project::open(directory.path().join("projects/Selected")).unwrap();
        let (_, remaining) = crate::projects::Project::open(scratch).unwrap();
        assert_eq!(moved.len(), 1);
        assert_eq!(moved[0].persistent_id, moved_uuid);
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].persistent_id, remaining_uuid);
        cx.update(|_, cx| {
            let app = view.read(cx);
            assert!(app.project_state.folder.is_none());
            assert_eq!(app.staged_attachments[0].persistent_id, remaining_uuid);
            assert!(app.project_state.pending_move_to_new.is_none());
        });
    }
}

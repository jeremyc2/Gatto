use std::{cell::Cell, collections::HashMap, path::PathBuf, rc::Rc, sync::Arc};

use gpui_kit::assets::IconName as AssetIconName;
use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Icon, IconName, Sizable as _, StyledExt as _,
    WindowExt as _,
    attachment::{
        Attachment, AttachmentActions, AttachmentContent, AttachmentDescription, AttachmentMedia,
        AttachmentStatus, AttachmentTitle,
    },
    button::{Button, ButtonVariants as _},
    checkbox::Checkbox,
    clipboard::Clipboard,
    combobox::{Combobox, ComboboxEvent, ComboboxState},
    dialog::{DialogAction, DialogClose, DialogFooter},
    h_flex,
    input::{Input, InputState},
    kbd::Kbd,
    menu::{DropdownMenu as _, PopupMenuItem},
    notification::Notification,
    searchable_list::{SearchableGroup, SearchableVec},
    spinner::Spinner,
    switch::Switch,
    v_flex,
};
use gpui_kit::{
    Anchor, AnyWindowHandle, App, AppContext as _, ClipboardEntry, ClipboardItem, Context, Entity,
    ExternalPaths, FocusHandle, Focusable, Image, ImageFormat, InteractiveElement as _,
    IntoElement, KeyBinding, Keystroke, ObjectFit, ParentElement as _, PathPromptOptions, Render,
    SharedString, StatefulInteractiveElement as _, Styled as _, StyledImage as _, Subscription,
    Window, WindowBounds, WindowOptions, actions, div, img, prelude::FluentBuilder as _, px, size,
};

use crate::{
    diagnostics::Diagnostics,
    github,
    image_preview::ImagePreview,
    log_viewer::LogViewer,
    menu_bar::MenuBarController,
    model::{Repository, StagedImage, parse_organization},
    settings::AppSettings,
    window_limits,
};

actions!(
    gatto,
    [
        PasteImage,
        PasteAndPreview,
        PasteToMarkdown,
        PasteToUrl,
        CopyUploadUrl,
        CopyUploadMarkdown,
        SwitchToBulkUpload
    ]
);

const KEY_CONTEXT: &str = "Gatto";
const UNSUPPORTED_MESSAGE: &str =
    "Unsupported format. Please paste or drop a PNG, JPEG, GIF, or WebP image.";

type RepositoryPicker = ComboboxState<SearchableVec<SearchableGroup<String>>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LoadState {
    NeedsSetup,
    Loading,
    Ready,
    Failed,
}

#[derive(Clone)]
struct UploadedImage {
    id: u64,
    url: String,
    markdown: String,
}

#[derive(Clone)]
struct UploadResult {
    images: Vec<UploadedImage>,
}

impl UploadResult {
    fn urls(&self) -> String {
        self.images
            .iter()
            .map(|image| image.url.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn markdown(&self) -> String {
        self.images
            .iter()
            .map(|image| image.markdown.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MenuPasteMode {
    Preview,
    Markdown,
    Url,
}

pub struct UploaderApp {
    organization_input: Entity<InputState>,
    repository_picker: Entity<RepositoryPicker>,
    repositories: HashMap<String, Repository>,
    selected_repository: Option<Repository>,
    token: Option<String>,
    load_state: LoadState,
    staged_images: Vec<StagedImage>,
    bulk_upload: bool,
    paste_generation: u64,
    upload_result: Option<UploadResult>,
    uploading: bool,
    settings: AppSettings,
    settings_open: bool,
    pending_paste_repository: Option<String>,
    pending_menu_paste: Option<MenuPasteMode>,
    pending_replacement_images: Option<Vec<StagedImage>>,
    log_window: Option<AnyWindowHandle>,
    preview_window: Option<(AnyWindowHandle, Entity<ImagePreview>)>,
    menu_bar: MenuBarController,
    diagnostics: Diagnostics,
    header_mark: Arc<Image>,
    focus_handle: FocusHandle,
    _repository_subscription: Subscription,
}

impl UploaderApp {
    pub fn new(
        window: &mut Window,
        diagnostics: Diagnostics,
        menu_bar: MenuBarController,
        cx: &mut Context<Self>,
    ) -> Self {
        let (settings, settings_error) = match AppSettings::load() {
            Ok(settings) => {
                diagnostics.info("Loaded application preferences.");
                (settings, None)
            }
            Err(error) => (
                AppSettings::default(),
                Some(format!("App preferences could not be loaded: {error}")),
            ),
        };
        if settings_error.is_some() {
            diagnostics.error("Could not load application preferences; using defaults.");
        }
        let initial_organization = settings.organization.clone().unwrap_or_default();
        let organization_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("organization")
                .default_value(initial_organization)
        });
        let repository_picker = cx.new(|cx| {
            ComboboxState::new(
                SearchableVec::new(Vec::<SearchableGroup<String>>::new()),
                vec![],
                window,
                cx,
            )
            .searchable(true)
        });
        let subscription = cx.subscribe(
            &repository_picker,
            |this: &mut Self,
             _,
             event: &ComboboxEvent<SearchableVec<SearchableGroup<String>>>,
             cx| {
                if let ComboboxEvent::Change(values) = event {
                    this.selected_repository = values
                        .first()
                        .and_then(|name| this.repositories.get(name))
                        .cloned();
                    this.settings.last_repository = this
                        .selected_repository
                        .as_ref()
                        .map(|repository| repository.name.clone());
                    if let Err(error) = this.settings.save() {
                        this.diagnostics.error(format!(
                            "Could not remember the selected repository: {error}"
                        ));
                    }
                    cx.notify();
                }
            },
        );

        let mut this = Self {
            organization_input,
            repository_picker,
            repositories: HashMap::new(),
            selected_repository: None,
            token: None,
            load_state: LoadState::NeedsSetup,
            staged_images: Vec::new(),
            bulk_upload: false,
            paste_generation: 0,
            upload_result: None,
            uploading: false,
            settings,
            settings_open: false,
            pending_paste_repository: None,
            pending_menu_paste: None,
            pending_replacement_images: None,
            log_window: None,
            preview_window: None,
            menu_bar,
            diagnostics,
            header_mark: Arc::new(Image::from_bytes(
                ImageFormat::Png,
                include_bytes!("../packaging/rocket-cat-transparent.png").to_vec(),
            )),
            focus_handle: cx.focus_handle(),
            _repository_subscription: subscription,
        };
        if let Some(error) = settings_error {
            window.push_notification(Notification::error(error), cx);
        }
        this.load_repositories(window, cx);
        this
    }

    pub fn show_settings(&mut self, cx: &mut Context<Self>) {
        self.diagnostics.info("Opened application preferences.");
        self.settings_open = true;
        cx.notify();
    }

    pub fn has_pinned_repository(&self) -> bool {
        !self.settings.pinned_repositories.is_empty()
    }

    fn paste_and_preview_from_menu(
        &mut self,
        _: &PasteAndPreview,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.paste_from_menu(MenuPasteMode::Preview, window, cx);
    }

    fn paste_to_markdown_from_menu(
        &mut self,
        _: &PasteToMarkdown,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.paste_from_menu(MenuPasteMode::Markdown, window, cx);
    }

    fn paste_to_url_from_menu(
        &mut self,
        _: &PasteToUrl,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.paste_from_menu(MenuPasteMode::Url, window, cx);
    }

    /// Stages the current clipboard image and chooses the first pinned repository
    /// (alphabetically). Direct-copy modes upload as soon as both are ready.
    fn paste_from_menu(
        &mut self,
        mode: MenuPasteMode,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.settings_open = false;
        self.pending_menu_paste = Some(mode);

        let repository_name = self
            .settings
            .last_repository
            .as_ref()
            .filter(|name| self.settings.pinned_repositories.contains(*name))
            .cloned()
            .or_else(|| self.settings.pinned_repositories.first().cloned());
        let Some(repository_name) = repository_name else {
            self.pending_menu_paste = None;
            self.warn(
                "Paste actions need a pinned repository. Pin one in App Preferences first.",
                window,
                cx,
            );
            return;
        };
        if self.load_state == LoadState::NeedsSetup {
            self.pending_menu_paste = None;
            self.warn(
                "Paste actions need an organization in App Preferences first.",
                window,
                cx,
            );
            return;
        }

        self.diagnostics.info(format!(
            "Menu paste requested with pinned repository {repository_name}."
        ));
        self.pending_paste_repository = Some(repository_name.clone());
        if self.load_state == LoadState::Ready {
            if !self.select_repository(&repository_name, window, cx) {
                self.pending_paste_repository = None;
                self.pending_menu_paste = None;
                self.warn(
                    "The pinned repository is not available to the active GitHub account.",
                    window,
                    cx,
                );
                return;
            }
            self.pending_paste_repository = None;
        }
        if self.load_state == LoadState::Failed {
            self.diagnostics
                .info("Menu paste is retrying the repository list.");
            self.load_repositories(window, cx);
        }

        self.on_paste(&PasteImage, window, cx);
    }

    /// Clears a pasted image when the main window is hidden, or when the user
    /// explicitly discards it. Incrementing the generation keeps a pending
    /// clipboard conversion from restoring an image after it was cleared.
    pub fn clear_staged_images(&mut self, reason: &str, cx: &mut Context<Self>) {
        self.paste_generation = self.paste_generation.wrapping_add(1);
        self.pending_menu_paste = None;
        self.pending_paste_repository = None;
        if !self.staged_images.is_empty() {
            self.staged_images.clear();
            self.upload_result = None;
            self.diagnostics.info(reason);
            cx.notify();
        }
    }

    pub fn report_menu_bar_error(
        &mut self,
        error: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.diagnostics.error(error.into());
        window.push_notification(
            Notification::error(
                "The menu bar could not be set up. See Application logs for details.",
            ),
            cx,
        );
    }

    fn load_repositories(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.token = None;
        self.selected_repository = None;
        self.repositories.clear();
        self.repository_picker.update(cx, |picker, cx| {
            picker.set_selected_indices([], window, cx);
            picker.set_items(
                SearchableVec::new(Vec::<SearchableGroup<String>>::new()),
                window,
                cx,
            );
        });
        let Some(organization) = self.settings.organization.clone() else {
            self.diagnostics
                .info("Repository loading is waiting for an organization setting.");
            self.load_state = LoadState::NeedsSetup;
            cx.notify();
            return;
        };

        self.load_state = LoadState::Loading;
        self.diagnostics.info(format!(
            "Loading repositories for organization {organization}."
        ));
        cx.notify();

        let diagnostics = self.diagnostics.clone();
        cx.spawn_in(window, async move |this, window| {
            let requested = organization.clone();
            let result = window
                .background_spawn(async move { github::load_session(&organization, &diagnostics) })
                .await;
            let _ = this.update_in(window, move |this, window, cx| {
                if this.settings.organization.as_deref() != Some(requested.as_str()) {
                    return;
                }
                match result {
                    Ok(session) => {
                        this.diagnostics.info(format!(
                            "Loaded {} accessible repositories.",
                            session.repositories.len()
                        ));
                        this.repositories = session
                            .repositories
                            .into_iter()
                            .map(|repository| (repository.name.clone(), repository))
                            .collect();
                        let groups = this.repository_groups();
                        this.token = Some(session.token);
                        this.load_state = LoadState::Ready;
                        this.repository_picker.update(cx, |picker, cx| {
                            picker.set_items(SearchableVec::new(groups), window, cx);
                        });
                        if let Some(repository_name) = this.pending_paste_repository.take() {
                            if this.select_repository(&repository_name, window, cx) {
                                this.diagnostics.info(format!(
                                    "Menu paste selected pinned repository {repository_name}."
                                ));
                            } else {
                                this.pending_menu_paste = None;
                                this.diagnostics.error(
                                    "Menu paste could not find its pinned repository in the loaded list.",
                                );
                                window.push_notification(
                                    Notification::error(
                                        "The pinned repository is not available to the active GitHub account.",
                                    ),
                                    cx,
                                );
                            }
                        } else if let Some(repository_name) = this
                            .settings
                            .last_repository
                            .clone()
                            .filter(|name| this.repositories.contains_key(name))
                            .or_else(|| this.settings.pinned_repositories.first().cloned())
                        {
                            this.select_repository(&repository_name, window, cx);
                        }
                        this.try_complete_menu_paste(window, cx);
                    }
                    Err(error) => {
                        this.diagnostics
                            .error(format!("Repository loading failed: {error}"));
                        this.load_state = LoadState::Failed;
                        this.pending_menu_paste = None;
                        window.push_notification(Notification::error(error.to_string()), cx);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn choose_file(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.diagnostics.info("Opened the image file picker.");
        let multiple = self.bulk_upload;
        let prompt = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple,
            prompt: Some(if multiple {
                "Choose images".into()
            } else {
                "Choose an image".into()
            }),
        });

        cx.spawn_in(window, async move |this, window| {
            let Some(paths) = prompt.await.ok().and_then(Result::ok).flatten() else {
                return;
            };
            let results = window
                .background_spawn(async move {
                    paths
                        .into_iter()
                        .map(StagedImage::from_path)
                        .collect::<Vec<_>>()
                })
                .await;
            let _ = this.update_in(window, |this, window, cx| {
                this.accept_image_results(results, window, cx);
            });
        })
        .detach();
    }

    fn on_paste(&mut self, _: &PasteImage, window: &mut Window, cx: &mut Context<Self>) {
        if self.load_state == LoadState::NeedsSetup {
            return;
        }
        if self.uploading {
            self.warn(
                "Wait for the current upload to finish before adding images.",
                window,
                cx,
            );
            return;
        }
        let Some(clipboard) = cx.read_from_clipboard() else {
            self.pending_menu_paste = None;
            self.pending_paste_repository = None;
            self.warn(UNSUPPORTED_MESSAGE, window, cx);
            return;
        };

        if let Some(image) = clipboard.entries().iter().find_map(|entry| match entry {
            ClipboardEntry::Image(image) => Some(image.clone()),
            _ => None,
        }) {
            let paste_generation = self.paste_generation;
            let task = cx.background_spawn(async move { StagedImage::from_clipboard(&image) });
            cx.spawn_in(window, async move |this, window| {
                let result = task.await;
                let _ = this.update_in(window, |this, window, cx| {
                    this.accept_pasted_image_result(paste_generation, result, window, cx);
                });
            })
            .detach();
            return;
        }

        if let Some(paths) = clipboard.entries().iter().find_map(|entry| match entry {
            ClipboardEntry::ExternalPaths(paths) => Some(paths.paths().to_vec()),
            _ => None,
        }) {
            let paths = if self.bulk_upload {
                paths
            } else {
                paths.into_iter().take(1).collect()
            };
            Self::load_dropped_paths(cx.entity().downgrade(), paths, window, cx);
            return;
        }

        self.pending_menu_paste = None;
        self.pending_paste_repository = None;
        self.warn(UNSUPPORTED_MESSAGE, window, cx);
    }

    fn load_dropped_paths(
        view: gpui_kit::WeakEntity<Self>,
        paths: Vec<PathBuf>,
        window: &mut Window,
        cx: &mut App,
    ) {
        if paths.is_empty() {
            return;
        }
        let task = cx.background_spawn(async move {
            paths
                .into_iter()
                .map(StagedImage::from_path)
                .collect::<Vec<_>>()
        });
        window
            .spawn(cx, async move |window| {
                let results = task.await;
                let _ = view.update_in(window, |this, window, cx| {
                    this.accept_image_results(results, window, cx);
                });
            })
            .detach();
    }

    fn accept_image_result(
        &mut self,
        result: anyhow::Result<StagedImage>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.accept_image_results(vec![result], window, cx);
    }

    fn accept_image_results(
        &mut self,
        results: Vec<anyhow::Result<StagedImage>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut images = Vec::new();
        let mut errors = Vec::new();
        for result in results {
            match result {
                Ok(image) => images.push(image),
                Err(error) => {
                    self.diagnostics
                        .error(format!("Image staging failed: {error}"));
                    errors.push(error.to_string());
                }
            }
        }

        if images.is_empty() {
            self.pending_menu_paste = None;
            self.pending_paste_repository = None;
            if let Some(error) = errors.first() {
                self.warn(error.clone(), window, cx);
            }
            return;
        }
        if !errors.is_empty() {
            self.warn(
                format!(
                    "Added {} image{}; {} file{} could not be read.",
                    images.len(),
                    if images.len() == 1 { "" } else { "s" },
                    errors.len(),
                    if errors.len() == 1 { "" } else { "s" },
                ),
                window,
                cx,
            );
        }
        self.stage_images(images, window, cx);
    }

    fn stage_images(
        &mut self,
        images: Vec<StagedImage>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.bulk_upload || self.staged_images.is_empty() {
            self.append_staged_images(images, window, cx);
        } else {
            self.open_replace_image_dialog(images, window, cx);
        }
    }

    fn append_staged_images(
        &mut self,
        mut images: Vec<StagedImage>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let added = images.len();
        self.staged_images.append(&mut images);
        self.sync_upload_result();
        self.diagnostics.info(format!(
            "Staged {added} image{} for upload.",
            if added == 1 { "" } else { "s" }
        ));
        self.try_complete_menu_paste(window, cx);
        cx.notify();
    }

    fn replace_staged_image(
        &mut self,
        image: StagedImage,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.staged_images.clear();
        self.append_staged_images(vec![image], window, cx);
    }

    fn change_image_description(
        &mut self,
        image_id: u64,
        requested_name: &str,
        cx: &mut Context<Self>,
    ) -> anyhow::Result<()> {
        let image = self
            .staged_images
            .iter_mut()
            .find(|image| image.id == image_id)
            .ok_or_else(|| anyhow::anyhow!("That image is no longer staged."))?;
        image.description = requested_name.to_owned();
        let description = image.description.clone();
        self.sync_upload_result();
        self.diagnostics
            .info(format!("Changed image description to {description}."));
        cx.notify();
        Ok(())
    }

    fn open_description_dialog(
        &mut self,
        image_id: u64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(image) = self.staged_images.iter().find(|image| image.id == image_id) else {
            return;
        };
        let current_description = image.description.clone();
        let input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Descriptive image name")
                .default_value(current_description)
        });
        let rename_view = cx.entity().downgrade();
        let rename_input = input.clone();
        let select_initial_description = Rc::new(Cell::new(true));

        window.open_alert_dialog(cx, move |alert, window, cx| {
            let rename_input = rename_input.clone();
            let rename_view = rename_view.clone();
            if select_initial_description.replace(false) {
                input.update(cx, |input, cx| {
                    input.focus(window, cx);
                    input.select_all(window, cx);
                });
            }
            alert
                .width(px(440.))
                .title("Set description")
                .description(
                    "This description is used as the Markdown alt text. It does not rename the file sent to GitHub.",
                )
                .child(
                    v_flex()
                        .gap_2()
                        .child(div().text_xs().font_semibold().child("Alt text"))
                        .child(Input::new(&input).w_full()),
                )
                .show_cancel(true)
                .cancel_text("Cancel")
                .ok_text("Save description")
                .on_ok(move |_, window, cx| {
                    let requested_name = rename_input.read(cx).value().to_string();
                    match rename_view.update(cx, |this, cx| {
                        this.change_image_description(image_id, &requested_name, cx)
                    }) {
                        Ok(Ok(())) => true,
                        Ok(Err(error)) => {
                            window.push_notification(
                                Notification::warning(error.to_string()),
                                cx,
                            );
                            false
                        }
                        Err(_) => true,
                    }
                })
        });
    }

    fn open_replace_image_dialog(
        &mut self,
        images: Vec<StagedImage>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if window.has_active_dialog(cx) {
            window.close_dialog(cx);
        }
        let Some(replacement) = images.first().cloned() else {
            return;
        };
        self.pending_replacement_images = Some(images.clone());
        let view = cx.entity().downgrade();
        let bulk_shortcut_focus = cx.focus_handle();
        let focus_bulk_shortcut = Rc::new(Cell::new(true));
        window.open_alert_dialog(cx, move |alert, window, cx| {
            let close_view = view.clone();
            let replace_view = view.clone();
            let replacement = replacement.clone();
            let bulk_view = view.clone();
            let bulk_images = images.clone();
            let shortcut_bulk_view = view.clone();
            let shortcut_bulk_images = images.clone();
            let bulk_shortcut_focus = bulk_shortcut_focus.clone();

            if focus_bulk_shortcut.replace(false) {
                let bulk_shortcut_focus = bulk_shortcut_focus.clone();
                window.defer(cx, move |window, cx| {
                    bulk_shortcut_focus.focus(window, cx);
                });
            }

            alert
                .width(px(460.))
                .icon(
                    Icon::new(IconName::CircleAlert)
                        .text_color(cx.theme().warning),
                )
                .title("Replace the current image?")
                .description(
                    "This will override the current image. To do more than one image at a time, switch to bulk upload.",
                )
                .child(
                    v_flex()
                        .track_focus(&bulk_shortcut_focus)
                        .on_action(move |_: &SwitchToBulkUpload, window, cx| {
                            let _ = shortcut_bulk_view.update(cx, |this, cx| {
                                this.pending_replacement_images = None;
                                this.bulk_upload = true;
                                this.append_staged_images(
                                    shortcut_bulk_images.clone(),
                                    window,
                                    cx,
                                );
                            });
                            window.close_dialog(cx);
                        })
                        .gap_2()
                        .rounded_lg()
                        .border_1()
                        .border_color(cx.theme().border)
                        .bg(cx.theme().muted.opacity(0.18))
                        .p_3()
                        .child(
                            div()
                                .text_sm()
                                .font_semibold()
                                .child("Want to keep both images?"),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child("Turn on bulk upload and add this image to the batch."),
                        )
                        .child(
                            Button::new("switch-to-bulk-upload")
                                .secondary()
                                .small()
                                .self_start()
                                .flex_none()
                                .label("Switch to bulk upload")
                                .child(Kbd::new(
                                    Keystroke::parse("b").expect("valid shortcut"),
                                ))
                                .on_click(move |_, window, cx| {
                                    let _ = bulk_view.update(cx, |this, cx| {
                                        this.pending_replacement_images = None;
                                        this.bulk_upload = true;
                                        this.append_staged_images(
                                            bulk_images.clone(),
                                            window,
                                            cx,
                                        );
                                    });
                                    window.close_dialog(cx);
                                }),
                        ),
                )
                .footer(
                    DialogFooter::new()
                        .child(DialogClose::new().trigger(|button| {
                            button.label("Cancel")
                        }))
                        .child(DialogAction::new().child(
                            Button::new("replace-image")
                                .primary()
                                .label("Replace image")
                                .child(Kbd::new(
                                    Keystroke::parse("enter").expect("valid shortcut"),
                                )),
                        )),
                )
                .on_ok(move |_, window, cx| {
                    let _ = replace_view.update(cx, |this, cx| {
                        this.pending_replacement_images = None;
                        this.replace_staged_image(replacement.clone(), window, cx);
                    });
                    true
                })
                .on_close(move |_, _, cx| {
                    let _ = close_view.update(cx, |this, _| {
                        this.pending_menu_paste = None;
                        this.pending_paste_repository = None;
                        this.pending_replacement_images = None;
                    });
                })
        });
    }

    fn switch_to_bulk_upload(
        &mut self,
        _: &SwitchToBulkUpload,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(images) = self.pending_replacement_images.take() else {
            return;
        };

        self.bulk_upload = true;
        self.append_staged_images(images, window, cx);
        window.close_dialog(cx);
    }

    fn accept_pasted_image_result(
        &mut self,
        paste_generation: u64,
        result: anyhow::Result<StagedImage>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if paste_generation != self.paste_generation {
            self.diagnostics
                .info("Discarded a stale clipboard image conversion.");
            return;
        }
        self.accept_image_result(result, window, cx);
    }

    fn warn(
        &mut self,
        message: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let message = message.into();
        self.diagnostics.warn(message.to_string());
        window.push_notification(Notification::warning(message), cx);
    }

    fn send(&mut self, _: &gpui_kit::ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.send_image(None, window, cx);
    }

    fn try_complete_menu_paste(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(mode) = self.pending_menu_paste else {
            return;
        };
        if self.staged_images.is_empty()
            || self.load_state != LoadState::Ready
            || self.selected_repository.is_none()
        {
            return;
        }

        self.pending_menu_paste = None;
        match mode {
            MenuPasteMode::Preview => cx.notify(),
            MenuPasteMode::Markdown | MenuPasteMode::Url => self.send_image(Some(mode), window, cx),
        }
    }

    fn sync_upload_result(&mut self) {
        let images = self
            .staged_images
            .iter()
            .filter_map(|image| {
                image.uploaded_url.as_ref().map(|url| UploadedImage {
                    id: image.id,
                    url: url.clone(),
                    markdown: markdown_image(&image.description, url),
                })
            })
            .collect::<Vec<_>>();
        self.upload_result = (!images.is_empty()).then_some(UploadResult { images });
    }

    fn send_image(
        &mut self,
        copy_after_upload: Option<MenuPasteMode>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.uploading {
            return;
        }
        let (Some(token), Some(repository)) =
            (self.token.clone(), self.selected_repository.clone())
        else {
            return;
        };
        if self.staged_images.is_empty() {
            return;
        }
        let images = self
            .staged_images
            .iter()
            .filter(|image| image.uploaded_url.is_none())
            .cloned()
            .collect::<Vec<_>>();
        let upload_count = images.len();
        if upload_count == 0 {
            return;
        }

        self.uploading = true;
        self.diagnostics.info(format!(
            "Started uploading {upload_count} image{}.",
            if upload_count == 1 { "" } else { "s" }
        ));
        cx.notify();

        let diagnostics = self.diagnostics.clone();
        cx.spawn_in(window, async move |this, window| {
            let results = window
                .background_spawn(async move {
                    images
                        .into_iter()
                        .map(|image| {
                            let result =
                                github::upload(&token, repository.id, &image, &diagnostics);
                            (image.id, image.name, image.description, result)
                        })
                        .collect::<Vec<_>>()
                })
                .await;
            let _ = this.update_in(window, move |this, window, cx| {
                this.uploading = false;
                let mut uploaded = Vec::new();
                let mut failures = Vec::new();
                for (id, name, description, result) in results {
                    match result {
                        Ok(url) => {
                            if let Some(image) =
                                this.staged_images.iter_mut().find(|image| image.id == id)
                            {
                                image.uploaded_url = Some(url.clone());
                            }
                            uploaded.push(UploadedImage {
                                id,
                                markdown: markdown_image(&description, &url),
                                url,
                            });
                        }
                        Err(error) => failures.push((name, error.to_string())),
                    }
                }

                if uploaded.is_empty() {
                    if let Some((_, error)) = failures.first() {
                        this.diagnostics
                            .error(format!("Image batch upload failed: {error}"));
                        window.push_notification(Notification::error(error.clone()), cx);
                    }
                    cx.notify();
                    return;
                }

                let upload_result = UploadResult { images: uploaded };
                let uploaded_count = upload_result.images.len();
                let urls = upload_result.urls();
                let markdown = upload_result.markdown();
                match copy_after_upload {
                    Some(MenuPasteMode::Markdown) => {
                        cx.write_to_clipboard(ClipboardItem::new_string(markdown));
                        this.diagnostics.info(format!(
                            "Uploaded {uploaded_count} image{} and copied Markdown.",
                            if uploaded_count == 1 { "" } else { "s" }
                        ));
                    }
                    Some(MenuPasteMode::Url) => {
                        cx.write_to_clipboard(ClipboardItem::new_string(urls));
                        this.diagnostics.info(format!(
                            "Uploaded {uploaded_count} image{} and copied URLs.",
                            if uploaded_count == 1 { "" } else { "s" }
                        ));
                    }
                    Some(MenuPasteMode::Preview) | None => {
                        this.diagnostics.info(format!(
                            "Uploaded {uploaded_count} image{} successfully.",
                            if uploaded_count == 1 { "" } else { "s" }
                        ));
                    }
                }
                this.sync_upload_result();

                if failures.is_empty() {
                    let message = match copy_after_upload {
                        Some(MenuPasteMode::Markdown) => {
                            format!(
                                "{uploaded_count} image{} uploaded; Markdown copied.",
                                if uploaded_count == 1 { "" } else { "s" }
                            )
                        }
                        Some(MenuPasteMode::Url) => {
                            format!(
                                "{uploaded_count} image{} uploaded; URL{} copied.",
                                if uploaded_count == 1 { "" } else { "s" },
                                if uploaded_count == 1 { "" } else { "s" }
                            )
                        }
                        Some(MenuPasteMode::Preview) | None => {
                            format!(
                                "{uploaded_count} image{} uploaded successfully.",
                                if uploaded_count == 1 { "" } else { "s" }
                            )
                        }
                    };
                    window.push_notification(Notification::success(message), cx);
                } else {
                    let failed_count = failures.len();
                    window.push_notification(
                        Notification::warning(format!(
                            "{uploaded_count} uploaded; {failed_count} failed and remain staged."
                        )),
                        cx,
                    );
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn copy_upload_url(&mut self, _: &CopyUploadUrl, window: &mut Window, cx: &mut Context<Self>) {
        let Some(result) = &self.upload_result else {
            return;
        };
        cx.write_to_clipboard(ClipboardItem::new_string(result.urls()));
        self.diagnostics
            .info("Copied uploaded image URLs to the clipboard.");
        window.push_notification(Notification::success("URLs copied to the clipboard."), cx);
    }

    fn copy_upload_markdown(
        &mut self,
        _: &CopyUploadMarkdown,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(result) = &self.upload_result else {
            return;
        };
        cx.write_to_clipboard(ClipboardItem::new_string(result.markdown()));
        self.diagnostics
            .info("Copied uploaded Markdown image snippets to the clipboard.");
        window.push_notification(
            Notification::success("Markdown image snippets copied to the clipboard."),
            cx,
        );
    }

    fn repository_groups(&self) -> Vec<SearchableGroup<String>> {
        let mut names = self.repositories.keys().cloned().collect::<Vec<_>>();
        names.sort_by_key(|name| name.to_ascii_lowercase());
        let (pinned, remaining): (Vec<_>, Vec<_>) = names
            .into_iter()
            .partition(|name| self.settings.pinned_repositories.contains(name));
        let mut groups = Vec::new();
        if !pinned.is_empty() {
            groups.push(SearchableGroup::new("📌  Pinned repositories").items(pinned));
        }
        if !remaining.is_empty() {
            groups.push(SearchableGroup::new("All repositories").items(remaining));
        }
        groups
    }

    fn select_repository(
        &mut self,
        name: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(repository) = self.repositories.get(name).cloned() else {
            return false;
        };
        self.selected_repository = Some(repository);
        self.repository_picker.update(cx, |picker, cx| {
            picker.set_selected_values(&[name.to_owned()], window, cx);
        });
        true
    }

    fn refresh_repository_picker(&self, window: &mut Window, cx: &mut Context<Self>) {
        let groups = self.repository_groups();
        let selected_values = self
            .selected_repository
            .as_ref()
            .map(|selected| vec![selected.name.clone()])
            .unwrap_or_default();
        self.repository_picker.update(cx, |picker, cx| {
            picker.set_items(SearchableVec::new(groups), window, cx);
            picker.set_selected_values(&selected_values, window, cx);
        });
    }

    fn pin_selected_repository(
        &mut self,
        _: &gpui_kit::ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(repository) = self.selected_repository.as_ref() else {
            return;
        };
        let name = repository.name.clone();
        if !self.settings.pinned_repositories.insert(name.clone()) {
            return;
        }
        if let Err(error) = self.settings.save() {
            self.settings.pinned_repositories.remove(&name);
            window.push_notification(
                Notification::error(format!("Could not save preferences: {error}")),
                cx,
            );
        } else {
            self.refresh_repository_picker(window, cx);
            self.menu_bar.set_paste_actions_visible(true);
        }
        cx.notify();
    }

    fn unpin_repository(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        if !self.settings.pinned_repositories.remove(name) {
            return;
        }
        if let Err(error) = self.settings.save() {
            self.settings.pinned_repositories.insert(name.to_owned());
            window.push_notification(
                Notification::error(format!("Could not save preferences: {error}")),
                cx,
            );
        } else {
            self.refresh_repository_picker(window, cx);
            self.menu_bar
                .set_paste_actions_visible(!self.settings.pinned_repositories.is_empty());
        }
        cx.notify();
    }

    fn save_organization(
        &mut self,
        _: &gpui_kit::ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let value = self.organization_input.read(cx).value();
        let organization = match parse_organization(&value) {
            Ok(organization) => organization,
            Err(error) => {
                window.push_notification(Notification::error(error.to_string()), cx);
                return;
            }
        };

        let previous = self.settings.organization.replace(organization.clone());
        if let Err(error) = self.settings.save() {
            self.settings.organization = previous;
            window.push_notification(
                Notification::error(format!("Could not save preferences: {error}")),
                cx,
            );
            return;
        }

        self.load_repositories(window, cx);
    }

    fn set_start_at_login(&mut self, enabled: bool, window: &mut Window, cx: &mut Context<Self>) {
        if let Err(error) = self.settings.set_start_at_login(enabled) {
            window.push_notification(
                Notification::error(format!("Could not update Start at Login: {error}")),
                cx,
            );
        }
        cx.notify();
    }

    fn open_log_window(
        &mut self,
        _: &gpui_kit::ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(log_window) = self.log_window {
            if log_window
                .update(cx, |_, window, _| window.activate_window())
                .is_ok()
            {
                return;
            }
            self.log_window = None;
        }

        self.diagnostics.info("Opened the application logs window.");
        let diagnostics = self.diagnostics.clone();
        let uploader = cx.entity().downgrade();
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(820.), px(620.)), cx)),
            window_min_size: Some(size(px(540.), px(420.))),
            ..Default::default()
        };
        let result = gpui_kit::open_window(options, cx, move |window, cx| {
            window.set_window_title("Gatto Logs");
            window_limits::set_maximum_content_size(window, 1200., 1000.);
            window.on_window_should_close(cx, move |_, cx| {
                let _ = uploader.update(cx, |this, _| {
                    this.log_window = None;
                });
                true
            });
            cx.new(|cx| LogViewer::new(diagnostics, cx))
        });
        match result {
            Ok((handle, _)) => self.log_window = Some(handle),
            Err(error) => {
                self.diagnostics.error(format!(
                    "Could not open the application logs window: {error}"
                ));
                window.push_notification(
                    Notification::error(
                        "Could not open Application logs. See the menu bar and try again.",
                    ),
                    cx,
                );
            }
        }
    }

    fn open_image_preview(
        &mut self,
        selected_id: u64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(image) = self
            .staged_images
            .iter()
            .find(|image| image.id == selected_id)
        else {
            return;
        };
        let preview_name = image.name.clone();
        let preview_images = self.staged_images.clone();

        if let Some((preview_window, preview)) = self.preview_window.clone() {
            let title = preview_name.clone();
            if preview_window
                .update(cx, move |_, window, _| {
                    window.set_window_title(&format!("{title} — Gatto Preview"));
                    window.activate_window();
                })
                .is_ok()
            {
                preview.update(cx, |preview, cx| {
                    preview.show(preview_images, selected_id, cx);
                });
                return;
            }
            self.preview_window = None;
        }

        let uploader = cx.entity().downgrade();
        let window_title = format!("{preview_name} — Gatto Preview");
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(980.), px(720.)), cx)),
            window_min_size: Some(size(px(420.), px(320.))),
            ..Default::default()
        };
        let result = gpui_kit::open_window(options, cx, move |window, cx| {
            window.set_window_title(&window_title);
            window_limits::set_maximum_content_size(window, 2200., 1600.);
            window.on_window_should_close(cx, move |_, cx| {
                let _ = uploader.update(cx, |this, _| {
                    this.preview_window = None;
                });
                true
            });
            let preview = cx.new(|cx| ImagePreview::new(preview_images, selected_id, cx));
            let focus = preview.read(cx).focus_handle(cx);
            window.focus(&focus, cx);
            preview
        });
        match result {
            Ok((handle, preview)) => self.preview_window = Some((handle, preview)),
            Err(error) => {
                self.diagnostics
                    .error(format!("Could not open image preview: {error}"));
                window.push_notification(
                    Notification::error("Could not open the image preview."),
                    cx,
                );
            }
        }
    }

    fn render_repository_picker(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let disabled = self.load_state != LoadState::Ready;
        let organization = self.settings.organization.clone().unwrap_or_default();
        v_flex()
            .w_full()
            .gap_2()
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .child(div().text_sm().font_semibold().child("Repository"))
                    .when(self.load_state == LoadState::Loading, |row| {
                        row.child(
                            h_flex()
                                .gap_2()
                                .items_center()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(Spinner::new().xsmall())
                                .child("Loading repositories"),
                        )
                    })
                    .when(self.load_state == LoadState::Failed, |row| {
                        row.child(
                            Button::new("retry-repositories")
                                .small()
                                .outline()
                                .icon(IconName::RefreshCw)
                                .label("Retry")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.load_repositories(window, cx);
                                })),
                        )
                    }),
            )
            .child(
                Combobox::new(&self.repository_picker)
                    .placeholder(if disabled {
                        "Repositories unavailable".to_owned()
                    } else {
                        format!("Search {organization} repositories…")
                    })
                    .search_placeholder("Type to filter repositories…")
                    .disabled(disabled)
                    .w_full(),
            )
    }

    fn render_setup_prompt(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .items_center()
            .justify_center()
            .rounded_xl()
            .border_2()
            .border_dashed()
            .border_color(cx.theme().primary.opacity(0.4))
            .bg(cx.theme().primary.opacity(0.06))
            .p_6()
            .child(
                Icon::new(IconName::Settings)
                    .size(px(30.))
                    .text_color(cx.theme().primary),
            )
            .child(div().text_sm().font_semibold().child("Set an organization"))
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(
                        "Enter a GitHub organization in App Preferences to load its repositories.",
                    ),
            )
            .child(
                Button::new("setup-open-settings")
                    .primary()
                    .small()
                    .label("Open App Preferences")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.show_settings(cx);
                    })),
            )
    }

    fn render_drop_zone(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity().downgrade();
        let bulk_upload = self.bulk_upload;
        let uploading = self.uploading;
        let has_staged_images = !self.staged_images.is_empty();
        div()
            .id("image-drop-zone")
            .w_full()
            .min_h(px(if has_staged_images { 112. } else { 150. }))
            .rounded_xl()
            .border_2()
            .border_dashed()
            .border_color(cx.theme().primary.opacity(0.4))
            .bg(cx.theme().primary.opacity(0.05))
            .when(!uploading, |zone| {
                zone.cursor_pointer()
                    .hover(|style| style.bg(cx.theme().primary.opacity(0.14)))
                    .drag_over::<ExternalPaths>(|style, _, _, cx| {
                        style
                            .border_color(cx.theme().primary)
                            .bg(cx.theme().primary.opacity(0.1))
                    })
                    .can_drop(|value, _, _| value.downcast_ref::<ExternalPaths>().is_some())
                    .on_drop(move |paths: &ExternalPaths, window, cx| {
                        let paths = if bulk_upload {
                            paths.paths().to_vec()
                        } else {
                            paths.paths().iter().take(1).cloned().collect()
                        };
                        Self::load_dropped_paths(view.clone(), paths, window, cx);
                    })
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.choose_file(window, cx);
                    }))
            })
            .when(uploading, |zone| zone.opacity(0.55))
            .flex()
            .items_center()
            .justify_center()
            .overflow_hidden()
            .child(
                v_flex()
                    .w_full()
                    .px_4()
                    .items_center()
                    .gap_3()
                    .child(
                        Icon::new(AssetIconName::Image)
                            .size(px(30.))
                            .text_color(cx.theme().muted_foreground),
                    )
                    .child(
                        h_flex()
                            .flex_wrap()
                            .justify_center()
                            .gap_1()
                            .items_center()
                            .text_sm()
                            .text_center()
                            .child("Press")
                            .child(Kbd::new(Keystroke::parse("cmd-v").expect("valid shortcut")))
                            .child(if bulk_upload {
                                "or drop images here"
                            } else {
                                "or drop an image here"
                            }),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_center()
                            .text_color(cx.theme().muted_foreground)
                            .child(if bulk_upload {
                                "or click to select one or more PNG, JPEG, GIF, or WebP files"
                            } else {
                                "or click to select a PNG, JPEG, GIF, or WebP file"
                            }),
                    ),
            )
    }

    fn render_bulk_upload_control(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity().downgrade();
        let has_batch = self.staged_images.len() > 1;
        let disabled = self.uploading || has_batch;
        h_flex()
            .w_full()
            .min_w_0()
            .items_center()
            .justify_between()
            .gap_3()
            .rounded_lg()
            .border_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().muted.opacity(0.18))
            .px_3()
            .py_2()
            .child(
                v_flex()
                    .min_w_0()
                    .gap_0p5()
                    .child(div().text_sm().font_semibold().child("Bulk upload"))
                    .child(
                        div()
                            .text_xs()
                            .whitespace_normal()
                            .text_color(cx.theme().muted_foreground)
                            .child(if has_batch {
                                "Upload or remove the batch before turning this off."
                            } else {
                                "Keep multiple images in one upload batch."
                            }),
                    ),
            )
            .child(
                Switch::new("bulk-upload-switch")
                    .checked(self.bulk_upload)
                    .disabled(disabled)
                    .accessibility_label("Bulk upload")
                    .tooltip(if has_batch {
                        "Upload or remove the batch before turning bulk upload off"
                    } else {
                        "Keep multiple images in one upload batch"
                    })
                    .on_change(move |checked, _, cx| {
                        let checked = *checked;
                        let _ = view.update(cx, |this, cx| {
                            this.bulk_upload = checked;
                            cx.notify();
                        });
                    }),
            )
    }

    fn remove_staged_image(&mut self, id: u64, cx: &mut Context<Self>) {
        let previous_len = self.staged_images.len();
        self.staged_images.retain(|image| image.id != id);
        if self.staged_images.len() != previous_len {
            self.sync_upload_result();
            self.pending_menu_paste = None;
            self.pending_paste_repository = None;
            self.diagnostics.info("Removed a staged image.");
            cx.notify();
        }
    }

    fn render_attachments(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let count = self.staged_images.len();
        let has_uploaded_images = self
            .staged_images
            .iter()
            .any(|image| image.uploaded_url.is_some());
        let attachments = self.staged_images.iter().map(|image| {
            let image_id = image.id;
            let source = image.preview.clone();
            let title = image.description.clone();
            let detail = format!(
                "{} · {} · {}",
                image.name,
                image.formatted_size(),
                image.mime_type
            );
            let status = if image.uploaded_url.is_some() {
                AttachmentStatus::Complete
            } else if self.uploading {
                AttachmentStatus::Uploading
            } else {
                AttachmentStatus::Pending
            };
            let preview_view = cx.entity().downgrade();
            let rename_view = cx.entity().downgrade();
            let remove_view = cx.entity().downgrade();
            Attachment::new()
                .id(format!("staged-image-attachment-{image_id}"))
                .large()
                .w_full()
                .cursor_pointer()
                .status(status)
                .media(AttachmentMedia::new().src(source))
                .content(
                    AttachmentContent::new()
                        .title(AttachmentTitle::new(title))
                        .description(AttachmentDescription::new(detail)),
                )
                .on_click(move |_, window, cx| {
                    let _ = preview_view.update(cx, |this, cx| {
                        this.open_image_preview(image_id, window, cx);
                    });
                })
                .when(!self.uploading, |attachment| {
                    attachment.actions(
                        AttachmentActions::new()
                            .child(
                                Button::new(format!("set-staged-image-description-{image_id}"))
                                    .ghost()
                                    .small()
                                    .icon(AssetIconName::ALargeSmall)
                                    .accessibility_label("Set description")
                                    .tooltip("Set description")
                                    .on_click(move |_, window, cx| {
                                        cx.stop_propagation();
                                        let _ = rename_view.update(cx, |this, cx| {
                                            this.open_description_dialog(image_id, window, cx);
                                        });
                                    }),
                            )
                            .child(
                                Button::new(format!("remove-staged-image-{image_id}"))
                                    .ghost()
                                    .small()
                                    .icon(IconName::Close)
                                    .accessibility_label("Remove staged image")
                                    .tooltip("Remove staged image")
                                    .on_click(move |_, _, cx| {
                                        cx.stop_propagation();
                                        let _ = remove_view.update(cx, |this, cx| {
                                            this.remove_staged_image(image_id, cx);
                                        });
                                    }),
                            ),
                    )
                })
                .into_any_element()
        });

        v_flex()
            .gap_2()
            .when(count > 1, |list| {
                list.child(
                    h_flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .text_xs()
                                .font_semibold()
                                .text_color(cx.theme().muted_foreground)
                                .child(format!("{count} images ready")),
                        )
                        .when(!self.uploading, |row| {
                            row.child(
                                Button::new("remove-all-staged-images")
                                    .ghost()
                                    .small()
                                    .label(if has_uploaded_images {
                                        "Reset"
                                    } else {
                                        "Remove all"
                                    })
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.clear_staged_images(
                                            "Discarded all staged images.",
                                            cx,
                                        );
                                    })),
                            )
                        }),
                )
            })
            .children(attachments)
    }

    fn render_upload_result(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let result = self.upload_result.as_ref().map(|result| {
            let image_count = result.images.len();
            let urls = result.urls();
            let markdown = result.markdown();
            let url_lines = result
                .images
                .iter()
                .map(|image| {
                    div()
                        .overflow_hidden()
                        .text_ellipsis()
                        .child(image.url.clone())
                        .into_any_element()
                })
                .collect::<Vec<_>>();
            let markdown_lines = result
                .images
                .iter()
                .map(|image| {
                    div()
                        .overflow_hidden()
                        .text_ellipsis()
                        .child(image.markdown.clone())
                        .into_any_element()
                })
                .collect::<Vec<_>>();
            let url_view = cx.entity().downgrade();
            let markdown_view = cx.entity().downgrade();
            let description_view = cx.entity().downgrade();
            let description_items = result
                .images
                .iter()
                .enumerate()
                .map(|(index, image)| (image.id, index + 1))
                .collect::<Vec<_>>();
            v_flex()
                .w_full()
                .gap_3()
                .rounded_lg()
                .border_1()
                .border_color(cx.theme().success.opacity(0.35))
                .bg(cx.theme().success.opacity(0.07))
                .p_3()
                .child(
                    h_flex()
                        .items_center()
                        .justify_between()
                        .child(div().text_sm().font_semibold().child(if image_count == 1 {
                            "Uploaded image".to_owned()
                        } else {
                            format!("Uploaded {image_count} images")
                        }))
                        .child(
                            h_flex()
                                .gap_1()
                                .child(
                                    Button::new("uploaded-image-actions")
                                        .ghost()
                                        .small()
                                        .icon(IconName::Ellipsis)
                                        .accessibility_label("Uploaded image actions")
                                        .tooltip("Uploaded image actions")
                                        .dropdown_menu_with_anchor(Anchor::TopRight, move |menu, _, _| {
                                            description_items.iter().fold(menu, |menu, (id, index)| {
                                                let description_view = description_view.clone();
                                                let id = *id;
                                                let label = if image_count == 1 { "Set description".to_owned() } else { format!("Set description for image {index}") };
                                                menu.item(PopupMenuItem::new(label).on_click(move |_, window, cx| {
                                                    let _ = description_view.update(cx, |this, cx| this.open_description_dialog(id, window, cx));
                                                }))
                                            })
                                        }),
                                )
                                .child(
                                    Button::new("dismiss-upload-result")
                                        .ghost()
                                        .small()
                                        .icon(IconName::Close)
                                        .tooltip("Dismiss upload result")
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.upload_result = None;
                                            cx.notify();
                                        })),
                                ),
                        ),
                )
                .child(
                    v_flex()
                        .gap_1()
                        .child(
                            div()
                                .text_xs()
                                .font_semibold()
                                .text_color(cx.theme().muted_foreground)
                                .child(if image_count == 1 { "URL" } else { "URLs" }),
                        )
                        .child(
                            h_flex()
                                .gap_2()
                                .items_center()
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .overflow_hidden()
                                        .rounded_md()
                                        .border_1()
                                        .border_color(cx.theme().border)
                                        .bg(cx.theme().background)
                                        .px_3()
                                        .py_2()
                                        .text_xs()
                                        .child(v_flex().gap_1().children(url_lines)),
                                )
                                .child(
                                    Clipboard::new("copy-upload-url")
                                        .small()
                                        .value(urls)
                                        .tooltip("Copy URLs (Command+Shift+C)")
                                        .accessibility_label("Copy uploaded image URLs")
                                        .on_copied(move |_, window, cx| {
                                            let _ = url_view.update(cx, |this, _| {
                                                this.diagnostics.info(
                                                    "Copied uploaded image URLs to the clipboard.",
                                                );
                                            });
                                            window.push_notification(
                                                Notification::success("URLs copied to the clipboard."),
                                                cx,
                                            );
                                        }),
                                ),
                        ),
                )
                .child(
                    v_flex()
                        .gap_1()
                        .child(
                            div()
                                .text_xs()
                                .font_semibold()
                                .text_color(cx.theme().muted_foreground)
                                .child(if image_count == 1 {
                                    "Markdown image"
                                } else {
                                    "Markdown images"
                                }),
                        )
                        .child(
                            h_flex()
                                .gap_2()
                                .items_center()
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .overflow_hidden()
                                        .rounded_md()
                                        .border_1()
                                        .border_color(cx.theme().border)
                                        .bg(cx.theme().background)
                                        .px_3()
                                        .py_2()
                                        .text_xs()
                                        .child(v_flex().gap_1().children(markdown_lines)),
                                )
                                .child(
                                    Clipboard::new("copy-upload-markdown")
                                        .small()
                                        .value(markdown)
                                        .tooltip("Copy Markdown (Command+Shift+M)")
                                        .accessibility_label("Copy uploaded image Markdown snippets")
                                        .on_copied(move |_, window, cx| {
                                            let _ = markdown_view.update(cx, |this, _| {
                                                this.diagnostics.info(
                                                    "Copied uploaded Markdown image snippets to the clipboard.",
                                                );
                                            });
                                            window.push_notification(
                                                Notification::success(
                                                    "Markdown image snippets copied to the clipboard.",
                                                ),
                                                cx,
                                            );
                                        }),
                                ),
                        ),
                )
                .into_any_element()
        });

        v_flex().when_some(result, |container, result| container.child(result))
    }

    fn render_settings(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .id("settings-page")
            .size_full()
            .overflow_y_scroll()
            .child(self.render_settings_content(cx))
    }

    fn render_settings_content(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let selected_is_pinned = self
            .selected_repository
            .as_ref()
            .is_some_and(|repository| self.settings.pinned_repositories.contains(&repository.name));
        let can_pin = self.selected_repository.is_some() && !selected_is_pinned;
        let pinned_repositories = self
            .settings
            .pinned_repositories
            .iter()
            .cloned()
            .collect::<Vec<_>>();

        v_flex()
            .flex_none()
            .p_6()
            .gap_5()
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .child(
                        v_flex()
                            .gap_1()
                            .child(div().font_semibold().text_lg().child("App Preferences"))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("Preferences are saved automatically."),
                            ),
                    )
                    .child(
                        Button::new("close-settings")
                            .outline()
                            .small()
                            .label("Done")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.settings_open = false;
                                cx.notify();
                            })),
                    ),
            )
            .child(
                v_flex()
                    .gap_3()
                    .rounded_lg()
                    .border_1()
                    .border_color(cx.theme().border)
                    .p_4()
                    .child(div().font_semibold().text_sm().child("Organization"))
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("Required. Repositories are loaded from this GitHub organization."),
                    )
                    .child(Input::new(&self.organization_input))
                    .child(
                        Button::new("save-organization")
                            .primary()
                            .small()
                            .self_start()
                            .label("Save organization")
                            .on_click(cx.listener(Self::save_organization)),
                    ),
            )
            .when(self.settings.organization.is_some(), |page| {
                page.child(
                    v_flex()
                        .gap_3()
                        .rounded_lg()
                        .border_1()
                        .border_color(cx.theme().border)
                        .p_4()
                        .child(div().font_semibold().text_sm().child("Pinned repositories"))
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child("Pinned repositories appear first in the repository picker."),
                        )
                        .child(self.render_repository_picker(cx))
                        .child(
                            Button::new("pin-selected-repository")
                                .outline()
                                .small()
                                .self_start()
                                .label(if selected_is_pinned {
                                    "Repository is pinned"
                                } else {
                                    "Pin selected repository"
                                })
                                .disabled(!can_pin)
                                .on_click(cx.listener(Self::pin_selected_repository)),
                        )
                        .when(pinned_repositories.is_empty(), |card| {
                            card.child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("No pinned repositories yet."),
                            )
                        })
                        .children(pinned_repositories.into_iter().map(|repository| {
                            let repository_for_action = repository.clone();
                            h_flex()
                                .items_center()
                                .justify_between()
                                .rounded_md()
                                .bg(cx.theme().muted.opacity(0.28))
                                .px_3()
                                .py_2()
                                .child(div().text_sm().child(repository))
                                .child(
                                    Button::new(format!("unpin-{repository_for_action}"))
                                        .ghost()
                                        .small()
                                        .label("Unpin")
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            this.unpin_repository(
                                                &repository_for_action,
                                                window,
                                                cx,
                                            );
                                        })),
                                )
                        })),
                )
            })
            .child(
                v_flex()
                    .gap_3()
                    .rounded_lg()
                    .border_1()
                    .border_color(cx.theme().border)
                    .p_4()
                    .child(div().font_semibold().text_sm().child("General"))
                    .child(
                        Checkbox::new("start-at-login")
                            .label("Start at Login")
                            .checked(self.settings.start_at_login)
                            .on_click(cx.listener(|this, checked, window, cx| {
                                this.set_start_at_login(*checked, window, cx);
                            })),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(
                                "Uses a per-user macOS LaunchAgent and takes effect at the next login.",
                            ),
                    ),
            )
            .child(
                v_flex()
                    .gap_2()
                    .rounded_lg()
                    .border_1()
                    .border_color(cx.theme().border)
                    .p_4()
                    .child(div().font_semibold().text_sm().child("About"))
                    .child(
                        h_flex()
                            .justify_between()
                            .text_xs()
                            .child("Version")
                            .child(env!("CARGO_PKG_VERSION")),
                    )
                    .child(
                        h_flex()
                            .justify_between()
                            .text_xs()
                            .child("Commit")
                            .child(option_env!("GIT_COMMIT_HASH").unwrap_or("unknown")),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("Authentication is managed by GitHub CLI."),
                    ),
            )
    }
}

impl Focusable for UploaderApp {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for UploaderApp {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let can_send = self.load_state == LoadState::Ready
            && self.selected_repository.is_some()
            && self
                .staged_images
                .iter()
                .any(|image| image.uploaded_url.is_none())
            && !self.uploading;
        let staged_count = self
            .staged_images
            .iter()
            .filter(|image| image.uploaded_url.is_none())
            .count();

        let root = v_flex()
            .key_context(KEY_CONTEXT)
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::on_paste))
            .on_action(cx.listener(Self::paste_and_preview_from_menu))
            .on_action(cx.listener(Self::paste_to_markdown_from_menu))
            .on_action(cx.listener(Self::paste_to_url_from_menu))
            .on_action(cx.listener(Self::switch_to_bulk_upload))
            .size_full()
            .bg(cx.theme().background);

        if self.settings_open {
            root.child(self.render_settings(cx)).into_any_element()
        } else {
            let needs_setup = self.load_state == LoadState::NeedsSetup;
            let root = root
                .on_action(cx.listener(Self::copy_upload_url))
                .on_action(cx.listener(Self::copy_upload_markdown));
            let content = v_flex()
                .id("main-page")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .child(
                    v_flex()
                        .flex_none()
                        .gap_5()
                        .px_6()
                        .pt_6()
                        .pb_5()
                        .child(
                            h_flex()
                                .w_full()
                                .min_w_0()
                                .items_start()
                                .gap_4()
                                .child(
                                    img(self.header_mark.clone())
                                        .size(px(72.))
                                        .flex_shrink_0()
                                        .object_fit(ObjectFit::Contain),
                                )
                                .child(
                                    v_flex()
                                        .flex_1()
                                        .min_w_0()
                                        .gap_3()
                                        .child(
                                            v_flex()
                                                .child(
                                                    div().text_lg().font_semibold().child("Gatto"),
                                                )
                                                .child(
                                                    div()
                                                        .max_w_full()
                                                        .min_w_0()
                                                        .text_xs()
                                                        .whitespace_normal()
                                                        .text_color(cx.theme().muted_foreground)
                                                        .child(
                                                            "Upload an image for GitHub Markdown",
                                                        ),
                                                ),
                                        )
                                        .child(
                                            h_flex()
                                                .w_full()
                                                .min_w_0()
                                                .gap_2()
                                                .flex_wrap()
                                                .child(
                                                    Button::new("open-logs")
                                                        .outline()
                                                        .small()
                                                        .label("Application logs")
                                                        .on_click(
                                                            cx.listener(Self::open_log_window),
                                                        ),
                                                )
                                                .child(
                                                    Button::new("open-settings")
                                                        .outline()
                                                        .small()
                                                        .label("App Preferences")
                                                        .on_click(cx.listener(|this, _, _, cx| {
                                                            this.show_settings(cx);
                                                        })),
                                                ),
                                        ),
                                ),
                        )
                        .when(needs_setup, |root| root.child(self.render_setup_prompt(cx)))
                        .when(!needs_setup, |root| {
                            root.child(self.render_repository_picker(cx))
                                .when(
                                    !self.uploading
                                        && self
                                            .staged_images
                                            .iter()
                                            .all(|image| image.uploaded_url.is_none()),
                                    |root| root.child(self.render_bulk_upload_control(cx)),
                                )
                                .child(self.render_drop_zone(cx))
                                .child(self.render_attachments(cx))
                                .child(self.render_upload_result(cx))
                        }),
                );
            root.child(content)
                .child(
                    v_flex().items_end().gap_3().px_6().py_3().child(
                        Button::new("send-image")
                            .primary()
                            .large()
                            .label(if self.uploading {
                                if staged_count == 1 {
                                    "Uploading image…".to_owned()
                                } else {
                                    format!("Uploading {staged_count} images…")
                                }
                            } else if staged_count == 1 {
                                "Upload image".to_owned()
                            } else if staged_count > 1 {
                                format!("Upload {staged_count} images")
                            } else {
                                "Upload images".to_owned()
                            })
                            .icon(AssetIconName::Upload)
                            .loading(self.uploading)
                            .disabled(!can_send)
                            .on_click(cx.listener(Self::send)),
                    ),
                )
                .into_any_element()
        }
    }
}

fn markdown_image(file_name: &str, url: &str) -> String {
    let alt_text = file_name
        .replace('\\', "\\\\")
        .replace('[', "\\[")
        .replace(']', "\\]")
        .replace(['\r', '\n'], " ");
    format!("![{alt_text}]({url})")
}

pub fn init_keybindings(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("cmd-v", PasteImage, Some(KEY_CONTEXT)),
        KeyBinding::new("ctrl-v", PasteImage, Some(KEY_CONTEXT)),
        KeyBinding::new("cmd-shift-c", CopyUploadUrl, Some(KEY_CONTEXT)),
        KeyBinding::new("cmd-shift-m", CopyUploadMarkdown, Some(KEY_CONTEXT)),
        KeyBinding::new("b", SwitchToBulkUpload, Some("Dialog")),
    ]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bulk_upload_result_preserves_order_with_newline_separators() {
        let result = UploadResult {
            images: vec![
                UploadedImage {
                    id: 1,
                    url: "https://example.com/first".into(),
                    markdown: "![first](https://example.com/first)".into(),
                },
                UploadedImage {
                    id: 2,
                    url: "https://example.com/second".into(),
                    markdown: "![second](https://example.com/second)".into(),
                },
            ],
        };

        assert_eq!(
            result.urls(),
            "https://example.com/first\nhttps://example.com/second"
        );
        assert_eq!(
            result.markdown(),
            "![first](https://example.com/first)\n![second](https://example.com/second)"
        );
    }
}

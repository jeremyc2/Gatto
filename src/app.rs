use std::{
    cell::Cell,
    collections::{HashMap, VecDeque},
    path::PathBuf,
    rc::Rc,
    sync::Arc,
};

use gpui_kit::assets::IconName as AssetIconName;
use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Icon, IconName, Sizable as _, StyledExt as _,
    WindowExt as _,
    attachment::{
        Attachment, AttachmentActions, AttachmentContent, AttachmentDescription, AttachmentMedia,
        AttachmentStatus, AttachmentTitle,
    },
    button::Button,
    checkbox::Checkbox,
    combobox::{Combobox, ComboboxEvent, ComboboxState},
    h_flex,
    input::{Input, InputEvent, InputState},
    kbd::Kbd,
    link::Link,
    menu::DropdownMenu as _,
    notification::Notification,
    progress::Progress,
    searchable_list::{SearchableGroup, SearchableVec},
    spinner::Spinner,
    switch::Switch,
    v_flex,
};
use gpui_kit::{
    AnyWindowHandle, App, AppContext as _, Bounds, BoxShadow, ClipboardEntry, ClipboardItem,
    Context, Entity, ExternalPaths, FocusHandle, Focusable, Image, ImageFormat,
    InteractiveElement as _, IntoElement, KeyBinding, KeyDownEvent, Keystroke, MouseButton,
    ObjectFit, ParentElement as _, PathPromptOptions, Render, SharedString,
    StatefulInteractiveElement as _, Styled as _, StyledImage as _, Subscription, Window,
    WindowBounds, WindowOptions, actions, div, img, prelude::FluentBuilder as _, px, size,
};

use crate::{
    compression,
    diagnostics::Diagnostics,
    github,
    global_shortcut::{GlobalShortcutController, canonicalize_shortcut, shortcut_from_keystroke},
    image_preview::ImagePreview,
    log_viewer::LogViewer,
    menu_bar::MenuBarController,
    model::{
        AttachmentUploadState, Media, OversizedVideo, Repository, StagedAttachment,
        UNSUPPORTED_MEDIA_MESSAGE, VideoUploadLimit, parse_owner,
    },
    settings::{AppSettings, ReleaseChannel},
    theme::{self, AppButtonStyle as _},
    updater::{UpdateState, Updater},
    window_limits,
};

actions!(
    gatto,
    [
        PasteImage,
        PreviewFromClipboard,
        CopyUploadMarkdown,
        SwitchToBulkUpload,
        SelectStableChannel,
        SelectBetaChannel
    ]
);

const KEY_CONTEXT: &str = "Gatto";
const WALKTHROUGH_URL: &str = "https://github.com/jeremyc2/Gatto/blob/main/docs/WALKTHROUGH.md";
const REPOSITORY_URL: &str = "https://github.com/jeremyc2/Gatto";

type RepositoryPicker = ComboboxState<SearchableVec<SearchableGroup<String>>>;

#[derive(Default)]
struct AppControllers {
    menu_bar: MenuBarController,
    global_shortcut: GlobalShortcutController,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LoadState {
    NeedsSetup,
    Idle,
    Loading,
    Ready,
    Failed,
}

#[derive(Clone)]
struct UploadedAttachment {
    markdown: String,
}

#[derive(Clone)]
struct UploadResult {
    attachments: Vec<UploadedAttachment>,
}

impl UploadResult {
    fn markdown(&self) -> String {
        self.attachments
            .iter()
            .map(|image| image.markdown.as_str())
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

enum UploadEvent {
    Progress {
        id: u64,
        sent: u64,
        total: u64,
    },
    Finished {
        id: u64,
        result: Result<String, github::GithubError>,
    },
    Complete {
        canceled: bool,
    },
}

struct PendingImport {
    remaining: VecDeque<anyhow::Result<StagedAttachment>>,
    processed: Vec<anyhow::Result<StagedAttachment>>,
}

struct CompressionJob {
    control: compression::CompressionControl,
    name: String,
    progress: compression::Progress,
}

enum CompressionEvent {
    Progress(compression::Progress),
    Finished(anyhow::Result<StagedAttachment>),
}

#[cfg(feature = "walkthrough-screenshots")]
#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub(crate) enum WalkthroughScenario {
    FirstRun,
    Preferences,
    ImageStaged,
    UploadComplete,
    UpdateAvailable,
    VideoStaged,
    MixedUploadProgress,
    VideoTooLarge,
    VideoCompressing,
}

#[cfg(feature = "walkthrough-screenshots")]
#[allow(dead_code)]
impl WalkthroughScenario {
    pub(crate) const ALL: [Self; 9] = [
        Self::FirstRun,
        Self::Preferences,
        Self::ImageStaged,
        Self::UploadComplete,
        Self::UpdateAvailable,
        Self::VideoStaged,
        Self::MixedUploadProgress,
        Self::VideoTooLarge,
        Self::VideoCompressing,
    ];

    pub(crate) const fn file_name(self) -> &'static str {
        match self {
            Self::FirstRun => "01-first-run.png",
            Self::Preferences => "02-preferences.png",
            Self::ImageStaged => "03-image-staged.png",
            Self::UploadComplete => "04-upload-complete.png",
            Self::UpdateAvailable => "05-update.png",
            Self::VideoStaged => "06-video-staged.png",
            Self::MixedUploadProgress => "07-mixed-upload-progress.png",
            Self::VideoTooLarge => "08-video-too-large.png",
            Self::VideoCompressing => "09-video-compressing.png",
        }
    }
}

pub struct UploaderApp {
    owner_input: Entity<InputState>,
    global_shortcut_input: Entity<InputState>,
    repository_picker: Entity<RepositoryPicker>,
    repositories: HashMap<String, Repository>,
    selected_repository: Option<Repository>,
    token: Option<String>,
    loading_token: bool,
    loading_repository: bool,
    load_state: LoadState,
    staged_attachments: Vec<StagedAttachment>,
    bulk_upload: bool,
    paste_generation: u64,
    state_generation: u64,
    upload_result: Option<UploadResult>,
    uploading: bool,
    upload_control: Option<github::UploadControl>,
    upload_generation: u64,
    pending_import: Option<PendingImport>,
    compression: Option<CompressionJob>,
    settings: AppSettings,
    settings_open: bool,
    recording_global_shortcut: bool,
    pending_paste_repository: Option<String>,
    pending_menu_paste: bool,
    pending_replacement_attachments: Option<Vec<StagedAttachment>>,
    log_window: Option<(AnyWindowHandle, Entity<LogViewer>)>,
    preview_window: Option<(AnyWindowHandle, Entity<ImagePreview>)>,
    menu_bar: MenuBarController,
    global_shortcut: GlobalShortcutController,
    diagnostics: Diagnostics,
    updater: Entity<Updater>,
    _update_subscription: Subscription,
    header_mark: Arc<Image>,
    rooftops_window: Option<AnyWindowHandle>,
    focus_handle: FocusHandle,
    _owner_subscription: Subscription,
    _global_shortcut_subscription: Subscription,
    _repository_subscription: Subscription,
}

impl UploaderApp {
    pub fn new(
        window: &mut Window,
        diagnostics: Diagnostics,
        menu_bar: MenuBarController,
        global_shortcut: GlobalShortcutController,
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
        Self::with_settings(
            window,
            diagnostics,
            AppControllers {
                menu_bar,
                global_shortcut,
            },
            settings,
            settings_error,
            true,
            cx,
        )
    }

    fn with_settings(
        window: &mut Window,
        diagnostics: Diagnostics,
        controllers: AppControllers,
        settings: AppSettings,
        settings_error: Option<String>,
        persist_repository_selection: bool,
        cx: &mut Context<Self>,
    ) -> Self {
        let initial_owner = settings.owner.clone().unwrap_or_default();
        let initial_shortcut = settings.global_quick_copy_shortcut.clone();
        let owner_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("account or organization")
                .default_value(initial_owner)
        });
        let owner_subscription = cx.subscribe(&owner_input, |_: &mut Self, _, event, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify();
            }
        });
        let global_shortcut_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Command+Shift+G")
                .default_value(initial_shortcut)
        });
        let global_shortcut_subscription =
            cx.subscribe(&global_shortcut_input, |_: &mut Self, _, event, cx| {
                if matches!(event, InputEvent::Change) {
                    cx.notify();
                }
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
            move |this: &mut Self,
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
                    this.settings.last_repository_id = this
                        .selected_repository
                        .as_ref()
                        .map(|repository| repository.id);
                    if persist_repository_selection && let Err(error) = this.settings.save() {
                        this.diagnostics.error(format!(
                            "Could not remember the selected repository: {error}"
                        ));
                    }
                    cx.notify();
                }
            },
        );

        let updater = Updater::shared(cx);
        let update_subscription = cx.observe(&updater, |_, _, cx| cx.notify());
        let mut this = Self {
            owner_input,
            global_shortcut_input,
            repository_picker,
            repositories: HashMap::new(),
            selected_repository: None,
            token: None,
            loading_token: false,
            loading_repository: false,
            load_state: if settings.owner.is_some() {
                LoadState::Idle
            } else {
                LoadState::NeedsSetup
            },
            staged_attachments: Vec::new(),
            bulk_upload: false,
            paste_generation: 0,
            state_generation: 0,
            upload_result: None,
            uploading: false,
            upload_control: None,
            upload_generation: 0,
            pending_import: None,
            compression: None,
            settings,
            settings_open: false,
            recording_global_shortcut: false,
            pending_paste_repository: None,
            pending_menu_paste: false,
            pending_replacement_attachments: None,
            log_window: None,
            preview_window: None,
            menu_bar: controllers.menu_bar,
            global_shortcut: controllers.global_shortcut,
            diagnostics,
            updater,
            _update_subscription: update_subscription,
            header_mark: Arc::new(Image::from_bytes(
                ImageFormat::Png,
                include_bytes!("../packaging/rocket-cat-transparent.png").to_vec(),
            )),
            rooftops_window: None,
            focus_handle: cx.focus_handle(),
            _owner_subscription: owner_subscription,
            _global_shortcut_subscription: global_shortcut_subscription,
            _repository_subscription: subscription,
        };
        if let Some(error) = settings_error {
            window.push_notification(Notification::error(error), cx);
        }
        if persist_repository_selection {
            this.restore_remembered_repository(window, cx);
            this.restore_global_shortcut(window, cx);
        }
        this
    }

    #[cfg(feature = "walkthrough-screenshots")]
    #[allow(dead_code)]
    pub(crate) fn for_walkthrough(
        scenario: WalkthroughScenario,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut settings = AppSettings::default();
        if !matches!(scenario, WalkthroughScenario::FirstRun) {
            settings.owner = Some("acme-labs".into());
            settings.pinned_repositories.insert("gatto".into());
            settings.last_repository = Some("gatto".into());
            settings.last_repository_id = Some(1001);
        }

        let mut this = Self::with_settings(
            window,
            Diagnostics::new(),
            AppControllers::default(),
            settings,
            None,
            false,
            cx,
        );

        if matches!(scenario, WalkthroughScenario::FirstRun) {
            return this;
        }

        for repository in [
            Repository {
                id: 1001,
                name: "gatto".into(),
            },
            Repository {
                id: 1002,
                name: "developer-portal".into(),
            },
            Repository {
                id: 1003,
                name: "documentation".into(),
            },
        ] {
            this.repositories
                .insert(repository.name.clone(), repository);
        }
        this.selected_repository = this.repositories.get("gatto").cloned();
        this.load_state = LoadState::Ready;
        this.settings_open = matches!(scenario, WalkthroughScenario::Preferences);

        if matches!(
            scenario,
            WalkthroughScenario::ImageStaged
                | WalkthroughScenario::UploadComplete
                | WalkthroughScenario::MixedUploadProgress
        ) {
            let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("packaging/rocket-cat-transparent.png");
            let mut image = StagedAttachment::from_path(path, Default::default())
                .expect("the walkthrough fixture image should always be valid");
            image.name = "rocket-cat.png".into();
            image.description = "Gatto rocket cat".into();

            if matches!(
                scenario,
                WalkthroughScenario::UploadComplete | WalkthroughScenario::MixedUploadProgress
            ) {
                let url = "https://github.com/user-attachments/assets/01234567-89ab-cdef-0123-456789abcdef";
                image.upload_state = AttachmentUploadState::Uploaded { url: url.into() };
                this.upload_result = Some(UploadResult {
                    attachments: vec![UploadedAttachment {
                        markdown: image.snippet(url, this.settings.image_copy_format()),
                    }],
                });
            }
            this.staged_attachments.push(image);
        }

        if matches!(
            scenario,
            WalkthroughScenario::VideoStaged | WalkthroughScenario::MixedUploadProgress
        ) {
            let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample.mp4");
            let mut video = StagedAttachment::from_path(path, Default::default())
                .expect("valid walkthrough video fixture");
            video.name = "interaction-recording.mp4".into();
            video.description = "Interaction recording".into();
            if matches!(scenario, WalkthroughScenario::MixedUploadProgress) {
                this.bulk_upload = true;
                this.uploading = true;
                this.upload_control = Some(github::UploadControl::default());
                video.upload_state = AttachmentUploadState::Uploading {
                    sent: video.len() * 65 / 100,
                    total: video.len(),
                };
            }
            this.staged_attachments.push(video);
        }

        if matches!(scenario, WalkthroughScenario::VideoCompressing) {
            this.pending_import = Some(PendingImport {
                remaining: VecDeque::new(),
                processed: Vec::new(),
            });
            this.compression = Some(CompressionJob {
                control: compression::CompressionControl::default(),
                name: "interaction-recording.mov".into(),
                progress: compression::Progress {
                    attempt: 1,
                    percent: 42,
                },
            });
        }
        if matches!(scenario, WalkthroughScenario::VideoTooLarge) {
            this.pending_import = Some(PendingImport {
                remaining: VecDeque::new(),
                processed: Vec::new(),
            });
            let view = cx.entity().downgrade();
            window.defer(cx, move |window, cx| {
                let _ = view.update(cx, |this, cx| {
                    this.prompt_compression(
                        OversizedVideo {
                            path: PathBuf::from("interaction-recording.mov"),
                            bytes: 24_000_000,
                            limit_mb: 10,
                        },
                        window,
                        cx,
                    )
                });
            });
        }

        if matches!(scenario, WalkthroughScenario::UpdateAvailable) {
            this.updater.update(cx, |updater, cx| {
                updater.state = UpdateState::Available;
                updater.message = "Install the new version and restart Gatto".into();
                cx.notify();
            });
        }
        this.refresh_repository_picker(window, cx);
        this
    }

    pub fn show_main_page(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.cancel_global_shortcut_recording(window, cx);
        self.settings_open = false;
        window.focus(&self.focus_handle, cx);
        cx.notify();
    }

    pub fn show_settings(&mut self, cx: &mut Context<Self>) {
        self.diagnostics.info("Opened application preferences.");
        self.settings_open = true;
        cx.notify();
    }

    pub(crate) fn dismiss(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.recording_global_shortcut {
            self.cancel_global_shortcut_recording(window, cx);
        } else if self.settings_open {
            self.show_main_page(window, cx);
        }
    }

    fn restore_global_shortcut(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.settings.global_quick_copy_shortcut_enabled {
            return;
        }
        match self
            .global_shortcut
            .set_shortcut(Some(&self.settings.global_quick_copy_shortcut))
        {
            Ok(Some(canonical)) => {
                self.settings.global_quick_copy_shortcut = canonical.clone();
                self.global_shortcut_input.update(cx, |input, cx| {
                    input.set_value(canonical, window, cx);
                });
                self.diagnostics
                    .info("Registered the global Quick Copy shortcut.");
            }
            Ok(None) => {}
            Err(error) => {
                self.settings.global_quick_copy_shortcut_enabled = false;
                let _ = self.settings.save();
                self.diagnostics.error(format!(
                    "Could not restore the global Quick Copy shortcut: {error}"
                ));
                window.push_notification(
                    Notification::error(format!("Global Quick Copy shortcut disabled: {error}")),
                    cx,
                );
            }
        }
    }

    fn paste_and_preview_from_menu(
        &mut self,
        _: &PreviewFromClipboard,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.paste_from_menu(window, cx);
    }

    /// Stages the current clipboard image and prefers the last selected repository
    /// when it is pinned, otherwise the first pinned repository alphabetically.
    fn paste_from_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.uploading
            || self.loading_token
            || self.loading_repository
            || self.pending_menu_paste
        {
            self.diagnostics
                .info("Ignored a clipboard action because another one is in progress.");
            return;
        }
        self.settings_open = false;
        self.pending_menu_paste = true;

        let repository_name = self
            .settings
            .last_repository
            .as_ref()
            .filter(|name| self.settings.pinned_repositories.contains(*name))
            .cloned()
            .or_else(|| self.settings.pinned_repositories.first().cloned());
        let Some(repository_name) = repository_name else {
            self.warn(
                "Paste actions need a pinned repository. Pin one in App Preferences first.",
                window,
                cx,
            );
            self.pending_menu_paste = false;
            return;
        };
        if self.load_state == LoadState::NeedsSetup {
            self.warn(
                "Paste actions need an account or organization in App Preferences first.",
                window,
                cx,
            );
            self.pending_menu_paste = false;
            return;
        }

        self.diagnostics.info(format!(
            "Menu paste requested with pinned repository {repository_name}."
        ));
        self.pending_paste_repository = Some(repository_name.clone());
        if matches!(self.load_state, LoadState::Idle | LoadState::Ready) {
            if !self.select_repository(&repository_name, window, cx) {
                self.pending_paste_repository = None;
                self.warn(
                    "The pinned repository is not available to the active GitHub account.",
                    window,
                    cx,
                );
                self.pending_menu_paste = false;
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

    /// Clears a pasted image when the main window is closed, or when the user
    /// explicitly discards it. Incrementing the generation keeps a pending
    /// clipboard conversion from restoring an image after it was cleared.
    pub fn clear_staged_attachments(&mut self, reason: &str, cx: &mut Context<Self>) {
        self.paste_generation = self.paste_generation.wrapping_add(1);
        self.upload_generation = self.upload_generation.wrapping_add(1);
        if let Some(control) = self.upload_control.take() {
            control.cancel();
        }
        if let Some(job) = self.compression.take() {
            job.control.cancel();
        }
        self.pending_import = None;
        self.uploading = false;
        self.pending_menu_paste = false;
        self.pending_paste_repository = None;
        if !self.staged_attachments.is_empty() {
            self.staged_attachments.clear();
            self.upload_result = None;
            self.diagnostics.info(reason);
        }
        cx.notify();
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

    pub fn report_custom_url_error(
        &mut self,
        error: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let error = error.into();
        self.diagnostics
            .error(format!("Could not handle a custom URL: {error}"));
        window.push_notification(Notification::error(error), cx);
    }

    fn load_repositories(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.repositories.clear();
        let Some(owner) = self.settings.owner.clone() else {
            self.diagnostics
                .info("Repository loading is waiting for an account or organization setting.");
            self.load_state = LoadState::NeedsSetup;
            cx.notify();
            return;
        };

        self.load_state = LoadState::Loading;
        self.repository_picker.update(cx, |picker, cx| {
            picker.set_items(
                SearchableVec::new(Vec::<SearchableGroup<String>>::new()),
                window,
                cx,
            );
        });
        self.diagnostics.info(format!(
            "Loading repositories for account or organization {owner}."
        ));
        cx.notify();

        let diagnostics = self.diagnostics.clone();
        let state_generation = self.state_generation;
        cx.spawn_in(window, async move |this, window| {
            let requested = owner.clone();
            let result = window
                .background_spawn(async move { github::load_session(&owner, &diagnostics) })
                .await;
            let _ = this.update_in(window, move |this, window, cx| {
                if this.state_generation != state_generation {
                    return;
                }
                if this.settings.owner.as_deref() != Some(requested.as_str()) {
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
                                this.pending_menu_paste = false;
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
                        } else {
                            this.selected_repository = None;
                            this.repository_picker.update(cx, |picker, cx| {
                                picker.set_selected_indices([], window, cx);
                            });
                        }
                        this.try_complete_menu_paste(cx);
                    }
                    Err(error) => {
                        this.diagnostics
                            .error(format!("Repository loading failed: {error}"));
                        this.load_state = LoadState::Failed;
                        this.pending_menu_paste = false;
                        window.push_notification(Notification::error(error.to_string()), cx);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn restore_remembered_repository(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(repository) = self.settings.preferred_repository().map(|name| Repository {
            id: self.settings.last_repository_id.unwrap_or_default(),
            name: name.to_owned(),
        }) else {
            return;
        };

        self.repositories
            .insert(repository.name.clone(), repository.clone());
        self.selected_repository = Some(repository.clone());
        self.repository_picker.update(cx, |picker, cx| {
            picker.set_items(
                SearchableVec::new(vec![
                    SearchableGroup::new("Selected repository")
                        .items(vec![repository.name.clone()]),
                ]),
                window,
                cx,
            );
            picker.set_selected_values(&[repository.name], window, cx);
        });
        self.diagnostics.info(
            "Restored the previously selected repository without loading the repository list.",
        );
    }

    fn load_repositories_when_opened(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.load_state == LoadState::Idle {
            self.load_repositories(window, cx);
        }
    }

    fn choose_file(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.diagnostics.info("Opened the attachment file picker.");
        if self.uploading || self.import_busy() {
            return;
        }
        let multiple = self.bulk_upload;
        let prompt = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple,
            prompt: Some(if multiple {
                "Choose images or videos".into()
            } else {
                "Choose an image or video".into()
            }),
        });
        let state_generation = self.state_generation;
        let staging_generation = self.upload_generation;
        let video_limit = self.settings.video_upload_limit();

        cx.spawn_in(window, async move |this, window| {
            let Some(paths) = prompt.await.ok().and_then(Result::ok).flatten() else {
                return;
            };
            let results = window
                .background_spawn(async move {
                    paths
                        .into_iter()
                        .map(|path| StagedAttachment::from_path(path, video_limit))
                        .collect::<Vec<_>>()
                })
                .await;
            let _ = this.update_in(window, |this, window, cx| {
                if this.state_generation != state_generation
                    || this.upload_generation != staging_generation
                {
                    return;
                }
                this.accept_attachment_results(results, window, cx);
            });
        })
        .detach();
    }

    fn on_paste(&mut self, _: &PasteImage, window: &mut Window, cx: &mut Context<Self>) {
        if self.load_state == LoadState::NeedsSetup {
            return;
        }
        if self.uploading || self.import_busy() {
            self.warn(
                "Finish the current upload or compression before adding attachments.",
                window,
                cx,
            );
            return;
        }
        let Some(clipboard) = cx.read_from_clipboard() else {
            self.warn(UNSUPPORTED_MEDIA_MESSAGE, window, cx);
            self.pending_menu_paste = false;
            self.pending_paste_repository = None;
            return;
        };

        if let Some(image) = clipboard.entries().iter().find_map(|entry| match entry {
            ClipboardEntry::Image(image) => Some(image.clone()),
            _ => None,
        }) {
            let paste_generation = self.paste_generation;
            let task = cx.background_spawn(async move { StagedAttachment::from_clipboard(&image) });
            cx.spawn_in(window, async move |this, window| {
                let result = task.await;
                let _ = this.update_in(window, |this, window, cx| {
                    this.accept_pasted_attachment_result(paste_generation, result, window, cx);
                });
            })
            .detach();
            return;
        }

        if let Some(paths) = clipboard.entries().iter().find_map(|entry| match entry {
            ClipboardEntry::ExternalPaths(paths) => Some(paths.paths().to_vec()),
            _ => None,
        }) {
            Self::load_dropped_paths(
                cx.entity().downgrade(),
                self.state_generation,
                self.upload_generation,
                paths,
                self.settings.video_upload_limit(),
                window,
                cx,
            );
            return;
        }

        self.warn(UNSUPPORTED_MEDIA_MESSAGE, window, cx);
        self.pending_menu_paste = false;
        self.pending_paste_repository = None;
    }

    fn load_dropped_paths(
        view: gpui_kit::WeakEntity<Self>,
        state_generation: u64,
        staging_generation: u64,
        paths: Vec<PathBuf>,
        video_limit: VideoUploadLimit,
        window: &mut Window,
        cx: &mut App,
    ) {
        if paths.is_empty() {
            return;
        }
        let task = cx.background_spawn(async move {
            paths
                .into_iter()
                .map(|path| StagedAttachment::from_path(path, video_limit))
                .collect::<Vec<_>>()
        });
        window
            .spawn(cx, async move |window| {
                let results = task.await;
                let _ = view.update_in(window, |this, window, cx| {
                    if this.state_generation != state_generation
                        || this.upload_generation != staging_generation
                    {
                        return;
                    }
                    this.accept_attachment_results(results, window, cx);
                });
            })
            .detach();
    }

    fn accept_attachment_result(
        &mut self,
        result: anyhow::Result<StagedAttachment>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.accept_attachment_results(vec![result], window, cx);
    }

    fn import_busy(&self) -> bool {
        self.pending_import.is_some() || self.compression.is_some()
    }

    fn accept_attachment_results(
        &mut self,
        results: Vec<anyhow::Result<StagedAttachment>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.uploading || self.import_busy() {
            self.warn(
                "Finish the current upload or compression before adding more files.",
                window,
                cx,
            );
            return;
        }
        self.pending_import = Some(PendingImport {
            remaining: results.into(),
            processed: Vec::new(),
        });
        self.advance_import(window, cx);
    }

    fn advance_import(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(mut batch) = self.pending_import.take() else {
            return;
        };
        while let Some(result) = batch.remaining.pop_front() {
            match result {
                Ok(attachment) => batch.processed.push(Ok(attachment)),
                Err(error) => match error.downcast::<OversizedVideo>() {
                    Ok(video) => {
                        self.pending_import = Some(batch);
                        self.prompt_compression(video, window, cx);
                        return;
                    }
                    Err(error) => batch.processed.push(Err(error)),
                },
            }
        }
        self.accept_ready_attachment_results(batch.processed, window, cx);
    }

    fn prompt_compression(
        &mut self,
        video: OversizedVideo,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let view = cx.entity().downgrade();
        let generation = self.upload_generation;
        let resolved = Rc::new(Cell::new(false));
        let limit_mb = video.limit_mb;
        let name = video
            .path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Video".into());
        window.open_alert_dialog(cx, move |alert, _, _| {
            let accept_view = view.clone();
            let close_view = view.clone();
            let path = video.path.clone();
            let accepted = resolved.clone();
            let closed = resolved.clone();
            alert.width(px(480.)).title(format!("Video exceeds {limit_mb} MB"))
                .description(format!("{name} is {:.1} MB. Compress it to an H.264 MP4 below {limit_mb} MB? The original stays unchanged. Resolution is preserved; visual quality may decrease and audio becomes mono AAC. For best results, keep recordings under 15 seconds.", video.bytes as f64 / 1_000_000.))
                .show_cancel(true).cancel_text("Cancel").ok_text("Compress").footer(theme::confirmation_footer("ok", "Compress", 104.))
                .on_ok(move |_, window, cx| {
                    accepted.set(true);
                    let _ = accept_view.update(cx, |this, cx| {
                        if this.upload_generation == generation && this.pending_import.is_some() {
                            this.start_compression(path.clone(), window, cx);
                        }
                    });
                    true
                })
                .on_close(move |_, window, cx| {
                    if !closed.replace(true) {
                        let close_view = close_view.clone();
                        window.defer(cx, move |window, cx| {
                            let _ = close_view.update(cx, |this, cx| {
                                if this.upload_generation == generation { this.advance_import(window, cx); }
                            });
                        });
                    }
                })
        });
        cx.notify();
    }

    fn start_compression(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        let control = compression::CompressionControl::default();
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Video".into());
        self.compression = Some(CompressionJob {
            control: control.clone(),
            name,
            progress: compression::Progress {
                attempt: 1,
                percent: 0,
            },
        });
        let generation = self.upload_generation;
        let (sender, receiver) = async_channel::unbounded();
        let video_limit = self.settings.video_upload_limit();
        cx.background_spawn(async move {
            let progress = sender.clone();
            let result = compression::compress(&path, video_limit, &control, move |value| {
                let _ = progress.try_send(CompressionEvent::Progress(value));
            });
            let _ = sender.try_send(CompressionEvent::Finished(result));
        })
        .detach();
        cx.spawn_in(window, async move |this, window| {
            while let Ok(event) = receiver.recv().await {
                let finished = matches!(event, CompressionEvent::Finished(_));
                let result = this.update_in(window, |this, window, cx| {
                    if this.upload_generation != generation || this.compression.is_none() {
                        return false;
                    }
                    match event {
                        CompressionEvent::Progress(value) => {
                            if let Some(job) = &mut this.compression {
                                job.progress = value;
                            }
                        }
                        CompressionEvent::Finished(result) => {
                            this.compression = None;
                            if let Some(batch) = &mut this.pending_import {
                                match result {
                                    Err(error)
                                        if error
                                            .downcast_ref::<compression::Canceled>()
                                            .is_some() =>
                                    {
                                        this.diagnostics.info("Video compression canceled.")
                                    }
                                    result => batch.processed.push(result),
                                }
                            }
                            this.advance_import(window, cx);
                        }
                    }
                    cx.notify();
                    true
                });
                if finished || !matches!(result, Ok(true)) {
                    break;
                }
            }
        })
        .detach();
        cx.notify();
    }

    fn render_compression(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity().downgrade();
        v_flex().when_some(self.compression.as_ref(), |content, job| {
            content.child(
                v_flex()
                    .gap_2()
                    .p_3()
                    .rounded_lg()
                    .border_1()
                    .border_color(cx.theme().border)
                    .child(div().text_sm().child(format!(
                        "Compressing {} · pass {} of 2 · {}%",
                        job.name, job.progress.attempt, job.progress.percent
                    )))
                    .child(
                        Progress::new("compression-progress")
                            .value(job.progress.percent as f32)
                            .color(cx.theme().primary)
                            .small()
                            .accessibility_label("Video compression progress"),
                    )
                    .child(
                        Button::new("cancel-compression")
                            .app_secondary()
                            .label(if job.control.is_canceled() {
                                "Canceling…"
                            } else {
                                "Cancel compression"
                            })
                            .disabled(job.control.is_canceled())
                            .on_click(move |_, _, cx| {
                                let _ = view.update(cx, |this, cx| {
                                    if let Some(job) = &this.compression {
                                        job.control.cancel();
                                    }
                                    cx.notify();
                                });
                            }),
                    ),
            )
        })
    }

    fn accept_ready_attachment_results(
        &mut self,
        results: Vec<anyhow::Result<StagedAttachment>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.uploading {
            self.warn(
                "Wait for the upload to finish before adding attachments.",
                window,
                cx,
            );
            return;
        }
        let mut images = Vec::new();
        let mut errors = Vec::new();
        for result in results {
            match result {
                Ok(image) => images.push(image),
                Err(error) => {
                    self.diagnostics
                        .error(format!("Attachment staging failed: {error}"));
                    errors.push(error.to_string());
                }
            }
        }

        if images.is_empty() {
            if let Some(error) = errors.first() {
                self.warn(error.clone(), window, cx);
            }
            self.pending_menu_paste = false;
            self.pending_paste_repository = None;
            return;
        }
        if !errors.is_empty() {
            self.warn(
                format!(
                    "Added {} attachment{}; {} file{} could not be read. {}",
                    images.len(),
                    if images.len() == 1 { "" } else { "s" },
                    errors.len(),
                    if errors.len() == 1 { "" } else { "s" },
                    errors[0],
                ),
                window,
                cx,
            );
        }
        self.stage_attachments(images, window, cx);
    }

    fn stage_attachments(
        &mut self,
        images: Vec<StagedAttachment>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.bulk_upload || self.staged_attachments.is_empty() {
            // Preserve multi-file input before deciding how to handle an
            // existing attachment. An empty draft can become a batch directly.
            self.bulk_upload |= images.len() > 1;
            self.append_staged_attachments(images, window, cx);
        } else {
            self.open_replace_attachment_dialog(images, window, cx);
        }
    }

    fn append_staged_attachments(
        &mut self,
        mut images: Vec<StagedAttachment>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let added = images.len();
        self.staged_attachments.append(&mut images);
        self.sync_upload_result();
        self.diagnostics.info(format!(
            "Staged {added} attachment{} for upload.",
            if added == 1 { "" } else { "s" }
        ));
        self.try_complete_menu_paste(cx);
        cx.notify();
    }

    fn replace_staged_attachment(
        &mut self,
        image: StagedAttachment,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.staged_attachments.clear();
        self.append_staged_attachments(vec![image], window, cx);
    }

    fn change_attachment_description(
        &mut self,
        image_id: u64,
        requested_name: &str,
        cx: &mut Context<Self>,
    ) -> anyhow::Result<()> {
        let image = self
            .staged_attachments
            .iter_mut()
            .find(|image| image.id == image_id)
            .ok_or_else(|| anyhow::anyhow!("That attachment is no longer staged."))?;
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
        let Some(image) = self
            .staged_attachments
            .iter()
            .find(|image| image.id == image_id)
        else {
            return;
        };
        let current_description = image.description.clone();
        let input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Descriptive attachment name")
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
                    "Images use this description as alt text. Video descriptions stay in Gatto; GitHub receives a video URL. The uploaded filename stays the same.",
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
                .footer(theme::confirmation_footer("ok", "Save description", 148.))
                .on_ok(move |_, window, cx| {
                    let requested_name = rename_input.read(cx).value().to_string();
                    match rename_view.update(cx, |this, cx| {
                        this.change_attachment_description(image_id, &requested_name, cx)
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

    fn open_replace_attachment_dialog(
        &mut self,
        images: Vec<StagedAttachment>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if window.has_active_dialog(cx) {
            window.close_dialog(cx);
        }
        let Some(replacement) = images.first().cloned() else {
            return;
        };
        self.pending_replacement_attachments = Some(images.clone());
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
                .icon(Icon::new(IconName::CircleAlert).text_color(cx.theme().warning))
                .title("Replace the current attachment?")
                .description(
                    "This will replace the current attachment. Switch to bulk upload to keep both.",
                )
                .child(
                    v_flex()
                        .track_focus(&bulk_shortcut_focus)
                        .on_action(move |_: &SwitchToBulkUpload, window, cx| {
                            let _ = shortcut_bulk_view.update(cx, |this, cx| {
                                this.pending_replacement_attachments = None;
                                this.bulk_upload = true;
                                this.append_staged_attachments(
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
                                .child("Turn on bulk upload and add this attachment to the batch."),
                        )
                        .child(
                            Button::new("switch-to-bulk-upload")
                                .app_secondary()
                                .self_start()
                                .flex_none()
                                .label("Switch to bulk upload")
                                .child(Kbd::new(Keystroke::parse("b").expect("valid shortcut")))
                                .on_click(move |_, window, cx| {
                                    let _ = bulk_view.update(cx, |this, cx| {
                                        this.pending_replacement_attachments = None;
                                        this.bulk_upload = true;
                                        this.append_staged_attachments(
                                            bulk_images.clone(),
                                            window,
                                            cx,
                                        );
                                    });
                                    window.close_dialog(cx);
                                }),
                        ),
                )
                .footer(theme::confirmation_footer(
                    "replace-image",
                    "Replace attachment",
                    176.,
                ))
                .on_ok(move |_, window, cx| {
                    let _ = replace_view.update(cx, |this, cx| {
                        this.pending_replacement_attachments = None;
                        this.replace_staged_attachment(replacement.clone(), window, cx);
                    });
                    true
                })
                .on_close(move |_, _, cx| {
                    let _ = close_view.update(cx, |this, _| {
                        this.pending_menu_paste = false;
                        this.pending_paste_repository = None;
                        this.pending_replacement_attachments = None;
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
        let Some(images) = self.pending_replacement_attachments.take() else {
            return;
        };

        self.bulk_upload = true;
        self.append_staged_attachments(images, window, cx);
        window.close_dialog(cx);
    }

    fn accept_pasted_attachment_result(
        &mut self,
        paste_generation: u64,
        result: anyhow::Result<StagedAttachment>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if paste_generation != self.paste_generation {
            self.diagnostics
                .info("Discarded a stale clipboard image conversion.");
            return;
        }
        self.accept_attachment_result(result, window, cx);
    }

    fn warn(
        &mut self,
        message: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let message: SharedString = message.into();
        self.diagnostics.warn(message.to_string());

        window.push_notification(Notification::warning(message), cx);
    }

    fn send(&mut self, _: &gpui_kit::ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.send_attachments(window, cx);
    }

    fn try_complete_menu_paste(&mut self, cx: &mut Context<Self>) {
        if !self.pending_menu_paste {
            return;
        }
        if self.staged_attachments.is_empty()
            || !matches!(self.load_state, LoadState::Idle | LoadState::Ready)
            || self.selected_repository.is_none()
        {
            return;
        }

        self.pending_menu_paste = false;
        cx.notify();
    }

    fn sync_upload_result(&mut self) {
        let images = self
            .staged_attachments
            .iter()
            .filter_map(|image| {
                image
                    .upload_state
                    .uploaded_url()
                    .map(|url| UploadedAttachment {
                        markdown: image.snippet(url, self.settings.image_copy_format()),
                    })
            })
            .collect::<Vec<_>>();
        self.upload_result = (!images.is_empty()).then_some(UploadResult {
            attachments: images,
        });
    }

    fn send_attachments(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.uploading
            || self.import_busy()
            || self.updater.read(cx).state == UpdateState::Installing
        {
            return;
        }
        let Some(repository) = self.selected_repository.clone() else {
            return;
        };
        if self.staged_attachments.is_empty() {
            return;
        }
        if repository.id == 0 {
            self.load_repository_id_then_send(window, cx);
            return;
        }
        let Some(token) = self.token.clone() else {
            self.load_token_then_send(window, cx);
            return;
        };
        let diagnostics = self.diagnostics.clone();
        self.start_upload(window, cx, move |item, control, progress| {
            github::upload(
                &token,
                repository.id,
                item,
                &diagnostics,
                control,
                move |sent, total| progress(sent, total),
            )
        });
    }

    fn start_upload(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        upload: impl Fn(
            &StagedAttachment,
            &github::UploadControl,
            Arc<dyn Fn(u64, u64) + Send + Sync>,
        ) -> Result<String, github::GithubError>
        + Send
        + 'static,
    ) {
        if self.uploading {
            return;
        }
        for item in self
            .staged_attachments
            .iter()
            .filter(|item| !item.upload_state.is_uploaded())
        {
            if let Err(error) = item.validate_upload_size(self.settings.video_upload_limit()) {
                self.warn(format!("{}: {error}", item.name), window, cx);
                return;
            }
        }
        let attachments = self
            .staged_attachments
            .iter_mut()
            .filter(|item| !item.upload_state.is_uploaded())
            .map(|item| {
                item.upload_state = AttachmentUploadState::Queued;
                item.clone()
            })
            .collect::<Vec<_>>();
        if attachments.is_empty() {
            return;
        }
        let control = github::UploadControl::default();
        self.upload_control = Some(control.clone());
        self.uploading = true;
        self.upload_generation = self.upload_generation.wrapping_add(1);
        let generation = self.upload_generation;
        let state_generation = self.state_generation;
        let (sender, receiver) = async_channel::unbounded();
        cx.background_spawn(async move {
            for item in attachments {
                if control.is_canceled() {
                    break;
                }
                let id = item.id;
                let progress = sender.clone();
                let result = upload(
                    &item,
                    &control,
                    Arc::new(move |sent, total| {
                        let _ = progress.try_send(UploadEvent::Progress { id, sent, total });
                    }),
                );
                let canceled = matches!(result, Err(github::GithubError::Canceled));
                let _ = sender.try_send(UploadEvent::Finished { id, result });
                if canceled {
                    break;
                }
            }
            let _ = sender.try_send(UploadEvent::Complete {
                canceled: control.is_canceled(),
            });
        })
        .detach();
        cx.spawn_in(window, async move |this, window| {
            let mut uploaded = 0;
            let mut failures = 0;
            while let Ok(event) = receiver.recv().await {
                let complete = matches!(event, UploadEvent::Complete { .. });
                let result = this.update_in(window, |this, window, cx| {
                    if this.state_generation != state_generation || this.upload_generation != generation { return false; }
                    match event {
                        UploadEvent::Progress { id, sent, total } => {
                            if let Some(item) = this.staged_attachments.iter_mut().find(|item| item.id == id) {
                                item.upload_state = AttachmentUploadState::Uploading { sent, total };
                            }
                        }
                        UploadEvent::Finished { id, result } => {
                            if let Some(item) = this.staged_attachments.iter_mut().find(|item| item.id == id) {
                                match result {
                                    Ok(url) => { uploaded += 1; item.upload_state = AttachmentUploadState::Uploaded { url }; }
                                    Err(error) => {
                                        failures += 1;
                                        this.diagnostics.error(format!("Attachment upload failed: {error}"));
                                        item.upload_state = AttachmentUploadState::Failed { message: error.to_string() };
                                    }
                                }
                            }
                            this.sync_upload_result();
                        }
                        UploadEvent::Complete { canceled } => {
                            this.uploading = false;
                            this.upload_control = None;
                            for item in &mut this.staged_attachments {
                                if matches!(item.upload_state, AttachmentUploadState::Queued | AttachmentUploadState::Uploading { .. }) {
                                    item.upload_state = AttachmentUploadState::Failed { message: "Batch canceled before this attachment completed. You can retry it.".into() };
                                }
                            }
                            let message = if canceled {
                                format!("Upload canceled. {uploaded} uploaded; remaining attachments can be retried. An in-flight upload may already exist on GitHub.")
                            } else if failures > 0 {
                                format!("{uploaded} uploaded; {failures} failed. See each attachment for details, then retry.")
                            } else { format!("{uploaded} attachment(s) uploaded successfully.") };
                            this.diagnostics.info(&message);
                            window.push_notification(if canceled || failures > 0 { Notification::warning(message) } else { Notification::success(message) }, cx);
                        }
                    }
                    cx.notify();
                    true
                });
                if !matches!(result, Ok(true)) || complete { break; }
            }
        }).detach();
        cx.notify();
    }

    fn cancel_upload(&mut self, _: &gpui_kit::ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(control) = &self.upload_control {
            control.cancel();
        }
        cx.notify();
    }

    fn load_token_then_send(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.loading_token {
            return;
        }
        self.loading_token = true;
        self.diagnostics
            .info("Checking GitHub authentication before uploading.");
        cx.notify();

        let diagnostics = self.diagnostics.clone();
        let state_generation = self.state_generation;
        cx.spawn_in(window, async move |this, window| {
            let result = window
                .background_spawn(async move { github::load_token(&diagnostics) })
                .await;
            let _ = this.update_in(window, move |this, window, cx| {
                if this.state_generation != state_generation {
                    return;
                }
                this.loading_token = false;
                match result {
                    Ok(token) => {
                        this.token = Some(token);
                        this.send_attachments(window, cx);
                    }
                    Err(error) => {
                        this.diagnostics
                            .error(format!("GitHub authentication failed: {error}"));

                        window.push_notification(Notification::error(error.to_string()), cx);

                        cx.notify();
                    }
                }
            });
        })
        .detach();
    }

    fn load_repository_id_then_send(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.loading_repository {
            return;
        }
        let (Some(owner), Some(repository)) = (
            self.settings.owner.clone(),
            self.selected_repository.clone(),
        ) else {
            return;
        };
        self.loading_repository = true;
        self.diagnostics
            .info("Resolving the remembered repository before uploading.");
        cx.notify();

        let diagnostics = self.diagnostics.clone();
        let state_generation = self.state_generation;
        cx.spawn_in(window, async move |this, window| {
            let requested_owner = owner.clone();
            let requested_repository = repository.name.clone();
            let result = window
                .background_spawn(async move {
                    github::load_repository_id(&owner, &repository.name, &diagnostics)
                })
                .await;
            let _ = this.update_in(window, move |this, window, cx| {
                if this.state_generation != state_generation
                    || this.settings.owner.as_deref() != Some(requested_owner.as_str())
                {
                    return;
                }
                this.loading_repository = false;
                if this
                    .selected_repository
                    .as_ref()
                    .map(|repo| repo.name.as_str())
                    != Some(requested_repository.as_str())
                {
                    cx.notify();
                    return;
                }
                match result {
                    Ok(id) => {
                        if let Some(repository) = this.selected_repository.as_mut() {
                            repository.id = id;
                            this.repositories
                                .insert(repository.name.clone(), repository.clone());
                            this.settings.last_repository_id = Some(id);
                            if let Err(error) = this.settings.save() {
                                this.diagnostics.error(format!(
                                    "Could not remember the selected repository: {error}"
                                ));
                            }
                        }
                        this.send_attachments(window, cx);
                    }
                    Err(error) => {
                        this.diagnostics.error(format!(
                            "Could not resolve the remembered repository: {error}"
                        ));

                        window.push_notification(Notification::error(error.to_string()), cx);

                        cx.notify();
                    }
                }
            });
        })
        .detach();
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
        self.finish_copy(
            "Markdown image snippets copied to the clipboard.",
            window,
            cx,
        );
    }

    fn copy_all_upload_markdown(
        &mut self,
        _: &gpui_kit::ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(result) = &self.upload_result else {
            return;
        };
        cx.write_to_clipboard(ClipboardItem::new_string(result.markdown()));
        self.diagnostics
            .info("Copied uploaded image Markdown snippets to the clipboard.");
        self.finish_copy(
            "Uploaded Markdown snippets copied to the clipboard.",
            window,
            cx,
        );
    }

    fn finish_copy(&mut self, message: &str, window: &mut Window, cx: &mut Context<Self>) {
        if self.settings.close_window_after_copy {
            self.diagnostics
                .info("Closing the main window after copying an upload result.");
            window.defer(cx, |window, cx| window_limits::request_close(window, cx));
        } else {
            window.push_notification(Notification::success(message), cx);
        }
    }

    fn repository_groups(&self) -> Vec<SearchableGroup<String>> {
        let mut names = self.repositories.keys().cloned().collect::<Vec<_>>();
        names.sort_by_key(|name| name.to_ascii_lowercase());
        let (pinned, remaining): (Vec<_>, Vec<_>) = names
            .into_iter()
            .partition(|name| self.settings.pinned_repositories.contains(name));
        let mut groups = Vec::new();
        if !pinned.is_empty() {
            groups.push(SearchableGroup::new("Pinned repositories").items(pinned));
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

    fn save_owner(
        &mut self,
        _: &gpui_kit::ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let value = self.owner_input.read(cx).value();
        let owner = match parse_owner(&value) {
            Ok(owner) => owner,
            Err(error) => {
                window.push_notification(Notification::error(error.to_string()), cx);
                return;
            }
        };

        let mut settings = self.settings.clone();
        let owner_changed = settings.set_owner(owner);
        if let Err(error) = settings.save() {
            window.push_notification(
                Notification::error(format!("Could not save preferences: {error}")),
                cx,
            );
            return;
        }
        self.settings = settings;
        if owner_changed {
            self.selected_repository = None;
            self.loading_repository = false;
            self.pending_paste_repository = None;
            self.pending_menu_paste = false;
            self.menu_bar.set_paste_actions_visible(false);
            self.repository_picker.update(cx, |picker, cx| {
                picker.set_selected_indices([], window, cx);
            });
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

    fn start_global_shortcut_recording(
        &mut self,
        _: &gpui_kit::ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.recording_global_shortcut {
            self.cancel_global_shortcut_recording(window, cx);
            return;
        }

        if self.settings.global_quick_copy_shortcut_enabled
            && let Err(error) = self.global_shortcut.set_shortcut(None)
        {
            window.push_notification(Notification::error(error.to_string()), cx);
            return;
        }

        self.recording_global_shortcut = true;
        window.focus(&self.focus_handle, cx);
        cx.notify();
    }

    fn handle_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.recording_global_shortcut {
            self.record_global_shortcut(event, window, cx);
        } else if self.settings_open && event.keystroke.key == "escape" {
            window.prevent_default();
            cx.stop_propagation();
            self.dismiss(window, cx);
        }
    }

    fn record_global_shortcut(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.recording_global_shortcut {
            return;
        }

        window.prevent_default();
        cx.stop_propagation();
        if event.is_held {
            return;
        }
        if event.keystroke.key == "escape" {
            self.cancel_global_shortcut_recording(window, cx);
            return;
        }

        let canonical = match shortcut_from_keystroke(&event.keystroke) {
            Ok(shortcut) => shortcut,
            Err(error) => {
                window.push_notification(Notification::error(error.to_string()), cx);
                return;
            }
        };
        let previous = self.settings.global_quick_copy_shortcut.clone();

        if self.settings.global_quick_copy_shortcut_enabled
            && let Err(error) = self.global_shortcut.set_shortcut(Some(&canonical))
        {
            let _ = self.global_shortcut.set_shortcut(Some(&previous));
            self.recording_global_shortcut = false;
            window.push_notification(Notification::error(error.to_string()), cx);
            cx.notify();
            return;
        }

        self.settings.global_quick_copy_shortcut = canonical.clone();
        if let Err(error) = self.settings.save() {
            if self.settings.global_quick_copy_shortcut_enabled {
                let _ = self.global_shortcut.set_shortcut(Some(&previous));
            }
            self.settings.global_quick_copy_shortcut = previous;
            self.recording_global_shortcut = false;
            window.push_notification(
                Notification::error(format!("Could not save preferences: {error}")),
                cx,
            );
            cx.notify();
            return;
        }

        self.global_shortcut_input.update(cx, |input, cx| {
            input.set_value(canonical, window, cx);
        });
        self.diagnostics
            .info("Saved the global Quick Copy shortcut.");
        window.push_notification(Notification::success("Global shortcut saved."), cx);
        self.recording_global_shortcut = false;
        cx.notify();
    }

    fn cancel_global_shortcut_recording(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.recording_global_shortcut {
            return;
        }

        self.recording_global_shortcut = false;
        if self.settings.global_quick_copy_shortcut_enabled
            && let Err(error) = self
                .global_shortcut
                .set_shortcut(Some(&self.settings.global_quick_copy_shortcut))
        {
            self.settings.global_quick_copy_shortcut_enabled = false;
            let _ = self.settings.save();
            self.diagnostics.error(format!(
                "Could not restore the global Quick Copy shortcut: {error}"
            ));
            window.push_notification(
                Notification::error(format!("Global Quick Copy shortcut disabled: {error}")),
                cx,
            );
        }
        cx.notify();
    }

    pub fn prepare_to_close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.cancel_global_shortcut_recording(window, cx);
    }

    fn close_settings(
        &mut self,
        _: &gpui_kit::ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.show_main_page(window, cx);
    }

    fn set_global_shortcut_enabled(
        &mut self,
        enabled: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if enabled == self.settings.global_quick_copy_shortcut_enabled {
            return;
        }

        let raw_shortcut = self.global_shortcut_input.read(cx).value();
        let canonical = if enabled {
            match canonicalize_shortcut(&raw_shortcut) {
                Ok(shortcut) => shortcut,
                Err(error) => {
                    window.push_notification(Notification::error(error.to_string()), cx);
                    return;
                }
            }
        } else {
            self.settings.global_quick_copy_shortcut.clone()
        };
        let previous_shortcut = self.settings.global_quick_copy_shortcut.clone();

        let update = if enabled {
            self.global_shortcut.set_shortcut(Some(&canonical))
        } else {
            self.global_shortcut.set_shortcut(None)
        };
        if let Err(error) = update {
            window.push_notification(Notification::error(error.to_string()), cx);
            return;
        }

        self.settings.global_quick_copy_shortcut_enabled = enabled;
        self.settings.global_quick_copy_shortcut = canonical.clone();
        if let Err(error) = self.settings.save() {
            let rollback = if enabled {
                self.global_shortcut.set_shortcut(None)
            } else {
                self.global_shortcut.set_shortcut(Some(&previous_shortcut))
            };
            if let Err(rollback_error) = rollback {
                self.diagnostics.error(format!(
                    "Could not restore the previous global shortcut: {rollback_error}"
                ));
            }
            self.settings.global_quick_copy_shortcut_enabled = !enabled;
            self.settings.global_quick_copy_shortcut = previous_shortcut;
            window.push_notification(
                Notification::error(format!("Could not save preferences: {error}")),
                cx,
            );
            return;
        }

        self.global_shortcut_input.update(cx, |input, cx| {
            input.set_value(canonical, window, cx);
        });
        self.diagnostics.info(if enabled {
            "Enabled the global Quick Copy shortcut."
        } else {
            "Disabled the global Quick Copy shortcut."
        });
        cx.notify();
    }

    fn set_copy_images_as_markdown(
        &mut self,
        enabled: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let previous = self.settings.copy_images_as_markdown;
        self.settings.copy_images_as_markdown = enabled;
        if let Err(error) = self.settings.save() {
            self.settings.copy_images_as_markdown = previous;
            window.push_notification(
                Notification::error(format!("Could not save preferences: {error}")),
                cx,
            );
        }
        // Existing uploads use the selected format on their next copy too.
        self.sync_upload_result();
        cx.notify();
    }

    fn set_allow_large_videos(
        &mut self,
        enabled: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.uploading || self.import_busy() {
            return;
        }
        let previous = self.settings.allow_large_videos;
        self.settings.allow_large_videos = enabled;
        if let Err(error) = self.settings.save() {
            self.settings.allow_large_videos = previous;
            window.push_notification(
                Notification::error(format!("Could not save preferences: {error}")),
                cx,
            );
        } else {
            // Invalidate file-picker, drop and clipboard results started under
            // the previous policy. Already staged files are checked at upload.
            self.upload_generation = self.upload_generation.wrapping_add(1);
            self.paste_generation = self.paste_generation.wrapping_add(1);
        }
        cx.notify();
    }

    fn set_close_window_after_copy(
        &mut self,
        enabled: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let previous = self.settings.close_window_after_copy;
        self.settings.close_window_after_copy = enabled;
        if let Err(error) = self.settings.save() {
            self.settings.close_window_after_copy = previous;
            window.push_notification(
                Notification::error(format!("Could not save preferences: {error}")),
                cx,
            );
        }
        cx.notify();
    }

    fn confirm_reset(
        &mut self,
        _: &gpui_kit::ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.cancel_global_shortcut_recording(window, cx);
        let uploader = cx.entity().downgrade();
        window.open_alert_dialog(cx, move |alert, _, _| {
            let uploader = uploader.clone();
            alert
                .width(px(440.))
                .title("Reset Gatto?")
                .description(
                    "This clears all Gatto data on this device and returns the app to its first-run state. This can’t be undone.",
                )
                .show_cancel(true)
                .cancel_text("Cancel")
                .ok_text("Reset app")
                .footer(theme::confirmation_footer("ok", "Reset app", 104.))
                .on_ok(move |_, window, cx| {
                    match uploader.update(cx, |this, cx| this.reset_app(window, cx)) {
                        Ok(Ok(())) => true,
                        Ok(Err(error)) => {
                            window.push_notification(
                                Notification::error(format!("Could not reset the app: {error}")),
                                cx,
                            );
                            false
                        }
                        Err(_) => true,
                    }
                })
        });
    }

    fn set_release_channel(
        &mut self,
        channel: ReleaseChannel,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if channel == self.settings.release_channel {
            return;
        }
        if self.updater.read(cx).state == UpdateState::Installing {
            return;
        }
        let mut settings = self.settings.clone();
        settings.release_channel = channel;
        if let Err(error) = settings.save() {
            window.push_notification(
                Notification::error(format!("Could not save release channel: {error}")),
                cx,
            );
            return;
        }
        self.settings = settings;
        if let Err(error) = self
            .updater
            .update(cx, |updater, cx| updater.set_channel(channel, cx))
        {
            self.diagnostics
                .error(format!("Could not change release channel: {error}"));
        }
        cx.notify();
    }

    fn reset_app(&mut self, window: &mut Window, cx: &mut Context<Self>) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.updater.read(cx).state != UpdateState::Installing,
            "Finish the update before resetting Gatto."
        );
        self.global_shortcut.set_shortcut(None)?;
        AppSettings::reset()?;

        self.settings = AppSettings::default();
        self.updater.update(cx, |updater, cx| {
            updater.set_channel(ReleaseChannel::Stable, cx)
        })?;
        self.owner_input.update(cx, |input, cx| {
            input.set_value("", window, cx);
        });
        let default_shortcut = self.settings.global_quick_copy_shortcut.clone();
        self.global_shortcut_input.update(cx, |input, cx| {
            input.set_value(default_shortcut, window, cx);
        });
        self.repositories.clear();
        self.selected_repository = None;
        self.token = None;
        self.loading_token = false;
        self.loading_repository = false;
        self.load_state = LoadState::NeedsSetup;
        self.paste_generation = self.paste_generation.wrapping_add(1);
        self.state_generation = self.state_generation.wrapping_add(1);
        if let Some(control) = self.upload_control.take() {
            control.cancel();
        }
        if let Some(job) = self.compression.take() {
            job.control.cancel();
        }
        self.pending_import = None;
        self.staged_attachments.clear();
        self.bulk_upload = false;
        self.pending_paste_repository = None;
        self.pending_menu_paste = false;
        self.pending_replacement_attachments = None;
        self.upload_result = None;
        self.uploading = false;
        self.recording_global_shortcut = false;
        self.repository_picker.update(cx, |picker, cx| {
            picker.set_selected_indices([], window, cx);
            picker.set_items(
                SearchableVec::new(Vec::<SearchableGroup<String>>::new()),
                window,
                cx,
            );
        });
        self.menu_bar.set_paste_actions_visible(false);
        self.diagnostics
            .info("Reset app state and cleared saved preferences.");
        window.push_notification(
            Notification::success("App reset. Set up Gatto to continue."),
            cx,
        );
        cx.notify();
        Ok(())
    }

    fn open_rooftops(
        &mut self,
        _: &gpui_kit::ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(handle) = self.rooftops_window
            && handle
                .update(cx, |_, window, _| window.activate_window())
                .is_ok()
        {
            return;
        }
        match gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::centered(size(px(960.), px(600.)), cx)),
                window_min_size: Some(size(px(680.), px(440.))),
                ..Default::default()
            },
            cx,
            |window, cx| {
                window.set_window_title("Rooftops");
                let game = cx.new(|cx| crate::rooftops::Rooftops::new(window, cx));
                window.focus(&game.read(cx).focus_handle(cx), cx);
                game
            },
        ) {
            Ok((handle, _)) => self.rooftops_window = Some(handle),
            Err(error) => {
                self.diagnostics
                    .error(format!("Could not open Rooftops: {error}"));
                window.push_notification(Notification::error("Could not open Rooftops."), cx);
            }
        }
    }

    fn open_log_window(
        &mut self,
        _: &gpui_kit::ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some((log_window, log_view)) = self.log_window.clone() {
            if log_window
                .update(cx, |_, window, _| window.activate_window())
                .is_ok()
            {
                log_view.update(cx, |logs, cx| logs.reload(cx));
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
            window.on_window_should_close(cx, move |window, cx| {
                let should_close = window_limits::close(window, cx);
                let _ = uploader.update(cx, |this, _| {
                    if should_close {
                        this.log_window = None;
                    }
                    this.diagnostics.info("Closed the application logs window.");
                });
                should_close
            });
            cx.new(|cx| LogViewer::new(diagnostics, cx))
        });
        match result {
            Ok(result) => self.log_window = Some(result),
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

    fn open_walkthrough(
        &mut self,
        _: &gpui_kit::ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.diagnostics.info("Opened the Gatto walkthrough.");
        cx.open_url(WALKTHROUGH_URL);
    }

    fn open_attachment_preview(
        &mut self,
        selected_id: u64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(image) = self
            .staged_attachments
            .iter()
            .find(|image| image.id == selected_id)
        else {
            return;
        };
        if let Media::Video(video) = &image.media {
            let video = video.clone();
            let task = cx.background_spawn(async move { video.preview() });
            cx.spawn_in(window, async move |this, window| {
                if let Err(error) = task.await {
                    let _ = this.update_in(window, |this, window, cx| {
                        this.diagnostics
                            .error(format!("Video preview failed: {error}"));
                        window.push_notification(Notification::error(error.to_string()), cx);
                    });
                }
            })
            .detach();
            return;
        }
        let preview_name = image.name.clone();
        let preview_images = self
            .staged_attachments
            .iter()
            .filter(|item| item.image().is_some())
            .cloned()
            .collect();

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
            window_bounds: Some(WindowBounds::Maximized(Bounds::centered(
                None,
                size(px(980.), px(720.)),
                cx,
            ))),
            window_min_size: Some(size(px(640.), px(500.))),
            ..Default::default()
        };
        let result = gpui_kit::open_window(options, cx, move |window, cx| {
            window.set_window_title(&window_title);
            let preview = cx.new(|cx| ImagePreview::new(preview_images, selected_id, cx));
            let close_preview = preview.downgrade();
            window.on_window_should_close(cx, move |window, cx| {
                if close_preview
                    .update(cx, |preview, cx| preview.should_close(window, cx))
                    .is_ok_and(|approved| !approved)
                {
                    return false;
                }
                let should_close = window_limits::close(window, cx);
                let _ = uploader.update(cx, |this, _| {
                    if should_close {
                        this.preview_window = None;
                    }
                    this.diagnostics.info("Closed the image preview window.");
                });
                should_close
            });
            let focus = preview.read(cx).focus_handle(cx);
            window.focus(&focus, cx);
            preview
        });
        match result {
            Ok((handle, preview)) => {
                cx.subscribe(
                    &preview,
                    move |this, preview, event: &crate::image_preview::AppliedAnnotation, cx| {
                        let result = this.apply_annotations(event);
                        let _ = handle.update(cx, |_, window, cx| {
                            preview.update(cx, |preview, cx| {
                                preview.annotation_saved(event.id, result, window, cx);
                            });
                        });
                        cx.notify();
                    },
                )
                .detach();
                self.preview_window = Some((handle, preview));
            }
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

    fn apply_annotations(
        &mut self,
        event: &crate::image_preview::AppliedAnnotation,
    ) -> Result<(), &'static str> {
        if self.uploading || self.import_busy() {
            return Err("Finish the current upload or import before applying annotations.");
        }
        let Some(attachment) = self
            .staged_attachments
            .iter_mut()
            .find(|item| item.id == event.id)
        else {
            return Err("The original attachment was removed. Add the image again to edit it.");
        };
        let mut edited = event.attachment.clone();
        edited.id = attachment.id;
        edited.description = attachment.description.clone();
        if let (Media::Image(original), Media::Image(replacement)) =
            (&attachment.media, &mut edited.media)
        {
            replacement.annotation_session = original.annotation_session.clone();
        }
        *attachment = edited;
        self.sync_upload_result();
        self.diagnostics
            .info("Applied image annotations to a staged attachment.");
        Ok(())
    }

    pub fn install_update(
        &mut self,
        quick_copy_busy: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.uploading || self.import_busy() || quick_copy_busy {
            window.push_notification(
                Notification::info("Finish the current upload before updating."),
                cx,
            );
            return;
        }
        self.updater.update(cx, |updater, cx| updater.install(cx));
    }

    fn render_repository_picker(
        &self,
        heading: &'static str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let disabled = self.load_state == LoadState::NeedsSetup;
        let loading = self.load_state == LoadState::Loading;
        let owner = self.settings.owner.clone().unwrap_or_default();
        let view = cx.entity();
        v_flex()
            .w_full()
            .gap_2()
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .child(div().text_sm().font_semibold().child(heading))
                    .when(self.load_state == LoadState::Failed, |row| {
                        row.child(
                            Button::new("retry-repositories")
                                .app_icon()
                                .icon(IconName::RefreshCw)
                                .tooltip("Retry")
                                .accessibility_label("Retry loading repositories")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.load_repositories(window, cx);
                                })),
                        )
                    }),
            )
            .child(
                Combobox::new(&self.repository_picker)
                    .placeholder(format!("Search {owner} repositories…"))
                    .search_placeholder("Type to filter repositories…")
                    .disabled(disabled)
                    .empty(move |_, cx| {
                        if loading {
                            v_flex()
                                .debug_selector(|| "repository-picker-loading".to_owned())
                                .items_center()
                                .justify_center()
                                .py_6()
                                .child(Spinner::new().large())
                                .into_any_element()
                        } else {
                            h_flex()
                                .justify_center()
                                .py_6()
                                .text_color(cx.theme().muted_foreground.opacity(0.6))
                                .child(Icon::new(IconName::Inbox).size(px(28.)))
                                .into_any_element()
                        }
                    })
                    .render_trigger(move |trigger, _, _| {
                        let trigger_is_open = trigger.is_open();
                        let repository_name = trigger
                            .selection()
                            .first()
                            .map(|(_, repository)| SharedString::from(repository.clone()))
                            .unwrap_or_else(|| {
                                trigger
                                    .placeholder()
                                    .cloned()
                                    .unwrap_or_else(|| "Select a repository".into())
                            });
                        let view = view.clone();
                        h_flex()
                            .w_full()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .child(
                                div()
                                    .min_w_0()
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .truncate()
                                    .child(repository_name),
                            )
                            .child(Icon::new(IconName::ChevronDown).xsmall())
                            .debug_selector(move || {
                                if trigger_is_open {
                                    "repository-picker-trigger-open".to_owned()
                                } else {
                                    "repository-picker-trigger-closed".to_owned()
                                }
                            })
                            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                                view.update(cx, |this, cx| {
                                    this.load_repositories_when_opened(window, cx);
                                });
                            })
                    })
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
            .child(div().text_sm().font_semibold().child("Set an account or organization"))
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(
                        "Enter a GitHub account/organization in App Preferences to load its repositories.",
                    ),
            )
            .child(
                Button::new("setup-open-settings")
                    .app_primary()
                    .label("Open App Preferences")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.show_settings(cx);
                    })),
            )
    }

    fn render_drop_zone(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity().downgrade();
        let bulk_upload = self.bulk_upload;
        let state_generation = self.state_generation;
        let staging_generation = self.upload_generation;
        let video_limit = self.settings.video_upload_limit();
        let uploading = self.uploading || self.import_busy();
        let has_staged_attachments = !self.staged_attachments.is_empty();
        div()
            .id("image-drop-zone")
            .debug_selector(|| "image-drop-zone".into())
            .w_full()
            .min_h(px(if has_staged_attachments { 112. } else { 150. }))
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
                        Self::load_dropped_paths(
                            view.clone(),
                            state_generation,
                            staging_generation,
                            paths.paths().to_vec(),
                            video_limit,
                            window,
                            cx,
                        );
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
                                "or drop images or videos here"
                            } else {
                                "or drop an image or video here"
                            }),
                    ),
            )
    }

    fn render_bulk_upload_control(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity().downgrade();
        let has_batch = self.staged_attachments.len() > 1;
        let disabled = self.uploading || self.import_busy() || has_batch;
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
                div()
                    .min_w_0()
                    .text_sm()
                    .font_semibold()
                    .child("Bulk upload"),
            )
            .child(
                Switch::new("bulk-upload-switch")
                    .checked(self.bulk_upload)
                    .disabled(disabled)
                    .accessibility_label("Bulk upload")
                    .when(has_batch, |toggle| {
                        toggle.tooltip("Finish or remove the batch first")
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

    fn remove_staged_attachment(&mut self, id: u64, cx: &mut Context<Self>) {
        let previous_len = self.staged_attachments.len();
        self.staged_attachments.retain(|image| image.id != id);
        if self.staged_attachments.len() != previous_len {
            self.sync_upload_result();
            self.pending_menu_paste = false;
            self.pending_paste_repository = None;
            self.diagnostics.info("Removed a staged attachment.");
            cx.notify();
        }
    }

    fn render_attachments(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let count = self.staged_attachments.len();
        let has_uploaded_images = self
            .staged_attachments
            .iter()
            .any(|image| image.upload_state.is_uploaded());
        let attachments = self.staged_attachments.iter().map(|image| {
            let image_id = image.id;
            let source = image.image().map(|data| data.preview.clone());
            let title = image.description.clone();
            let detail = format!(
                "{} · {} · {}",
                image.name,
                image.formatted_size(),
                image.mime_type
            );
            let (status, status_icon, status_label, status_color) = match &image.upload_state {
                AttachmentUploadState::Preview => (
                    AttachmentStatus::Pending,
                    AssetIconName::Clock,
                    "Draft".to_owned(),
                    cx.theme().warning,
                ),
                AttachmentUploadState::Queued => (
                    AttachmentStatus::Pending, AssetIconName::Clock, "Queued".to_owned(), cx.theme().muted_foreground,
                ),
                AttachmentUploadState::Uploading { sent, total } => (
                    AttachmentStatus::Uploading, AssetIconName::LoaderCircle,
                    if sent >= total { "Waiting for GitHub…".to_owned() } else {
                        let (unit, divisor) = if *total >= 1_000_000 { ("MB", 1_000_000.) } else { ("KB", 1000.) };
                        format!("Sending {}% · {:.1} / {:.1} {unit}", sent * 100 / total.max(&1), *sent as f64 / divisor, *total as f64 / divisor)
                    },
                    cx.theme().primary,
                ),
                AttachmentUploadState::Uploaded { .. } => (
                    AttachmentStatus::Complete,
                    AssetIconName::CircleCheck,
                    "Uploaded".to_owned(),
                    cx.theme().success,
                ),
                AttachmentUploadState::Failed { .. } => (
                    AttachmentStatus::Failed,
                    AssetIconName::CircleX,
                    "Failed".to_owned(),
                    cx.theme().danger,
                ),
            };
            let upload_percentage = match image.upload_state {
                AttachmentUploadState::Uploading { sent, total } => Some(sent as f32 / total.max(1) as f32 * 100.),
                _ => None,
            };
            let status_row = h_flex()
                .items_center()
                .gap_1()
                .text_xs()
                .font_semibold()
                .text_color(status_color)
                .child(Icon::new(status_icon).size(px(13.)))
                .child(status_label);
            let markdown = image
                .upload_state
                .uploaded_url()
                .map(|url| image.snippet(url, self.settings.image_copy_format()));
            let failure_message = image
                .upload_state
                .failure_message()
                .map(ToOwned::to_owned);
            let preview_view = cx.entity().downgrade();
            let rename_view = cx.entity().downgrade();
            let copy_view = cx.entity().downgrade();
            let remove_view = cx.entity().downgrade();
            Attachment::new()
                .id(format!("staged-image-attachment-{image_id}"))
                .large()
                .w_full()
                .cursor_pointer()
                .status(status)
                .when(image.image().is_none(), |attachment| attachment.tooltip("Open video in your macOS video app"))
                .media(match source {
                    Some(source) => AttachmentMedia::new().src(source),
                    None => AttachmentMedia::new().child(Icon::new(AssetIconName::Play).size(px(26.))),
                })
                .content(
                    AttachmentContent::new()
                        .title(AttachmentTitle::new(title))
                        .description(AttachmentDescription::new(detail))
                        .child(status_row)
                        .when_some(upload_percentage, |content, value| content.child(
                            Progress::new(format!("attachment-progress-{image_id}")).value(value).color(status_color).xsmall().w_full()
                                .accessibility_label("Attachment upload progress")
                        )),
                )
                .on_click(move |_, window, cx| {
                    let _ = preview_view.update(cx, |this, cx| {
                        this.open_attachment_preview(image_id, window, cx);
                    });
                })
                .when(!self.uploading, |attachment| {
                    attachment.actions(
                        AttachmentActions::new()
                            .child(
                                Button::new(format!("set-staged-image-description-{image_id}"))
                                    .app_icon()
                                    .icon(AssetIconName::ALargeSmall)
                                    .accessibility_label("Edit description")
                                    .tooltip("Edit description")
                                    .on_click(move |_, window, cx| {
                                        cx.stop_propagation();
                                        let _ = rename_view.update(cx, |this, cx| {
                                            this.open_description_dialog(image_id, window, cx);
                                        });
                                    }),
                            )
                            .when_some(markdown, |actions, markdown| {
                                actions.child(
                                    Button::new(format!("copy-staged-image-markdown-{image_id}"))
                                        .debug_selector(move || format!("copy-staged-image-markdown-{image_id}"))
                                        .app_icon()
                                        .icon(AssetIconName::Copy)
                                        .accessibility_label("Copy Markdown")
                                        .tooltip("Copy Markdown")
                                        .on_click(move |_, window, cx| {
                                            cx.stop_propagation();
                                            cx.write_to_clipboard(ClipboardItem::new_string(
                                                markdown.clone(),
                                            ));
                                            let _ = copy_view.update(cx, |this, cx| {
                                                this.diagnostics.info(
                                                    "Copied an uploaded attachment's Markdown to the clipboard.",
                                                );
                                                this.finish_copy(
                                                    "Markdown copied to the clipboard.",
                                                    window,
                                                    cx,
                                                );
                                            });
                                        }),
                                )
                            })
                            .child(
                                Button::new(format!("remove-staged-image-{image_id}"))
                                    .app_icon()
                                    .icon(IconName::Close)
                                    .accessibility_label("Remove staged attachment")
                                    .tooltip("Remove")
                                    .on_click(move |_, _, cx| {
                                        cx.stop_propagation();
                                        let _ = remove_view.update(cx, |this, cx| {
                                            this.remove_staged_attachment(image_id, cx);
                                        });
                                    }),
                            ),
                    )
                })
                .when_some(failure_message, |attachment, message| {
                    attachment.tooltip(message)
                })
                .into_any_element()
        });

        v_flex()
            .debug_selector(|| "staged-attachments".into())
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
                                .child(format!("{count} attachments")),
                        )
                        .child(
                            h_flex()
                                .items_center()
                                .gap_2()
                                .when(count > 1 && self.upload_result.is_some(), |actions| {
                                    actions.child(
                                        Button::new("copy-all-staged-image-markdown")
                                            .debug_selector(|| {
                                                "copy-all-staged-image-markdown".into()
                                            })
                                            .app_secondary()
                                            .icon(AssetIconName::Copy)
                                            .label("Copy all")
                                            .on_click(cx.listener(Self::copy_all_upload_markdown)),
                                    )
                                })
                                .when(!self.uploading, |actions| {
                                    actions.child(
                                        Button::new("remove-all-staged-images")
                                            .app_secondary()
                                            .label(if has_uploaded_images {
                                                "Reset"
                                            } else {
                                                "Remove all"
                                            })
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.clear_staged_attachments(
                                                    "Discarded all staged attachments.",
                                                    cx,
                                                );
                                            })),
                                    )
                                }),
                        ),
                )
            })
            .when_some(self.upload_result.as_ref(), |list, result| {
                list.child(
                    h_flex()
                        .id("upload-markdown-copy-hint")
                        .debug_selector(|| "upload-markdown-copy-hint".into())
                        .items_center()
                        .gap_1()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child("Press")
                        .child(Kbd::new(Keystroke::parse("cmd-c").expect("valid shortcut")))
                        .child(if result.attachments.len() == 1 {
                            "to copy the uploaded Markdown snippet."
                        } else {
                            "to copy all uploaded Markdown snippets."
                        }),
                )
            })
            .children(attachments)
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
        let owner_is_saved = parse_owner(&self.owner_input.read(cx).value())
            .is_ok_and(|owner| self.settings.owner.as_deref() == Some(owner.as_str()));
        let channel = self.settings.release_channel;
        v_flex()
            .flex_none()
            .p_6()
            .gap_5()
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .child(div().font_semibold().text_lg().child("App Preferences"))
                    .child(
                        Button::new("close-settings")
                            .app_secondary()
                            .label("Done")
                            .on_click(cx.listener(Self::close_settings)),
                    ),
            )
            .child(
                v_flex()
                    .gap_3()
                    .rounded_lg()
                    .border_1()
                    .border_color(cx.theme().border)
                    .p_4()
                    .child(
                        v_flex()
                            .gap_0p5()
                            .child(
                                div()
                                    .font_semibold()
                                    .text_sm()
                                    .child("GitHub account/organization"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("Enter a personal account or organization login."),
                            ),
                    )
                    .child(Input::new(&self.owner_input))
                    .child(
                        Button::new("save-owner")
                            .app_primary()
                            .self_start()
                            .label(if owner_is_saved {
                                "Account/organization is saved"
                            } else {
                                "Save account/organization"
                            })
                            .disabled(owner_is_saved)
                            .on_click(cx.listener(Self::save_owner)),
                    ),
            )
            .when(self.settings.owner.is_some(), |page| {
                page.child(
                    v_flex()
                        .gap_3()
                        .rounded_lg()
                        .border_1()
                        .border_color(cx.theme().border)
                        .p_4()
                        .child(self.render_repository_picker("Pinned repositories", cx))
                        .when(can_pin, |card| {
                            card.child(
                                Button::new("pin-selected-repository")
                                    .app_secondary()
                                    .self_start()
                                    .label("Pin")
                                    .on_click(cx.listener(Self::pin_selected_repository)),
                            )
                        })
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
                                        .app_secondary()
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
                    .child(div().font_semibold().text_sm().child("Quick Copy shortcut"))
                    .child(
                        Checkbox::new("enable-global-quick-copy-shortcut")
                            .label("Enable")
                            .checked(self.settings.global_quick_copy_shortcut_enabled)
                            .on_click(cx.listener(|this, checked, window, cx| {
                                this.set_global_shortcut_enabled(*checked, window, cx);
                            })),
                    )
                    .child(
                        h_flex()
                            .w_full()
                            .gap_2()
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .child(Input::new(&self.global_shortcut_input).readonly(true)),
                            )
                            .child(
                                Button::new("record-global-quick-copy-shortcut")
                                    .app_secondary()
                                    .label(if self.recording_global_shortcut {
                                        "Press shortcut…"
                                    } else {
                                        "Record shortcut"
                                    })
                                    .on_click(cx.listener(Self::start_global_shortcut_recording)),
                            ),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("At least two modifiers are required."),
                    ),
            )
            .child(
                v_flex()
                    .gap_3()
                    .rounded_lg()
                    .border_1()
                    .border_color(cx.theme().border)
                    .p_4()
                    .child(div().font_semibold().text_sm().child("Image copying"))
                    .child(
                        Checkbox::new("copy-images-as-markdown")
                            .label("Copy images as Markdown")
                            .checked(self.settings.copy_images_as_markdown)
                            .on_click(cx.listener(|this, checked, window, cx| {
                                this.set_copy_images_as_markdown(*checked, window, cx);
                            })),
                    )
                    .child(
                        div().text_xs().text_color(cx.theme().muted_foreground)
                            .child("Use ![description](URL) instead of an image tag. Markdown images do not include width or height. Applies to individual copy, bulk copy, and Quick Copy."),
                    ),
            )
            .child(
                v_flex()
                    .gap_3()
                    .rounded_lg()
                    .border_1()
                    .border_color(cx.theme().border)
                    .p_4()
                    .child(div().font_semibold().text_sm().child("Video uploads"))
                    .child(
                        Checkbox::new("allow-large-videos")
                            .label("Allow videos up to 100 MB")
                            .checked(self.settings.allow_large_videos)
                            .disabled(self.uploading || self.import_busy())
                            .on_click(cx.listener(|this, checked, window, cx| {
                                this.set_allow_large_videos(*checked, window, cx);
                            })),
                    )
                    .child(
                        div().text_xs().text_color(cx.theme().muted_foreground)
                            .child("Enable only if the destination repository is on a paid GitHub plan and you’re eligible to upload larger videos. Gatto does not verify your plan. Images and GIFs stay limited to 10 MB."),
                    ),
            )
            .child(
                v_flex()
                    .gap_3()
                    .rounded_lg()
                    .border_1()
                    .border_color(cx.theme().border)
                    .p_4()
                    .child(div().font_semibold().text_sm().child("General"))
                    .child(
                        h_flex().justify_between().items_center()
                            .child("Release channel")
                            .child(Button::new("release-channel").app_secondary()
                                .label(channel.label()).icon(IconName::ChevronDown)
                                .accessibility_label("Release channel")
                                .disabled(self.updater.read(cx).state == UpdateState::Installing)
                                .dropdown_menu(move |menu, _, _| {
                                    menu.menu_with_check("Stable", channel == ReleaseChannel::Stable, Box::new(SelectStableChannel))
                                        .menu_with_check("Beta", channel == ReleaseChannel::Beta, Box::new(SelectBetaChannel))
                                })),
                    )
                    .child(div().text_xs().text_color(cx.theme().muted_foreground)
                        .child("Beta includes preview releases and newer stable releases. Returning to Stable keeps your installed beta until a stable release catches up. Changing channels never installs an update."))
                    .child(Button::new("check-channel-updates").app_secondary().self_start()
                        .label("Check for Updates…")
                        .disabled(self.updater.read(cx).state == UpdateState::Installing)
                        .on_click(|_, window, cx| window.dispatch_action(Box::new(crate::application_menu::CheckForUpdates), cx)))
                    .when(!self.updater.read(cx).message.is_empty(), |section| {
                        section.child(div().text_xs().child(self.updater.read(cx).message.clone()))
                    })
                    .child(
                        Checkbox::new("start-at-login")
                            .label("Start at Login")
                            .checked(self.settings.start_at_login)
                            .on_click(cx.listener(|this, checked, window, cx| {
                                this.set_start_at_login(*checked, window, cx);
                            })),
                    )
                    .child(
                        Checkbox::new("close-window-after-copy")
                            .label("Close the window after copy")
                            .checked(self.settings.close_window_after_copy)
                            .on_click(cx.listener(|this, checked, window, cx| {
                                this.set_close_window_after_copy(*checked, window, cx);
                            })),
                    )
                    .child(
                        Button::new("reset-app")
                            .app_secondary()
                            .self_start()
                            .label("Reset app")
                            .on_click(cx.listener(Self::confirm_reset)),
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
                        h_flex()
                            .justify_between()
                            .items_start()
                            .text_xs()
                            .child("Repository")
                            .child(
                                v_flex()
                                    .items_end()
                                    .child(
                                        Link::new("repository-link")
                                            .href(REPOSITORY_URL)
                                            .child("GitHub"),
                                    )
                                    .child(
                                        div()
                                            .text_color(cx.theme().muted_foreground)
                                            .child("Jeremy Chandler"),
                                    ),
                            ),
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
        let can_send = self.selected_repository.is_some()
            && self
                .staged_attachments
                .iter()
                .any(|image| !image.upload_state.is_uploaded())
            && !self.uploading
            && !self.import_busy()
            && !self.loading_token
            && !self.loading_repository
            && self.updater.read(cx).state != UpdateState::Installing;
        let root = v_flex()
            .on_action(cx.listener(|this, _: &SelectStableChannel, window, cx| {
                this.set_release_channel(ReleaseChannel::Stable, window, cx)
            }))
            .on_action(cx.listener(|this, _: &SelectBetaChannel, window, cx| {
                this.set_release_channel(ReleaseChannel::Beta, window, cx)
            }))
            .key_context(KEY_CONTEXT)
            .track_focus(&self.focus_handle)
            .capture_key_down(cx.listener(Self::handle_key_down))
            .on_action(cx.listener(Self::on_paste))
            .on_action(cx.listener(Self::paste_and_preview_from_menu))
            .on_action(cx.listener(Self::switch_to_bulk_upload))
            .size_full()
            .bg(cx.theme().background);

        if self.settings_open {
            root.child(self.render_settings(cx)).into_any_element()
        } else {
            let needs_setup = self.load_state == LoadState::NeedsSetup;
            let root = root.on_action(cx.listener(Self::copy_upload_markdown));
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
                                    div()
                                        .id("rooftops-entry")
                                        .flex_shrink_0()
                                        .rounded_full()
                                        .cursor_pointer()
                                        .hover(|style| {
                                            style.bg(cx.theme().primary.opacity(0.06)).shadow(vec![
                                                BoxShadow::new(
                                                    px(0.),
                                                    px(0.),
                                                    cx.theme().primary.opacity(0.18),
                                                )
                                                .blur_radius(px(16.))
                                                .spread_radius(px(2.)),
                                            ])
                                        })
                                        .on_click(cx.listener(Self::open_rooftops))
                                        .child(
                                            img(self.header_mark.clone())
                                                .size(px(72.))
                                                .flex_shrink_0()
                                                .object_fit(ObjectFit::Contain),
                                        ),
                                )
                                .child(
                                    v_flex()
                                        .flex_1()
                                        .min_w_0()
                                        .gap_3()
                                        .child(div().text_lg().font_semibold().child("Gatto"))
                                        .child(
                                            h_flex()
                                                .w_full()
                                                .min_w_0()
                                                .gap_2()
                                                .flex_wrap()
                                                .when(self.updater.read(cx).state != UpdateState::Idle, |row| {
                                                    let update = self.updater.read(cx);
                                                    row.child(Button::new("install-update")
                                                        .debug_selector(|| "install-update".into())
                                                        .app_icon()
                                                        .label(match update.state {
                                                            UpdateState::Installing => "Updating…",
                                                            UpdateState::Failed => "↻ Update",
                                                            _ => "↑ Update",
                                                        })
                                                        .disabled(self.uploading || self.import_busy() || update.state == UpdateState::Installing)
                                                        .accessibility_label("Install update and restart Gatto")
                                                        .tooltip(update.message.clone())
                                                        .on_click(|_, window, cx| {
                                                            window.dispatch_action(Box::new(crate::application_menu::InstallUpdate), cx);
                                                        }))
                                                })
                                                .child(
                                                    Button::new("open-logs")
                                                        .app_icon()
                                                        .icon(AssetIconName::Logs)
                                                        .accessibility_label("Application logs")
                                                        .tooltip("Logs")
                                                        .on_click(
                                                            cx.listener(Self::open_log_window),
                                                        ),
                                                )
                                                .child(
                                                    Button::new("open-settings")
                                                        .app_icon()
                                                        .icon(IconName::Settings)
                                                        .accessibility_label("App Preferences")
                                                        .tooltip("Preferences")
                                                        .on_click(cx.listener(|this, _, _, cx| {
                                                            this.show_settings(cx);
                                                        })),
                                                )
                                                .child(
                                                    Button::new("open-walkthrough")
                                                        .app_icon()
                                                        .icon(AssetIconName::CircleQuestionMark)
                                                        .accessibility_label("Open walkthrough")
                                                        .tooltip("Help")
                                                        .on_click(
                                                            cx.listener(Self::open_walkthrough),
                                                        ),
                                                ),
                                        ),
                                ),
                        )
                        .when(needs_setup, |root| root.child(self.render_setup_prompt(cx)))
                        .when(!needs_setup, |root| {
                            root.child(self.render_repository_picker("Repository", cx))
                                .when(
                                    !self.uploading
                                        && self
                                            .staged_attachments
                                            .iter()
                                            .all(|image| !image.upload_state.is_uploaded()),
                                    |root| root.child(self.render_bulk_upload_control(cx)),
                                )
                                .child(self.render_drop_zone(cx))
                                .when(self.compression.is_some(), |root| {
                                    root.child(self.render_compression(cx))
                                })
                                .child(self.render_attachments(cx))
                        }),
                );
            root.child(content)
                .child(
                    v_flex().items_end().gap_3().px_6().py_3()
                    .when(self.uploading, |footer| footer.child(
                        Button::new("cancel-upload").app_secondary().label("Cancel upload")
                            .disabled(self.upload_control.as_ref().is_some_and(|control| control.is_canceled()))
                            .on_click(cx.listener(Self::cancel_upload))
                    ).child(div().text_xs().child("Canceling cannot remove an attachment already accepted by GitHub.")))
                    .child(
                        Button::new("send-image")
                            .app_primary()
                            .label(if self.uploading {
                                "Uploading…".to_owned()
                            } else {
                                "Upload".to_owned()
                            })
                            .icon(AssetIconName::Upload)
                            .min_w(px(112.))
                            .loading(self.uploading)
                            .disabled(!can_send)
                            .on_click(cx.listener(Self::send)),
                    ),
                )
                .into_any_element()
        }
    }
}

pub fn init_keybindings(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("cmd-v", PasteImage, Some(KEY_CONTEXT)),
        KeyBinding::new("ctrl-v", PasteImage, Some(KEY_CONTEXT)),
        KeyBinding::new("cmd-c", CopyUploadMarkdown, Some(KEY_CONTEXT)),
        KeyBinding::new("b", SwitchToBulkUpload, Some("Dialog")),
    ]);
}

#[cfg(test)]
mod tests {
    use gpui_kit::{Modifiers, TestAppContext};

    use super::*;

    fn upload_window(
        cx: &mut TestAppContext,
    ) -> (Entity<UploaderApp>, &mut gpui_kit::VisualTestContext) {
        cx.update(gpui_kit::init);
        let captured = Rc::new(std::cell::RefCell::new(None));
        let capture = captured.clone();
        let (_, window_cx) = cx.add_window_view(move |window, cx| {
            let app = cx.new(|cx| {
                UploaderApp::with_settings(
                    window,
                    Diagnostics::new(),
                    AppControllers::default(),
                    AppSettings {
                        owner: Some("acme".into()),
                        ..Default::default()
                    },
                    None,
                    false,
                    cx,
                )
            });
            *capture.borrow_mut() = Some(app.clone());
            gpui_kit::base::Root::new(app, window, cx)
        });
        let view = captured.borrow().clone().unwrap();
        (view, window_cx)
    }

    fn mixed_attachments() -> Vec<StagedAttachment> {
        [
            "packaging/rocket-cat-transparent.png",
            "tests/fixtures/sample.mp4",
        ]
        .map(|path| {
            StagedAttachment::from_path(
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(path),
                Default::default(),
            )
            .unwrap()
        })
        .into()
    }

    fn large_video() -> tempfile::NamedTempFile {
        use std::io::Write as _;
        let mut file = tempfile::Builder::new().suffix(".mp4").tempfile().unwrap();
        file.write_all(include_bytes!("../tests/fixtures/sample-audio.mp4"))
            .unwrap();
        // Legal padding grows the input above the gate without adding footage.
        file.write_all(&((crate::model::MAX_ATTACHMENT_BYTES + 8) as u32).to_be_bytes())
            .unwrap();
        file.write_all(b"free").unwrap();
        file.as_file()
            .set_len(file.as_file().metadata().unwrap().len() + crate::model::MAX_ATTACHMENT_BYTES)
            .unwrap();
        file
    }

    #[gpui_kit::test]
    fn applying_annotations_preserves_identity_and_description_and_invalidates_only_its_url(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = upload_window(cx);
        cx.update(|_, cx| {
            view.update(cx, |app, _| {
                let mut attachments = mixed_attachments();
                attachments[0].description = "Keep this alt text".into();
                for (i, attachment) in attachments.iter_mut().enumerate() {
                    attachment.upload_state = AttachmentUploadState::Uploaded {
                        url: format!("https://github.com/user-attachments/assets/{i}"),
                    };
                }
                let original = attachments[0].image().unwrap();
                let mut history = original
                    .annotation_session
                    .open(&original.bytes, original.preview.clone())
                    .unwrap();
                history.document.checkpoint();
                history.document.marks.push(crate::annotation::Mark::new(
                    crate::annotation::Tool::Rectangle,
                    crate::annotation::Pos::new(10., 10.),
                    Default::default(),
                ));
                original.annotation_session.retain(
                    history.document,
                    history.image,
                    history.saved_marks,
                );
                let id = attachments[0].id;
                app.staged_attachments = attachments;
                app.sync_upload_result();
                let edited = StagedAttachment::from_bytes(
                    "edited.png".into(),
                    include_bytes!("../packaging/menu-bar-icon.png").to_vec(),
                    None,
                )
                .unwrap();
                let event = crate::image_preview::AppliedAnnotation {
                    id,
                    attachment: edited,
                };
                app.uploading = true;
                assert!(app.apply_annotations(&event).is_err());
                assert!(app.staged_attachments[0].upload_state.is_uploaded());
                app.uploading = false;
                app.apply_annotations(&event).unwrap();
                let attachment = &app.staged_attachments[0];
                assert_eq!(attachment.id, id);
                assert_eq!(attachment.description, "Keep this alt text");
                assert_eq!(attachment.name, "edited.png");
                assert_eq!(attachment.upload_state, AttachmentUploadState::Preview);
                let image = attachment.image().unwrap();
                let history = image
                    .annotation_session
                    .open(&image.bytes, image.preview.clone())
                    .unwrap();
                assert_eq!(history.document.marks.len(), 1);
                assert!(
                    history.document.can_undo(),
                    "Applying must carry the attachment's original history forward"
                );
                assert_eq!(app.upload_result.as_ref().unwrap().attachments.len(), 1);
                assert!(app.staged_attachments[1].upload_state.is_uploaded());
            })
        });
    }

    fn four_image_paths() -> (tempfile::TempDir, Vec<PathBuf>) {
        let directory = tempfile::tempdir().unwrap();
        let paths = (1..=4)
            .map(|index| {
                let path = directory.path().join(format!("dropped-{index}.png"));
                std::fs::write(&path, include_bytes!("../packaging/menu-bar-icon.png")).unwrap();
                path
            })
            .collect::<Vec<_>>();
        (directory, paths)
    }

    fn drop_paths(cx: &mut gpui_kit::VisualTestContext, paths: Vec<PathBuf>) {
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let position = cx.debug_bounds("image-drop-zone").unwrap().center();
        cx.update(|window, cx| {
            use gpui_kit::{FileDropEvent, InputEvent as _};
            window.dispatch_event(
                FileDropEvent::Entered {
                    position,
                    paths: ExternalPaths(paths.into_iter().collect()),
                }
                .to_platform_input(),
                cx,
            );
            window.draw(cx).clear(cx);
            window.dispatch_event(FileDropEvent::Submit { position }.to_platform_input(), cx);
            window.dispatch_event(FileDropEvent::Ended.to_platform_input(), cx);
        });
        cx.run_until_parked();
    }

    #[gpui_kit::test]
    fn dropping_four_images_then_switching_to_bulk_keeps_and_uploads_every_image(
        cx: &mut TestAppContext,
    ) {
        cx.update(init_keybindings);
        let (view, cx) = upload_window(cx);
        let (_directory, paths) = four_image_paths();
        cx.update(|_, cx| {
            view.update(cx, |app, cx| {
                app.load_state = LoadState::Ready;
                app.staged_attachments = vec![mixed_attachments().remove(0)];
                cx.notify();
            });
        });
        drop_paths(cx, paths);
        cx.update(|window, cx| {
            view.update(cx, |app, _| {
                assert_eq!(app.staged_attachments.len(), 1);
                assert_eq!(
                    app.pending_replacement_attachments.as_ref().unwrap().len(),
                    4
                );
            });
            window.draw(cx).clear(cx);
        });
        cx.update(|window, cx| {
            use gpui_kit::test::TestWindowExt as _;
            window.click("switch-to-bulk-upload", cx);
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            view.update(cx, |app, cx| {
                assert!(app.bulk_upload);
                assert_eq!(app.staged_attachments.len(), 5);
                assert_eq!(
                    app.staged_attachments[1..]
                        .iter()
                        .map(|item| item.name.as_str())
                        .collect::<Vec<_>>(),
                    [
                        "dropped-1.png",
                        "dropped-2.png",
                        "dropped-3.png",
                        "dropped-4.png"
                    ]
                );
                app.start_upload(window, cx, |item, _, _| {
                    Ok(format!(
                        "https://github.com/user-attachments/assets/{}",
                        item.id
                    ))
                });
            });
        });
        cx.run_until_parked();
        cx.update(|_, cx| {
            view.update(cx, |app, _| {
                assert_eq!(app.upload_result.as_ref().unwrap().attachments.len(), 5);
                assert!(
                    app.staged_attachments
                        .iter()
                        .all(|item| item.upload_state.is_uploaded())
                );
            });
        });
    }

    #[gpui_kit::test]
    fn dropping_multiple_images_into_an_empty_draft_enables_bulk_upload(cx: &mut TestAppContext) {
        let (view, cx) = upload_window(cx);
        let (_directory, paths) = four_image_paths();
        drop_paths(cx, paths);
        cx.update(|window, cx| {
            view.update(cx, |app, _| {
                assert!(app.bulk_upload);
                assert_eq!(app.staged_attachments.len(), 4);
                assert!(app.pending_replacement_attachments.is_none());
            });
            assert!(!window.has_active_dialog(cx));
        });
    }

    #[gpui_kit::test]
    fn pasting_multiple_finder_files_preserves_the_whole_batch_for_the_bulk_choice(
        cx: &mut TestAppContext,
    ) {
        cx.update(init_keybindings);
        let (view, cx) = upload_window(cx);
        let (_directory, paths) = four_image_paths();
        cx.update(|window, cx| {
            cx.write_to_clipboard(ClipboardItem {
                entries: vec![ClipboardEntry::ExternalPaths(ExternalPaths(
                    paths.into_iter().collect(),
                ))],
            });
            view.update(cx, |app, cx| {
                app.load_state = LoadState::Ready;
                app.staged_attachments = vec![mixed_attachments().remove(0)];
                app.on_paste(&PasteImage, window, cx);
            });
        });
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.simulate_keystrokes("b");
        cx.update(|_, cx| {
            view.update(cx, |app, _| {
                assert!(app.bulk_upload);
                assert_eq!(app.staged_attachments.len(), 5);
            });
        });
    }

    #[gpui_kit::test]
    fn canceling_or_replacing_a_multi_file_drop_keeps_the_explicit_choice(cx: &mut TestAppContext) {
        let (_directory, paths) = four_image_paths();
        for key in ["escape", "enter"] {
            let (view, cx) = upload_window(cx);
            cx.update(|_, cx| {
                view.update(cx, |app, cx| {
                    app.load_state = LoadState::Ready;
                    app.staged_attachments = vec![mixed_attachments().remove(0)];
                    cx.notify();
                });
            });
            drop_paths(cx, paths.clone());
            cx.update(|window, cx| window.draw(cx).clear(cx));
            if key == "escape" {
                cx.simulate_keystrokes(key);
            } else {
                cx.update(|window, cx| {
                    use gpui_kit::test::TestWindowExt as _;
                    window.click("replace-image", cx);
                });
            }
            cx.update(|window, cx| {
                view.update(cx, |app, _| {
                    assert!(!app.bulk_upload);
                    assert_eq!(app.staged_attachments.len(), 1);
                    assert!(app.pending_replacement_attachments.is_none());
                    assert_eq!(
                        app.staged_attachments[0].name,
                        if key == "escape" {
                            "rocket-cat-transparent.png"
                        } else {
                            "dropped-1.png"
                        }
                    );
                });
                assert!(!window.has_active_dialog(cx));
            });
        }
    }

    #[gpui_kit::test]
    fn escape_preserves_the_main_window_and_its_draft(cx: &mut TestAppContext) {
        cx.update(crate::application_menu::init);
        let (view, cx) = upload_window(cx);
        cx.update(|window, cx| {
            assert!(!crate::application_menu::is_extra_window(window, cx));
            view.update(cx, |app, cx| {
                app.load_state = LoadState::Ready;
                app.staged_attachments = mixed_attachments();
                window.focus(&app.focus_handle, cx);
                window.activate_window();
            });
        });
        cx.simulate_keystrokes("escape");
        cx.update(|_, cx| {
            assert_eq!(view.read(cx).staged_attachments.len(), 2);
        });
    }

    #[gpui_kit::test]
    fn oversized_video_cancel_skips_it_and_preserves_other_import_order(cx: &mut TestAppContext) {
        let (view, cx) = upload_window(cx);
        let source = large_video();
        cx.update(|window, cx| {
            view.update(cx, |app, cx| {
                app.bulk_upload = true;
                let mut before = mixed_attachments().remove(0);
                before.name = "before.png".into();
                let mut after = mixed_attachments().remove(0);
                after.name = "after.png".into();
                app.accept_attachment_results(
                    vec![
                        Ok(before),
                        StagedAttachment::from_path(source.path().into(), Default::default()),
                        Ok(after),
                    ],
                    window,
                    cx,
                );
                assert!(app.import_busy());
                assert!(
                    app.compression.is_none(),
                    "compression needs an explicit choice"
                );
            });
            window.draw(cx).clear(cx);
        });
        cx.simulate_keystrokes("escape");
        cx.run_until_parked();
        cx.update(|_, cx| {
            view.update(cx, |app, _| {
                assert!(!app.import_busy());
                assert_eq!(
                    app.staged_attachments
                        .iter()
                        .map(|item| item.name.as_str())
                        .collect::<Vec<_>>(),
                    ["before.png", "after.png"]
                );
            })
        });
    }

    #[cfg(target_os = "macos")]
    #[gpui_kit::test]
    fn oversized_video_compress_choice_stages_only_the_finished_copy(cx: &mut TestAppContext) {
        let (view, cx) = upload_window(cx);
        let source = large_video();
        let original_len = source.as_file().metadata().unwrap().len();
        cx.update(|window, cx| {
            view.update(cx, |app, cx| {
                app.accept_attachment_result(
                    StagedAttachment::from_path(source.path().into(), Default::default()),
                    window,
                    cx,
                );
                assert!(app.staged_attachments.is_empty());
                assert!(app.import_busy());
            });
            window.draw(cx).clear(cx);
        });
        cx.simulate_keystrokes("enter");
        cx.run_until_parked();
        cx.update(|_, cx| {
            view.update(cx, |app, _| {
                assert!(!app.import_busy());
                assert_eq!(app.staged_attachments.len(), 1);
                assert!(app.staged_attachments[0].name.ends_with("-compressed.mp4"));
                assert!(app.staged_attachments[0].len() < crate::model::MAX_ATTACHMENT_BYTES);
                assert!(
                    app.upload_result.is_none(),
                    "compression must not upload automatically"
                );
            })
        });
        assert_eq!(source.as_file().metadata().unwrap().len(), original_len);
    }

    #[gpui_kit::test]
    fn lowering_video_limit_blocks_an_already_staged_large_upload(cx: &mut TestAppContext) {
        let (view, cx) = upload_window(cx);
        let source = large_video();
        let staged =
            StagedAttachment::from_path(source.path().into(), VideoUploadLimit::Paid).unwrap();
        cx.update(|window, cx| {
            view.update(cx, |app, cx| {
                app.staged_attachments = vec![staged];
                assert!(!app.settings.allow_large_videos);
                app.start_upload(window, cx, |_, _, _| {
                    panic!("the lower limit must block this upload")
                });
                assert!(!app.uploading);
                assert_eq!(app.staged_attachments.len(), 1);
                app.settings.allow_large_videos = true;
                app.start_upload(window, cx, |item, _, _| {
                    assert!(item.len() > crate::model::MAX_ATTACHMENT_BYTES);
                    Ok("https://github.com/user-attachments/assets/large-video".into())
                });
            });
        });
        cx.run_until_parked();
        cx.update(|_, cx| {
            view.update(cx, |app, _| {
                assert!(app.staged_attachments[0].upload_state.is_uploaded());
            })
        });
    }

    #[gpui_kit::test]
    fn idle_attachment_preview_keeps_original_spacing_below_drop_zone(cx: &mut TestAppContext) {
        let (view, cx) = upload_window(cx);
        for item in mixed_attachments() {
            cx.update(|window, cx| {
                view.update(cx, |app, cx| {
                    app.load_state = LoadState::Ready;
                    app.staged_attachments = vec![item];
                    cx.notify();
                });
                window.draw(cx).clear(cx);
            });
            let drop_zone = cx.debug_bounds("image-drop-zone").unwrap();
            let previews = cx.debug_bounds("staged-attachments").unwrap();
            let gap = previews.origin.y - drop_zone.bottom();
            assert_eq!(
                gap,
                px(20.),
                "idle preview should have one section gap below the drop zone"
            );
        }
    }

    #[gpui_kit::test]
    fn mixed_batch_keeps_successes_and_retries_only_failed_attachments(cx: &mut TestAppContext) {
        let (view, cx) = upload_window(cx);
        cx.update(|window, cx| {
            view.update(cx, |app, cx| {
                app.staged_attachments = mixed_attachments();
                app.start_upload(window, cx, |item, _, progress| {
                    progress(item.len(), item.len());
                    if item.image().is_none() {
                        Err(github::GithubError::Upload("temporary failure".into()))
                    } else {
                        Ok("https://github.com/user-attachments/assets/image".into())
                    }
                });
            })
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            view.update(cx, |app, cx| {
                assert!(!app.uploading);
                assert!(app.staged_attachments[0].upload_state.is_uploaded());
                assert!(
                    app.staged_attachments[1]
                        .upload_state
                        .failure_message()
                        .unwrap()
                        .contains("temporary failure")
                );
                app.start_upload(window, cx, |item, _, progress| {
                    assert!(
                        item.image().is_none(),
                        "successful image must not upload twice"
                    );
                    progress(item.len(), item.len());
                    Ok("https://github.com/user-attachments/assets/video".into())
                });
            })
        });
        cx.run_until_parked();
        cx.update(|_, cx| {
            view.update(cx, |app, _| {
                assert!(
                    app.staged_attachments
                        .iter()
                        .all(|item| item.upload_state.is_uploaded())
                );
                let snippet = app.upload_result.as_ref().unwrap().markdown();
                assert!(snippet.starts_with("<img width="));
                assert!(
                    snippet.ends_with(" />\n\nhttps://github.com/user-attachments/assets/video")
                );
            })
        });
    }

    #[gpui_kit::test]
    fn canceled_batch_preserves_completed_urls_and_releases_queued_items_for_retry(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = upload_window(cx);
        cx.update(|window, cx| {
            view.update(cx, |app, cx| {
                app.staged_attachments = mixed_attachments();
                app.start_upload(window, cx, |_, control, _| {
                    control.cancel();
                    Ok("https://github.com/user-attachments/assets/image".into())
                });
            })
        });
        cx.run_until_parked();
        cx.update(|_, cx| {
            view.update(cx, |app, _| {
                assert!(!app.uploading);
                assert!(app.staged_attachments[0].upload_state.is_uploaded());
                assert!(
                    app.staged_attachments[1]
                        .upload_state
                        .failure_message()
                        .unwrap()
                        .contains("canceled")
                );
                assert_eq!(app.upload_result.as_ref().unwrap().attachments.len(), 1);
            })
        });
    }

    #[gpui_kit::test]
    fn clearing_during_upload_cancels_work_and_does_not_restore_an_old_draft(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = upload_window(cx);
        cx.update(|window, cx| {
            view.update(cx, |app, cx| {
                app.staged_attachments = mixed_attachments();
                app.start_upload(window, cx, |_, _, _| {
                    Ok("https://github.com/user-attachments/assets/old".into())
                });
                let control = app.upload_control.clone().unwrap();
                app.clear_staged_attachments("Closed window", cx);
                assert!(control.is_canceled());
                app.staged_attachments = mixed_attachments();
            })
        });
        cx.run_until_parked();
        cx.update(|_, cx| {
            view.update(cx, |app, _| {
                assert!(!app.uploading);
                assert!(app.upload_result.is_none());
                assert!(
                    app.staged_attachments
                        .iter()
                        .all(|item| item.upload_state == AttachmentUploadState::Preview)
                );
            })
        });
    }

    #[test]
    fn bulk_upload_result_preserves_markdown_order_with_newline_separators() {
        let result = UploadResult {
            attachments: vec![
                UploadedAttachment {
                    markdown: "![first](https://example.com/first)".into(),
                },
                UploadedAttachment {
                    markdown: "![second](https://example.com/second)".into(),
                },
            ],
        };

        assert_eq!(
            result.markdown(),
            "![first](https://example.com/first)\n\n![second](https://example.com/second)"
        );
    }

    #[gpui_kit::test]
    fn update_badge_reacts_to_background_state_and_dispatches_one_click(cx: &mut TestAppContext) {
        let clicks = Rc::new(Cell::new(0));
        let clicked = clicks.clone();
        cx.update(move |cx| {
            gpui_kit::init(cx);
            cx.on_action(move |_: &crate::application_menu::InstallUpdate, _| {
                clicked.set(clicked.get() + 1);
            });
        });
        let (view, cx) = cx.add_window_view(|window, cx| {
            UploaderApp::with_settings(
                window,
                Diagnostics::new(),
                AppControllers::default(),
                AppSettings::default(),
                None,
                false,
                cx,
            )
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert!(cx.debug_bounds("install-update").is_none());
        cx.update(|_, cx| {
            Updater::shared(cx).update(cx, |updater, cx| {
                updater.state = UpdateState::Available;
                updater.message = "Install and restart Gatto".into();
                cx.notify();
            });
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let button = cx
            .debug_bounds("install-update")
            .expect("background checks should reveal the badge");
        cx.simulate_mouse_down(button.center(), MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(button.center(), MouseButton::Left, Modifiers::default());
        assert_eq!(clicks.get(), 1);
        cx.update(|window, cx| {
            view.update(cx, |app, cx| {
                app.uploading = true;
                cx.notify();
            });
            window.draw(cx).clear(cx);
        });
        cx.simulate_mouse_down(button.center(), MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(button.center(), MouseButton::Left, Modifiers::default());
        assert_eq!(clicks.get(), 1, "active uploads must disable installation");
    }

    #[gpui_kit::test]
    fn returning_to_main_page_leaves_preferences_and_restores_focus(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let (view, cx) = cx.add_window_view(|window, cx| {
            UploaderApp::with_settings(
                window,
                Diagnostics::new(),
                AppControllers::default(),
                AppSettings::default(),
                None,
                false,
                cx,
            )
        });
        cx.update(|window, cx| {
            view.update(cx, |this, cx| {
                this.show_settings(cx);
                assert!(this.settings_open);
                this.show_main_page(window, cx);
                assert!(!this.settings_open);
                assert!(this.focus_handle.is_focused(window));
            });
        });
    }

    #[gpui_kit::test]
    fn escape_leaves_preferences_from_inputs_and_cancels_recording_first(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_kit::init(cx);
            init_keybindings(cx);
            crate::image_preview::init_keybindings(cx);
            crate::application_menu::init(cx);
        });
        let (view, cx) = upload_window(cx);

        for focus in ["page", "owner", "shortcut", "none"] {
            cx.update(|window, cx| {
                window.activate_window();
                view.update(cx, |app, cx| {
                    app.show_settings(cx);
                    match focus {
                        "owner" => app
                            .owner_input
                            .update(cx, |input, cx| input.focus(window, cx)),
                        "shortcut" => app
                            .global_shortcut_input
                            .update(cx, |input, cx| input.focus(window, cx)),
                        "none" => window.blur(cx),
                        _ => app.focus_handle.focus(window, cx),
                    }
                });
                window.draw(cx).clear(cx);
            });
            cx.simulate_keystrokes("escape");
            cx.update(|window, cx| {
                view.update(cx, |app, _| {
                    assert!(!app.settings_open);
                    assert!(app.focus_handle.is_focused(window));
                });
            });
        }

        cx.update(|window, cx| {
            view.update(cx, |app, cx| {
                app.show_settings(cx);
                app.recording_global_shortcut = true;
                app.focus_handle.focus(window, cx);
            });
            window.draw(cx).clear(cx);
        });
        cx.simulate_keystrokes("escape");
        cx.update(|window, cx| {
            view.update(cx, |app, _| {
                assert!(app.settings_open);
                assert!(!app.recording_global_shortcut);
            });
            window.draw(cx).clear(cx);
        });
        cx.simulate_keystrokes("escape");
        cx.update(|window, cx| {
            view.update(cx, |app, _| {
                assert!(!app.settings_open);
                assert!(app.focus_handle.is_focused(window));
            });
        });
    }

    #[gpui_kit::test]
    fn escape_dismisses_a_preferences_dialog_before_leaving_preferences(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_kit::init(cx);
            init_keybindings(cx);
            crate::application_menu::init(cx);
        });
        let (view, cx) = upload_window(cx);
        cx.update(|window, cx| {
            window.activate_window();
            view.update(cx, |app, cx| {
                app.show_settings(cx);
                app.focus_handle.focus(window, cx);
                app.confirm_reset(&gpui_kit::ClickEvent::default(), window, cx);
            });
            window.draw(cx).clear(cx);
            assert!(window.has_active_dialog(cx));
        });
        cx.simulate_keystrokes("escape");
        cx.update(|window, cx| {
            assert!(!window.has_active_dialog(cx));
            assert!(view.read(cx).settings_open);
            window.draw(cx).clear(cx);
        });
        cx.simulate_keystrokes("escape");
        cx.update(|_, cx| assert!(!view.read(cx).settings_open));
    }

    #[gpui_kit::test]
    fn markdown_copy_hint_and_shortcut_work_for_single_and_bulk_uploads(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_kit::init(cx);
            init_keybindings(cx);
        });
        let captured = Rc::new(std::cell::RefCell::new(None));
        let capture = captured.clone();
        let (_, cx) = cx.add_window_view(move |window, cx| {
            let view = cx.new(|cx| {
                let mut app = UploaderApp::with_settings(
                    window,
                    Diagnostics::new(),
                    AppControllers::default(),
                    AppSettings {
                        owner: Some("acme-labs".into()),
                        ..AppSettings::default()
                    },
                    None,
                    false,
                    cx,
                );
                app.load_state = LoadState::Ready;
                app
            });
            *capture.borrow_mut() = Some(view.clone());
            gpui_kit::base::Root::new(view, window, cx)
        });
        let view = captured.borrow().clone().unwrap();
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert!(cx.debug_bounds("upload-markdown-copy-hint").is_none());

        let image_path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("packaging/rocket-cat-transparent.png");
        for (bulk_upload, copy_images_as_markdown) in
            [(false, false), (true, false), (false, true), (true, true)]
        {
            let pixel_size = StagedAttachment::from_path(image_path.clone(), Default::default())
                .unwrap()
                .image()
                .unwrap()
                .pixel_size;
            let snippet = |description: &str, url: &str| {
                if copy_images_as_markdown {
                    format!("![{description}]({url})")
                } else {
                    crate::model::html_image(description, url, pixel_size)
                }
            };
            let markdown = snippet("first", "https://example.com/first");
            let expected = if bulk_upload {
                format!(
                    "{markdown}\n\n{}",
                    snippet("second", "https://example.com/second")
                )
            } else {
                markdown.to_owned()
            };
            cx.update(|window, cx| {
                view.update(cx, |app, cx| {
                    app.bulk_upload = bulk_upload;
                    app.settings.copy_images_as_markdown = copy_images_as_markdown;
                    app.staged_attachments.clear();
                    let mut image =
                        StagedAttachment::from_path(image_path.clone(), Default::default())
                            .unwrap();
                    image.id = 1;
                    image.description = "first".into();
                    image.upload_state = AttachmentUploadState::Uploaded {
                        url: "https://example.com/first".into(),
                    };
                    app.staged_attachments.push(image);
                    if bulk_upload {
                        let mut image =
                            StagedAttachment::from_path(image_path.clone(), Default::default())
                                .unwrap();
                        image.id = 2;
                        image.description = "second".into();
                        image.upload_state = AttachmentUploadState::Uploaded {
                            url: "https://example.com/second".into(),
                        };
                        app.staged_attachments.push(image);
                    }
                    app.sync_upload_result();
                    app.focus_handle.focus(window, cx);
                    cx.notify();
                });
                window.draw(cx).clear(cx);
            });
            assert!(cx.debug_bounds("upload-markdown-copy-hint").is_some());
            let copy = cx
                .debug_bounds("copy-staged-image-markdown-1")
                .expect("individual copy button is visible");
            cx.simulate_mouse_down(copy.center(), MouseButton::Left, Modifiers::default());
            cx.simulate_mouse_up(copy.center(), MouseButton::Left, Modifiers::default());
            cx.update(|_, cx| {
                assert_eq!(
                    cx.read_from_clipboard().and_then(|item| item.text()),
                    Some(markdown.clone())
                );
            });
            cx.simulate_keystrokes("cmd-c");
            cx.update(|_, cx| {
                assert_eq!(
                    cx.read_from_clipboard().and_then(|item| item.text()),
                    Some(expected.clone()),
                );
            });
            if bulk_upload {
                let copy_all = cx
                    .debug_bounds("copy-all-staged-image-markdown")
                    .expect("bulk copy button is visible");
                cx.simulate_mouse_down(copy_all.center(), MouseButton::Left, Modifiers::default());
                cx.simulate_mouse_up(copy_all.center(), MouseButton::Left, Modifiers::default());
                cx.update(|_, cx| {
                    assert_eq!(
                        cx.read_from_clipboard().and_then(|item| item.text()),
                        Some(expected.clone())
                    );
                });
            }
        }
    }

    #[gpui_kit::test]
    fn repository_picker_click_survives_the_loading_transition(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let settings = AppSettings {
            owner: Some("acme-labs".into()),
            ..AppSettings::default()
        };
        let (view, cx) = cx.add_window_view(move |window, cx| {
            let mut app = UploaderApp::with_settings(
                window,
                Diagnostics::new(),
                AppControllers::default(),
                settings,
                None,
                false,
                cx,
            );
            app.load_state = LoadState::Ready;
            app
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));

        let trigger = cx
            .debug_bounds("repository-picker-trigger-closed")
            .expect("the repository picker trigger should be visible");
        cx.simulate_mouse_down(trigger.center(), MouseButton::Left, Modifiers::default());
        cx.update(|window, cx| {
            view.update(cx, |app, cx| {
                app.load_state = LoadState::Loading;
                app.repository_picker.update(cx, |picker, cx| {
                    picker.set_items(
                        SearchableVec::new(Vec::<SearchableGroup<String>>::new()),
                        window,
                        cx,
                    );
                });
                cx.notify();
            });
            window.draw(cx).clear(cx);
        });
        cx.simulate_mouse_up(trigger.center(), MouseButton::Left, Modifiers::default());
        cx.update(|window, cx| window.draw(cx).clear(cx));

        assert!(
            cx.debug_bounds("repository-picker-trigger-open").is_some(),
            "the repository picker should open even while repositories are loading"
        );
        assert!(
            cx.debug_bounds("repository-picker-loading").is_some(),
            "the open repository picker should show its loading indicator"
        );

        cx.update(|window, cx| {
            view.update(cx, |app, cx| {
                app.load_state = LoadState::Ready;
                app.repositories.insert(
                    "gatto".into(),
                    Repository {
                        id: 1,
                        name: "gatto".into(),
                    },
                );
                let groups = app.repository_groups();
                app.repository_picker.update(cx, |picker, cx| {
                    picker.set_items(SearchableVec::new(groups), window, cx);
                });
                cx.notify();
            });
            window.draw(cx).clear(cx);
        });

        assert!(
            cx.debug_bounds("repository-picker-trigger-open").is_some(),
            "loaded repository items should replace the spinner without closing the picker"
        );
        assert!(cx.debug_bounds("repository-picker-loading").is_none());
    }
}

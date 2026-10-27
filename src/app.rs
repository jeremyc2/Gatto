use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet, VecDeque},
    path::PathBuf,
    rc::Rc,
    sync::Arc,
};

mod attachment_actions;
mod attachment_drag;
mod projects;

use gpui_kit::assets::IconName as AssetIconName;
use gpui_kit::base::TestSupportExt as _;
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
    input::{Input, InputEvent, InputState, Textarea, TextareaState},
    kbd::Kbd,
    link::Link,
    menu::DropdownMenu as _,
    notification::Notification,
    progress::Progress,
    searchable_list::{SearchableGroup, SearchableVec},
    spinner::Spinner,
    v_flex,
};
use gpui_kit::{
    AnyWindowHandle, App, AppContext as _, Bounds, BoxShadow, ClipboardEntry, ClipboardItem,
    Context, Entity, EventEmitter, ExternalPaths, FocusHandle, Focusable, Image, ImageFormat,
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
        UNSUPPORTED_MEDIA_MESSAGE, parse_owner,
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
        StageFromClipboard,
        CopyUploadMarkdown,
        SelectStableChannel,
        SelectBetaChannel,
        ToggleProjectSidebar,
        CreateProject
    ]
);

const KEY_CONTEXT: &str = "Gatto";
const REPOSITORY_URL: &str = "https://github.com/jeremyc2/Gatto";

type RepositoryPicker = ComboboxState<SearchableVec<SearchableGroup<String>>>;

pub(crate) struct ConvertAttachmentRequested(pub StagedAttachment);
impl EventEmitter<ConvertAttachmentRequested> for UploaderApp {}

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
    Finished(Box<anyhow::Result<StagedAttachment>>),
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
    Project,
    AttachmentDetails,
    AttachmentReorder,
    SidebarCollapsed,
    EmptyProjects,
    ProjectRename,
}

#[cfg(feature = "walkthrough-screenshots")]
#[allow(dead_code)]
impl WalkthroughScenario {
    pub(crate) const ALL: [Self; 15] = [
        Self::FirstRun,
        Self::Preferences,
        Self::ImageStaged,
        Self::UploadComplete,
        Self::UpdateAvailable,
        Self::VideoStaged,
        Self::MixedUploadProgress,
        Self::VideoTooLarge,
        Self::VideoCompressing,
        Self::Project,
        Self::AttachmentDetails,
        Self::AttachmentReorder,
        Self::SidebarCollapsed,
        Self::EmptyProjects,
        Self::ProjectRename,
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
            Self::Project => "10-project.png",
            Self::AttachmentDetails => "11-attachment-details.png",
            Self::AttachmentReorder => "12-attachment-reorder.png",
            Self::SidebarCollapsed => "13-sidebar-collapsed.png",
            Self::EmptyProjects => "14-empty-projects.png",
            Self::ProjectRename => "15-project-rename.png",
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
    selected_attachments: HashSet<u64>,
    annotation_selection: Option<Vec<u64>>,
    pending_comparison: Option<[StagedAttachment; 2]>,
    pending_open_files: Vec<PathBuf>,
    attachment_drag: Option<attachment_drag::ReorderSession>,
    attachment_bounds: Rc<RefCell<HashMap<u64, Bounds<gpui_kit::Pixels>>>>,
    project_bounds: Rc<RefCell<HashMap<PathBuf, Bounds<gpui_kit::Pixels>>>>,
    main_scroll: gpui_kit::ScrollHandle,
    reorder_revision: u64,
    project_state: projects::ProjectState,
    persist_preferences: bool,
    paste_generation: u64,
    state_generation: u64,
    upload_result: Option<UploadResult>,
    uploading: bool,
    upload_control: Option<github::UploadControl>,
    upload_generation: u64,
    pending_import: Option<PendingImport>,
    loading_attachment_paths: bool,
    compression: Option<CompressionJob>,
    combining_comparison: bool,
    settings: AppSettings,
    settings_open: bool,
    recording_global_shortcut: bool,
    pending_menu_paste: bool,
    log_window: Option<(AnyWindowHandle, Entity<LogViewer>)>,
    preview_open: bool,
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
                    this.project_state.metadata.repository = this.selected_repository.clone();
                    this.project_state.metadata.repository_owner = this.settings.owner.clone();
                    if this.project_state.folder.is_none() {
                        this.settings.last_repository =
                            this.selected_repository.as_ref().map(|r| r.name.clone());
                        this.settings.last_repository_id =
                            this.selected_repository.as_ref().map(|r| r.id);
                        if persist_repository_selection {
                            let _ = this.settings.save();
                        }
                    }
                    this.save_project(cx);
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
            selected_attachments: HashSet::new(),
            annotation_selection: None,
            pending_comparison: None,
            pending_open_files: Vec::new(),
            attachment_drag: None,
            attachment_bounds: Rc::default(),
            project_bounds: Rc::default(),
            main_scroll: gpui_kit::ScrollHandle::new(),
            reorder_revision: 0,
            project_state: projects::ProjectState::default(),
            persist_preferences: persist_repository_selection,
            paste_generation: 0,
            state_generation: 0,
            upload_result: None,
            uploading: false,
            upload_control: None,
            upload_generation: 0,
            pending_import: None,
            loading_attachment_paths: false,
            compression: None,
            combining_comparison: false,
            settings,
            settings_open: false,
            recording_global_shortcut: false,
            pending_menu_paste: false,
            log_window: None,
            preview_open: false,
            preview_window: None,
            menu_bar: controllers.menu_bar,
            global_shortcut: controllers.global_shortcut,
            diagnostics,
            updater,
            _update_subscription: update_subscription,
            header_mark: Arc::new(Image::from_bytes(
                ImageFormat::Png,
                include_bytes!("../packaging/app-cat-pixel.png").to_vec(),
            )),
            rooftops_window: None,
            focus_handle: cx.focus_handle(),
            _owner_subscription: owner_subscription,
            _global_shortcut_subscription: global_shortcut_subscription,
            _repository_subscription: subscription,
        };
        this.register_project_shutdown(cx);
        this.register_attachment_reorder_controls(cx);
        this.initialize_workspaces(window, cx);
        if let Some(error) = settings_error {
            window.push_notification(Notification::error(error), cx);
        }
        if persist_repository_selection {
            // Workspace loading restores its own repository selection.
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
            settings.owner = Some("field-notes".into());
            settings.pinned_repositories.insert("pixel-tools".into());
            settings.last_repository = Some("pixel-tools".into());
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
                name: "pixel-tools".into(),
            },
            Repository {
                id: 1002,
                name: "game-client".into(),
            },
            Repository {
                id: 1003,
                name: "documentation".into(),
            },
        ] {
            this.repositories
                .insert(repository.name.clone(), repository);
        }
        this.selected_repository = this.repositories.get("pixel-tools").cloned();
        this.load_state = LoadState::Ready;
        this.settings_open = matches!(scenario, WalkthroughScenario::Preferences);

        if matches!(
            scenario,
            WalkthroughScenario::ImageStaged
                | WalkthroughScenario::UploadComplete
                | WalkthroughScenario::MixedUploadProgress
                | WalkthroughScenario::Project
                | WalkthroughScenario::AttachmentDetails
                | WalkthroughScenario::AttachmentReorder
                | WalkthroughScenario::SidebarCollapsed
                | WalkthroughScenario::ProjectRename
        ) {
            let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("docs/images/walkthrough/source/canvas.png");
            let mut image = StagedAttachment::from_path(path, Default::default())
                .expect("the walkthrough fixture image should always be valid");
            image.name = "interface-review.png".into();
            image.title = "Interface review".into();

            if matches!(
                scenario,
                WalkthroughScenario::UploadComplete
                    | WalkthroughScenario::MixedUploadProgress
                    | WalkthroughScenario::Project
                    | WalkthroughScenario::ProjectRename
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
            video.title = "Interaction recording".into();
            if matches!(scenario, WalkthroughScenario::MixedUploadProgress) {
                this.uploading = true;
                this.upload_control = Some(github::UploadControl::default());
                video.upload_state = AttachmentUploadState::Uploading {
                    sent: video.len() * 65 / 100,
                    total: video.len(),
                };
            }
            this.staged_attachments.push(video);
        }

        if matches!(scenario, WalkthroughScenario::MixedUploadProgress) {
            let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("docs/images/walkthrough/source/timeline.png");
            let mut image = StagedAttachment::from_path(path, Default::default())
                .expect("the walkthrough timeline fixture should always be valid");
            image.name = "timeline-feedback.png".into();
            image.title = "Timeline feedback".into();
            image.upload_state = AttachmentUploadState::Queued;
            this.staged_attachments.push(image);
        }

        if matches!(
            scenario,
            WalkthroughScenario::AttachmentReorder | WalkthroughScenario::SidebarCollapsed
        ) {
            let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("docs/images/walkthrough/source/timeline.png");
            let mut image = StagedAttachment::from_path(path, Default::default())
                .expect("valid timeline fixture");
            image.title = "Timeline feedback".into();
            this.staged_attachments.push(image);
            this.project_state.folders.push(PathBuf::from(
                "/Users/example/.gatto/projects/Interface review",
            ));
            this.settings.sidebar_collapsed =
                matches!(scenario, WalkthroughScenario::SidebarCollapsed);
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
        if matches!(
            scenario,
            WalkthroughScenario::Project
                | WalkthroughScenario::AttachmentDetails
                | WalkthroughScenario::ProjectRename
        ) {
            let folder = PathBuf::from("/Users/example/.gatto/projects/Interface review");
            this.project_state.folders.push(folder.clone());
            this.project_state.folder = Some(folder);
            this.staged_attachments[0].description =
                "Review the repository picker and upload controls.".into();
            if matches!(scenario, WalkthroughScenario::AttachmentDetails) {
                let image_id = this.staged_attachments[0].id;
                let view = cx.entity().downgrade();
                window.defer(cx, move |window, cx| {
                    let _ = view.update(cx, |this, cx| {
                        this.open_description_dialog(image_id, window, cx)
                    });
                });
            }
        }
        this.refresh_repository_picker(window, cx);
        if matches!(scenario, WalkthroughScenario::ProjectRename)
            && let Some(folder) = this.project_state.folder.clone()
        {
            let view = cx.entity().downgrade();
            window.defer(cx, move |window, cx| {
                let _ = view.update(cx, |this, cx| {
                    this.edit_project_metadata(folder.clone(), window, cx)
                });
            });
        }
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
        if self.attachment_drag.is_some() {
            self.cancel_attachment_reorder(window, cx);
        } else if self.recording_global_shortcut {
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

    fn stage_from_clipboard_menu(
        &mut self,
        _: &StageFromClipboard,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.paste_from_menu(window, cx);
    }

    /// Stages the current clipboard image and prefers the last selected repository
    /// when it is pinned, otherwise the first pinned repository alphabetically.
    fn paste_from_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.settings_open = false;
        if self.project_state.loading {
            self.pending_menu_paste = true;
            return;
        }
        self.on_paste(&PasteImage, window, cx);
    }

    /// Clears a pasted image when the main window is closed, or when the user
    /// explicitly discards it. Incrementing the generation keeps a pending
    /// clipboard conversion from restoring an image after it was cleared.
    #[cfg(test)]
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
        self.loading_attachment_paths = false;
        self.combining_comparison = false;
        self.uploading = false;
        self.pending_menu_paste = false;
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

    pub fn report_open_error(
        &mut self,
        error: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let error = error.into();
        self.diagnostics
            .error(format!("Could not open the request: {error}"));
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
                        this.accept_repository_session(session, window, cx);
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

    fn accept_repository_session(
        &mut self,
        session: github::GithubSession,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.diagnostics.info(format!(
            "Loaded {} accessible repositories.",
            session.repositories.len()
        ));
        self.repositories = session
            .repositories
            .into_iter()
            .map(|repository| (repository.name.clone(), repository))
            .collect();
        let groups = self.repository_groups();
        self.token = Some(session.token);
        self.load_state = LoadState::Ready;
        self.repository_picker.update(cx, |picker, cx| {
            picker.set_items(SearchableVec::new(groups), window, cx);
        });
        if let Some(repository_name) = self
            .project_state
            .metadata
            .repository_for_owner(self.settings.owner.as_deref())
            .map(|repository| repository.name.clone())
            .filter(|name| self.repositories.contains_key(name))
        {
            self.select_repository(&repository_name, window, cx);
        } else {
            self.selected_repository = None;
            self.repository_picker
                .update(cx, |picker, cx| picker.set_selected_indices([], window, cx));
        }
        self.try_complete_menu_paste(cx);
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
        let prompt = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: true,
            prompt: Some("Choose images or videos".into()),
        });
        let state_generation = self.state_generation;
        let staging_generation = self.upload_generation;

        cx.spawn_in(window, async move |this, window| {
            let Some(paths) = prompt.await.ok().and_then(Result::ok).flatten() else {
                return;
            };
            let _ = this.update_in(window, |this, window, cx| {
                if this.state_generation != state_generation
                    || this.upload_generation != staging_generation
                {
                    return;
                }
                this.load_attachment_paths(paths, window, cx);
            });
        })
        .detach();
    }

    #[cfg(feature = "walkthrough-screenshots")]
    #[allow(dead_code)]
    pub(crate) fn walkthrough_comparison(&self) -> Option<StagedAttachment> {
        self.staged_attachments
            .iter()
            .find(|image| image.name == "side-by-side.png")
            .cloned()
    }

    pub(crate) fn stage_comparison_in_scratchpad(
        &mut self,
        sources: [StagedAttachment; 2],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), &'static str> {
        self.request_workspace(None, window, cx);
        if self.project_state.loading {
            self.pending_comparison = Some(sources);
            Ok(())
        } else if self.project_state.folder.is_none() {
            self.begin_comparison(sources, window, cx)
        } else {
            Err("Finish the current work first.")
        }
    }

    pub(crate) fn begin_comparison(
        &mut self,
        sources: [StagedAttachment; 2],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), &'static str> {
        self.combine_comparison(sources, None, window, cx)
    }

    pub(crate) fn replace_with_comparison(
        &mut self,
        sources: [StagedAttachment; 2],
        replace_id: u64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), &'static str> {
        let Some(original) = self
            .staged_attachments
            .iter()
            .find(|item| item.id == replace_id)
        else {
            return Err("The original image is no longer available.");
        };
        let source = sources.iter().find(|item| item.id == replace_id);
        if !source
            .and_then(StagedAttachment::image)
            .zip(original.image())
            .is_some_and(|(source, current)| {
                !current.is_comparison && source.bytes == current.bytes
            })
        {
            return Err("The original image has changed. Open conversion again.");
        }
        if self.project_state.loading {
            return Err("Finish loading the current project first.");
        }
        self.combine_comparison(sources, Some(replace_id), window, cx)
    }

    fn combine_comparison(
        &mut self,
        sources: [StagedAttachment; 2],
        replace_id: Option<u64>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), &'static str> {
        if self.project_state.loading && self.project_state.storage_folder.is_none() {
            self.pending_comparison = Some(sources);
            return Ok(());
        }
        if self.uploading || self.import_busy() {
            return Err(
                "Finish the current upload or import, then remove an image and add it again to retry.",
            );
        }
        self.show_main_page(window, cx);
        self.combining_comparison = true;
        let generation = self.upload_generation;
        cx.notify();
        cx.spawn_in(window, async move |this, window| {
            let result = window
                .background_spawn(async move { crate::image_composer::combine(sources) })
                .await;
            let _ = this.update_in(window, |this, window, cx| {
                if this.upload_generation != generation {
                    return;
                }
                this.combining_comparison = false;
                if let Some(replace_id) = replace_id {
                    match result {
                        Ok(mut comparison) => {
                            if let Some(original) = this
                                .staged_attachments
                                .iter_mut()
                                .find(|item| item.id == replace_id)
                            {
                                comparison.id = original.id;
                                comparison.persistent_id = original.persistent_id;
                                comparison.title = original.title.clone();
                                comparison.description = original.description.clone();
                                *original = comparison;
                                this.sync_upload_result();
                                this.save_project(cx);
                            }
                        }
                        Err(error) => this.warn(error.to_string(), window, cx),
                    }
                } else {
                    this.accept_attachment_result(result, window, cx);
                }
                cx.notify();
            });
        })
        .detach();
        Ok(())
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
            self.load_attachment_paths(paths, window, cx);
            return;
        }

        self.warn(UNSUPPORTED_MEDIA_MESSAGE, window, cx);
        self.pending_menu_paste = false;
    }

    fn load_attachment_paths(
        &mut self,
        paths: Vec<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if paths.is_empty() || self.uploading || self.import_busy() {
            return;
        }
        self.loading_attachment_paths = true;
        let state_generation = self.state_generation;
        let staging_generation = self.upload_generation;
        let video_limit = self.settings.video_upload_limit();
        let task = cx.background_spawn(async move {
            paths
                .into_iter()
                .map(|path| StagedAttachment::from_path(path, video_limit))
                .collect::<Vec<_>>()
        });
        cx.spawn_in(window, async move |view, window| {
            let results = task.await;
            let _ = view.update_in(window, |this, window, cx| {
                if this.state_generation != state_generation
                    || this.upload_generation != staging_generation
                {
                    return;
                }
                this.loading_attachment_paths = false;
                this.accept_attachment_results(results, window, cx);
            });
        })
        .detach();
        cx.notify();
    }

    /// Finder/open-document requests stage for review even before GitHub setup.
    pub fn stage_opened_files(
        &mut self,
        paths: Vec<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.project_state.loading && self.project_state.storage_folder.is_none() {
            self.pending_open_files.extend(paths);
            return;
        }
        if paths.is_empty() {
            return;
        }
        if self.uploading
            || self.import_busy()
            || window.has_active_dialog(cx)
            || self.updater.read(cx).state == UpdateState::Installing
        {
            self.warn("Finish the current upload, import, update, or dialog, then open the files with Gatto again. Your current draft has been kept.", window, cx);
            return;
        }
        self.show_main_page(window, cx);
        self.load_attachment_paths(paths, window, cx);
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
        self.attachment_drag.is_some()
            || self.project_state.loading
            || self.loading_attachment_paths
            || self.pending_import.is_some()
            || self.compression.is_some()
            || self.combining_comparison
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
        window.open_alert_dialog(cx, move |alert, _, _| {
            let accept_view = view.clone();
            let close_view = view.clone();
            let path = video.path.clone();
            let accepted = resolved.clone();
            let closed = resolved.clone();
            alert.width(px(480.)).title("Video too large.")
                .description(format!("This video is {:.1} MB. Do you want to compress it to below the {limit_mb} MB threshold?", video.bytes as f64 / 1_000_000.))
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
            let _ = sender.try_send(CompressionEvent::Finished(Box::new(result)));
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
                                match *result {
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
        self.append_staged_attachments(images, window, cx);
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
        self.save_project(cx);
        self.diagnostics.info(format!(
            "Staged {added} attachment{} for upload.",
            if added == 1 { "" } else { "s" }
        ));
        self.try_complete_menu_paste(cx);
        cx.notify();
    }

    fn change_attachment_description(
        &mut self,
        image_id: u64,
        requested_name: &str,
        description: &str,
        cx: &mut Context<Self>,
    ) -> anyhow::Result<()> {
        anyhow::ensure!(
            !self.uploading && !self.project_state.loading,
            "Wait for the current upload or project to finish."
        );
        let image = self
            .staged_attachments
            .iter_mut()
            .find(|image| image.id == image_id)
            .ok_or_else(|| anyhow::anyhow!("That attachment is no longer staged."))?;
        image.title = requested_name.to_owned();
        image.description = description.to_owned();
        let description = image.title.clone();
        self.sync_upload_result();
        self.save_project(cx);
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
        let current_description = image.title.clone();
        let description_input = cx.new(|cx| {
            TextareaState::new(window, cx)
                .placeholder("Optional description")
                .default_value(image.description.clone())
        });
        let input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Descriptive attachment name")
                .default_value(current_description)
        });
        let feedback = theme::DialogFeedback::default();
        let rename_view = cx.entity().downgrade();
        let rename_input = input.clone();
        let select_initial_description = Rc::new(Cell::new(true));

        window.open_alert_dialog(cx, move |alert, window, cx| {
            let feedback = feedback.clone();
            let rename_input = rename_input.clone();
            let description_input = description_input.clone();
            let rename_view = rename_view.clone();
            if select_initial_description.replace(false) {
                input.update(cx, |input, cx| {
                    input.focus(window, cx);
                    input.select_all(window, cx);
                });
            }
            alert
                .width(px(480.))
                .title("Edit attachment")
                .child(
                    v_flex()
                        .gap_2()
                        .child(div().text_xs().font_semibold().child("Title"))
                        .child(Input::new(&input).w_full().aria_label("Title"))
                        .child(div().mt_2().text_xs().font_semibold().child("Description"))
                        .child(
                            Textarea::new(&description_input)
                                .h(px(120.))
                                .w_full()
                                .aria_label("Description"),
                        ),
                )
                .children(feedback.element(cx))
                .show_cancel(true)
                .cancel_text("Cancel")
                .ok_text("Save description")
                .footer(theme::confirmation_footer("ok", "Save description", 148.))
                .on_ok(move |_, window, cx| {
                    let requested_name = rename_input.read(cx).value().to_string();
                    match rename_view.update(cx, |this, cx| {
                        this.change_attachment_description(
                            image_id,
                            &requested_name,
                            description_input.read(cx).value().as_ref(),
                            cx,
                        )
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
                            this.save_project(cx);
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
        let upload_generation = self.upload_generation;
        cx.spawn_in(window, async move |this, window| {
            let result = window
                .background_spawn(async move { github::load_token(&diagnostics) })
                .await;
            let _ = this.update_in(window, move |this, window, cx| {
                if this.state_generation != state_generation {
                    return;
                }
                this.accept_upload_token(upload_generation, result, window, cx);
            });
        })
        .detach();
    }

    fn accept_upload_token(
        &mut self,
        upload_generation: u64,
        result: Result<String, github::GithubError>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.upload_generation != upload_generation {
            return;
        }
        self.loading_token = false;
        match result {
            Ok(token) => {
                self.token = Some(token);
                self.send_attachments(window, cx);
            }
            Err(error) => {
                self.diagnostics
                    .error(format!("GitHub authentication failed: {error}"));

                window.push_notification(Notification::error(error.to_string()), cx);

                cx.notify();
            }
        }
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
        let upload_generation = self.upload_generation;
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
                this.accept_upload_repository(
                    upload_generation,
                    &requested_repository,
                    result,
                    window,
                    cx,
                );
            });
        })
        .detach();
    }

    fn accept_upload_repository(
        &mut self,
        upload_generation: u64,
        requested_repository: &str,
        result: Result<u64, github::GithubError>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.upload_generation != upload_generation {
            return;
        }
        self.loading_repository = false;
        if self
            .selected_repository
            .as_ref()
            .map(|repo| repo.name.as_str())
            != Some(requested_repository)
        {
            cx.notify();
            return;
        }
        match result {
            Ok(id) => {
                if let Some(repository) = self.selected_repository.as_mut() {
                    repository.id = id;
                    self.repositories
                        .insert(repository.name.clone(), repository.clone());
                    self.project_state.metadata.repository = Some(repository.clone());
                    if self.project_state.folder.is_none() {
                        self.settings.last_repository_id = Some(id);
                    }
                    self.save_project(cx);
                    if let Err(error) = self.settings.save() {
                        self.diagnostics.error(format!(
                            "Could not remember the selected repository: {error}"
                        ));
                    }
                }
                self.send_attachments(window, cx);
            }
            Err(error) => {
                self.diagnostics.error(format!(
                    "Could not resolve the remembered repository: {error}"
                ));

                window.push_notification(Notification::error(error.to_string()), cx);

                cx.notify();
            }
        }
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
        if self.attachment_drag.is_some() && event.keystroke.key == "escape" {
            window.prevent_default();
            cx.stop_propagation();
            self.cancel_attachment_reorder(window, cx);
        } else if self.recording_global_shortcut {
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

    pub fn prepare_to_close(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.attachment_drag.is_some() {
            self.cancel_attachment_reorder(window, cx);
        }
        if self.project_state.saving
            || self.project_state.loading
            || self.project_state.error.is_some()
        {
            window.push_notification(
                Notification::warning(
                    "Wait for project saving to finish, retry the failed save, or save a copy before closing.",
                ),
                cx,
            );
            return false;
        }
        if self.uploading
            || self.loading_attachment_paths
            || self.pending_import.is_some()
            || self.compression.is_some()
            || self.combining_comparison
        {
            self.pending_work_dialog(None, window, cx);
            return false;
        }
        self.cancel_global_shortcut_recording(window, cx);
        true
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
        let feedback = theme::DialogFeedback::default();
        let uploader = cx.entity().downgrade();
        window.open_alert_dialog(cx, move |alert, _, cx| {
            let feedback = feedback.clone();
            let uploader = uploader.clone();
            alert
                .width(px(440.))
                .title("Reset Gatto?")
                .description(
                    "This clears all Gatto data on this device and returns the app to its first-run state. This can’t be undone.",
                )
                .children(feedback.element(cx))
                .show_cancel(true)
                .cancel_text("Cancel")
                .ok_text("Reset app")
                .footer(theme::confirmation_footer("ok", "Reset app", 104.))
                .on_ok(move |_, window, cx| {
                    match uploader.update(cx, |this, cx| this.begin_reset(window, cx)) {
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
        anyhow::ensure!(
            !self.project_state.saving
                && !self.project_state.loading
                && self.project_state.error.is_none(),
            "Wait for project saving or retry a failed save before resetting Gatto."
        );
        self.global_shortcut.set_shortcut(None)?;
        AppSettings::reset()?;

        self.project_state = projects::ProjectState::default();
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
        self.loading_attachment_paths = false;
        self.combining_comparison = false;
        self.staged_attachments.clear();
        self.pending_menu_paste = false;
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

    fn open_attachment_preview(
        &mut self,
        selected_id: u64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.project_state.loading {
            return;
        }
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
            .filter(|item| {
                item.image().is_some()
                    && self
                        .annotation_selection
                        .as_ref()
                        .is_none_or(|ids| ids.contains(&item.id))
            })
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
                self.preview_open = true;
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
                let _ = uploader.update(cx, |this, cx| this.note_preview_closed(should_close, cx));
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
                        if result.is_ok() {
                            this.save_project(cx);
                        }
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
                self.preview_open = true;
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

    fn note_preview_closed(&mut self, removed: bool, cx: &mut Context<Self>) {
        // macOS hides and retains this window; its handle alone does not mean editing is active.
        self.preview_open = false;
        if removed {
            self.preview_window = None;
        }
        self.diagnostics.info("Closed the image preview window.");
        cx.notify();
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
        edited.persistent_id = attachment.persistent_id;
        edited.title = attachment.title.clone();
        edited.description = attachment.description.clone();
        if let (Media::Image(original), Media::Image(replacement)) =
            (&attachment.media, &mut edited.media)
        {
            replacement.is_comparison = original.is_comparison;
            replacement.annotation_session = original.annotation_session.clone();
            if replacement.bytes == original.bytes {
                edited.upload_state = attachment.upload_state.clone();
            }
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
        if self.uploading
            || self.import_busy()
            || self.project_state.saving
            || self.project_state.error.is_some()
            || quick_copy_busy
        {
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
                    .on_drop(cx.listener(|this, paths: &ExternalPaths, window, cx| {
                        this.load_attachment_paths(paths.paths().to_vec(), window, cx);
                    }))
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
                            .child(Kbd::new(Keystroke::parse("cmd-v").expect("valid shortcut")))
                            .child("/ Drag and drop / Click to add media"),
                    ),
            )
    }

    pub(super) fn render_attachment_card(
        &self,
        image: &StagedAttachment,
        floating: bool,
        cx: &mut Context<Self>,
    ) -> gpui_kit::AnyElement {
        let image_id = image.id;
        let source = image.image().map(|data| data.preview.clone());
        let title = image.title.clone();
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
                AttachmentStatus::Pending,
                AssetIconName::Clock,
                "Queued".to_owned(),
                cx.theme().muted_foreground,
            ),
            AttachmentUploadState::Uploading { sent, total } => (
                AttachmentStatus::Uploading,
                AssetIconName::LoaderCircle,
                if sent >= total {
                    "Waiting for GitHub…".to_owned()
                } else {
                    let (unit, divisor) = if *total >= 1_000_000 {
                        ("MB", 1_000_000.)
                    } else {
                        ("KB", 1000.)
                    };
                    format!(
                        "Sending {}% · {:.1} / {:.1} {unit}",
                        sent * 100 / total.max(&1),
                        *sent as f64 / divisor,
                        *total as f64 / divisor
                    )
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
            AttachmentUploadState::Uploading { sent, total } => {
                Some(sent as f32 / total.max(1) as f32 * 100.)
            }
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
        let failure_message = image.upload_state.failure_message().map(ToOwned::to_owned);
        let preview_view = cx.entity().downgrade();
        let marker_view = cx.entity().downgrade();
        let copy_view = cx.entity().downgrade();
        let draggable = !self.uploading && (!self.import_busy() || self.attachment_drag.is_some());
        let card = Attachment::new()
                .id(format!("staged-image-attachment-{image_id}"))
                .large()
                .border_0()
                .h(px(62.))
                .flex_1()
                .min_w_0()
                .w_full()
                .when(!floating, |card| card.cursor_pointer())
                .status(status)
                .when(image.image().is_none(), |attachment| attachment.tooltip("Open video in your macOS video app"))
                .media(match source {
                    Some(source) => AttachmentMedia::new().src(source).cursor(gpui_kit::CursorStyle::OpenHand),
                    None => AttachmentMedia::new().cursor(gpui_kit::CursorStyle::OpenHand).child(Icon::new(AssetIconName::Play).size(px(26.))),
                })
                .content(
                    AttachmentContent::new()
                        .title(AttachmentTitle::new(title))
                        .description(AttachmentDescription::new(detail))
                        .child(h_flex().gap_2().items_center().child(status_row)
                            .when(image.image().is_some_and(|image| !image.marker_descriptions.is_empty()), |row| row.child(
                                Button::new(format!("marker-descriptions-{image_id}"))
                                    .debug_selector(move || format!("marker-descriptions-{image_id}"))
                                    .app_icon().icon(AssetIconName::ListOrdered)
                                    .accessibility_label("Marker descriptions")
                                    .tooltip("Marker descriptions · included when copying this image")
                                    .on_click(move |_, window, cx| {
                                        cx.stop_propagation();
                                        let _ = marker_view.update(cx, |this, cx| this.open_attachment_preview(image_id, window, cx));
                                    })
                            )))
                        .when_some(upload_percentage, |content, value| content.child(
                            Progress::new(format!("attachment-progress-{image_id}")).value(value).color(status_color).xsmall().w_full()
                                .accessibility_label("Attachment upload progress")
                        )),
                )
                .when(!self.uploading && !self.project_state.loading, |attachment| {
                    attachment.actions(AttachmentActions::new().child(
                        h_flex().gap_1().id(format!("attachment-action-controls-{image_id}"))
                            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                            .child(self.attachment_options(vec![image_id], true, cx))
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

                    ))
                })
                .when_some(failure_message, |attachment, message| {
                    attachment.tooltip(message)
                })
                .into_any_element();

        let can_drag = draggable;
        let border = if matches!(image.upload_state, AttachmentUploadState::Failed { .. }) {
            cx.theme().danger
        } else {
            cx.theme().border
        };
        let owner = cx.entity().downgrade();
        let attachment = image.clone();
        let layout = self.attachment_bounds.clone();
        h_flex()
            .id(format!(
                "attachment-card-{image_id}{}",
                if floating { "-floating" } else { "" }
            ))
            .debug_selector(move || {
                format!(
                    "attachment-card-{image_id}{}",
                    if floating { "-floating" } else { "" }
                )
            })
            .relative()
            .w_full()
            .h(px(64.))
            .items_center()
            .rounded_lg()
            .border_1()
            .border_color(border)
            .when(
                matches!(
                    image.upload_state,
                    AttachmentUploadState::Preview | AttachmentUploadState::Queued
                ),
                |card| card.border_dashed(),
            )
            .bg(cx.theme().background)
            .overflow_hidden()
            .child({
                let owner = cx.entity().downgrade();
                div()
                    .pl_2()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(
                        Checkbox::new(format!("select-attachment-{image_id}"))
                            .debug_selector(move || format!("select-attachment-{image_id}"))
                            .checked(self.selected_attachments.contains(&image_id))
                            .disabled(floating)
                            .on_click(move |checked, _, cx| {
                                cx.stop_propagation();
                                let _ = owner.update(cx, |this, cx| {
                                    if *checked {
                                        this.selected_attachments.insert(image_id);
                                    } else {
                                        this.selected_attachments.remove(&image_id);
                                    }
                                    cx.notify();
                                });
                            }),
                    )
            })
            .child(card)
            .when(!floating, |card| {
                card.cursor_pointer().on_click(move |_, window, cx| {
                    let _ = preview_view.update(cx, |this, cx| {
                        if this.attachment_drag.is_none() {
                            this.open_attachment_preview(image_id, window, cx);
                        }
                    });
                })
            })
            .when(!floating && can_drag, |card| {
                card.on_drag(
                    attachment_drag::AttachmentDrag { id: image_id },
                    move |drag, offset, window, cx| {
                        let bounds = layout.borrow().get(&image_id).copied().unwrap_or_default();
                        let _ = owner.update(cx, |this, cx| {
                            this.begin_attachment_reorder(drag.id, offset.y, window, cx)
                        });
                        let owner = owner.clone();
                        let attachment = attachment.clone();
                        cx.new(|cx| {
                            if let Some(entity) = owner.upgrade() {
                                cx.observe(&entity, |_, _, cx| cx.notify()).detach();
                            }
                            attachment_drag::AttachmentDragPreview {
                                owner,
                                attachment,
                                width: bounds.size.width,
                                offset,
                            }
                        })
                    },
                )
            })
            .when(floating, |card| card.shadow_lg())
            .into_any_element()
    }

    fn render_attachments(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let count = self.staged_attachments.len();
        let selected = self
            .staged_attachments
            .iter()
            .filter(|item| self.selected_attachments.contains(&item.id))
            .map(|item| item.id)
            .collect::<Vec<_>>();
        let all_selected = count > 0 && selected.len() == count;
        v_flex()
            .debug_selector(|| "staged-attachments".into())
            .gap_2()
            .when(count > 0, |list| {
                list.child(
                    h_flex()
                        .items_center()
                        .justify_between()
                        .gap_2()
                        .child(
                            h_flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    div()
                                        .text_xs()
                                        .font_semibold()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(format!(
                                            "{count} attachment{}",
                                            if count == 1 { "" } else { "s" }
                                        )),
                                )
                                .child(
                                    Button::new("select-all-attachments")
                                        .debug_selector(|| "select-all-attachments".into())
                                        .app_secondary()
                                        .small()
                                        .min_w_0()
                                        .px_2()
                                        .h(px(28.))
                                        .label(if all_selected {
                                            "Deselect All"
                                        } else {
                                            "Select All"
                                        })
                                        .disabled(self.uploading || self.import_busy())
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            if all_selected {
                                                this.selected_attachments.clear();
                                            } else {
                                                this.selected_attachments = this
                                                    .staged_attachments
                                                    .iter()
                                                    .map(|item| item.id)
                                                    .collect();
                                            }
                                            cx.notify();
                                        })),
                                )
                                .when(
                                    !selected.is_empty() && !self.uploading && !self.import_busy(),
                                    |row| row.child(self.attachment_options(selected, false, cx)),
                                ),
                        )
                        .when(count > 1 && self.upload_result.is_some(), |row| {
                            row.child(
                                Button::new("copy-all-upload-markdown")
                                    .debug_selector(|| "copy-all-staged-image-markdown".into())
                                    .app_secondary()
                                    .label("Copy all")
                                    .on_click(cx.listener(Self::copy_all_upload_markdown)),
                            )
                        }),
                )
            })
            .when_some(self.upload_result.as_ref(), |list, _result| {
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
                        .child("to copy all attachments."),
                )
            })
            .children(
                self.staged_attachments
                    .iter()
                    .map(|image| self.render_reorder_row(image, cx))
                    .collect::<Vec<_>>(),
            )
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
                            .child("Use ![title](URL) instead of an image tag. Markdown images do not include width or height. Applies to individual copy, bulk copy, and Quick Copy."),
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
                    .child(
                        h_flex().justify_between().items_center()
                            .child(div().font_semibold().text_sm().child("Release channel"))
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
                    }),
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
                    .child(h_flex().gap_2().flex_wrap()
                        .child(Button::new("delete-all-projects").app_secondary().label("Delete All Projects").on_click(cx.listener(|this, _, window, cx| this.delete_all_projects(window, cx))))
                        .child(Button::new("reset-app").app_secondary().label("Reset app").on_click(cx.listener(Self::confirm_reset)))),

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
            .on_drag_move::<attachment_drag::AttachmentDrag>(cx.listener(
                |this,
                 event: &gpui_kit::DragMoveEvent<attachment_drag::AttachmentDrag>,
                 window,
                 cx| this.move_attachment_reorder(event.event.position, window, cx),
            ))
            .capture_any_mouse_up(cx.listener(|this, _, window, cx| {
                if this.attachment_drag.is_some() {
                    this.drop_attachment(window, cx);
                    cx.stop_propagation();
                }
            }))
            .on_action(cx.listener(Self::on_paste))
            .on_action(cx.listener(Self::stage_from_clipboard_menu))
            .on_action(
                cx.listener(|this, _: &ToggleProjectSidebar, _, cx| {
                    this.toggle_project_sidebar(cx)
                }),
            )
            .when(self.can_create_project(), |root| {
                root.on_action(cx.listener(Self::create_project))
            })
            .size_full()
            .bg(cx.theme().background);

        if self.settings_open {
            root.child(self.render_settings(cx)).into_any_element()
        } else {
            let needs_setup = self.load_state == LoadState::NeedsSetup;
            let workspace_title = self.project_state.title.clone().unwrap_or_else(|| {
                self.project_state.folder.as_ref().map_or_else(
                    || "Scratchpad".to_owned(),
                    |folder| {
                        folder
                            .file_name()
                            .unwrap_or(folder.as_os_str())
                            .to_string_lossy()
                            .into_owned()
                    },
                )
            });
            let root = root.on_action(cx.listener(Self::copy_upload_markdown));
            let content = v_flex()
                .id("main-page")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .track_scroll(&self.main_scroll)
                .child(
                    v_flex()
                        .flex_none()
                        .gap_5()
                        .px_6()
                        .pt_6()
                        .pb_5()
                        .when_some(self.project_state.error.clone(), |content, error| content.child(
                            v_flex().gap_2().child(div().text_sm().text_color(cx.theme().danger).child(error))
                                .child(Button::new("retry-project-save").app_secondary().label("Retry save").disabled(self.project_state.saving)
                                    .on_click(cx.listener(|this, _, window, cx| { if this.project_state.storage_folder.is_none() && this.persist_preferences { this.initialize_workspaces(window, cx); } else { this.save_project(cx); } })))

                        ))
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
                                        .child(div().text_2xl().font_semibold().child(if self.project_state.folder.is_some() { format!("Project: {workspace_title}") } else { workspace_title }))
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
                                                .when(self.settings.sidebar_collapsed, |row| row.child(
                                                    Button::new("show-project-sidebar").debug_selector(|| "show-project-sidebar".into()).app_icon().icon(AssetIconName::PanelLeftOpen)
                                                        .accessibility_label("Show sidebar").tooltip("Show sidebar · ⌘B")
                                                        .on_click(cx.listener(|this, _, _, cx| this.toggle_project_sidebar(cx)))
                                                ))
                                                .child(
                                                    Button::new("open-logs")
                                                        .app_icon()
                                                        .icon(AssetIconName::Logs)
                                                        .accessibility_label("Application logs")
                                                        .tooltip("App logs")
                                                        .on_click(
                                                            cx.listener(Self::open_log_window),
                                                        ),
                                                )
                                                .child(
                                                    Button::new("open-settings")
                                                        .app_icon()
                                                        .icon(IconName::Settings)
                                                        .accessibility_label("App Preferences")
                                                        .tooltip("App preferences")
                                                        .on_click(cx.listener(|this, _, _, cx| {
                                                            this.show_settings(cx);
                                                        })),
                                                )
                                                .child(
                                                    Button::new("workspace-information").debug_selector(|| "workspace-information".into())
                                                        .app_icon()
                                                        .icon(AssetIconName::CircleQuestionMark)
                                                        .accessibility_label("Project information")
                                                        .tooltip("Project information")
                                                        .on_click(cx.listener(|this, _, window, cx| this.open_project_info(window, cx))),
                                                ),
                                        ),
                                ),
                        )
                        .when(needs_setup, |root| root.child(self.render_setup_prompt(cx)))
                        .when(!needs_setup, |root| {
                            root.child(self.render_repository_picker("Repository", cx))
                                .child(self.render_drop_zone(cx))
                                .when(self.compression.is_some(), |root| {
                                    root.child(self.render_compression(cx))
                                })
                                .when(self.combining_comparison, |root| {
                                    root.child(
                                        div().id("comparison-processing").test_support().child(
                                            Attachment::new()
                                                .large().w_full()
                                                .status(AttachmentStatus::Processing)
                                                .media(AttachmentMedia::new())
                                                .content(AttachmentContent::new()
                                                    .title(AttachmentTitle::new("Preparing comparison…"))
                                                    .description(AttachmentDescription::new("Resizing and combining images"))),
                                        )
                                    )
                                })
                                .child(self.render_attachments(cx))
                        }),
                );
            let main = v_flex().flex_1().min_w_0().h_full().child(content).child(
                h_flex()
                    .w_full()
                    .flex_none()
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .justify_end()
                    .items_center()
                    .gap_2()
                    .px_6()
                    .py_3()
                    .when(self.uploading, |footer| {
                        footer.child(
                            Button::new("cancel-upload")
                                .app_secondary()
                                .label("Cancel upload")
                                .disabled(
                                    self.upload_control
                                        .as_ref()
                                        .is_some_and(|control| control.is_canceled()),
                                )
                                .on_click(cx.listener(Self::cancel_upload)),
                        )
                    })
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
                            .when(!can_send, |button| button.cursor_not_allowed())
                            .when(self.staged_attachments.is_empty(), |button| {
                                button.tooltip("Add an image or video first")
                            })
                            .on_click(cx.listener(Self::send)),
                    ),
            );
            root.child(
                h_flex()
                    .size_full()
                    .child(self.render_project_sidebar(cx))
                    .child(main),
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
        KeyBinding::new("cmd-b", ToggleProjectSidebar, Some(KEY_CONTEXT)),
    ]);
}

#[cfg(test)]
mod tests {
    use crate::model::VideoUploadLimit;
    use gpui_kit::test::TestWindowExt as _;
    use gpui_kit::{Modifiers, TestAppContext};

    use super::*;

    pub(super) fn upload_window(
        cx: &mut TestAppContext,
    ) -> (Entity<UploaderApp>, &mut gpui_kit::VisualTestContext) {
        cx.update(|cx| {
            gpui_kit::init(cx);
            // Pointer tests need resting dialog bounds under parallel test load.
            cx.set_reduce_motion(true);
        });
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

    #[gpui_kit::test]
    fn automatic_comparison_shows_loading_and_preserves_existing_draft_choices(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = upload_window(cx);
        let original = mixed_attachments().remove(0);
        let original_id = original.id;
        cx.update(|window, cx| {
            view.update(cx, |app, cx| {
                app.staged_attachments = vec![original.clone()];
                app.uploading = true;
                assert!(
                    app.begin_comparison([original.clone(), original.clone()], window, cx)
                        .is_err()
                );
                assert_eq!(app.staged_attachments[0].id, original_id);
                app.uploading = false;
                app.begin_comparison([original.clone(), original.clone()], window, cx)
                    .unwrap();
                assert!(app.combining_comparison && app.import_busy());
                assert!(
                    app.begin_comparison([original.clone(), original.clone()], window, cx)
                        .is_err()
                );
            })
        });
        cx.update(|window, cx| {
            window.render_frame(cx);
            assert!(window.try_find("comparison-processing").is_some());
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            view.update(cx, |app, _| {
                assert!(!app.combining_comparison);
                assert_eq!(app.staged_attachments[0].id, original_id);
                let composition = app.staged_attachments.last().unwrap();
                assert_eq!(composition.name, "side-by-side.png");
                assert!(composition.image().is_some());
            });
            assert!(!window.has_active_dialog(cx));
        });
    }
    #[gpui_kit::test]
    fn converting_replaces_original_in_place_and_keeps_metadata(cx: &mut TestAppContext) {
        let (view, cx) = upload_window(cx);
        let original = mixed_attachments().remove(0);
        let before = StagedAttachment::from_bytes(
            "before.png".into(),
            include_bytes!("../packaging/menu-bar-icon.png").to_vec(),
            None,
        )
        .unwrap();
        let after = StagedAttachment::from_bytes(
            "after.png".into(),
            include_bytes!("../packaging/menu-bar-icon.png").to_vec(),
            None,
        )
        .unwrap();
        let identity = original.persistent_id;
        let id = original.id;
        cx.update(|window, cx| {
            view.update(cx, |app, cx| {
                let mut original = original.clone();
                original.title = "Keep title".into();
                original.description = "Keep description".into();
                app.staged_attachments = vec![before.clone(), original.clone(), after.clone()];
                app.replace_with_comparison([original, before.clone()], id, window, cx)
                    .unwrap();
            });
        });
        cx.run_until_parked();
        cx.update(|_, cx| {
            let app = view.read(cx);
            assert_eq!(app.staged_attachments.len(), 3);
            assert_eq!(app.staged_attachments[0].id, before.id);
            assert_eq!(app.staged_attachments[2].id, after.id);
            let converted = &app.staged_attachments[1];
            assert_eq!(converted.id, id);
            assert_eq!(converted.persistent_id, identity);
            assert_eq!(converted.title, "Keep title");
            assert_eq!(converted.description, "Keep description");
            assert!(converted.image().unwrap().is_comparison);
        });
    }
    #[gpui_kit::test]
    fn clearing_the_main_draft_drops_a_pending_comparison(cx: &mut TestAppContext) {
        let (view, cx) = upload_window(cx);
        cx.update(|window, cx| {
            view.update(cx, |app, cx| {
                let source = mixed_attachments().remove(0);
                app.begin_comparison([source.clone(), source], window, cx)
                    .unwrap();
                app.clear_staged_attachments("test clearing during comparison", cx);
            })
        });
        cx.run_until_parked();
        cx.update(|_, cx| {
            assert!(!view.read(cx).combining_comparison);
            assert!(view.read(cx).staged_attachments.is_empty());
        });
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
                attachments[0].title = "Keep this alt text".into();
                attachments[0].description = "Keep the separate description".into();
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
                    crate::annotation::EditState {
                        next_marker: 1,
                        marks: history.saved_marks,
                        comparison: history.saved_comparison,
                    },
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
                assert_eq!(attachment.title, "Keep this alt text");
                assert_eq!(attachment.description, "Keep the separate description");
                assert_eq!(attachment.name, "edited.png");
                assert_eq!(attachment.upload_state, AttachmentUploadState::Preview);
                let image = attachment.image().unwrap();
                let history = image
                    .annotation_session
                    .open(&image.bytes, image.preview.clone())
                    .unwrap();
                assert_eq!(history.document.marks.len(), 1);
                assert!(
                    !history.document.can_undo(),
                    "Reopening preserves marks without carrying closed-window undo steps"
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

    #[gpui_kit::test]
    fn finder_selection_stages_supported_files_in_order_without_uploading(cx: &mut TestAppContext) {
        let (view, cx) = upload_window(cx);
        let (directory, mut paths) = four_image_paths();
        let unsupported = directory.path().join("notes.txt");
        std::fs::write(&unsupported, "not an image").unwrap();
        paths.insert(1, unsupported);
        paths.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample.mp4"));
        cx.update(|window, cx| {
            view.update(cx, |app, cx| {
                app.stage_opened_files(paths, window, cx);
                assert!(app.import_busy());
            })
        });
        cx.run_until_parked();
        cx.update(|_, cx| {
            view.update(cx, |app, _| {
                assert!(!app.import_busy());
                assert_eq!(
                    app.staged_attachments
                        .iter()
                        .map(|item| item.name.as_str())
                        .collect::<Vec<_>>(),
                    [
                        "dropped-1.png",
                        "dropped-2.png",
                        "dropped-3.png",
                        "dropped-4.png",
                        "sample.mp4"
                    ]
                );
                assert!(!app.uploading);
                assert!(app.upload_result.is_none());
                assert!(app.diagnostics.text().contains("1 file could not be read"));
                assert!(
                    app.staged_attachments
                        .iter()
                        .all(|item| item.upload_state == AttachmentUploadState::Preview)
                );
            })
        });
    }

    #[gpui_kit::test]
    fn finder_selection_appends_to_existing_attachments(cx: &mut TestAppContext) {
        let (view, cx) = upload_window(cx);
        let (_directory, paths) = four_image_paths();
        cx.update(|window, cx| {
            view.update(cx, |app, cx| {
                app.staged_attachments = vec![mixed_attachments().remove(0)];
                app.stage_opened_files(paths, window, cx);
            })
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            assert_eq!(view.read(cx).staged_attachments.len(), 5);
            assert!(!window.has_active_dialog(cx));
        });
    }

    #[gpui_kit::test]
    fn finder_single_file_and_unsupported_only_selection_keep_normal_staging(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = upload_window(cx);
        let (directory, paths) = four_image_paths();
        cx.update(|window, cx| {
            view.update(cx, |app, cx| {
                app.stage_opened_files(vec![paths[0].clone()], window, cx);
            })
        });
        cx.run_until_parked();
        let invalid = directory.path().join("unsupported.txt");
        std::fs::write(&invalid, "unsupported").unwrap();
        cx.update(|window, cx| {
            view.update(cx, |app, cx| {
                assert_eq!(app.staged_attachments.len(), 1);
                app.stage_opened_files(vec![invalid], window, cx);
            })
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            view.update(cx, |app, cx| {
                assert_eq!(app.staged_attachments.len(), 1);
                assert_eq!(app.staged_attachments[0].name, "dropped-1.png");
                assert!(!window.has_active_dialog(cx));
                assert!(app.upload_result.is_none());
            })
        });
    }

    #[gpui_kit::test]
    fn finder_import_blocks_reentry_and_close_invalidates_pending_work(cx: &mut TestAppContext) {
        let (view, cx) = upload_window(cx);
        let (_directory, paths) = four_image_paths();
        cx.update(|window, cx| {
            view.update(cx, |app, cx| {
                app.stage_opened_files(paths.clone(), window, cx);
                app.stage_opened_files(paths, window, cx);
                app.clear_staged_attachments("Closed during Finder import", cx);
            })
        });
        cx.run_until_parked();
        cx.update(|_, cx| {
            view.update(cx, |app, _| {
                assert!(!app.import_busy());
                assert!(app.staged_attachments.is_empty());
            })
        });
    }

    #[gpui_kit::test]
    fn pasted_files_block_a_finder_request_until_import_finishes(cx: &mut TestAppContext) {
        let (view, cx) = upload_window(cx);
        let (_directory, paths) = four_image_paths();
        cx.update(|window, cx| {
            cx.write_to_clipboard(ClipboardItem {
                entries: vec![ClipboardEntry::ExternalPaths(ExternalPaths(
                    vec![paths[0].clone()].into_iter().collect(),
                ))],
            });
            view.update(cx, |app, cx| {
                app.load_state = LoadState::Ready;
                app.on_paste(&PasteImage, window, cx);
                app.stage_opened_files(paths[1..].to_vec(), window, cx);
            });
        });
        cx.run_until_parked();
        cx.update(|_, cx| {
            let app = view.read(cx);
            assert!(!app.import_busy());
            assert_eq!(app.staged_attachments.len(), 1);
            assert_eq!(app.staged_attachments[0].name, "dropped-1.png");
            assert!(
                app.diagnostics
                    .text()
                    .contains("Your current draft has been kept")
            );
        });
    }

    #[gpui_kit::test]
    fn finder_oversized_video_keeps_compression_prompt_and_other_files(cx: &mut TestAppContext) {
        let (view, cx) = upload_window(cx);
        let (_directory, paths) = four_image_paths();
        let video = large_video();
        cx.update(|window, cx| {
            view.update(cx, |app, cx| {
                app.stage_opened_files(
                    vec![paths[0].clone(), video.path().into(), paths[1].clone()],
                    window,
                    cx,
                );
            })
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            assert!(window.has_active_dialog(cx));
            assert!(view.read(cx).compression.is_none());
            window.draw(cx).clear(cx);
        });
        cx.simulate_keystrokes("escape");
        cx.run_until_parked();
        cx.update(|_, cx| {
            view.update(cx, |app, _| {
                assert_eq!(app.staged_attachments.len(), 2);
                assert!(!app.import_busy());
                assert!(app.upload_result.is_none());
            })
        });
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
    fn dropped_files_append_and_upload_in_order(cx: &mut TestAppContext) {
        let (view, cx) = upload_window(cx);
        let (_directory, paths) = four_image_paths();
        cx.update(|_, cx| {
            view.update(cx, |app, cx| {
                app.staged_attachments = vec![mixed_attachments().remove(0)];
                cx.notify();
            })
        });
        drop_paths(cx, paths);
        cx.update(|window, cx| {
            view.update(cx, |app, cx| {
                assert_eq!(app.staged_attachments.len(), 5);
                app.start_upload(window, cx, |item, _, _| {
                    Ok(format!("https://example.com/{}", item.id))
                });
            })
        });
        cx.run_until_parked();
        cx.update(|_, cx| {
            assert_eq!(
                view.read(cx)
                    .upload_result
                    .as_ref()
                    .unwrap()
                    .attachments
                    .len(),
                5
            )
        });
    }

    #[gpui_kit::test]
    fn dropping_multiple_images_stages_every_attachment(cx: &mut TestAppContext) {
        let (view, cx) = upload_window(cx);
        let (_directory, paths) = four_image_paths();
        drop_paths(cx, paths);
        cx.update(|window, cx| {
            view.update(cx, |app, _| {
                assert_eq!(app.staged_attachments.len(), 4);
            });
            assert!(!window.has_active_dialog(cx));
        });
    }

    #[gpui_kit::test]
    fn pasting_multiple_finder_files_appends_the_whole_batch(cx: &mut TestAppContext) {
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
        cx.update(|_, cx| {
            view.update(cx, |app, _| {
                assert_eq!(app.staged_attachments.len(), 5);
            });
        });
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
        cx.update(|_, cx| crate::application_menu::init(cx));
        let source = large_video();
        cx.update(|window, cx| {
            view.update(cx, |app, cx| {
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
    fn shortcut_recorder_receives_command_shift_g(cx: &mut TestAppContext) {
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
                app.settings.global_quick_copy_shortcut = "Command+Shift+U".into();
                app.settings.global_quick_copy_shortcut_enabled = true;
                // No native service is installed in a GPUI test. Receiving the
                // key ends recording at registration failure, before saving.
                app.recording_global_shortcut = true;
                app.focus_handle.focus(window, cx);
            });
            window.draw(cx).clear(cx);
        });
        cx.simulate_keystrokes("cmd-shift-g");
        cx.update(|_, cx| {
            assert!(!view.read(cx).recording_global_shortcut);
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
            let markdown = format!(
                "{}\n\n1. \n2. \n3. \n4. \n5. Choose repository\n6. \n7. ",
                snippet("first", "https://example.com/first")
            );
            let expected = if bulk_upload {
                format!(
                    "{markdown}\n\n{}\n\n1. ",
                    snippet("second", "https://example.com/second")
                )
            } else {
                markdown.to_owned()
            };
            cx.update(|window, cx| {
                view.update(cx, |app, cx| {
                    app.settings.copy_images_as_markdown = copy_images_as_markdown;
                    app.staged_attachments.clear();
                    let mut image =
                        StagedAttachment::from_path(image_path.clone(), Default::default())
                            .unwrap();
                    image.id = 1;
                    image.title = "first".into();
                    if let Media::Image(data) = &mut image.media {
                        data.marker_descriptions = (1..=7)
                            .map(|number| {
                                (
                                    number,
                                    if number == 5 {
                                        "Choose repository".into()
                                    } else {
                                        String::new()
                                    },
                                )
                            })
                            .collect();
                    }
                    image.upload_state = AttachmentUploadState::Uploaded {
                        url: "https://example.com/first".into(),
                    };
                    app.staged_attachments.push(image);
                    if bulk_upload {
                        let mut image =
                            StagedAttachment::from_path(image_path.clone(), Default::default())
                                .unwrap();
                        image.id = 2;
                        image.title = "second".into();
                        if let Media::Image(data) = &mut image.media {
                            data.marker_descriptions = vec![(1, String::new())];
                        }
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
            assert!(cx.debug_bounds("marker-descriptions-1").is_some());
            if bulk_upload {
                assert!(
                    cx.debug_bounds("marker-descriptions-2").is_some(),
                    "Markers with blank descriptions must still show the icon"
                );
            }
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

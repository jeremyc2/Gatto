//! Windowless clipboard staging and explicit or opted-in upload/copy.
use std::{cell::Cell, path::PathBuf, rc::Rc, sync::Arc};

use anyhow::{Context as _, Result, bail, ensure};
use gpui_kit::{
    App, AppContext as _, ClipboardEntry, ClipboardItem, SystemNotification,
    SystemNotificationAction, SystemNotificationResponse, Task,
};
use serde::{Deserialize, Serialize};

use crate::{
    diagnostics::Diagnostics,
    github,
    model::{AttachmentUploadState, ImageCopyFormat, Media, Repository, StagedAttachment},
    projects::Project,
    settings::AppSettings,
};

const TAG_PREFIX: &str = "gatto-add-from-clipboard:";
pub const UPLOAD_AND_COPY: &str = "upload-and-copy";
pub type Store = Rc<dyn Fn(ClipboardUpdate, &mut App) -> Task<Result<ClipboardReceipt>>>;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SavedClipboardItem {
    pub folder: String,
    pub project_id: uuid::Uuid,
    pub attachment_id: uuid::Uuid,
}

impl SavedClipboardItem {
    pub fn notification_tag(&self) -> String {
        format!(
            "{TAG_PREFIX}{}",
            serde_json::to_string(self).expect("notification identity is serializable")
        )
    }

    pub fn from_notification(response: &SystemNotificationResponse) -> Option<Self> {
        if response.action_id.as_deref() != Some(UPLOAD_AND_COPY) {
            return None;
        }
        serde_json::from_str(response.tag.strip_prefix(TAG_PREFIX)?).ok()
    }

    fn directory(&self, settings: &AppSettings) -> Result<PathBuf> {
        let parent = settings.project_directory()?;
        let path = std::path::Path::new(&self.folder);
        ensure!(
            path.components().count() == 1
                && matches!(
                    path.components().next(),
                    Some(std::path::Component::Normal(_))
                ),
            "The saved item must be in a project folder."
        );
        crate::projects::project_in_directory(&parent, &parent.join(path))
    }
}

#[derive(Clone)]
pub struct ClipboardReceipt {
    pub item: SavedClipboardItem,
    pub project_title: String,
    pub snippet: Option<String>,
}

pub struct ClipboardUpdate {
    pub folder: PathBuf,
    pub project_id: uuid::Uuid,
    pub project_title: String,
    pub attachment: StagedAttachment,
    pub original: Option<StagedAttachment>,
    pub copy_format: ImageCopyFormat,
}

impl ClipboardUpdate {
    pub fn receipt(&self, item: &StagedAttachment) -> ClipboardReceipt {
        ClipboardReceipt {
            item: SavedClipboardItem {
                folder: self
                    .folder
                    .file_name()
                    .expect("project folder")
                    .to_string_lossy()
                    .into_owned(),
                project_id: self.project_id,
                attachment_id: item.persistent_id,
            },
            project_title: self.project_title.clone(),
            snippet: item
                .upload_state
                .uploaded_url()
                .map(|url| item.snippet(url, self.copy_format)),
        }
    }

    /// Runs in the background; a notification must never overwrite a newer edit.
    pub fn validate_original(&self, current: &StagedAttachment) -> Result<()> {
        if let Some(original) = &self.original {
            ensure!(
                current.persistent_id == original.persistent_id
                    && crate::projects::media_hash(current)?
                        == crate::projects::media_hash(original)?
                    && (current.image().is_some() || current.name == original.name),
                "The attachment changed while uploading. Its latest edits have been kept; upload it again."
            );
        }
        Ok(())
    }

    pub fn same_instance(current: &StagedAttachment, snapshot: &StagedAttachment) -> bool {
        match (&current.media, &snapshot.media) {
            (Media::Image(a), Media::Image(b)) => Arc::ptr_eq(&a.bytes, &b.bytes),
            (Media::Video(a), Media::Video(b)) => {
                Arc::ptr_eq(a, b) && current.name == snapshot.name
            }
            _ => false,
        }
    }

    pub fn apply_uploaded(&self, current: &mut StagedAttachment) -> ClipboardReceipt {
        current.upload_state = self.attachment.upload_state.clone();
        self.receipt(current)
    }

    pub fn persist(self) -> Result<ClipboardReceipt> {
        let (mut project, mut attachments) = Project::open(self.folder.clone())?;
        ensure!(
            project.id == self.project_id,
            "The original project is no longer available."
        );
        let receipt = if self.original.is_some() {
            let current = attachments
                .iter_mut()
                .find(|item| item.persistent_id == self.attachment.persistent_id)
                .context("The attachment was removed from its project.")?;
            self.validate_original(current)?;
            self.apply_uploaded(current)
        } else {
            let receipt = self.receipt(&self.attachment);
            attachments.push(self.attachment.clone());
            receipt
        };
        project
            .save(&attachments)
            .context("Could not save the clipboard item to its project")?;
        Ok(receipt)
    }
}

#[derive(Clone)]
pub struct AddFromClipboard {
    busy: Rc<Cell<bool>>,
    diagnostics: Diagnostics,
}

#[derive(Clone)]
enum Work {
    Clipboard,
    UploadSaved(SavedClipboardItem),
}

impl AddFromClipboard {
    pub fn new(diagnostics: Diagnostics, busy: Rc<Cell<bool>>) -> Self {
        Self { busy, diagnostics }
    }
    pub fn is_busy(&self) -> bool {
        self.busy.get()
    }

    pub fn start(
        &self,
        cx: &mut App,
        saved: Option<async_channel::Receiver<Result<(), String>>>,
        store: Store,
    ) {
        self.start_work(cx, Work::Clipboard, saved, store);
    }

    pub fn upload_saved(
        &self,
        item: SavedClipboardItem,
        cx: &mut App,
        saved: Option<async_channel::Receiver<Result<(), String>>>,
        store: Store,
    ) {
        self.start_work(cx, Work::UploadSaved(item), saved, store);
    }

    fn start_work(
        &self,
        cx: &mut App,
        work: Work,
        saved: Option<async_channel::Receiver<Result<(), String>>>,
        store: Store,
    ) {
        if crate::updater::Updater::shared(cx).read(cx).state
            == crate::updater::UpdateState::Installing
        {
            show_notification(
                cx,
                "Updating Gatto",
                "Try Add from Clipboard again after Gatto restarts.",
                None,
            );
            return;
        }
        self.run_with(
            cx,
            work,
            AppSettings::load,
            Arc::new(GithubBackend),
            saved,
            store,
        );
    }

    fn run_with(
        &self,
        cx: &mut App,
        work: Work,
        settings: impl FnOnce() -> Result<AppSettings> + Send + 'static,
        backend: Arc<dyn Backend>,
        saved: Option<async_channel::Receiver<Result<(), String>>>,
        store: Store,
    ) {
        if self.busy.replace(true) {
            show_notification(
                cx,
                "Add from Clipboard in progress",
                "Wait for the current item to finish.",
                None,
            );
            return;
        }
        let clipboard = if matches!(work, Work::Clipboard) {
            cx.read_from_clipboard()
        } else {
            None
        };
        let mut retry = match &work {
            Work::UploadSaved(item) => Some(item.clone()),
            Work::Clipboard => None,
        };
        let diagnostics = self.diagnostics.clone();
        let job_backend = backend.clone();
        let task = cx.background_spawn(async move {
            if let Some(saved) = saved {
                saved.recv().await?.map_err(anyhow::Error::msg)?;
            }
            let settings = settings()?;
            let (update, upload_after) = match work {
                Work::Clipboard => (
                    stage_clipboard(&settings, clipboard)?,
                    settings.automatically_copy_add_from_clipboard,
                ),
                Work::UploadSaved(item) => (
                    upload_saved_item(&settings, &item, &diagnostics, job_backend.as_ref())?,
                    false,
                ),
            };
            Ok::<_, anyhow::Error>((settings, update, upload_after))
        });
        let controller = self.clone();
        cx.spawn(async move |cx| {
            let result = async {
                let (settings, update, upload_after) = task.await?;
                let receipt = cx.update(|cx| store(update, cx)).await?;
                retry = Some(receipt.item.clone());
                if !upload_after {
                    return Ok(receipt);
                }
                let item = receipt.item;
                let diagnostics = controller.diagnostics.clone();
                let update = cx
                    .background_spawn(async move {
                        upload_saved_item(&settings, &item, &diagnostics, backend.as_ref())
                    })
                    .await?;
                cx.update(|cx| store(update, cx)).await
            }
            .await;
            cx.update(|cx| {
                controller.busy.set(false);
                match result {
                    Ok(receipt) => {
                        if let Some(snippet) = receipt.snippet {
                            cx.dismiss_system_notification(&receipt.item.notification_tag());
                            cx.write_to_clipboard(ClipboardItem::new_string(snippet));
                            show_notification(
                                cx,
                                "Add from Clipboard complete",
                                &format!(
                                    "Uploaded in {}. Output copied to the clipboard.",
                                    receipt.project_title
                                ),
                                None,
                            );
                        } else {
                            show_notification(
                                cx,
                                "Added from Clipboard",
                                &format!("Saved in {}.", receipt.project_title),
                                Some(&receipt.item),
                            );
                        }
                        controller
                            .diagnostics
                            .info("Add from Clipboard saved the item to its project.");
                    }
                    Err(error) => {
                        controller
                            .diagnostics
                            .error(format!("Add from Clipboard failed: {error:#}"));
                        show_notification(
                            cx,
                            "Add from Clipboard failed",
                            &format!("{error:#}"),
                            retry.as_ref(),
                        );
                    }
                }
            });
        })
        .detach();
    }
}

fn show_notification(cx: &mut App, title: &str, body: &str, item: Option<&SavedClipboardItem>) {
    cx.show_system_notification(SystemNotification {
        tag: item
            .map(SavedClipboardItem::notification_tag)
            .unwrap_or_else(|| "gatto-add-from-clipboard-status".into())
            .into(),
        title: title.to_owned().into(),
        body: body.to_owned().into(),
        actions: item
            .map(|_| {
                vec![SystemNotificationAction {
                    id: UPLOAD_AND_COPY.into(),
                    label: "Upload and copy".into(),
                }]
            })
            .unwrap_or_default(),
    });
}

trait Backend: Send + Sync {
    fn upload(
        &self,
        owner: &str,
        repository: &Repository,
        image: &StagedAttachment,
        diagnostics: &Diagnostics,
    ) -> Result<String>;
}
struct GithubBackend;
impl Backend for GithubBackend {
    fn upload(
        &self,
        owner: &str,
        repository: &Repository,
        image: &StagedAttachment,
        diagnostics: &Diagnostics,
    ) -> Result<String> {
        let token = github::load_token(diagnostics)?;
        let id = github::load_repository_id(owner, &repository.name, diagnostics)?;
        ensure!(
            repository.id == 0 || repository.id == id,
            "The saved repository name now belongs to a different repository. Select the repository again in Gatto."
        );
        Ok(github::upload(
            &token,
            id,
            image,
            diagnostics,
            &github::UploadControl::default(),
            |_, _| {},
        )?)
    }
}

fn stage_clipboard(
    settings: &AppSettings,
    clipboard: Option<ClipboardItem>,
) -> Result<ClipboardUpdate> {
    if settings.add_from_clipboard_project.is_none() && !settings.scratchpad_directory()?.exists() {
        crate::projects::open_scratchpad(settings.scratchpad_directory()?)?;
    }
    let folder = settings
        .add_from_clipboard_directory()
        .context("Choose a project to use for Add from Clipboard first.")?;
    let summary = crate::projects::summary(&folder)?;
    let clipboard = clipboard.context("Copy an image or an image/video file in Finder first.")?;
    let attachment = if let Some(image) = clipboard.entries().iter().find_map(|entry| match entry {
        ClipboardEntry::Image(image) => Some(image),
        _ => None,
    }) {
        StagedAttachment::from_clipboard(image)?
    } else if let Some(path) = clipboard.entries().iter().find_map(|entry| match entry {
        ClipboardEntry::ExternalPaths(paths) => paths.paths().first(),
        _ => None,
    }) {
        StagedAttachment::from_path(path.clone(), settings.video_upload_limit())?
    } else {
        bail!("Copy a PNG, JPEG, GIF, or WebP image, or an image/video file in Finder first.");
    };
    Ok(ClipboardUpdate {
        folder,
        project_id: summary.id,
        project_title: summary.title,
        attachment,
        original: None,
        copy_format: settings.image_copy_format(),
    })
}

fn upload_saved_item(
    settings: &AppSettings,
    item: &SavedClipboardItem,
    diagnostics: &Diagnostics,
    backend: &dyn Backend,
) -> Result<ClipboardUpdate> {
    let folder = item.directory(settings)?;
    let (project, attachments) = Project::open(folder.clone())?;
    ensure!(
        project.id == item.project_id,
        "The original project is no longer available."
    );
    let original = attachments
        .into_iter()
        .find(|attachment| attachment.persistent_id == item.attachment_id)
        .context("The attachment is no longer in that project.")?;
    let mut attachment = original.clone();
    if !attachment.upload_state.is_uploaded() {
        let owner = settings
            .owner
            .as_deref()
            .filter(|name| !name.trim().is_empty())
            .context("Set an account or organization in App Preferences before uploading.")?;
        let repository = project
            .metadata
            .repository_for_owner(Some(owner))
            .context("Select a repository in this project before uploading.")?;
        attachment.validate_upload_size(settings.video_upload_limit())?;
        let url = backend.upload(owner, repository, &attachment, diagnostics)?;
        attachment.upload_state = AttachmentUploadState::Uploaded { url };
    }
    Ok(ClipboardUpdate {
        folder,
        project_id: project.id,
        project_title: project.title,
        attachment,
        original: Some(original),
        copy_format: settings.image_copy_format(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::{Image, ImageFormat, TestAppContext};
    use std::sync::Mutex;

    #[derive(Default)]
    struct FakeBackend {
        calls: Mutex<Vec<(String, String)>>,
        fail: bool,
    }
    impl Backend for FakeBackend {
        fn upload(
            &self,
            owner: &str,
            repository: &Repository,
            _: &StagedAttachment,
            _: &Diagnostics,
        ) -> Result<String> {
            self.calls
                .lock()
                .unwrap()
                .push((owner.into(), repository.name.clone()));
            if self.fail {
                bail!("Upload rejected");
            }
            Ok("https://github.com/user-attachments/assets/test".into())
        }
    }
    struct Fixture {
        _directory: tempfile::TempDir,
        settings: AppSettings,
    }
    impl Fixture {
        fn new() -> Self {
            let directory = tempfile::tempdir().unwrap();
            let projects = directory.path().join("projects");
            let (mut scratchpad, _) =
                crate::projects::open_scratchpad(projects.join("Scratchpad")).unwrap();
            scratchpad.metadata.repository_owner = Some("acme".into());
            scratchpad.metadata.repository = Some(Repository {
                id: 42,
                name: "images".into(),
            });
            scratchpad.save(&[]).unwrap();
            Self {
                _directory: directory,
                settings: AppSettings {
                    owner: Some("acme".into()),
                    project_directory_override: Some(projects),
                    ..Default::default()
                },
            }
        }
    }
    fn image() -> ClipboardItem {
        ClipboardItem::new_image(&Image::from_bytes(
            ImageFormat::Png,
            include_bytes!("../packaging/app-cat-pixel.png").to_vec(),
        ))
    }
    fn store() -> Store {
        Rc::new(|update, cx| cx.background_spawn(async move { update.persist() }))
    }
    fn start(
        service: &AddFromClipboard,
        cx: &mut TestAppContext,
        settings: AppSettings,
        backend: Arc<FakeBackend>,
        work: Work,
    ) {
        cx.update(|cx| {
            cx.set_app_identity("com.jeremy-chandler.gatto", "Gatto");
            service.run_with(cx, work, move || Ok(settings), backend, None, store());
        });
    }

    #[gpui_kit::test]
    async fn defaults_to_saving_without_upload_or_clipboard_changes_and_always_notifies(
        cx: &mut TestAppContext,
    ) {
        let fixture = Fixture::new();
        let mut settings = fixture.settings.clone();
        settings.owner = None;
        let (mut project, _) =
            Project::open(settings.add_from_clipboard_directory().unwrap()).unwrap();
        project.metadata.repository = None;
        project.save(&[]).unwrap();
        assert!(settings.add_from_clipboard_available());
        let backend = Arc::new(FakeBackend::default());
        let service = AddFromClipboard::new(Diagnostics::new(), Default::default());
        cx.update(|cx| cx.write_to_clipboard(image()));
        start(&service, cx, settings, backend.clone(), Work::Clipboard);
        cx.run_until_parked();
        let (_, items) = Project::open(project.folder).unwrap();
        assert_eq!(items.len(), 1);
        assert!(!items[0].upload_state.is_uploaded());
        assert!(backend.calls.lock().unwrap().is_empty());
        cx.update(|cx| {
            assert!(cx.windows().is_empty());
            assert!(matches!(
                cx.read_from_clipboard().unwrap().entries()[0],
                ClipboardEntry::Image(_)
            ));
        });
        let shown = cx.shown_system_notifications();
        let notification = shown.last().unwrap();
        assert_eq!(notification.title.as_ref(), "Added from Clipboard");
        assert!(notification.body.contains("Scratchpad"));
        assert_eq!(notification.actions[0].label.as_ref(), "Upload and copy");
        assert!(!service.is_busy());
    }

    #[gpui_kit::test]
    async fn opted_in_upload_saves_as_uploaded_then_copies_without_windows(
        cx: &mut TestAppContext,
    ) {
        let fixture = Fixture::new();
        let mut settings = fixture.settings.clone();
        settings.automatically_copy_add_from_clipboard = true;
        settings.copy_images_as_markdown = true;
        let backend = Arc::new(FakeBackend::default());
        let service = AddFromClipboard::new(Diagnostics::new(), Default::default());
        cx.update(|cx| cx.write_to_clipboard(image()));
        start(&service, cx, settings, backend.clone(), Work::Clipboard);
        cx.run_until_parked();
        let (_, items) =
            Project::open(fixture.settings.add_from_clipboard_directory().unwrap()).unwrap();
        assert_eq!(items.len(), 1);
        assert!(items[0].upload_state.is_uploaded());
        assert_eq!(
            *backend.calls.lock().unwrap(),
            vec![("acme".into(), "images".into())]
        );
        cx.update(|cx| {
            assert!(cx.windows().is_empty());
            assert!(
                cx.read_from_clipboard()
                    .unwrap()
                    .text()
                    .unwrap()
                    .starts_with("![")
            );
            assert!(
                cx.read_from_clipboard()
                    .unwrap()
                    .text()
                    .unwrap()
                    .contains("https://github.com/user-attachments/assets/test")
            );
        });
        let shown = cx.shown_system_notifications();
        assert!(shown.last().unwrap().actions.is_empty());
        assert_eq!(
            shown.last().unwrap().title.as_ref(),
            "Add from Clipboard complete"
        );
    }

    #[gpui_kit::test]
    async fn notification_uploads_its_saved_item_after_clipboard_and_project_selection_change(
        cx: &mut TestAppContext,
    ) {
        let fixture = Fixture::new();
        let backend = Arc::new(FakeBackend::default());
        let service = AddFromClipboard::new(Diagnostics::new(), Default::default());
        cx.update(|cx| cx.write_to_clipboard(image()));
        start(
            &service,
            cx,
            fixture.settings.clone(),
            backend.clone(),
            Work::Clipboard,
        );
        cx.run_until_parked();
        let shown = cx.shown_system_notifications();
        let response = SystemNotificationResponse {
            tag: shown.last().unwrap().tag.clone(),
            action_id: Some(UPLOAD_AND_COPY.into()),
        };
        let mut settings = fixture.settings.clone();
        settings.add_from_clipboard_project = Some("Missing other project".into());
        let handler = service.clone();
        let handler_backend = backend.clone();
        cx.update(|cx| {
            cx.write_to_clipboard(ClipboardItem::new_string("new clipboard text".into()));
            cx.on_system_notification_response(move |response, cx| {
                if let Some(item) = SavedClipboardItem::from_notification(&response) {
                    let settings = settings.clone();
                    handler.run_with(
                        cx,
                        Work::UploadSaved(item),
                        move || Ok(settings),
                        handler_backend.clone(),
                        None,
                        store(),
                    );
                }
            });
        });
        cx.simulate_system_notification_response(response.clone());
        cx.run_until_parked();
        let (_, items) =
            Project::open(fixture.settings.add_from_clipboard_directory().unwrap()).unwrap();
        assert_eq!(items.len(), 1);
        assert!(items[0].upload_state.is_uploaded());
        assert_eq!(backend.calls.lock().unwrap().len(), 1);
        // Clicking an old notification again reuses the saved URL, not another upload.
        cx.simulate_system_notification_response(response);
        cx.run_until_parked();
        assert_eq!(backend.calls.lock().unwrap().len(), 1);
        cx.update(|cx| assert!(cx.windows().is_empty()));
    }

    #[gpui_kit::test]
    async fn failed_automatic_upload_keeps_the_saved_draft_and_notifies_with_retry(
        cx: &mut TestAppContext,
    ) {
        let fixture = Fixture::new();
        let mut settings = fixture.settings.clone();
        settings.automatically_copy_add_from_clipboard = true;
        let backend = Arc::new(FakeBackend {
            fail: true,
            ..Default::default()
        });
        let service = AddFromClipboard::new(Diagnostics::new(), Default::default());
        cx.update(|cx| cx.write_to_clipboard(image()));
        start(&service, cx, settings, backend, Work::Clipboard);
        cx.run_until_parked();
        let (_, items) =
            Project::open(fixture.settings.add_from_clipboard_directory().unwrap()).unwrap();
        assert_eq!(items.len(), 1);
        assert!(!items[0].upload_state.is_uploaded());
        let shown = cx.shown_system_notifications();
        assert_eq!(
            shown.last().unwrap().title.as_ref(),
            "Add from Clipboard failed"
        );
        assert_eq!(
            shown.last().unwrap().actions[0].id.as_ref(),
            UPLOAD_AND_COPY
        );
        cx.update(|cx| assert!(cx.read_from_clipboard().unwrap().text().is_none()));
    }

    #[test]
    fn notification_identity_survives_restart_and_rejects_invalid_actions_and_paths() {
        let fixture = Fixture::new();
        let update = stage_clipboard(&fixture.settings, Some(image())).unwrap();
        let receipt = update.persist().unwrap();
        let mut response = SystemNotificationResponse {
            tag: receipt.item.notification_tag().into(),
            action_id: Some(UPLOAD_AND_COPY.into()),
        };
        assert_eq!(
            SavedClipboardItem::from_notification(&response),
            Some(receipt.item.clone())
        );
        response.action_id = None;
        assert!(SavedClipboardItem::from_notification(&response).is_none());
        response.action_id = Some("unknown".into());
        assert!(SavedClipboardItem::from_notification(&response).is_none());
        let mut invalid = receipt.item;
        invalid.folder = "../outside".into();
        assert!(invalid.directory(&fixture.settings).is_err());
    }

    #[test]
    fn notification_cannot_upload_removed_items_or_replaced_projects() {
        let fixture = Fixture::new();
        let receipt = stage_clipboard(&fixture.settings, Some(image()))
            .unwrap()
            .persist()
            .unwrap();
        let backend = FakeBackend::default();
        let mut wrong_project = receipt.item.clone();
        wrong_project.project_id = uuid::Uuid::new_v4();
        assert!(
            upload_saved_item(
                &fixture.settings,
                &wrong_project,
                &Diagnostics::new(),
                &backend
            )
            .is_err()
        );
        let (mut project, _) =
            Project::open(fixture.settings.add_from_clipboard_directory().unwrap()).unwrap();
        project.save(&[]).unwrap();
        assert!(
            upload_saved_item(
                &fixture.settings,
                &receipt.item,
                &Diagnostics::new(),
                &backend
            )
            .is_err()
        );
        assert!(backend.calls.lock().unwrap().is_empty());
    }

    #[test]
    fn upload_commit_keeps_newer_edits_instead_of_marking_them_uploaded() {
        let fixture = Fixture::new();
        let receipt = stage_clipboard(&fixture.settings, Some(image()))
            .unwrap()
            .persist()
            .unwrap();
        let update = upload_saved_item(
            &fixture.settings,
            &receipt.item,
            &Diagnostics::new(),
            &FakeBackend::default(),
        )
        .unwrap();
        let (mut project, items) =
            Project::open(fixture.settings.add_from_clipboard_directory().unwrap()).unwrap();
        let mut edited = StagedAttachment::from_bytes(
            "edited.png".into(),
            include_bytes!("../packaging/menu-bar-icon.png").to_vec(),
            None,
        )
        .unwrap();
        edited.persistent_id = items[0].persistent_id;
        project.save(&[edited]).unwrap();
        assert!(update.persist().is_err());
        let (_, items) = Project::open(project.folder).unwrap();
        assert_eq!(items[0].name, "edited.png");
        assert!(!items[0].upload_state.is_uploaded());
    }

    #[gpui_kit::test]
    async fn duplicate_requests_are_rejected_and_each_attempt_notifies(cx: &mut TestAppContext) {
        let fixture = Fixture::new();
        let backend = Arc::new(FakeBackend::default());
        let service = AddFromClipboard::new(Diagnostics::new(), Default::default());
        cx.update(|cx| cx.write_to_clipboard(image()));
        start(
            &service,
            cx,
            fixture.settings.clone(),
            backend.clone(),
            Work::Clipboard,
        );
        start(
            &service,
            cx,
            fixture.settings.clone(),
            backend,
            Work::Clipboard,
        );
        cx.run_until_parked();
        assert_eq!(
            Project::open(fixture.settings.add_from_clipboard_directory().unwrap())
                .unwrap()
                .1
                .len(),
            1
        );
        assert_eq!(cx.shown_system_notifications().len(), 2);
    }

    #[gpui_kit::test]
    async fn save_barrier_failure_prevents_staging_and_upload(cx: &mut TestAppContext) {
        let fixture = Fixture::new();
        let backend = Arc::new(FakeBackend::default());
        let service = AddFromClipboard::new(Diagnostics::new(), Default::default());
        let (done, saved) = async_channel::bounded(1);
        done.try_send(Err("Could not save the project".into()))
            .unwrap();
        let settings = fixture.settings.clone();
        cx.update(|cx| {
            cx.set_app_identity("com.jeremy-chandler.gatto", "Gatto");
            cx.write_to_clipboard(image());
            service.run_with(
                cx,
                Work::Clipboard,
                move || Ok(settings),
                backend.clone(),
                Some(saved),
                store(),
            );
        });
        cx.run_until_parked();
        assert!(backend.calls.lock().unwrap().is_empty());
        assert!(
            Project::open(fixture.settings.add_from_clipboard_directory().unwrap())
                .unwrap()
                .1
                .is_empty()
        );
        assert_eq!(
            cx.shown_system_notifications()
                .last()
                .unwrap()
                .title
                .as_ref(),
            "Add from Clipboard failed"
        );
    }
}

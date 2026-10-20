//! Window-independent clipboard uploads. Only clipboard and notification effects
//! run on the UI thread; validation, settings, CLI, and HTTP work run in the background.
use std::{cell::Cell, rc::Rc, sync::Arc};

use anyhow::{Context as _, Result, bail};
use gpui_kit::{App, AppContext as _, ClipboardEntry, ClipboardItem, SystemNotification};

use crate::{diagnostics::Diagnostics, github, model::StagedAttachment, settings::AppSettings};

#[derive(Clone)]
pub struct QuickCopy {
    busy: Rc<Cell<bool>>,
    diagnostics: Diagnostics,
}

impl QuickCopy {
    pub fn new(diagnostics: Diagnostics) -> Self {
        Self {
            busy: Rc::new(Cell::new(false)),
            diagnostics,
        }
    }

    pub fn is_busy(&self) -> bool {
        self.busy.get()
    }

    pub fn start(&self, cx: &mut App) {
        if crate::updater::Updater::shared(cx).read(cx).state
            == crate::updater::UpdateState::Installing
        {
            notify(
                cx,
                "Updating Gatto",
                "Try Quick Copy again after Gatto restarts.",
            );
            return;
        }
        self.start_with(cx, AppSettings::load, Arc::new(GithubBackend));
    }

    fn start_with(
        &self,
        cx: &mut App,
        settings: impl FnOnce() -> Result<AppSettings> + Send + 'static,
        backend: Arc<dyn Backend>,
    ) {
        if self.busy.replace(true) {
            notify(
                cx,
                "Quick Copy in progress",
                "Wait for the current upload to finish.",
            );
            return;
        }
        let clipboard = cx.read_from_clipboard();
        let diagnostics = self.diagnostics.clone();
        let task = cx.background_spawn(async move {
            upload_clipboard(&settings()?, clipboard, &diagnostics, backend.as_ref())
        });
        let controller = self.clone();
        cx.spawn(async move |cx| {
            let result = task.await;
            cx.update(|cx| {
                controller.busy.set(false);
                match result {
                    Ok(markdown) => {
                        cx.write_to_clipboard(ClipboardItem::new_string(markdown));
                        controller.diagnostics.info(
                            "Quick Copy uploaded the clipboard attachment and copied Markdown.",
                        );
                        notify(
                            cx,
                            "Quick Copy complete",
                            "Markdown copied to the clipboard.",
                        );
                    }
                    Err(error) => {
                        controller
                            .diagnostics
                            .error(format!("Quick Copy failed: {error}"));
                        notify(cx, "Quick Copy failed", &error.to_string());
                    }
                }
            });
        })
        .detach();
    }
}

fn notify(cx: &mut App, title: &str, body: &str) {
    cx.show_system_notification(SystemNotification {
        tag: "gatto-quick-copy".into(),
        title: title.to_owned().into(),
        body: body.to_owned().into(),
        actions: Vec::new(),
    });
}

// This boundary keeps GitHub credentials and network effects out of regression
// checks while exercising the actual background job and clipboard delivery.
trait Backend: Send + Sync {
    fn upload(
        &self,
        owner: &str,
        repository: &str,
        image: &StagedAttachment,
        diagnostics: &Diagnostics,
    ) -> Result<String>;
}

struct GithubBackend;
impl Backend for GithubBackend {
    fn upload(
        &self,
        owner: &str,
        repository: &str,
        image: &StagedAttachment,
        diagnostics: &Diagnostics,
    ) -> Result<String> {
        let token = github::load_token(diagnostics)?;
        let id = github::load_repository_id(owner, repository, diagnostics)?;
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

fn upload_clipboard(
    settings: &AppSettings,
    clipboard: Option<ClipboardItem>,
    diagnostics: &Diagnostics,
    backend: &dyn Backend,
) -> Result<String> {
    let repository = settings
        .last_repository
        .as_ref()
        .filter(|name| settings.pinned_repositories.contains(*name))
        .or_else(|| settings.pinned_repositories.first())
        .context("Quick Copy needs a pinned repository. Pin one in App Preferences first.")?;
    let owner = settings
        .owner
        .as_deref()
        .filter(|name| !name.trim().is_empty())
        .context("Quick Copy needs an account or organization in App Preferences first.")?;
    let clipboard = clipboard
        .context("Copy a PNG, JPEG, GIF, or WebP image, or an image/video file in Finder first.")?;
    let image = if let Some(image) = clipboard.entries().iter().find_map(|entry| match entry {
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
    let url = backend.upload(owner, repository, &image, diagnostics)?;
    Ok(image.snippet(&url, settings.image_copy_format()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::{Image, ImageFormat, TestAppContext};
    use std::sync::Mutex;

    struct FakeBackend {
        calls: Mutex<Vec<(String, String)>>,
        fail: bool,
    }
    impl Backend for FakeBackend {
        fn upload(
            &self,
            owner: &str,
            repository: &str,
            _: &StagedAttachment,
            _: &Diagnostics,
        ) -> Result<String> {
            self.calls
                .lock()
                .unwrap()
                .push((owner.into(), repository.into()));
            if self.fail {
                bail!("Upload rejected");
            }
            Ok("https://github.com/user-attachments/assets/test-image".into())
        }
    }
    fn settings() -> AppSettings {
        AppSettings {
            owner: Some("acme".into()),
            pinned_repositories: ["alpha".into(), "beta".into()].into(),
            last_repository: Some("beta".into()),
            ..AppSettings::default()
        }
    }
    fn image() -> ClipboardItem {
        ClipboardItem::new_image(&Image::from_bytes(
            ImageFormat::Png,
            include_bytes!("../packaging/rocket-cat-transparent.png").to_vec(),
        ))
    }

    #[gpui_kit::test]
    async fn uploads_without_windows_and_delivers_markdown_and_notification(
        cx: &mut TestAppContext,
    ) {
        let backend = Arc::new(FakeBackend {
            calls: Mutex::new(Vec::new()),
            fail: false,
        });
        let service = QuickCopy::new(Diagnostics::new());
        cx.update(|cx| {
            cx.set_app_identity("com.jeremy-chandler.gatto", "Gatto");
            cx.write_to_clipboard(image());
            service.start_with(cx, || Ok(settings()), backend.clone());
            service.start_with(cx, || Ok(settings()), backend.clone());
            assert!(cx.windows().is_empty());
        });
        cx.run_until_parked();
        cx.update(|cx| {
            assert!(cx.windows().is_empty());
            let staged = StagedAttachment::from_path(
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("packaging/rocket-cat-transparent.png"),
                Default::default(),
            )
            .unwrap();
            let (width, height) = staged.image().unwrap().pixel_size;
            let copied = cx.read_from_clipboard().unwrap().text().unwrap();
            assert!(copied.starts_with(&format!(
                r#"<img width="{width}" height="{height}" alt="clipboard-"#
            )));
            assert!(
                copied
                    .ends_with(r#"src="https://github.com/user-attachments/assets/test-image" />"#)
            );
        });
        assert_eq!(
            *backend.calls.lock().unwrap(),
            vec![("acme".into(), "beta".into())]
        );
        assert_eq!(
            cx.shown_system_notifications()
                .last()
                .unwrap()
                .title
                .as_ref(),
            "Quick Copy complete"
        );
        assert!(!service.busy.get());
    }

    #[gpui_kit::test]
    async fn errors_preserve_clipboard_without_creating_windows(cx: &mut TestAppContext) {
        for (settings, clipboard, fail, message) in [
            (AppSettings::default(), image(), false, "pinned repository"),
            (
                settings(),
                ClipboardItem::new_string("keep this text".into()),
                false,
                "Copy a PNG",
            ),
            (
                AppSettings {
                    owner: None,
                    ..settings()
                },
                image(),
                false,
                "account or organization",
            ),
            (settings(), image(), true, "Upload rejected"),
        ] {
            let backend = Arc::new(FakeBackend {
                calls: Mutex::new(Vec::new()),
                fail,
            });
            let service = QuickCopy::new(Diagnostics::new());
            let before = clipboard.clone();
            cx.update(|cx| {
                cx.set_app_identity("com.jeremy-chandler.gatto", "Gatto");
                cx.write_to_clipboard(clipboard);
                service.start_with(cx, move || Ok(settings), backend.clone());
            });
            cx.run_until_parked();
            cx.update(|cx| {
                assert!(cx.windows().is_empty());
                let after = cx.read_from_clipboard().unwrap();
                assert_eq!(after.text(), before.text());
                if let ClipboardEntry::Image(before_image) = &before.entries()[0] {
                    let ClipboardEntry::Image(after_image) = &after.entries()[0] else {
                        panic!("clipboard image was replaced");
                    };
                    assert_eq!(after_image.bytes, before_image.bytes);
                }
            });
            let notifications = cx.shown_system_notifications();
            let notification = notifications.last().unwrap();
            assert_eq!(notification.title.as_ref(), "Quick Copy failed");
            assert!(notification.body.contains(message));
            if !fail {
                assert!(backend.calls.lock().unwrap().is_empty());
            }
            assert!(!service.busy.get());
        }
    }

    #[test]
    fn uploads_to_a_personal_accounts_pinned_repository() {
        let backend = FakeBackend {
            calls: Mutex::new(Vec::new()),
            fail: false,
        };
        let settings = AppSettings {
            owner: Some("octocat".into()),
            ..settings()
        };
        upload_clipboard(&settings, Some(image()), &Diagnostics::new(), &backend).unwrap();
        assert_eq!(
            *backend.calls.lock().unwrap(),
            vec![("octocat".into(), "beta".into())]
        );
    }

    #[test]
    fn ignores_unpinned_last_repository_and_uses_pinned_fallback() {
        let backend = FakeBackend {
            calls: Mutex::new(Vec::new()),
            fail: false,
        };
        let mut settings = settings();
        settings.last_repository = Some("unpinned".into());
        upload_clipboard(&settings, Some(image()), &Diagnostics::new(), &backend).unwrap();
        assert_eq!(
            *backend.calls.lock().unwrap(),
            vec![("acme".into(), "alpha".into())]
        );
    }

    #[test]
    fn large_finder_video_requires_saved_opt_in_before_quick_copy_uploads() {
        use std::io::Write as _;
        let mut file = tempfile::Builder::new().suffix(".mp4").tempfile().unwrap();
        let video = include_bytes!("../tests/fixtures/sample.mp4");
        file.write_all(video).unwrap();
        let padding = crate::model::MAX_ATTACHMENT_BYTES + 8;
        file.write_all(&(padding as u32).to_be_bytes()).unwrap();
        file.write_all(b"free").unwrap();
        file.as_file()
            .set_len(video.len() as u64 + padding)
            .unwrap();
        let clipboard = || {
            ClipboardItem::from(ClipboardEntry::ExternalPaths(gpui_kit::ExternalPaths(
                [file.path().to_owned()].into_iter().collect(),
            )))
        };
        let backend = FakeBackend {
            calls: Mutex::new(Vec::new()),
            fail: false,
        };
        let mut settings = settings();
        let error = upload_clipboard(&settings, Some(clipboard()), &Diagnostics::new(), &backend)
            .unwrap_err();
        assert!(error.to_string().contains("10 MB"));
        assert!(backend.calls.lock().unwrap().is_empty());
        settings.allow_large_videos = true;
        upload_clipboard(&settings, Some(clipboard()), &Diagnostics::new(), &backend).unwrap();
        assert_eq!(backend.calls.lock().unwrap().len(), 1);
    }

    #[test]
    fn image_copy_format_is_shared_by_quick_copy() {
        let backend = FakeBackend {
            calls: Mutex::new(Vec::new()),
            fail: false,
        };
        let mut settings = settings();
        for enabled in [true, false] {
            settings.copy_images_as_markdown = enabled;
            let copied =
                upload_clipboard(&settings, Some(image()), &Diagnostics::new(), &backend).unwrap();
            if enabled {
                assert!(copied.starts_with("![clipboard-"));
                assert!(
                    copied.ends_with("](https://github.com/user-attachments/assets/test-image)")
                );
                assert!(!copied.contains("width="));
            } else {
                assert!(copied.starts_with("<img width="));
            }
        }
    }

    #[test]
    fn copied_finder_video_uses_the_same_upload_flow_and_returns_a_bare_url() {
        let backend = FakeBackend {
            calls: Mutex::new(Vec::new()),
            fail: false,
        };
        let path =
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample.mp4");
        let clipboard = ClipboardItem::from(ClipboardEntry::ExternalPaths(
            gpui_kit::ExternalPaths([path].into_iter().collect()),
        ));
        let settings = AppSettings {
            copy_images_as_markdown: true,
            ..settings()
        };
        let copied =
            upload_clipboard(&settings, Some(clipboard), &Diagnostics::new(), &backend).unwrap();
        assert_eq!(
            copied,
            "https://github.com/user-attachments/assets/test-image"
        );
        assert_eq!(backend.calls.lock().unwrap().len(), 1);
    }
}

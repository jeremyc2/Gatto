use std::{path::PathBuf, time::Duration};

use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Icon, IconName, Sizable as _, StyledExt as _,
    button::{Button, ButtonVariants as _},
    checkbox::Checkbox,
    h_flex,
    input::{Input, InputState},
    kbd::Kbd,
    spinner::Spinner,
    v_flex,
};
use gpui_kit::{
    App, AppContext as _, ClipboardEntry, ClipboardItem, Context, Entity, ExternalPaths,
    FocusHandle, Focusable, InteractiveElement as _, IntoElement, KeyBinding, Keystroke, ObjectFit,
    ParentElement as _, PathPromptOptions, Render, SharedString, StatefulInteractiveElement as _,
    Styled as _, StyledImage as _, Window, actions, div, img, prelude::FluentBuilder as _, px,
};

use crate::{
    github,
    model::{Repository, StagedImage, parse_repository},
    settings::AppSettings,
};

actions!(
    github_image_upload,
    [PasteImage, CopyUploadUrl, CopyUploadMarkdown]
);

const KEY_CONTEXT: &str = "GitHubImageUpload";
const UNSUPPORTED_MESSAGE: &str =
    "Unsupported format. Please paste or drop a PNG, JPEG, GIF, or WebP image.";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LoadState {
    NeedsSetup,
    Loading,
    Ready,
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StatusKind {
    Info,
    Success,
    Warning,
    Error,
}

struct InlineStatus {
    kind: StatusKind,
    message: SharedString,
}

#[derive(Clone)]
struct UploadResult {
    url: String,
    markdown: String,
}

pub struct UploaderApp {
    repository_input: Entity<InputState>,
    repository: Option<Repository>,
    token: Option<String>,
    load_state: LoadState,
    staged_image: Option<StagedImage>,
    upload_result: Option<UploadResult>,
    uploading: bool,
    status: InlineStatus,
    settings: AppSettings,
    settings_status: Option<InlineStatus>,
    settings_open: bool,
    focus_handle: FocusHandle,
}

impl UploaderApp {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let (settings, settings_status) = match AppSettings::load() {
            Ok(settings) => (settings, None),
            Err(error) => (
                AppSettings::default(),
                Some(InlineStatus {
                    kind: StatusKind::Error,
                    message: format!("Settings could not be loaded: {error}").into(),
                }),
            ),
        };
        let initial_repository = settings.repository.clone().unwrap_or_default();
        let repository_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("owner/name")
                .default_value(initial_repository)
        });

        let mut this = Self {
            repository_input,
            repository: None,
            token: None,
            load_state: LoadState::NeedsSetup,
            staged_image: None,
            upload_result: None,
            uploading: false,
            status: InlineStatus {
                kind: StatusKind::Info,
                message: "".into(),
            },
            settings,
            settings_status,
            settings_open: false,
            focus_handle: cx.focus_handle(),
        };
        this.load_repository(window, cx);
        this
    }

    pub fn show_settings(&mut self, cx: &mut Context<Self>) {
        self.settings_open = true;
        cx.notify();
    }

    pub fn report_menu_bar_error(
        &mut self,
        error: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) {
        self.status = InlineStatus {
            kind: StatusKind::Error,
            message: error.into(),
        };
        cx.notify();
    }

    fn load_repository(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.token = None;
        self.repository = None;
        let Some(full_name) = self.settings.repository.clone() else {
            self.load_state = LoadState::NeedsSetup;
            self.status = InlineStatus {
                kind: StatusKind::Info,
                message: "Set a repository in Settings to get started.".into(),
            };
            cx.notify();
            return;
        };

        self.load_state = LoadState::Loading;
        self.status = InlineStatus {
            kind: StatusKind::Info,
            message: format!("Connecting to {full_name}…").into(),
        };
        cx.notify();

        cx.spawn_in(window, async move |this, window| {
            let requested = full_name.clone();
            let result = window
                .background_spawn(async move { github::load_session(&full_name) })
                .await;
            let _ = this.update_in(window, move |this, _, cx| {
                if this.settings.repository.as_deref() != Some(requested.as_str()) {
                    return;
                }
                match result {
                    Ok(session) => {
                        this.repository = Some(session.repository);
                        this.token = Some(session.token);
                        this.load_state = LoadState::Ready;
                        this.status = InlineStatus {
                            kind: StatusKind::Info,
                            message: "Choose, paste, or drop an image to continue.".into(),
                        };
                    }
                    Err(error) => {
                        this.load_state = LoadState::Failed;
                        this.status = InlineStatus {
                            kind: StatusKind::Error,
                            message: error.to_string().into(),
                        };
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn choose_file(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let prompt = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Choose an image".into()),
        });

        cx.spawn_in(window, async move |this, window| {
            let Some(path) = prompt
                .await
                .ok()
                .and_then(Result::ok)
                .flatten()
                .and_then(|paths| paths.into_iter().next())
            else {
                return;
            };
            let result = window
                .background_spawn(async move { StagedImage::from_path(path) })
                .await;
            let _ = this.update_in(window, |this, window, cx| {
                this.accept_image_result(result, window, cx);
            });
        })
        .detach();
    }

    fn on_paste(&mut self, _: &PasteImage, window: &mut Window, cx: &mut Context<Self>) {
        if self.load_state == LoadState::NeedsSetup {
            return;
        }
        let Some(clipboard) = cx.read_from_clipboard() else {
            self.warn(UNSUPPORTED_MESSAGE, window, cx);
            return;
        };

        if let Some(image) = clipboard.entries().iter().find_map(|entry| match entry {
            ClipboardEntry::Image(image) => Some(image.clone()),
            _ => None,
        }) {
            let task = cx.background_spawn(async move { StagedImage::from_clipboard(&image) });
            cx.spawn_in(window, async move |this, window| {
                let result = task.await;
                let _ = this.update_in(window, |this, window, cx| {
                    this.accept_image_result(result, window, cx);
                });
            })
            .detach();
            return;
        }

        if let Some(path) = clipboard.entries().iter().find_map(|entry| match entry {
            ClipboardEntry::ExternalPaths(paths) => paths.paths().first().cloned(),
            _ => None,
        }) {
            Self::load_dropped_path(cx.entity().downgrade(), path, window, cx);
            return;
        }

        self.warn(UNSUPPORTED_MESSAGE, window, cx);
    }

    fn load_dropped_path(
        view: gpui_kit::WeakEntity<Self>,
        path: PathBuf,
        window: &mut Window,
        cx: &mut App,
    ) {
        let task = cx.background_spawn(async move { StagedImage::from_path(path) });
        window
            .spawn(cx, async move |window| {
                let result = task.await;
                let _ = view.update_in(window, |this, window, cx| {
                    this.accept_image_result(result, window, cx);
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
        match result {
            Ok(image) => {
                self.status = InlineStatus {
                    kind: StatusKind::Info,
                    message: format!("{} is ready to upload.", image.name).into(),
                };
                self.staged_image = Some(image);
                self.upload_result = None;
            }
            Err(error) => {
                self.warn(error.to_string(), window, cx);
                return;
            }
        }
        cx.notify();
    }

    fn warn(
        &mut self,
        message: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.status = InlineStatus {
            kind: StatusKind::Warning,
            message: message.into(),
        };
        cx.notify();

        let expected = self.status.message.clone();
        cx.spawn_in(window, async move |this, window| {
            window
                .background_executor()
                .timer(Duration::from_secs(4))
                .await;
            let _ = this.update_in(window, move |this, _, cx| {
                if this.status.kind == StatusKind::Warning && this.status.message == expected {
                    this.status = InlineStatus {
                        kind: StatusKind::Info,
                        message: "Choose, paste, or drop an image to continue.".into(),
                    };
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn send(&mut self, _: &gpui_kit::ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.uploading {
            return;
        }
        let (Some(token), Some(repository), Some(image)) = (
            self.token.clone(),
            self.repository.clone(),
            self.staged_image.clone(),
        ) else {
            return;
        };

        self.uploading = true;
        self.upload_result = None;
        self.status = InlineStatus {
            kind: StatusKind::Info,
            message: format!("Uploading {} to {}…", image.name, repository.full_name).into(),
        };
        let uploaded_image_id = image.preview.id();
        let image_name = image.name.clone();
        cx.notify();

        cx.spawn_in(window, async move |this, window| {
            let result = window
                .background_spawn(async move { github::upload(&token, repository.id, &image) })
                .await;
            let _ = this.update_in(window, move |this, _, cx| {
                this.uploading = false;
                match result {
                    Ok(url) => {
                        if this
                            .staged_image
                            .as_ref()
                            .is_some_and(|staged| staged.preview.id() == uploaded_image_id)
                        {
                            this.staged_image = None;
                        }
                        this.upload_result = Some(UploadResult {
                            markdown: markdown_image(&image_name, &url),
                            url,
                        });
                        this.status = InlineStatus {
                            kind: StatusKind::Success,
                            message: "Upload complete. Copy the URL or Markdown below.".into(),
                        };
                    }
                    Err(error) => {
                        this.status = InlineStatus {
                            kind: StatusKind::Error,
                            message: error.to_string().into(),
                        };
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn copy_upload_url(&mut self, _: &CopyUploadUrl, _: &mut Window, cx: &mut Context<Self>) {
        let Some(result) = &self.upload_result else {
            return;
        };
        cx.write_to_clipboard(ClipboardItem::new_string(result.url.clone()));
        self.status = InlineStatus {
            kind: StatusKind::Success,
            message: "URL copied to the clipboard.".into(),
        };
        cx.notify();
    }

    fn copy_upload_markdown(
        &mut self,
        _: &CopyUploadMarkdown,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(result) = &self.upload_result else {
            return;
        };
        cx.write_to_clipboard(ClipboardItem::new_string(result.markdown.clone()));
        self.status = InlineStatus {
            kind: StatusKind::Success,
            message: "Markdown image snippet copied to the clipboard.".into(),
        };
        cx.notify();
    }

    fn save_repository(
        &mut self,
        _: &gpui_kit::ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let value = self.repository_input.read(cx).value();
        let repository = match parse_repository(&value) {
            Ok(repository) => repository,
            Err(error) => {
                self.settings_status = Some(InlineStatus {
                    kind: StatusKind::Error,
                    message: error.to_string().into(),
                });
                cx.notify();
                return;
            }
        };

        let previous = self.settings.repository.replace(repository.clone());
        if let Err(error) = self.settings.save() {
            self.settings.repository = previous;
            self.settings_status = Some(InlineStatus {
                kind: StatusKind::Error,
                message: format!("Could not save settings: {error}").into(),
            });
            cx.notify();
            return;
        }

        self.settings_status = Some(InlineStatus {
            kind: StatusKind::Success,
            message: format!("Repository set to {repository}.").into(),
        });
        self.load_repository(window, cx);
    }

    fn set_start_at_login(&mut self, enabled: bool, cx: &mut Context<Self>) {
        match self.settings.set_start_at_login(enabled) {
            Ok(()) => {
                self.settings_status = Some(InlineStatus {
                    kind: StatusKind::Success,
                    message: if enabled {
                        "The app will start when you log in.".into()
                    } else {
                        "Start at Login is off.".into()
                    },
                });
            }
            Err(error) => {
                self.settings_status = Some(InlineStatus {
                    kind: StatusKind::Error,
                    message: format!("Could not update Start at Login: {error}").into(),
                });
            }
        }
        cx.notify();
    }

    fn render_repository_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let name = self.settings.repository.clone().unwrap_or_default();
        h_flex()
            .items_center()
            .justify_between()
            .gap_3()
            .child(
                v_flex()
                    .gap_1()
                    .child(div().text_sm().font_semibold().child("Repository"))
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(name),
                    ),
            )
            .when(self.load_state == LoadState::Loading, |row| {
                row.child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(Spinner::new().xsmall())
                        .child("Connecting"),
                )
            })
            .when(self.load_state == LoadState::Failed, |row| {
                row.child(
                    Button::new("retry-repository")
                        .small()
                        .outline()
                        .icon(IconName::RefreshCw)
                        .label("Retry")
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.load_repository(window, cx);
                        })),
                )
            })
    }

    fn render_setup_prompt(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .items_center()
            .justify_center()
            .rounded_xl()
            .border_2()
            .border_dashed()
            .border_color(cx.theme().border)
            .bg(cx.theme().muted.opacity(0.22))
            .p_6()
            .child(Icon::new(IconName::Settings).size(px(30.)))
            .child(div().text_sm().font_semibold().child("Choose a repository"))
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child("Images are uploaded through a repository you choose in Settings."),
            )
            .child(
                Button::new("setup-open-settings")
                    .primary()
                    .small()
                    .label("Open Settings")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.show_settings(cx);
                    })),
            )
    }

    fn render_drop_zone(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity().downgrade();
        let has_image = self.staged_image.is_some();
        div()
            .id("image-drop-zone")
            .w_full()
            .h(px(150.))
            .rounded_xl()
            .border_2()
            .border_dashed()
            .border_color(if self.status.kind == StatusKind::Warning {
                cx.theme().warning
            } else if has_image {
                cx.theme().primary.opacity(0.7)
            } else {
                cx.theme().border
            })
            .bg(if has_image {
                cx.theme().primary.opacity(0.055)
            } else {
                cx.theme().muted.opacity(0.22)
            })
            .cursor_pointer()
            .hover(|style| style.bg(cx.theme().muted.opacity(0.42)))
            .drag_over::<ExternalPaths>(|style, _, _, cx| {
                style
                    .border_color(cx.theme().primary)
                    .bg(cx.theme().primary.opacity(0.1))
            })
            .can_drop(|value, _, _| value.downcast_ref::<ExternalPaths>().is_some())
            .on_drop(move |paths: &ExternalPaths, window, cx| {
                if let Some(path) = paths.paths().first().cloned() {
                    Self::load_dropped_path(view.clone(), path, window, cx);
                }
            })
            .on_click(cx.listener(|this, _, window, cx| {
                this.choose_file(window, cx);
            }))
            .flex()
            .items_center()
            .justify_center()
            .child(
                v_flex()
                    .items_center()
                    .gap_3()
                    .child(
                        Icon::new(if has_image {
                            IconName::Frame
                        } else {
                            IconName::Plus
                        })
                        .size(px(30.))
                        .text_color(if has_image {
                            cx.theme().primary
                        } else {
                            cx.theme().muted_foreground
                        }),
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            .items_center()
                            .text_sm()
                            .child("Press")
                            .child(Kbd::new(Keystroke::parse("cmd-v").expect("valid shortcut")))
                            .child("or drop an image here"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("or click to select a PNG, JPEG, GIF, or WebP file"),
                    ),
            )
    }

    fn render_preview(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let preview = self.staged_image.as_ref().map(|image| {
            let source = image.preview.clone();
            let title = image.name.clone();
            let detail = format!("{} · {}", image.formatted_size(), image.mime_type);
            v_flex()
                .gap_2()
                .child(div().text_sm().font_semibold().child("Preview"))
                .child(
                    div()
                        .w_full()
                        .h(px(245.))
                        .rounded_xl()
                        .border_1()
                        .border_color(cx.theme().border)
                        .bg(cx.theme().muted.opacity(0.2))
                        .overflow_hidden()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(img(source).size_full().object_fit(ObjectFit::Contain)),
                )
                .child(
                    h_flex()
                        .justify_between()
                        .text_xs()
                        .child(div().font_semibold().child(title))
                        .child(div().text_color(cx.theme().muted_foreground).child(detail)),
                )
                .into_any_element()
        });

        v_flex().when_some(preview, |container, preview| container.child(preview))
    }

    fn render_inline_status(
        &self,
        status: &InlineStatus,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let color = match status.kind {
            StatusKind::Info => cx.theme().muted_foreground,
            StatusKind::Success => cx.theme().success,
            StatusKind::Warning => cx.theme().warning,
            StatusKind::Error => cx.theme().danger,
        };
        let icon = match status.kind {
            StatusKind::Info => IconName::Info,
            StatusKind::Success => IconName::CircleCheck,
            StatusKind::Warning | StatusKind::Error => IconName::TriangleAlert,
        };
        h_flex()
            .w_full()
            .min_h(px(42.))
            .gap_2()
            .items_center()
            .rounded_lg()
            .border_1()
            .border_color(color.opacity(0.25))
            .bg(color.opacity(0.08))
            .px_3()
            .py_2()
            .text_xs()
            .text_color(color)
            .child(Icon::new(icon).small())
            .child(div().flex_1().child(status.message.clone()))
    }

    fn render_status(&self, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_inline_status(&self.status, cx)
    }

    fn render_upload_result(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let result = self.upload_result.as_ref().map(|result| {
            v_flex()
                .gap_3()
                .rounded_lg()
                .border_1()
                .border_color(cx.theme().success.opacity(0.35))
                .bg(cx.theme().success.opacity(0.07))
                .p_3()
                .child(div().text_sm().font_semibold().child("Uploaded image"))
                .child(
                    v_flex()
                        .gap_1()
                        .child(
                            div()
                                .text_xs()
                                .font_semibold()
                                .text_color(cx.theme().muted_foreground)
                                .child("URL"),
                        )
                        .child(
                            h_flex()
                                .gap_2()
                                .items_center()
                                .child(
                                    div()
                                        .flex_1()
                                        .overflow_hidden()
                                        .rounded_md()
                                        .border_1()
                                        .border_color(cx.theme().border)
                                        .bg(cx.theme().background)
                                        .px_3()
                                        .py_2()
                                        .text_xs()
                                        .child(result.url.clone()),
                                )
                                .child(
                                    Button::new("copy-upload-url")
                                        .small()
                                        .outline()
                                        .label("Copy URL")
                                        .tooltip("Copy URL (Command+Shift+C)")
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.copy_upload_url(&CopyUploadUrl, window, cx);
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
                                .child("Markdown image"),
                        )
                        .child(
                            h_flex()
                                .gap_2()
                                .items_center()
                                .child(
                                    div()
                                        .flex_1()
                                        .overflow_hidden()
                                        .rounded_md()
                                        .border_1()
                                        .border_color(cx.theme().border)
                                        .bg(cx.theme().background)
                                        .px_3()
                                        .py_2()
                                        .text_xs()
                                        .child(result.markdown.clone()),
                                )
                                .child(
                                    Button::new("copy-upload-markdown")
                                        .small()
                                        .primary()
                                        .label("Copy Markdown")
                                        .tooltip("Copy Markdown (Command+Shift+M)")
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.copy_upload_markdown(
                                                &CopyUploadMarkdown,
                                                window,
                                                cx,
                                            );
                                        })),
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
            .gap_5()
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .child(
                        v_flex()
                            .gap_1()
                            .child(div().font_semibold().text_lg().child("Settings"))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("Settings are saved automatically."),
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
                    .child(div().font_semibold().text_sm().child("Repository"))
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("Images are uploaded through this repository, as owner/name."),
                    )
                    .child(Input::new(&self.repository_input))
                    .child(
                        Button::new("save-repository")
                            .primary()
                            .small()
                            .label("Save repository")
                            .on_click(cx.listener(Self::save_repository)),
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
                        Checkbox::new("start-at-login")
                            .label("Start at Login")
                            .checked(self.settings.start_at_login)
                            .on_click(cx.listener(|this, checked, _, cx| {
                                this.set_start_at_login(*checked, cx);
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
            .when_some(self.settings_status.as_ref(), |settings, status| {
                settings.child(self.render_inline_status(status, cx))
            })
            .child(div().flex_1())
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
            && self.repository.is_some()
            && self.staged_image.is_some()
            && !self.uploading;

        let root = v_flex()
            .key_context(KEY_CONTEXT)
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::on_paste))
            .size_full()
            .bg(cx.theme().background)
            .p_6()
            .gap_5();

        if self.settings_open {
            root.child(self.render_settings(cx)).into_any_element()
        } else {
            let needs_setup = self.load_state == LoadState::NeedsSetup;
            let root = root
                .on_action(cx.listener(Self::copy_upload_url))
                .on_action(cx.listener(Self::copy_upload_markdown));
            root.child(
                h_flex()
                    .gap_3()
                    .items_center()
                    .justify_between()
                    .child(
                        h_flex()
                            .gap_3()
                            .items_center()
                            .child(
                                div()
                                    .size_10()
                                    .rounded_lg()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .bg(cx.theme().primary)
                                    .text_color(cx.theme().primary_foreground)
                                    .child(Icon::new(IconName::Github).size(px(22.))),
                            )
                            .child(
                                v_flex()
                                    .child(
                                        div()
                                            .text_lg()
                                            .font_semibold()
                                            .child("GitHub Image Upload"),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground)
                                            .child("Upload an image for GitHub Markdown"),
                                    ),
                            ),
                    )
                    .child(
                        Button::new("open-settings")
                            .outline()
                            .small()
                            .label("Settings")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.show_settings(cx);
                            })),
                    ),
            )
            .when(needs_setup, |root| root.child(self.render_setup_prompt(cx)))
            .when(!needs_setup, |root| {
                root.child(self.render_repository_header(cx))
                    .child(self.render_drop_zone(cx))
                    .child(self.render_preview(cx))
                    .child(self.render_upload_result(cx))
            })
            .child(div().flex_1())
            .child(self.render_status(cx))
            .child(
                Button::new("send-image")
                    .primary()
                    .large()
                    .w_full()
                    .label(if self.uploading {
                        "Uploading…"
                    } else {
                        "Upload image"
                    })
                    .icon(IconName::ArrowUp)
                    .loading(self.uploading)
                    .disabled(!can_send)
                    .on_click(cx.listener(Self::send)),
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
    ]);
}

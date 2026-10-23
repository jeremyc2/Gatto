//! Collect two image attachments and combine them for the ordinary editor.
use crate::{
    annotation::{Comparison, Document, Rect, comparison},
    model::StagedAttachment,
    theme::AppButtonStyle as _,
    window_limits,
};
use anyhow::{Context as _, Result, bail};
use gpui_kit::assets::IconName;
use gpui_kit::base::TestSupportExt as _;
use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Icon, Sizable as _, StyledExt as _, WindowExt as _,
    attachment::{
        Attachment, AttachmentActions, AttachmentContent, AttachmentDescription, AttachmentMedia,
        AttachmentTitle,
    },
    button::Button,
    h_flex,
    kbd::Kbd,
    notification::Notification,
    v_flex,
};
use gpui_kit::{
    App, AppContext as _, ClipboardEntry, Context, EventEmitter, ExternalPaths, FocusHandle,
    Focusable, InteractiveElement as _, IntoElement, KeyDownEvent, Keystroke, ParentElement as _,
    PathPromptOptions, Render, StatefulInteractiveElement as _, Styled as _, Window, div,
    prelude::FluentBuilder as _, px,
};
use std::path::PathBuf;

pub(crate) const TITLE: &str = "Side-by-Side Comparison";
pub(crate) const WINDOW_WIDTH: f32 = 540.;
pub(crate) const WINDOW_HEIGHT: f32 = 500.;
const PADDING: u32 = 16;
const GAP: u32 = 24;
const MIN_PANEL_WIDTH: u32 = 400;

/// Preserve source proportions without upscaling. Wider columns leave room for
/// captions below small images; both captions start beneath the shared image row.
fn layout(dimensions: [(u32, u32); 2]) -> (u32, u32, [(u32, u32); 2], Comparison) {
    let available_width = comparison::MAX_WIDTH - PADDING * 2 - GAP;
    let available_height = comparison::MAX_HEIGHT - 128 - PADDING * 2;
    let max_width = dimensions.iter().map(|size| size.0).max().unwrap().max(1);
    let max_height = dimensions.iter().map(|size| size.1).max().unwrap().max(1);
    let sum_width = dimensions
        .iter()
        .map(|size| u64::from(size.0))
        .sum::<u64>()
        .max(1);
    let scale = (f64::from(available_width) / sum_width as f64)
        .min(f64::from(available_height) / f64::from(max_height))
        .min(f64::from(available_width - MIN_PANEL_WIDTH) / f64::from(max_width))
        .min(1.);
    let sizes = dimensions.map(|(w, h)| {
        (
            ((f64::from(w) * scale).floor() as u32).max(1),
            ((f64::from(h) * scale).floor() as u32).max(1),
        )
    });
    let widths = sizes.map(|(w, _)| w.max(MIN_PANEL_WIDTH));
    let height = sizes[0].1.max(sizes[1].1);
    let width = PADDING * 2 + GAP + widths[0] + widths[1];
    let panels = [
        Rect {
            x: PADDING as f32,
            y: PADDING as f32,
            w: widths[0] as f32,
            h: height as f32,
        },
        Rect {
            x: (PADDING + widths[0] + GAP) as f32,
            y: PADDING as f32,
            w: widths[1] as f32,
            h: height as f32,
        },
    ];
    let comparison = Comparison::new(panels);
    let total_height =
        (comparison.caption_band().y + comparison.caption_band().h).ceil() as u32 + PADDING;
    (width, total_height, sizes, comparison)
}

pub(crate) fn combine(sources: [StagedAttachment; 2]) -> Result<StagedAttachment> {
    let dimensions = sources.each_ref().map(|source| {
        source
            .image()
            .map(|image| image.pixel_size)
            .context("Choose an image for each side.")
    });
    let [left, right] = dimensions;
    let (width, height, sizes, comparison) = layout([left?, right?]);
    let mut canvas =
        image::RgbaImage::from_pixel(width, height, image::Rgba(comparison::BACKGROUND));
    for (index, source) in sources.iter().enumerate() {
        validate_source(source)?;
        let pixels = image::load_from_memory(&source.image().unwrap().bytes)?
            .resize_exact(
                sizes[index].0,
                sizes[index].1,
                image::imageops::FilterType::Lanczos3,
            )
            .into_rgba8();
        image::imageops::overlay(
            &mut canvas,
            &pixels,
            (comparison.panels[index].x + (comparison.panels[index].w - sizes[index].0 as f32) / 2.)
                .floor() as i64,
            comparison.panels[index].y as i64,
        );
    }
    let mut document = Document::from_image(canvas);
    document.comparison = Some(comparison);
    let (bytes, pixel_size, _) = document.comparison_png(comparison::INITIAL_PNG_BUDGET)?;
    let attachment =
        StagedAttachment::from_rendered_png("side-by-side.png".into(), bytes, pixel_size)?;
    // The PNG is already complete for staging/upload. The session keeps the
    // unlabeled base and editable captions for the ordinary annotation editor.
    let image = attachment.image().unwrap();
    let saved = document.edit_state();
    image
        .annotation_session
        .retain(document, image.preview.clone(), saved);
    Ok(attachment)
}

fn validate_source(source: &StagedAttachment) -> Result<()> {
    if source.image().is_none() || source.mime_type == "image/gif" {
        bail!("Choose PNG, JPEG, or WebP images for the comparison.");
    }
    Ok(())
}
fn load_source(path: PathBuf) -> Result<StagedAttachment> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !matches!(extension.as_str(), "png" | "jpg" | "jpeg" | "webp") {
        bail!("Choose PNG, JPEG, or WebP images for the comparison.");
    }
    let source = StagedAttachment::from_path(path, Default::default())?;
    validate_source(&source)?;
    Ok(source)
}

pub(crate) struct ComparisonRequested(pub [StagedAttachment; 2]);
impl EventEmitter<ComparisonRequested> for ImageComposer {}

pub(crate) struct ImageComposer {
    sources: [Option<StagedAttachment>; 2],
    loading: [bool; 2],
    combining: bool,
    generation: u64,
    focus_handle: FocusHandle,
}
impl ImageComposer {
    pub(crate) fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            sources: [None, None],
            loading: [false; 2],
            combining: false,
            generation: 0,
            focus_handle: cx.focus_handle(),
        }
    }
    fn busy(&self) -> bool {
        self.combining || self.loading.iter().any(|loading| *loading)
    }
    pub(crate) fn reset(&mut self, cx: &mut Context<Self>) {
        self.sources = [None, None];
        self.loading = [false; 2];
        self.combining = false;
        self.generation = self.generation.wrapping_add(1);
        cx.notify();
    }
    fn load(
        &mut self,
        index: usize,
        load: impl FnOnce() -> Result<StagedAttachment> + Send + 'static,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.combining || self.loading[index] || window.has_active_dialog(cx) {
            return;
        }
        self.loading[index] = true;
        let generation = self.generation;
        cx.notify();
        cx.spawn_in(window, async move |this, window| {
            let result = window
                .background_spawn(async move {
                    let source = load()?;
                    validate_source(&source)?;
                    Ok::<_, anyhow::Error>(source)
                })
                .await;
            let _ = this.update_in(window, |this, window, cx| {
                if this.generation != generation {
                    return;
                }
                this.loading[index] = false;
                match result {
                    Ok(source) => {
                        this.sources[index] = Some(source);
                        this.finish(cx);
                    }
                    Err(error) => {
                        window.push_notification(Notification::warning(error.to_string()), cx)
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }
    fn load_paths(
        &mut self,
        index: usize,
        paths: Vec<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if paths.len() != 1 {
            window.push_notification(Notification::warning("Add one image to each side."), cx);
            return;
        }
        self.load(
            index,
            move || load_source(paths.into_iter().next().unwrap()),
            window,
            cx,
        );
    }
    fn choose(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.combining || self.loading[index] {
            return;
        }
        let generation = self.generation;
        let prompt = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some(format!("Choose the {} image", ["left", "right"][index]).into()),
        });
        cx.spawn_in(window, async move |this, window| {
            if let Some(paths) = prompt.await.ok().and_then(Result::ok).flatten() {
                let _ = this.update_in(window, |this, window, cx| {
                    if this.generation == generation {
                        this.load_paths(index, paths, window, cx);
                    }
                });
            }
        })
        .detach();
    }
    fn paste(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.combining {
            return;
        }
        let Some(index) = self.paste_target() else {
            return;
        };
        let Some(clipboard) = cx.read_from_clipboard() else {
            return;
        };
        if let Some(image) = clipboard.entries().iter().find_map(|entry| {
            if let ClipboardEntry::Image(image) = entry {
                Some(image.clone())
            } else {
                None
            }
        }) {
            self.load(
                index,
                move || StagedAttachment::from_clipboard(&image),
                window,
                cx,
            );
        } else if let Some(paths) = clipboard.entries().iter().find_map(|entry| {
            if let ClipboardEntry::ExternalPaths(paths) = entry {
                Some(paths.paths().to_vec())
            } else {
                None
            }
        }) {
            self.load_paths(index, paths, window, cx);
        }
    }
    fn paste_target(&self) -> Option<usize> {
        self.sources
            .iter()
            .enumerate()
            .position(|(index, source)| source.is_none() && !self.loading[index])
    }
    fn finish(&mut self, cx: &mut Context<Self>) {
        if self.busy() {
            return;
        }
        if let [Some(left), Some(right)] = self.sources.clone() {
            self.combining = true;
            cx.emit(ComparisonRequested([left, right]));
        }
    }
    pub(crate) fn requested(
        &mut self,
        result: Result<(), &'static str>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.combining = false;
        match result {
            Ok(()) => window_limits::request_close(window, cx),
            Err(message) => window.push_notification(Notification::warning(message), cx),
        }
        cx.notify();
    }
    pub(crate) fn dismiss(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.busy() {
            window_limits::request_close(window, cx);
        }
    }
    pub(crate) fn should_close(&mut self, _: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.busy() {
            return false;
        }
        self.reset(cx);
        true
    }
    fn slot(&self, index: usize, cx: &mut Context<Self>) -> impl IntoElement {
        let busy = self.combining || self.loading[index];
        v_flex()
            .id(("comparison-slot", index))
            .debug_selector(move || format!("comparison-slot-{index}"))
            .w_full()
            .min_w_0()
            .gap_2()
            .capture_any_mouse_down(cx.listener(move |this, _, window, cx| {
                window.focus(&this.focus_handle, cx);
            }))
            .can_drop(move |value, _, _| !busy && value.downcast_ref::<ExternalPaths>().is_some())
            .on_drop(cx.listener(move |this, paths: &ExternalPaths, window, cx| {
                this.load_paths(index, paths.paths().to_vec(), window, cx);
            }))
            .child(
                div()
                    .id(("comparison-slot-label", index))
                    .test_support()
                    .font_medium()
                    .child(["Left image", "Right image"][index]),
            )
            .child(v_flex().min_h(px(160.)).justify_center().map(|body| {
                if let Some(source) = &self.sources[index] {
                    body.child(
                        div()
                            .id(("comparison-attachment", index))
                            .test_support()
                            .w_full()
                            .child(
                                Attachment::new()
                                    .id(("comparison-file", index))
                                    .large()
                                    .w_full()
                                    .media(
                                        AttachmentMedia::new()
                                            .src(source.image().unwrap().preview.clone()),
                                    )
                                    .content(
                                        AttachmentContent::new()
                                            .title(AttachmentTitle::new(source.name.clone()))
                                            .description(AttachmentDescription::new(
                                                source.formatted_size(),
                                            )),
                                    )
                                    .actions(
                                        AttachmentActions::new().child(
                                            Button::new(("remove-comparison-image", index))
                                                .app_icon()
                                                .icon(IconName::X)
                                                .accessibility_label(format!(
                                                    "Remove {} image",
                                                    ["left", "right"][index]
                                                ))
                                                .tooltip("Remove image")
                                                .disabled(busy)
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    this.sources[index] = None;
                                                    cx.notify();
                                                })),
                                        ),
                                    ),
                            ),
                    )
                } else {
                    body.child(
                        div()
                            .id(("comparison-drop-zone", index))
                            .test_support()
                            .debug_selector(move || format!("comparison-drop-zone-{index}"))
                            .h(px(160.))
                            .w_full()
                            .rounded_xl()
                            .border_2()
                            .border_dashed()
                            .border_color(cx.theme().primary.opacity(0.4))
                            .bg(cx.theme().primary.opacity(0.05))
                            .when(!busy, |zone| {
                                zone.cursor_pointer()
                                    .hover(|style| style.bg(cx.theme().primary.opacity(0.14)))
                                    .drag_over::<ExternalPaths>(|style, _, _, cx| {
                                        style
                                            .border_color(cx.theme().primary)
                                            .bg(cx.theme().primary.opacity(0.1))
                                    })
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.choose(index, window, cx)
                                    }))
                            })
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                v_flex()
                                    .items_center()
                                    .gap_3()
                                    .px_3()
                                    .child(
                                        Icon::new(if self.loading[index] {
                                            IconName::LoaderCircle
                                        } else {
                                            IconName::Image
                                        })
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
                                            .map(|hint| {
                                                if self.paste_target() == Some(index) {
                                                    hint.child("Press")
                                                        .child(Kbd::new(
                                                            Keystroke::parse("cmd-v").unwrap(),
                                                        ))
                                                        .child("or drop an image here")
                                                } else {
                                                    hint.child("Drop second image here")
                                                }
                                            }),
                                    ),
                            ),
                    )
                }
            }))
    }
}
impl Focusable for ImageComposer {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
impl Render for ImageComposer {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .p_6()
            .gap_5()
            .key_context("ImageComparison")
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if event.keystroke.modifiers.platform && event.keystroke.key == "v" {
                    this.paste(window, cx);
                    cx.stop_propagation();
                }
            }))
            .child(
                div()
                    .id("comparison-heading")
                    .test_support()
                    .text_xl()
                    .font_semibold()
                    .child(TITLE),
            )
            .child(self.slot(0, cx))
            .child(self.slot(1, cx))
    }
}

#[cfg(feature = "walkthrough-screenshots")]
#[allow(dead_code)]
impl ImageComposer {
    pub(crate) fn load_walkthrough(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("docs/images/walkthrough")
            .join(["03-image-staged.png", "04-upload-complete.png"][index]);
        self.load(index, move || load_source(path), window, cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::TestAppContext;
    use gpui_kit::test::TestWindowExt as _;
    use std::sync::Arc;
    fn fixture(width: u32, height: u32, color: [u8; 4]) -> StagedAttachment {
        let pixels = image::RgbaImage::from_pixel(width, height, image::Rgba(color));
        let mut bytes = std::io::Cursor::new(Vec::new());
        pixels
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        StagedAttachment::from_bytes("fixture.png".into(), bytes.into_inner(), None).unwrap()
    }
    #[test]
    fn whole_comparison_preserves_detail_within_4k_and_preserves_small_images_without_upscaling() {
        for dimensions in [
            [(3840, 2160), (3840, 2160)],
            [(2160, 3840), (100, 800)],
            [(8000, 1), (1, 8000)],
            [(80, 60), (125, 92)],
            [(1, 1), (1, 1)],
        ] {
            let (width, height, sizes, comparison) = layout(dimensions);
            assert!(width <= comparison::MAX_WIDTH && height <= comparison::MAX_HEIGHT);
            assert_eq!(comparison.panels[0].y, comparison.panels[1].y);
            assert_eq!(comparison.panels[0].h, comparison.panels[1].h);
            for (input, size) in dimensions.into_iter().zip(sizes) {
                assert!(size.0 <= input.0 && size.1 <= input.1);
                // Integer rounding may contribute at most one output pixel.
                assert!(
                    (size.0 as f64 / input.0 as f64 - size.1 as f64 / input.1 as f64).abs()
                        <= 1. / f64::from(input.0.min(input.1))
                );
            }
        }
        let (_, _, sizes, comparison) = layout([(80, 60), (125, 92)]);
        assert_eq!(sizes, [(80, 60), (125, 92)]);
        assert!(comparison.panels[0].w > 80. && comparison.panels[1].w > 125.);
        assert_eq!(comparison.caption_rect(0).y, comparison.caption_rect(1).y);
    }
    #[test]
    fn combined_png_has_top_aligned_sources_gap_labels_and_its_own_edit_session() {
        let left = fixture(80, 60, [255, 0, 0, 255]);
        let right = fixture(125, 92, [0, 0, 255, 255]);
        let left_bytes = left.image().unwrap().bytes.clone();
        let output = combine([left.clone(), right]).unwrap();
        assert_eq!(
            output.description, output.name,
            "use the normal attachment description"
        );
        let image = output.image().unwrap();
        let retained = image
            .annotation_session
            .open(&image.bytes, image.preview.clone())
            .unwrap();
        assert!(retained.document.marks.is_empty());
        let comparison = retained.document.comparison.as_ref().unwrap();
        assert_eq!(comparison.labels, ["Before", "After"]);
        let pixels = image::load_from_memory(&image.bytes).unwrap().to_rgba8();
        let x0 = (comparison.panels[0].x + (comparison.panels[0].w - 80.) / 2.).floor() as u32;
        let x1 = (comparison.panels[1].x + (comparison.panels[1].w - 125.) / 2.).floor() as u32;
        let y = PADDING;
        assert_eq!(pixels.get_pixel(x0, y).0, [255, 0, 0, 255]);
        assert_eq!(pixels.get_pixel(x1, y).0, [0, 0, 255, 255]);
        assert_eq!(pixels.get_pixel(x0, y + 60).0, comparison::BACKGROUND);
        assert_eq!(pixels.get_pixel(x1 - GAP / 2, y).0, comparison::BACKGROUND);
        let caption = comparison.caption_rect(0);
        assert_eq!(
            retained
                .document
                .base
                .get_pixel(caption.x as u32, caption.y as u32)
                .0,
            comparison::BACKGROUND
        );
        assert!(
            pixels
                .enumerate_pixels()
                .any(|(_, y, pixel)| y as f32 >= caption.y && pixel.0[..3] == [244, 244, 245])
        );
        assert!(Arc::ptr_eq(&left_bytes, &left.image().unwrap().bytes));
        assert!(
            left.image()
                .unwrap()
                .annotation_session
                .open(&left_bytes, left.image().unwrap().preview.clone())
                .unwrap()
                .document
                .comparison
                .is_none()
        );
    }
    #[test]
    fn gifs_are_rejected_by_content_even_with_a_png_name() {
        let mut bytes = Vec::new();
        image::codecs::gif::GifEncoder::new(&mut bytes)
            .encode(&[255, 0, 0, 255], 1, 1, image::ExtendedColorType::Rgba8)
            .unwrap();
        let gif = StagedAttachment::from_bytes("misleading.png".into(), bytes, None).unwrap();
        assert!(validate_source(&gif).is_err());
        assert!(load_source("image.gif".into()).is_err());
        assert!(load_source("video.mp4".into()).is_err());
    }
    fn picker_window(
        cx: &mut TestAppContext,
    ) -> (
        gpui_kit::Entity<ImageComposer>,
        &mut gpui_kit::VisualTestContext,
    ) {
        cx.update(gpui_kit::init);
        let captured = std::rc::Rc::new(std::cell::RefCell::new(None));
        let capture = captured.clone();
        let (_, window_cx) = cx.add_window_view(move |window, cx| {
            let picker = cx.new(|cx| ImageComposer::new(window, cx));
            window.focus(&picker.read(cx).focus_handle(cx), cx);
            *capture.borrow_mut() = Some(picker.clone());
            gpui_kit::base::Root::new(picker, window, cx)
        });
        let picker = captured.borrow().clone().unwrap();
        (picker, window_cx)
    }
    #[gpui_kit::test]
    fn paste_fills_left_then_right_and_automatically_requests_combination(cx: &mut TestAppContext) {
        let (picker, cx) = picker_window(cx);
        let requests = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let capture = requests.clone();
        cx.update(|window, cx| {
            cx.subscribe(&picker, move |_, event: &ComparisonRequested, _| {
                capture.borrow_mut().push(event.0.clone());
            })
            .detach();
            window.draw(cx).clear(cx);
            cx.write_to_clipboard(gpui_kit::ClipboardItem {
                entries: vec![ClipboardEntry::Image(
                    fixture(80, 60, [255, 0, 0, 255])
                        .image()
                        .unwrap()
                        .preview
                        .as_ref()
                        .clone(),
                )],
            });
        });
        cx.simulate_keystrokes("cmd-v");
        cx.run_until_parked();
        cx.update(|window, cx| {
            assert_eq!(picker.read(cx).paste_target(), Some(1));
            assert!(requests.borrow().is_empty());
            window.render_frame(cx);
            assert!(
                window
                    .try_find(("comparison-attachment", 0_usize))
                    .is_some()
            );
            assert!(window.try_find(("comparison-drop-zone", 1_usize)).is_some());
            window.click(("remove-comparison-image", 0_usize), cx);
            assert_eq!(picker.read(cx).paste_target(), Some(0));
        });
        cx.simulate_keystrokes("cmd-v");
        cx.run_until_parked();
        cx.update(|_, cx| {
            cx.write_to_clipboard(gpui_kit::ClipboardItem {
                entries: vec![ClipboardEntry::Image(
                    fixture(90, 70, [0, 0, 255, 255])
                        .image()
                        .unwrap()
                        .preview
                        .as_ref()
                        .clone(),
                )],
            });
        });
        cx.simulate_keystrokes("cmd-v");
        cx.run_until_parked();
        let requests = requests.borrow();
        assert_eq!(requests.len(), 1);
        assert_eq!(
            requests[0]
                .each_ref()
                .map(|source| source.image().unwrap().pixel_size),
            [(80, 60), (90, 70)]
        );
        cx.update(|_, cx| assert!(picker.read(cx).combining));
    }
    #[gpui_kit::test]
    fn consecutive_pastes_can_fill_both_slots_before_either_decode_completes(
        cx: &mut TestAppContext,
    ) {
        let (picker, cx) = picker_window(cx);
        cx.update(|window, cx| {
            picker.update(cx, |picker, cx| {
                for color in [[255, 0, 0, 255], [0, 0, 255, 255]] {
                    cx.write_to_clipboard(gpui_kit::ClipboardItem {
                        entries: vec![ClipboardEntry::Image(
                            fixture(80, 60, color)
                                .image()
                                .unwrap()
                                .preview
                                .as_ref()
                                .clone(),
                        )],
                    });
                    picker.paste(window, cx);
                }
                assert_eq!(picker.loading, [true, true]);
            })
        });
        cx.run_until_parked();
        cx.update(|_, cx| {
            let picker = picker.read(cx);
            assert!(picker.combining);
            let colors = picker.sources.each_ref().map(|source| {
                image::load_from_memory(&source.as_ref().unwrap().image().unwrap().bytes)
                    .unwrap()
                    .to_rgba8()
                    .get_pixel(0, 0)
                    .0
            });
            assert_eq!(colors, [[255, 0, 0, 255], [0, 0, 255, 255]]);
        });
    }
    #[gpui_kit::test]
    fn right_can_be_attached_first_without_changing_the_clipboard_target(cx: &mut TestAppContext) {
        let (picker, cx) = picker_window(cx);
        cx.update(|window, cx| {
            picker.update(cx, |picker, cx| {
                picker.load(1, || Ok(fixture(80, 60, [0, 0, 255, 255])), window, cx);
            })
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            assert_eq!(picker.read(cx).paste_target(), Some(0));
            window.render_frame(cx);
            assert!(
                window
                    .try_find(("comparison-attachment", 1_usize))
                    .is_some()
            );
            assert!(window.try_find(("comparison-drop-zone", 0_usize)).is_some());
        });
    }
}

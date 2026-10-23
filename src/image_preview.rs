mod editor;
pub use editor::AppliedAnnotation;

use crate::annotation::{Document, Pos, PreviewCache, Session};
use std::{
    cell::RefCell,
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

use gpui_kit::assets::IconName;
use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Sizable as _, StyledExt as _,
    button::{Button, ButtonVariants as _},
    h_flex,
    menu::{ContextMenuExt as _, DropdownMenu as _},
};
use gpui_kit::{
    App, Bounds, Context, Corners, CursorStyle, FocusHandle, Focusable, Image,
    InteractiveElement as _, IntoElement, KeyBinding, MouseButton, MouseDownEvent, MouseMoveEvent,
    ParentElement as _, PinchEvent, Pixels, Point, Render, ScrollWheelEvent, SharedString,
    StatefulInteractiveElement as _, Styled as _, Window, actions, canvas, div, point,
    prelude::FluentBuilder as _, px, size,
};

use crate::model::StagedAttachment;

actions!(
    image_preview,
    [
        ZoomIn,
        ZoomOut,
        FitImage,
        ActualSize,
        PreviousImage,
        NextImage,
        BringToFront,
        BringForward,
        SendBackward,
        SendToBack
    ]
);

const KEY_CONTEXT: &str = "ImagePreview";
const MIN_ZOOM: f32 = 0.1;
const MAX_ZOOM: f32 = 8.;
const ZOOM_STEP: f32 = 1.2;
const TOOLBAR_HEIGHT: f32 = 52.;

#[derive(Clone, Copy)]
struct DragState {
    pointer_origin: Point<Pixels>,
    image_origin: Point<Pixels>,
}

pub struct ImagePreview {
    images: Vec<PreviewImage>,
    current_index: usize,
    zoom: f32,
    offset: Point<Pixels>,
    drag: Option<DragState>,
    focus_handle: FocusHandle,
    animation: Rc<RefCell<AnimationClock>>,
    editor: editor::Editor,
    close_confirmed: bool,
    session_generation: u64,
    saving: Option<editor::SaveBatch>,
}

struct PreviewImage {
    id: u64,
    image: Arc<Image>,
    name: SharedString,
    pixel_size: (u32, u32),
    document: Option<Document>,
    saved_marks: Vec<crate::annotation::Mark>,
    saved_comparison: Option<crate::annotation::Comparison>,
    session: Session,
    annotations: Rc<RefCell<PreviewCache>>,
}

impl PreviewImage {
    fn from_attachment(attachment: StagedAttachment) -> Option<Self> {
        let image = attachment.image()?;
        let retained = if attachment.mime_type == "image/gif" {
            None
        } else {
            image
                .annotation_session
                .open(&image.bytes, image.preview.clone())
                .ok()
        };
        let (document, source, saved_marks, saved_comparison) =
            retained.map_or((None, image.preview.clone(), Vec::new(), None), |session| {
                (
                    Some(session.document),
                    session.image,
                    session.saved_marks,
                    session.saved_comparison,
                )
            });
        let pixel_size = document
            .as_ref()
            .map_or(image.pixel_size, |doc| doc.base.dimensions());
        let mut annotations = PreviewCache::default();
        if let Some(document) = &document {
            annotations.update(document);
        }
        Some(Self {
            id: attachment.id,
            image: source,
            name: attachment.name.clone().into(),
            pixel_size,
            document,
            saved_marks,
            saved_comparison,
            session: image.annotation_session.clone(),
            annotations: Rc::new(RefCell::new(annotations)),
        })
    }
}

impl Drop for PreviewImage {
    fn drop(&mut self) {
        if let Some(document) = self.document.take() {
            self.session.retain(
                document,
                self.image.clone(),
                crate::annotation::EditState {
                    marks: std::mem::take(&mut self.saved_marks),
                    comparison: self.saved_comparison.take(),
                },
            );
        }
    }
}

#[derive(Default)]
struct AnimationClock {
    frame: usize,
    last_frame: Option<Instant>,
    image_id: Option<gpui_kit::ImageId>,
    delays: Vec<Duration>,
    cycle: Duration,
}

impl AnimationClock {
    fn frame_at(
        &mut self,
        image: &gpui_kit::RenderImage,
        now: Instant,
        playing: bool,
        reduce_motion: bool,
    ) -> usize {
        if self.image_id != Some(image.id) {
            *self = Self::default();
            self.image_id = Some(image.id);
            self.delays = (0..image.frame_count())
                .map(|index| Duration::from(image.delay(index)).max(Duration::from_millis(10)))
                .collect();
            self.cycle = self.delays.iter().copied().sum();
        }
        if self.delays.len() <= 1 || reduce_motion {
            self.frame = 0;
            self.last_frame = None;
            return 0;
        }
        self.frame = self.frame.min(self.delays.len() - 1);
        if !playing {
            self.last_frame = None;
            return self.frame;
        }
        let mut last_frame = self.last_frame.unwrap_or(now);
        // Skip full cycles after occlusion/slow renders, then account for each
        // frame's own delay. Zero-delay frames must not cause a busy loop.
        let elapsed = now.duration_since(last_frame);
        if elapsed >= self.cycle {
            last_frame =
                now - Duration::from_nanos((elapsed.as_nanos() % self.cycle.as_nanos()) as u64);
        }
        loop {
            let delay = self.delays[self.frame];
            if now.duration_since(last_frame) < delay {
                break;
            }
            last_frame += delay;
            self.frame = (self.frame + 1) % self.delays.len();
        }
        self.last_frame = Some(last_frame);
        self.frame
    }
}

impl ImagePreview {
    pub fn new(images: Vec<StagedAttachment>, selected_id: u64, cx: &mut Context<Self>) -> Self {
        let images = images
            .into_iter()
            .filter_map(PreviewImage::from_attachment)
            .collect::<Vec<_>>();
        let current_index = images
            .iter()
            .position(|image| image.id == selected_id)
            .unwrap_or_default();
        Self {
            images,
            current_index,
            zoom: 1.,
            offset: point(px(0.), px(0.)),
            drag: None,
            focus_handle: cx.focus_handle(),
            animation: Rc::new(RefCell::new(AnimationClock::default())),
            editor: editor::Editor::default(),
            close_confirmed: false,
            session_generation: 0,
            saving: None,
        }
    }

    pub fn show(
        &mut self,
        images: Vec<StagedAttachment>,
        selected_id: u64,
        cx: &mut Context<Self>,
    ) {
        self.finish_text(cx);
        self.close_confirmed = false;
        // Keep open documents and reload closed sessions from their staged attachments.
        let mut previous = std::mem::take(&mut self.images);
        self.images = images
            .into_iter()
            .filter_map(|attachment| {
                if let Some(index) = previous.iter().position(|image| image.id == attachment.id) {
                    Some(previous.remove(index))
                } else {
                    PreviewImage::from_attachment(attachment)
                }
            })
            .collect();
        for image in previous {
            let textures = image.annotations.borrow_mut().retired_images(true);
            cx.defer(move |cx| {
                for texture in textures {
                    cx.drop_image(texture, None);
                }
            });
        }
        self.editor = editor::Editor::default();
        *self.animation.borrow_mut() = AnimationClock::default();
        self.current_index = self
            .images
            .iter()
            .position(|image| image.id == selected_id)
            .unwrap_or_default();
        self.fit(cx);
    }

    pub fn clear(&mut self, cx: &mut Context<Self>) {
        self.release_documents(true, cx);
    }

    fn release_documents(&mut self, discard: bool, cx: &mut Context<Self>) {
        self.finish_text(cx);
        self.session_generation = self.session_generation.wrapping_add(1);
        self.saving = None;
        for image in &mut self.images {
            if discard {
                // Restore the last save as editable marks on the original source.
                // A snapshot clears undo/redo without baking those marks into the image.
                if let Some(document) = &mut image.document {
                    *document = document.snapshot();
                    document.marks.clone_from(&image.saved_marks);
                    document.comparison.clone_from(&image.saved_comparison);
                }
            }
            let textures = image.annotations.borrow_mut().retired_images(true);
            cx.defer(move |cx| {
                for texture in textures {
                    cx.drop_image(texture, None);
                }
            });
        }
        self.images.clear();
        self.editor = editor::Editor::default();
        *self.animation.borrow_mut() = AnimationClock::default();
        self.current_index = 0;
        self.fit(cx);
    }

    fn current(&self) -> Option<&PreviewImage> {
        self.images.get(self.current_index)
    }

    fn update_window_title(&self, window: &mut Window) {
        if let Some(image) = self.current() {
            window.set_window_title(&format!("{} — Gatto Preview", image.name));
        }
    }

    fn select_image(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.saving.is_some() || index >= self.images.len() || index == self.current_index {
            return;
        }
        self.finish_text(cx);
        self.editor = editor::Editor::default();
        self.current_index = index;
        *self.animation.borrow_mut() = AnimationClock::default();
        self.fit(cx);
        self.update_window_title(window);
    }

    fn viewport_center(window: &Window) -> Point<Pixels> {
        let viewport = window.viewport_size();
        point(
            viewport.width / 2.,
            px(TOOLBAR_HEIGHT) + (viewport.height - px(TOOLBAR_HEIGHT)) / 2.,
        )
    }

    fn set_zoom_at(
        &mut self,
        requested_zoom: f32,
        anchor: Point<Pixels>,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        let old_zoom = self.zoom;
        let fit = self.fit_scale(window);
        let new_zoom = (requested_zoom * fit).clamp(MIN_ZOOM, MAX_ZOOM) / fit;
        if (new_zoom - old_zoom).abs() < f32::EPSILON {
            return;
        }

        let center = Self::viewport_center(window);
        let ratio = new_zoom / old_zoom;
        self.offset = point(
            anchor.x - center.x - (anchor.x - center.x - self.offset.x) * ratio,
            anchor.y - center.y - (anchor.y - center.y - self.offset.y) * ratio,
        );
        self.zoom = new_zoom;
        cx.notify();
    }

    fn zoom_by(&mut self, factor: f32, window: &Window, cx: &mut Context<Self>) {
        self.set_zoom_at(
            self.zoom * factor,
            Self::viewport_center(window),
            window,
            cx,
        );
    }

    /// Grow only the comparison's footer, keeping its raster source unchanged.
    /// Freeze the fitted extent while typing so wrapping cannot move the caret.
    fn view_pixel_size(&self) -> (u32, u32) {
        if let Some(size) = self.comparison_edit_view_size() {
            return size;
        }
        let Some(image) = self.current() else {
            return (1, 1);
        };
        let mut size = image.pixel_size;
        if let Some(comparison) = image
            .document
            .as_ref()
            .and_then(|doc| doc.comparison.as_ref())
        {
            let band = comparison.caption_band();
            size.1 = size.1.max((band.y + band.h + 16.).ceil() as u32);
        }
        size
    }

    fn comparison_bottom_inset(&self) -> f32 {
        if self.current().is_some_and(|image| {
            image
                .document
                .as_ref()
                .is_some_and(|doc| doc.comparison.is_some())
        }) {
            96.
        } else {
            0.
        }
    }

    fn fit_scale(&self, window: &Window) -> f32 {
        let viewport = window.viewport_size();
        let (w, h) = self.view_pixel_size();
        (viewport.width.as_f32() / w.max(1) as f32)
            .min(
                (viewport.height.as_f32() - TOOLBAR_HEIGHT - self.comparison_bottom_inset())
                    .max(1.)
                    / h.max(1) as f32,
            )
            .min(1.)
    }

    fn actual_size(&mut self, _: &ActualSize, window: &mut Window, cx: &mut Context<Self>) {
        self.set_zoom_at(
            1. / self.fit_scale(window),
            Self::viewport_center(window),
            window,
            cx,
        );
    }

    fn fit(&mut self, cx: &mut Context<Self>) {
        self.zoom = 1.;
        self.offset = point(px(0.), px(0.));
        self.drag = None;
        cx.notify();
    }

    fn zoom_in(&mut self, _: &ZoomIn, window: &mut Window, cx: &mut Context<Self>) {
        self.zoom_by(ZOOM_STEP, window, cx);
    }

    fn zoom_out(&mut self, _: &ZoomOut, window: &mut Window, cx: &mut Context<Self>) {
        self.zoom_by(1. / ZOOM_STEP, window, cx);
    }

    fn fit_image(&mut self, _: &FitImage, _: &mut Window, cx: &mut Context<Self>) {
        self.fit(cx);
    }

    fn previous_image(&mut self, _: &PreviousImage, window: &mut Window, cx: &mut Context<Self>) {
        if self.current_index > 0 {
            self.select_image(self.current_index - 1, window, cx);
        }
    }

    fn next_image(&mut self, _: &NextImage, window: &mut Window, cx: &mut Context<Self>) {
        self.select_image(self.current_index + 1, window, cx);
    }

    fn scroll(&mut self, event: &ScrollWheelEvent, window: &mut Window, cx: &mut Context<Self>) {
        let delta = event.delta.pixel_delta(window.line_height());
        if event.modifiers.platform || event.modifiers.control {
            let factor = (delta.y.as_f32() * 0.003).exp();
            self.set_zoom_at(self.zoom * factor, event.position, window, cx);
        } else {
            self.offset.x += delta.x;
            self.offset.y += delta.y;
            cx.notify();
        }
        cx.stop_propagation();
    }

    fn pinch(&mut self, event: &PinchEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.set_zoom_at(self.zoom * (1. + event.delta), event.position, window, cx);
        cx.stop_propagation();
    }

    fn render_image(&self, window: &Window) -> impl IntoElement {
        let brush = self.pixelate_preview(window);
        let current = self.current().map(|image| {
            (
                image.image.clone(),
                image.pixel_size,
                image.annotations.clone(),
            )
        });
        let zoom = self.zoom;
        let offset = self.offset;
        let bottom_inset = self.comparison_bottom_inset();
        let view_size = self.view_pixel_size();
        let animation = self.animation.clone();

        canvas(
            move |_, window, cx| {
                current
                    .as_ref()
                    .and_then(|(image, pixel_size, annotations)| {
                        image
                            .clone()
                            .use_render_image(window, cx)
                            .map(|render_image| (render_image, *pixel_size, annotations.clone()))
                    })
            },
            move |bounds, render_image, window, cx| {
                let Some((render_image, (natural_width, natural_height), annotations)) =
                    render_image
                else {
                    return;
                };
                let natural_width = natural_width.max(1) as f32;
                let natural_height = natural_height.max(1) as f32;
                let fit = (bounds.size.width.as_f32() / view_size.0 as f32)
                    .min((bounds.size.height.as_f32() - bottom_inset).max(1.) / view_size.1 as f32)
                    .min(1.);
                let image_size = size(
                    px(natural_width * fit * zoom),
                    px(natural_height * fit * zoom),
                );
                let original_origin = point(
                    bounds.origin.x
                        + (bounds.size.width - px(view_size.0 as f32 * fit * zoom)) / 2.
                        + offset.x,
                    bounds.origin.y
                        + (bounds.size.height
                            - px(bottom_inset)
                            - px(view_size.1 as f32 * fit * zoom))
                            / 2.
                        + offset.y,
                );
                let image_bounds = Bounds::new(original_origin, image_size);
                let playing = render_image.frame_count() > 1
                    && !cx.reduce_motion()
                    && window.is_window_active();
                let frame = animation.borrow_mut().frame_at(
                    &render_image,
                    Instant::now(),
                    playing,
                    cx.reduce_motion(),
                );
                if playing {
                    window.request_animation_frame();
                }
                let _ = window.paint_image(
                    bounds,
                    image_bounds,
                    Corners::default(),
                    render_image,
                    frame,
                    false,
                );
                annotations.borrow().paint(
                    Pos::new(original_origin.x.as_f32(), original_origin.y.as_f32()),
                    fit * zoom,
                    brush,
                    window,
                );
            },
        )
        .size_full()
    }
    fn zoom_controls(&self, window: &Window, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .absolute()
            .bottom_2()
            .left_2()
            .gap_0p5()
            .p_1()
            .rounded_lg()
            .bg(gpui_kit::rgb(0x202024))
            .border_1()
            .border_color(gpui_kit::rgb(0x383840))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
            .on_mouse_move(cx.listener(Self::editor_controls_hover))
            .child(
                Button::new("preview-zoom-out")
                    .ghost()
                    .small()
                    .icon(IconName::Minus)
                    .tooltip("Zoom out · ⌘−")
                    .accessibility_label("Zoom out")
                    .on_click(
                        cx.listener(|this, _, window, cx| this.zoom_by(1. / ZOOM_STEP, window, cx)),
                    ),
            )
            .child(
                Button::new("preview-fit")
                    .ghost()
                    .small()
                    .label(format!("{:.0}%", self.fit_scale(window) * self.zoom * 100.))
                    .tooltip("Zoom")
                    .accessibility_label("Zoom menu")
                    .dropdown_menu_with_anchor(gpui_kit::Anchor::BottomLeft, |menu, _, _| {
                        menu.menu("Zoom in", Box::new(ZoomIn))
                            .menu("Zoom out", Box::new(ZoomOut))
                            .menu("Zoom to 100%", Box::new(ActualSize))
                            .menu("Zoom to fit", Box::new(FitImage))
                    }),
            )
            .child(
                Button::new("preview-zoom-in")
                    .ghost()
                    .small()
                    .icon(IconName::Plus)
                    .tooltip("Zoom in · ⌘+")
                    .accessibility_label("Zoom in")
                    .on_click(
                        cx.listener(|this, _, window, cx| this.zoom_by(ZOOM_STEP, window, cx)),
                    ),
            )
    }
}

impl Focusable for ImagePreview {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ImagePreview {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Establish preset sizes before any zooming, using this image's opening view.
        self.annotation_scale(window);
        let editable = self.editable();
        let menu_view = cx.entity().downgrade();
        let cursor = if self.drag.is_some() {
            CursorStyle::ClosedHand
        } else if !editable || self.editor.tool == crate::annotation::Tool::Pan || self.editor.space
        {
            CursorStyle::OpenHand
        } else if self.editor.tool == crate::annotation::Tool::Select {
            self.selection_cursor().style()
        } else if self.editor.tool == crate::annotation::Tool::Text {
            CursorStyle::IBeam
        } else {
            CursorStyle::Crosshair
        };
        let image_count = self.images.len();
        let image_name = self
            .current()
            .map(|image| image.name.clone())
            .unwrap_or_else(|| "Image preview".into());
        let position_label = format!("{} of {}", self.current_index + 1, image_count);
        let can_go_back = self.saving.is_none() && self.current_index > 0;
        let can_go_forward = self.saving.is_none() && self.current_index + 1 < image_count;

        div()
            .key_context(KEY_CONTEXT)
            .track_focus(&self.focus_handle(cx))
            .on_action(cx.listener(Self::dismiss_editor))
            .on_action(cx.listener(Self::close_editor))
            .on_action(cx.listener(Self::bring_to_front))
            .on_action(cx.listener(Self::bring_forward))
            .on_action(cx.listener(Self::send_backward))
            .on_action(cx.listener(Self::send_to_back))
            .on_key_down(cx.listener(Self::editor_key_down))
            .on_key_up(cx.listener(Self::editor_key_up))
            .on_action(cx.listener(Self::zoom_in))
            .on_action(cx.listener(Self::zoom_out))
            .on_action(cx.listener(Self::fit_image))
            .on_action(cx.listener(Self::actual_size))
            .on_action(cx.listener(Self::previous_image))
            .on_action(cx.listener(Self::next_image))
            .size_full()
            .flex()
            .flex_col()
            .bg(cx.theme().background)
            .child(
                h_flex()
                    .h(px(TOOLBAR_HEIGHT))
                    .flex_none()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .px_4()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .text_sm()
                            .font_semibold()
                            .child(image_name),
                    )
                    .child(
                        h_flex()
                            .items_center()
                            .gap_1()
                            .when(editable, |toolbar| {
                                toolbar.child(self.annotation_actions(cx))
                            })
                            .when(image_count > 1, |toolbar| {
                                toolbar
                                    .child(
                                        Button::new("preview-previous-image")
                                            .ghost()
                                            .small()
                                            .icon(IconName::ChevronLeft)
                                            .accessibility_label("Previous image")
                                            .tooltip("Previous")
                                            .disabled(!can_go_back)
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                if this.current_index > 0 {
                                                    this.select_image(
                                                        this.current_index - 1,
                                                        window,
                                                        cx,
                                                    );
                                                }
                                            })),
                                    )
                                    .child(
                                        div()
                                            .w(px(48.))
                                            .text_center()
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(position_label),
                                    )
                                    .child(
                                        Button::new("preview-next-image")
                                            .ghost()
                                            .small()
                                            .icon(IconName::ChevronRight)
                                            .accessibility_label("Next image")
                                            .tooltip("Next")
                                            .disabled(!can_go_forward)
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.select_image(
                                                    this.current_index + 1,
                                                    window,
                                                    cx,
                                                );
                                            })),
                                    )
                            }),
                    ),
            )
            .child(
                div()
                    .id("image-preview-canvas")
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .cursor(cursor)
                    .on_mouse_down(MouseButton::Left, cx.listener(Self::editor_mouse_down))
                    .on_mouse_down(MouseButton::Middle, cx.listener(Self::editor_mouse_down))
                    .on_mouse_down(MouseButton::Right, cx.listener(Self::editor_right_click))
                    .on_mouse_move(cx.listener(Self::editor_mouse_move))
                    .on_hover(cx.listener(|this, hovered: &bool, window, cx| {
                        let p = window.mouse_position();
                        let size = window.viewport_size();
                        if !hovered
                            && (p.y < px(TOOLBAR_HEIGHT)
                                || p.x < px(0.)
                                || p.y > size.height
                                || p.x > size.width)
                        {
                            let was_active =
                                std::mem::replace(&mut this.editor.canvas_hover, false);
                            let brush = this.editor.brush_cursor.take().is_some();
                            if was_active
                                || brush
                                || this.editor.selection_cursor != Default::default()
                            {
                                this.editor.selection_cursor = Default::default();
                                cx.notify();
                            }
                        }
                    }))
                    .on_mouse_up(MouseButton::Left, cx.listener(Self::editor_mouse_up))
                    .on_mouse_up_out(MouseButton::Left, cx.listener(Self::editor_mouse_up))
                    .on_scroll_wheel(cx.listener(Self::scroll))
                    .on_pinch(cx.listener(Self::pinch))
                    .on_mouse_up(MouseButton::Middle, cx.listener(Self::editor_mouse_up))
                    .on_mouse_up_out(MouseButton::Middle, cx.listener(Self::editor_mouse_up))
                    .child(self.render_image(window))
                    .when(self.comparison_edit_view_size().is_none(), |surface| {
                        surface.child(self.zoom_controls(window, cx))
                    })
                    .when(editable, |surface| {
                        surface
                            .child(self.annotation_overlays(window, cx))
                            .child(self.comparison_captions(window, cx))
                            .when(self.comparison_edit_view_size().is_none(), |surface| {
                                surface
                                    .child(self.annotation_toolbar(cx))
                                    .child(self.annotation_properties(cx))
                            })
                            .child(self.preview_cursor_layer(cursor, window, cx))
                    })
                    .when(self.saving.is_some(), |surface| {
                        surface.child(
                            div()
                                .id("annotation-saving-shield")
                                .absolute()
                                .inset_0()
                                .occlude()
                                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                                .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
                                .on_mouse_down(MouseButton::Middle, |_, _, cx| {
                                    cx.stop_propagation()
                                })
                                .on_scroll_wheel(|_, _, cx| cx.stop_propagation()),
                        )
                    })
                    .context_menu(move |menu, _, cx| {
                        if let Some(view) = menu_view.upgrade() {
                            view.read(cx).stacking_menu(menu, cx)
                        } else {
                            menu
                        }
                    }),
            )
    }
}

pub fn init_keybindings(cx: &mut App) {
    editor::init_text_keybindings(cx);
    cx.bind_keys([
        KeyBinding::new("cmd-=", ZoomIn, Some(KEY_CONTEXT)),
        KeyBinding::new("cmd-+", ZoomIn, Some(KEY_CONTEXT)),
        KeyBinding::new("cmd--", ZoomOut, Some(KEY_CONTEXT)),
        KeyBinding::new("cmd-0", FitImage, Some(KEY_CONTEXT)),
        KeyBinding::new("shift-0", ActualSize, Some(KEY_CONTEXT)),
        KeyBinding::new("shift-1", FitImage, Some(KEY_CONTEXT)),
        KeyBinding::new("left", PreviousImage, Some(KEY_CONTEXT)),
        KeyBinding::new("right", NextImage, Some(KEY_CONTEXT)),
    ]);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn animated_gif() -> Arc<gpui_kit::RenderImage> {
        let mut bytes = Vec::new();
        {
            let mut encoder = image::codecs::gif::GifEncoder::new(&mut bytes);
            for (color, delay) in [([255, 0, 0, 255], 100), ([0, 255, 0, 255], 200)] {
                encoder
                    .encode_frame(image::Frame::from_parts(
                        image::RgbaImage::from_pixel(2, 2, image::Rgba(color)),
                        0,
                        0,
                        image::Delay::from_numer_denom_ms(delay, 1),
                    ))
                    .unwrap();
            }
        }
        Image::from_bytes(gpui_kit::ImageFormat::Gif, bytes)
            .to_image_data(gpui_kit::SvgRenderer::new(Arc::new(
                gpui_kit::assets::AllAssets,
            )))
            .unwrap()
    }

    #[test]
    fn gif_frames_advance_with_their_delays_loop_and_pause_for_accessibility() {
        let image = animated_gif();
        assert_eq!(image.frame_count(), 2);
        let now = Instant::now();
        let mut clock = AnimationClock::default();
        assert_eq!(clock.frame_at(&image, now, true, false), 0);
        let second = clock.frame_at(&image, now + Duration::from_millis(110), true, false);
        assert_eq!(second, 1);
        assert_ne!(image.as_bytes(0), image.as_bytes(second));
        assert_eq!(
            clock.frame_at(&image, now + Duration::from_millis(299), true, false),
            1
        );
        assert_eq!(
            clock.frame_at(&image, now + Duration::from_millis(300), true, false),
            0
        );
        assert_eq!(
            clock.frame_at(&image, now + Duration::from_millis(410), true, false),
            1
        );
        assert_eq!(
            clock.frame_at(&image, now + Duration::from_secs(10), false, false),
            1
        );
        assert_eq!(
            clock.frame_at(&image, now + Duration::from_secs(20), true, false),
            1
        );
        assert_eq!(
            clock.frame_at(&image, now + Duration::from_secs(21), true, true),
            0
        );
        assert_eq!(
            clock.frame_at(&image, now + Duration::from_secs(22), true, false),
            0
        );
    }

    #[test]
    fn zoom_is_clamped_to_preview_limits() {
        assert_eq!(0.01_f32.clamp(MIN_ZOOM, MAX_ZOOM), MIN_ZOOM);
        assert_eq!(20_f32.clamp(MIN_ZOOM, MAX_ZOOM), MAX_ZOOM);
    }
}

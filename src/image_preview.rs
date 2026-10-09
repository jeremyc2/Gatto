use std::sync::Arc;

use gpui_kit::assets::IconName;
use gpui_kit::component::{
    ActiveTheme as _, Sizable as _, StyledExt as _,
    button::{Button, ButtonVariants as _},
    h_flex,
};
use gpui_kit::{
    App, Bounds, Context, Corners, CursorStyle, FocusHandle, Focusable, Image,
    InteractiveElement as _, IntoElement, KeyBinding, MouseButton, MouseDownEvent, MouseMoveEvent,
    ParentElement as _, PinchEvent, Pixels, Point, Render, ScrollWheelEvent, SharedString,
    Styled as _, Window, actions, canvas, div, point, px, size,
};

actions!(image_preview, [ZoomIn, ZoomOut, FitImage, ClosePreview]);

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
    image: Arc<Image>,
    name: SharedString,
    pixel_size: (u32, u32),
    zoom: f32,
    offset: Point<Pixels>,
    drag: Option<DragState>,
    focus_handle: FocusHandle,
}

impl ImagePreview {
    pub fn new(
        image: Arc<Image>,
        name: impl Into<SharedString>,
        pixel_size: (u32, u32),
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            image,
            name: name.into(),
            pixel_size,
            zoom: 1.,
            offset: point(px(0.), px(0.)),
            drag: None,
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn show(
        &mut self,
        image: Arc<Image>,
        name: impl Into<SharedString>,
        pixel_size: (u32, u32),
        cx: &mut Context<Self>,
    ) {
        self.image = image;
        self.name = name.into();
        self.pixel_size = pixel_size;
        self.fit(cx);
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
        let new_zoom = requested_zoom.clamp(MIN_ZOOM, MAX_ZOOM);
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

    fn close_preview(&mut self, _: &ClosePreview, window: &mut Window, _: &mut Context<Self>) {
        window.remove_window();
    }

    fn mouse_down(&mut self, event: &MouseDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.drag = Some(DragState {
            pointer_origin: event.position,
            image_origin: self.offset,
        });
        cx.notify();
    }

    fn mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        let Some(drag) = self.drag else {
            return;
        };
        if !event.dragging() {
            self.drag = None;
            cx.notify();
            return;
        }
        self.offset = point(
            drag.image_origin.x + event.position.x - drag.pointer_origin.x,
            drag.image_origin.y + event.position.y - drag.pointer_origin.y,
        );
        cx.notify();
    }

    fn mouse_up(&mut self, _: &gpui_kit::MouseUpEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.drag = None;
        cx.notify();
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

    fn render_image(&self) -> impl IntoElement {
        let image = self.image.clone();
        let (natural_width, natural_height) = self.pixel_size;
        let zoom = self.zoom;
        let offset = self.offset;

        canvas(
            move |_, window, cx| image.use_render_image(window, cx),
            move |bounds, render_image, window, _| {
                let Some(render_image) = render_image else {
                    return;
                };
                let natural_width = natural_width.max(1) as f32;
                let natural_height = natural_height.max(1) as f32;
                let fit = (bounds.size.width.as_f32() / natural_width)
                    .min(bounds.size.height.as_f32() / natural_height);
                let image_size = size(
                    px(natural_width * fit * zoom),
                    px(natural_height * fit * zoom),
                );
                let image_bounds = Bounds::new(
                    point(
                        bounds.origin.x + (bounds.size.width - image_size.width) / 2. + offset.x,
                        bounds.origin.y + (bounds.size.height - image_size.height) / 2. + offset.y,
                    ),
                    image_size,
                );
                let _ = window.paint_image(
                    bounds,
                    image_bounds,
                    Corners::default(),
                    render_image,
                    0,
                    false,
                );
            },
        )
        .size_full()
    }
}

impl Focusable for ImagePreview {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ImagePreview {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let cursor = if self.drag.is_some() {
            CursorStyle::ClosedHand
        } else {
            CursorStyle::OpenHand
        };
        let zoom_label = format!("{:.0}%", self.zoom * 100.);

        div()
            .key_context(KEY_CONTEXT)
            .track_focus(&self.focus_handle(cx))
            .on_action(cx.listener(Self::zoom_in))
            .on_action(cx.listener(Self::zoom_out))
            .on_action(cx.listener(Self::fit_image))
            .on_action(cx.listener(Self::close_preview))
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
                            .child(self.name.clone()),
                    )
                    .child(
                        h_flex()
                            .items_center()
                            .gap_1()
                            .child(
                                Button::new("preview-zoom-out")
                                    .ghost()
                                    .small()
                                    .icon(IconName::ZoomOut)
                                    .accessibility_label("Zoom out")
                                    .tooltip("Zoom out")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.zoom_by(1. / ZOOM_STEP, window, cx);
                                    })),
                            )
                            .child(
                                div()
                                    .w(px(54.))
                                    .text_center()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(zoom_label),
                            )
                            .child(
                                Button::new("preview-zoom-in")
                                    .ghost()
                                    .small()
                                    .icon(IconName::ZoomIn)
                                    .accessibility_label("Zoom in")
                                    .tooltip("Zoom in")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.zoom_by(ZOOM_STEP, window, cx);
                                    })),
                            )
                            .child(
                                Button::new("preview-fit")
                                    .ghost()
                                    .small()
                                    .icon(IconName::Maximize2)
                                    .accessibility_label("Fit image")
                                    .tooltip("Fit image")
                                    .on_click(cx.listener(|this, _, _, cx| this.fit(cx))),
                            ),
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
                    .on_mouse_down(MouseButton::Left, cx.listener(Self::mouse_down))
                    .on_mouse_move(cx.listener(Self::mouse_move))
                    .on_mouse_up(MouseButton::Left, cx.listener(Self::mouse_up))
                    .on_mouse_up_out(MouseButton::Left, cx.listener(Self::mouse_up))
                    .on_scroll_wheel(cx.listener(Self::scroll))
                    .on_pinch(cx.listener(Self::pinch))
                    .child(self.render_image()),
            )
    }
}

pub fn init_keybindings(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("cmd-=", ZoomIn, Some(KEY_CONTEXT)),
        KeyBinding::new("cmd-+", ZoomIn, Some(KEY_CONTEXT)),
        KeyBinding::new("cmd--", ZoomOut, Some(KEY_CONTEXT)),
        KeyBinding::new("cmd-0", FitImage, Some(KEY_CONTEXT)),
        KeyBinding::new("escape", ClosePreview, Some(KEY_CONTEXT)),
    ]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoom_is_clamped_to_preview_limits() {
        assert_eq!(0.01_f32.clamp(MIN_ZOOM, MAX_ZOOM), MIN_ZOOM);
        assert_eq!(20_f32.clamp(MIN_ZOOM, MAX_ZOOM), MAX_ZOOM);
    }
}

//! Comparison captions use the ordinary text editor, anchored in a protected
//! footer. They never become movable or resizable annotation marks.
use super::*;
use gpui_kit::base::TestSupportExt as _;

pub(super) struct CaptionEdit {
    pub index: usize,
    pub input: Entity<TextInput>,
    pub view_size: (u32, u32),
    checkpointed: bool,
    _blur: Subscription,
}
impl ImagePreview {
    pub(super) fn comparison_annotation_bottom(&self) -> Option<f32> {
        self.doc()
            .and_then(|doc| doc.comparison.as_ref())
            .map(|comparison| comparison.panels[0].y + comparison.panels[0].h)
    }

    pub(in crate::image_preview) fn comparison_edit_view_size(&self) -> Option<(u32, u32)> {
        self.editor
            .comparison_edit
            .as_ref()
            .map(|edit| edit.view_size)
    }

    pub(super) fn finish_comparison_label(&mut self, cx: &mut Context<Self>) {
        if let Some(edit) = self.editor.comparison_edit.take() {
            let value = edit.input.read(cx).value().to_string();
            if let Some(comparison) = self.doc_mut().and_then(|doc| doc.comparison.as_mut()) {
                comparison.labels[edit.index] = if value.trim().is_empty() {
                    ["Before", "After"][edit.index].into()
                } else {
                    value
                };
                comparison.labels[edit.index] = comparison.label(edit.index);
            }
            self.editor.comparison_subscription = None;
            self.refresh_annotations(cx);
        }
    }
    fn edit_comparison_label(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.saving.is_some() {
            return;
        }
        self.finish_text(cx);
        let Some(comparison) = self.doc().and_then(|doc| doc.comparison.as_ref()) else {
            return;
        };
        let mark = comparison.label_mark(index);
        let view_size = self.view_pixel_size();
        let input = cx.new(|cx| TextInput::new(mark, cx));
        let blur = cx.on_blur(&input.read(cx).focus_handle(cx), window, |this, _, cx| {
            this.finish_comparison_label(cx)
        });
        self.editor.comparison_edit = Some(CaptionEdit {
            index,
            input: input.clone(),
            view_size,
            checkpointed: false,
            _blur: blur,
        });
        self.editor.comparison_subscription =
            Some(cx.subscribe(&input, |this, input, event, cx| {
                if matches!(event, InputEvent::Change) {
                    let value = input.read(cx).value().to_string();
                    let Some(edit) = this.editor.comparison_edit.as_ref() else {
                        return;
                    };
                    let index = edit.index;
                    if this
                        .doc()
                        .and_then(|doc| doc.comparison.as_ref())
                        .is_some_and(|comparison| comparison.labels[index] != value)
                    {
                        if !edit.checkpointed {
                            this.doc_mut().unwrap().checkpoint();
                            this.editor.comparison_edit.as_mut().unwrap().checkpointed = true;
                        }
                        this.doc_mut().unwrap().comparison.as_mut().unwrap().labels[index] = value;
                        this.refresh_annotations(cx);
                    }
                }
            }));
        cx.defer_in(window, move |_, window, cx| {
            input.update(cx, |input, cx| input.focus_and_select_all(window, cx))
        });
        cx.notify();
    }
    pub(in crate::image_preview) fn comparison_captions(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let (origin, scale) = self.transform(window);
        let comparison = self.doc().and_then(|doc| doc.comparison.clone());
        let dragging = self.drag.is_some()
            || self.editor.space
            || self.editor.tool == Tool::Pan
            || self.editor.gesture.is_some()
            || self.editor.group_gesture.is_some()
            || self.editor.marquee.is_some();
        div().absolute().inset_0().children(
            comparison
                .into_iter()
                .flat_map(|comparison| (0..2).map(move |index| (index, comparison.clone())))
                .map(|(index, comparison)| {
                    let rect = comparison.caption_rect(index);
                    let editing = self
                        .editor
                        .comparison_edit
                        .as_ref()
                        .filter(|edit| edit.index == index);
                    div()
                        .id(("comparison-caption", index))
                        .test_support()
                        .debug_selector(move || format!("comparison-caption-{index}"))
                        .absolute()
                        .left(px(origin.x + rect.x * scale))
                        .top(px(origin.y + rect.y * scale - TOOLBAR_HEIGHT))
                        .w(px(rect.w * scale))
                        .h(px(rect.h * scale))
                        .when(!dragging, |region| region.occlude().cursor_text())
                        .on_mouse_move(cx.listener(|this, event, window, cx| {
                            if this.drag.is_none()
                                && !this.editor.space
                                && this.editor.tool != Tool::Pan
                            {
                                this.editor_controls_hover(event, window, cx);
                            }
                        }))
                        .capture_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                            if this.editor.comparison_edit.is_some()
                                && (event.keystroke.key == "escape"
                                    || (event.keystroke.key == "enter"
                                        && !event.keystroke.modifiers.shift))
                            {
                                this.finish_comparison_label(cx);
                                window.focus(&this.focus_handle, cx);
                                cx.stop_propagation();
                            }
                        }))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                                if this.editor.space || this.editor.tool == Tool::Pan {
                                    this.editor_mouse_down(event, window, cx);
                                } else if event.click_count == 2
                                    && this
                                        .editor
                                        .comparison_edit
                                        .as_ref()
                                        .is_none_or(|edit| edit.index != index)
                                {
                                    window.prevent_default();
                                    this.edit_comparison_label(index, window, cx);
                                }
                                cx.stop_propagation();
                            }),
                        )
                        .on_mouse_down(
                            MouseButton::Middle,
                            cx.listener(|this, event, window, cx| {
                                this.editor_mouse_down(event, window, cx);
                                cx.stop_propagation();
                            }),
                        )
                        .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
                        .when_some(editing, |region, edit| {
                            edit.input.update(cx, |input, _| {
                                input.configure(comparison.label_mark(index), origin, scale)
                            });
                            region.child(edit.input.clone())
                        })
                }),
        )
    }
}

/// Constrain the gesture's geometry before publishing it. Pointer motion below
/// the image cannot create hidden content behind or underneath the captions.
pub(super) fn bounded_drag<T>(
    from: Pos,
    to: Pos,
    bottom: Option<f32>,
    mut build: impl FnMut(Pos) -> T,
    bounds: impl Fn(&T) -> Rect,
) -> T {
    let next = build(to);
    let Some(bottom) = bottom else {
        return next;
    };
    let fits = |value: &T| {
        let b = bounds(value);
        b.y + b.h <= bottom
    };
    if fits(&next) {
        return next;
    }
    let (mut low, mut high) = (0., 1.);
    let mut best = build(from);
    for _ in 0..14 {
        let t = (low + high) / 2.;
        let candidate = build(Pos::new(
            from.x + (to.x - from.x) * t,
            from.y + (to.y - from.y) * t,
        ));
        if fits(&candidate) {
            low = t;
            best = candidate;
        } else {
            high = t;
        }
    }
    best
}

pub(super) fn keep_above_bottom(mark: &mut Mark, bottom: Option<f32>) -> bool {
    let Some(bottom) = bottom else {
        return false;
    };
    let b = mark.visual_bounds();
    let overflow = b.y + b.h - bottom;
    if overflow > 0. {
        mark.translate(0., -overflow);
        return true;
    }
    false
}

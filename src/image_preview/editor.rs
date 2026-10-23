mod clipboard;
mod comparison;
mod cursor;
mod group;
mod selection;
mod text_input;
mod toolbar;
use super::*;
use crate::annotation::{Document, Mark, Pos, Rect, StackOrder, Style, Tool};
use crate::theme::{self, AppButtonStyle as _};
use gpui_kit::base::TestSupportExt as _;
use gpui_kit::component::{
    WindowExt as _,
    dialog::{DialogAction, DialogClose, DialogFooter},
    input::InputEvent,
    notification::Notification,
};
use gpui_kit::{AppContext as _, Entity, EventEmitter, KeyDownEvent, Subscription, rgb};
use selection::{Handle, SelectionCursor};
use text_input::TextInput;
pub(super) fn init_text_keybindings(cx: &mut App) {
    text_input::init(cx);
}

pub struct AppliedAnnotation {
    pub id: u64,
    pub attachment: StagedAttachment,
}
impl EventEmitter<AppliedAnnotation> for ImagePreview {}

pub(super) struct SaveBatch {
    pending: Vec<(u64, crate::annotation::EditState)>,
}

#[derive(Default)]
pub(super) struct Editor {
    comparison_edit: Option<comparison::CaptionEdit>,
    comparison_subscription: Option<Subscription>,
    pub tool: Tool,
    pub locked: bool,
    pub style: Style,
    pub selected: Option<usize>,
    group: Vec<usize>,
    group_frame: Option<crate::annotation::Frame>,
    group_gesture: Option<group::GroupGesture>,
    marquee: Option<group::Marquee>,
    gesture: Option<Gesture>,
    editing: Option<Entity<TextInput>>,
    editing_created: bool,
    editing_checkpointed: bool,
    subscription: Option<Subscription>,
    pub status: Option<String>,
    pub(super) space: bool,
    shapes_open: Option<ShapeMenu>,
    slider_dragging: Option<toolbar::SliderControl>,
    pub(super) brush_cursor: Option<Point<Pixels>>,
    pixelate_diameter: Option<f32>,
    pub(super) selection_cursor: SelectionCursor,
    pub(super) canvas_hover: bool,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum ShapeMenu {
    Toolbar,
    Properties,
}
#[derive(Clone)]
struct Gesture {
    start: Pos,
    index: usize,
    original: Mark,
    mode: Mode,
    checkpointed: bool,
    started: Instant,
    text_fixed: bool,
}
#[derive(Clone, Copy)]
enum Mode {
    Draw,
    Move,
    Resize(Handle),
    Rotate(usize),
    Bend,
    Endpoint(usize),
    Erase,
}

impl ImagePreview {
    pub(super) fn annotation_scale(&self, window: &Window) -> f32 {
        self.doc().map_or(1., |doc| {
            *doc.annotation_scale
                .get_or_init(|| 1. / self.fit_scale(window))
        })
    }
    pub(super) fn pixelate_preview(&self, window: &Window) -> Option<(Pos, f32)> {
        self.editor
            .brush_cursor
            .filter(|_| {
                self.editor.tool == Tool::Redact && !self.editor.space && self.drag.is_none()
            })
            .map(|p| {
                (
                    Pos::new(p.x.as_f32(), p.y.as_f32()),
                    self.pixelate_width() / self.transform(window).1,
                )
            })
    }
    fn pixelate_width(&self) -> f32 {
        self.editor.pixelate_diameter.unwrap_or(64.).clamp(
            toolbar::MIN_PIXELATE_DIAMETER,
            toolbar::MAX_PIXELATE_DIAMETER,
        )
    }
    pub(super) fn editor_controls_hover(
        &mut self,
        _: &MouseMoveEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.editor.gesture.is_some()
            || self.editor.group_gesture.is_some()
            || self.editor.marquee.is_some()
            || self.drag.is_some()
            || self
                .editor
                .comparison_edit
                .as_ref()
                .is_some_and(|edit| edit.input.read(cx).is_selecting())
            || self
                .editor
                .editing
                .as_ref()
                .is_some_and(|input| input.read(cx).is_selecting())
        {
            return;
        }
        if self.editor.canvas_hover {
            self.editor.canvas_hover = false;
            cx.notify();
        }
        if self.editor.brush_cursor.take().is_some()
            || self.editor.selection_cursor != SelectionCursor::Arrow
        {
            self.editor.selection_cursor = SelectionCursor::Arrow;
            cx.notify();
        }
        cx.stop_propagation();
    }
    fn doc(&self) -> Option<&Document> {
        self.current()?.document.as_ref()
    }
    fn doc_mut(&mut self) -> Option<&mut Document> {
        self.images.get_mut(self.current_index)?.document.as_mut()
    }
    pub(super) fn editable(&self) -> bool {
        self.doc().is_some()
    }
    fn refresh_annotations(&mut self, cx: &mut Context<Self>) {
        let bottom = self.comparison_annotation_bottom();
        if let Some(bottom) = bottom
            && self.editor.gesture.is_none()
            && self.editor.group_gesture.is_none()
        {
            let mut moved = false;
            if let Some(doc) = self.doc_mut() {
                for mark in &mut doc.marks {
                    moved |= comparison::keep_above_bottom(mark, Some(bottom));
                }
            }
            if moved {
                self.editor.group_frame = None;
            }
        }
        if let Some(image) = self.images.get_mut(self.current_index)
            && let Some(doc) = &image.document
        {
            let mut cache = image.annotations.borrow_mut();
            cache.update(doc);
            let retired = cache.retired_images(false);
            if !retired.is_empty() {
                // Defer until the current window is back in App.windows, then free its atlas entries.
                cx.defer(move |cx| {
                    for image in retired {
                        cx.drop_image(image, None);
                    }
                });
            }
        }
        cx.notify();
    }
    fn transform(&self, window: &Window) -> (Pos, f32) {
        let (w, h) = self.view_pixel_size();
        let scale = self.fit_scale(window) * self.zoom;
        let center = Self::viewport_center(window);
        (
            Pos::new(
                center.x.as_f32() + self.offset.x.as_f32() - w as f32 * scale / 2.,
                center.y.as_f32() - self.comparison_bottom_inset() / 2. + self.offset.y.as_f32()
                    - h as f32 * scale / 2.,
            ),
            scale,
        )
    }
    fn image_pos(&self, p: Point<Pixels>, window: &Window) -> Pos {
        let (origin, scale) = self.transform(window);
        Pos::new(
            (p.x.as_f32() - origin.x) / scale,
            (p.y.as_f32() - origin.y) / scale,
        )
    }
    fn select_tool(&mut self, tool: Tool, window: &mut Window, cx: &mut Context<Self>) {
        self.finish_text(cx);
        if tool == Tool::Text || tool.is_shape() || matches!(tool, Tool::Arrow | Tool::Line) {
            let tier = crate::annotation::STROKE_WIDTHS
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    (self.editor.style.width - **a)
                        .abs()
                        .total_cmp(&(self.editor.style.width - **b).abs())
                })
                .unwrap()
                .0;
            self.editor.style.font_size = if tool == Tool::Text {
                crate::annotation::TEXT_SIZES[tier]
            } else if tool.is_shape() {
                crate::annotation::LABEL_SIZES[tier]
            } else {
                crate::annotation::ARROW_TEXT_SIZES[tier]
            };
        }
        self.editor.tool = tool;
        self.editor.selection_cursor = SelectionCursor::Arrow;
        self.editor.shapes_open = None;
        self.clear_selection();
        self.editor.gesture = None;
        window.focus(&self.focus_handle, cx);
        cx.notify();
    }
    fn refocus_text(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(input) = self.editor.editing.clone() {
            let focus = input.read(cx).focus_handle(cx);
            window.focus(&focus, cx);
        }
    }
    pub(super) fn finish_text(&mut self, cx: &mut Context<Self>) {
        self.finish_comparison_label(cx);
        if let Some(input) = self.editor.editing.take() {
            let created = std::mem::take(&mut self.editor.editing_created);
            let value = input.read(cx).value().to_string();
            if let Some(index) = self.editor.selected
                && let Some(mark) = self.doc_mut().and_then(|d| d.marks.get_mut(index))
            {
                mark.set_text(value);
            }
            self.editor.subscription = None;
            if let Some(index) = self.editor.selected
                && let Some(doc) = self.doc_mut()
                && doc
                    .marks
                    .get(index)
                    .is_some_and(|m| m.tool == Tool::Text && m.text.trim().is_empty())
            {
                doc.marks.remove(index);
                self.clear_selection();
            }
            if !self.editor.locked {
                self.editor.tool = Tool::Select;
            }
            if created {
                self.clear_selection();
            }
        }
    }
    fn edit_text(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(mark) = self.doc().and_then(|d| d.marks.get(index)) else {
            return;
        };
        let mark = mark.clone();
        let input = cx.new(|cx| TextInput::new(mark, cx));
        self.set_selection(vec![index]);
        self.editor.editing_checkpointed = false;
        self.editor.subscription = Some(cx.subscribe(&input, |this, input, event, cx| {
            if matches!(event, InputEvent::Change) {
                let value = input.read(cx).value().to_string();
                let changed = this
                    .editor
                    .selected
                    .and_then(|i| this.doc()?.marks.get(i))
                    .is_some_and(|m| m.text != value);
                if changed && !this.editor.editing_checkpointed {
                    this.doc_mut().unwrap().checkpoint();
                    this.editor.editing_checkpointed = true;
                }
                if let Some(index) = this.editor.selected
                    && let Some(mark) = this.doc_mut().and_then(|d| d.marks.get_mut(index))
                {
                    mark.set_text(value);
                }
                this.refresh_annotations(cx);
            }
        }));
        self.editor.editing = Some(input);
        // The creating mouse event still has the old focus path. Focus the new
        // field after dispatch, once that event can no longer restore page focus.
        cx.defer_in(window, |this, window, cx| {
            if let Some(input) = this.editor.editing.clone() {
                input.update(cx, |input, cx| {
                    input.focus_and_select_all(window, cx);
                });
            }
        });
        cx.notify();
    }
    pub(super) fn editor_right_click(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.editable() {
            return;
        }
        if self
            .editor
            .editing
            .as_ref()
            .is_some_and(|input| input.read(cx).contains(event.position))
        {
            self.refocus_text(window, cx);
            return;
        }
        self.finish_text(cx);
        self.editor.gesture = None;
        self.editor.group_gesture = None;
        self.editor.marquee = None;
        self.drag = None;
        self.editor.shapes_open = None;
        let p = self.image_pos(event.position, window);
        let (_, scale) = self.transform(window);
        let hit = self.doc().and_then(|doc| doc.hit(p, 7. / scale));
        if !hit.is_some_and(|i| self.selected_indices().contains(&i)) {
            self.set_selection(hit.into_iter().collect());
        }
        self.editor.tool = Tool::Select;
        if let Some(mark) = self
            .editor
            .selected
            .and_then(|index| self.doc()?.marks.get(index))
        {
            self.editor.style = mark.style.clone().with_scale(1.);
        }
        window.focus(&self.focus_handle, cx);
        cx.notify();
    }
    pub(super) fn stacking_menu(
        &self,
        menu: gpui_kit::component::menu::PopupMenu,
        cx: &App,
    ) -> gpui_kit::component::menu::PopupMenu {
        if let Some(input) = &self.editor.editing {
            return input.read(cx).menu(menu);
        }
        let indices = self.selected_indices();
        if indices.is_empty() {
            return menu;
        }
        let Some(doc) = self.doc() else {
            return menu;
        };
        menu.menu_with_disabled(
            "Bring to front",
            Box::new(BringToFront),
            !doc.can_reorder_many(&indices, StackOrder::Front),
        )
        .menu_with_disabled(
            "Bring forward",
            Box::new(BringForward),
            !doc.can_reorder_many(&indices, StackOrder::Forward),
        )
        .menu_with_disabled(
            "Send backward",
            Box::new(SendBackward),
            !doc.can_reorder_many(&indices, StackOrder::Backward),
        )
        .menu_with_disabled(
            "Send to back",
            Box::new(SendToBack),
            !doc.can_reorder_many(&indices, StackOrder::Back),
        )
    }
    fn reorder_selection(&mut self, order: StackOrder, cx: &mut Context<Self>) {
        if self.saving.is_some() {
            return;
        }
        self.finish_text(cx);
        self.editor.gesture = None;
        let indices = self.selected_indices();
        if !indices.is_empty()
            && let Some(doc) = self.doc_mut()
        {
            let selected = doc.reorder_many(&indices, order);
            self.set_selection(selected);
        }
        self.refresh_annotations(cx);
    }
    pub(super) fn bring_to_front(
        &mut self,
        _: &BringToFront,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.reorder_selection(StackOrder::Front, cx);
    }
    pub(super) fn bring_forward(
        &mut self,
        _: &BringForward,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.reorder_selection(StackOrder::Forward, cx);
    }
    pub(super) fn send_backward(
        &mut self,
        _: &SendBackward,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.reorder_selection(StackOrder::Backward, cx);
    }
    pub(super) fn send_to_back(&mut self, _: &SendToBack, _: &mut Window, cx: &mut Context<Self>) {
        self.reorder_selection(StackOrder::Back, cx);
    }
    pub(super) fn editor_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.editable()
            || self.editor.tool == Tool::Pan
            || self.editor.space
            || event.button == MouseButton::Middle
        {
            self.finish_text(cx);
            self.drag = Some(DragState {
                pointer_origin: event.position,
                image_origin: self.offset,
            });
            cx.notify();
            return;
        }
        let p = self.image_pos(event.position, window);
        if self
            .comparison_annotation_bottom()
            .is_some_and(|bottom| p.y >= bottom)
        {
            self.finish_text(cx);
            return;
        }
        self.editor.shapes_open = None;
        let p = self.image_pos(event.position, window);
        let (_, scale) = self.transform(window);
        let tolerance = 7. / scale;
        self.finish_text(cx);
        window.focus(&self.focus_handle, cx);
        if event.click_count == 2 && self.editor.tool == Tool::Select {
            let target = self
                .editor
                .selected
                .filter(|index| {
                    self.doc()
                        .and_then(|d| d.marks.get(*index))
                        .is_some_and(|mark| mark.hit(p, tolerance) || mark.contains_body(p))
                })
                .or_else(|| self.doc().and_then(|d| d.hit(p, tolerance)));
            if let Some(index) = target
                && {
                    let mark = &self.doc().unwrap().marks[index];
                    mark.tool == Tool::Text || mark.can_edit_label()
                }
            {
                self.edit_text(index, window, cx);
                return;
            }
            self.create_text(p, None, window, cx);
            return;
        }
        if self.editor.tool == Tool::Select {
            self.begin_selection(event, p, scale, cx);
            return;
        }
        if self.editor.tool == Tool::Eraser {
            let doc = self.doc_mut().unwrap();
            doc.checkpoint();
            if let Some(index) = doc.hit(p, tolerance) {
                doc.marks.remove(index);
            }
            self.editor.gesture = Some(Gesture {
                start: p,
                index: 0,
                original: Mark::new(Tool::Eraser, p, self.editor.style.clone()),
                mode: Mode::Erase,
                checkpointed: true,
                started: Instant::now(),
                text_fixed: false,
            });
            self.clear_selection();
            self.refresh_annotations(cx);
            return;
        }
        let mut style = self.editor.style.clone();
        if self.editor.tool == Tool::Redact {
            style.width = self.pixelate_width();
            if style.width <= 0. {
                return;
            }
            self.editor.brush_cursor = Some(event.position);
        }
        let mut mark = Mark::new(
            self.editor.tool,
            p,
            style.with_scale(if self.editor.tool == Tool::Redact {
                // The brush is measured on screen; committed coverage stays in image space.
                1. / scale
            } else {
                self.annotation_scale(window)
            }),
        );
        comparison::keep_above_bottom(&mut mark, self.comparison_annotation_bottom());
        let doc = self.doc_mut().unwrap();
        // An arrow click is a no-op, including its undo/redo history. Commit
        // its checkpoint only when the completed gesture actually draws one.
        if mark.tool != Tool::Arrow {
            doc.checkpoint();
        }
        let index = doc.marks.len();
        doc.marks.push(mark.clone());
        self.editor.selected = (mark.tool != Tool::Redact).then_some(index);
        self.editor.gesture = Some(Gesture {
            start: p,
            index,
            original: mark,
            mode: Mode::Draw,
            checkpointed: self.editor.tool != Tool::Arrow,
            started: Instant::now(),
            text_fixed: false,
        });
        self.refresh_annotations(cx);
    }
    pub(super) fn editor_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.saving.is_some() {
            return;
        }
        if !self.editor.canvas_hover {
            self.editor.canvas_hover = true;
            cx.notify();
        }
        if let Some(input) = self
            .editor
            .comparison_edit
            .as_ref()
            .map(|edit| edit.input.clone())
            && input.read(cx).is_selecting()
        {
            input.update(cx, |input, cx| input.drag_selection(event, cx));
            return;
        }
        if let Some(input) = self.editor.editing.clone()
            && input.read(cx).is_selecting()
        {
            input.update(cx, |input, cx| input.drag_selection(event, cx));
            return;
        }
        if self.editor.gesture.is_none() && self.editor.group_gesture.is_none() {
            self.update_selection_cursor(event.position, window, cx);
        }
        if self.editor.tool == Tool::Redact && !self.editor.space && self.drag.is_none() {
            self.editor.brush_cursor = Some(event.position);
            cx.notify();
        }
        if let Some(drag) = self.drag {
            if event.dragging() {
                self.offset = point(
                    drag.image_origin.x + event.position.x - drag.pointer_origin.x,
                    drag.image_origin.y + event.position.y - drag.pointer_origin.y,
                );
                cx.notify();
            }
            return;
        }
        if self.update_group_gesture(event, window, cx) {
            self.update_selection_cursor(event.position, window, cx);
            return;
        }
        let Some(gesture) = self.editor.gesture.clone() else {
            return;
        };
        if !event.dragging() {
            return;
        }
        let mut p = self.image_pos(event.position, window);
        let (_, scale) = self.transform(window);
        let fixed_text = gesture.text_fixed
            || (gesture.original.tool == Tool::Text
                && gesture.started.elapsed() >= Duration::from_millis(150)
                && (p.x - gesture.start.x).abs() * scale > 24.);
        if fixed_text {
            self.editor.gesture.as_mut().unwrap().text_fixed = true;
        }
        if !gesture.checkpointed && !matches!(gesture.mode, Mode::Draw) {
            if gesture.start.distance(p) * scale < 0.5 {
                return;
            }
            self.doc_mut().unwrap().checkpoint();
            self.editor.gesture.as_mut().unwrap().checkpointed = true;
        }
        if matches!(gesture.mode, Mode::Erase) {
            if let Some(doc) = self.doc_mut()
                && let Some(index) = doc.hit(p, 8. / scale)
            {
                doc.marks.remove(index);
            }
            self.refresh_annotations(cx);
            return;
        }
        let bottom = self.comparison_annotation_bottom();
        if let Some(bottom) = bottom {
            // Most strokes can stop directly at the edge. The geometric solver
            // handles caps, rotation, reflow, and other extents beyond this inset.
            let margin = if gesture.original.tool == Tool::Text {
                0.
            } else {
                gesture.original.style.width / 2.
            };
            p.y = p.y.min(bottom - margin);
        }
        if bottom.is_none() {
            let Some(mark) = self
                .doc_mut()
                .and_then(|doc| doc.marks.get_mut(gesture.index))
            else {
                return;
            };
            apply_mark_gesture(mark, &gesture, p, event.modifiers.shift, fixed_text, scale);
        } else {
            let Some(current) = self
                .doc()
                .and_then(|doc| doc.marks.get(gesture.index))
                .cloned()
            else {
                return;
            };
            let build = |p| {
                projected_mark(
                    &current,
                    &gesture,
                    p,
                    event.modifiers.shift,
                    fixed_text,
                    scale,
                )
            };
            let next = if matches!(gesture.mode, Mode::Move) {
                let mut next = build(p);
                comparison::keep_above_bottom(&mut next, bottom);
                next
            } else {
                let start = if matches!(current.tool, Tool::Pen | Tool::Redact)
                    && matches!(gesture.mode, Mode::Draw)
                {
                    *current.points.last().unwrap()
                } else {
                    gesture.start
                };
                comparison::bounded_drag(start, p, bottom, build, |mark| mark.visual_bounds())
            };
            let mark = &mut self.doc_mut().unwrap().marks[gesture.index];
            *mark = next;
        }
        let mark = &self.doc().unwrap().marks[gesture.index];
        if mark.tool != Tool::Redact {
            self.editor.style = mark.style.clone().with_scale(1.);
        }
        self.update_selection_cursor(event.position, window, cx);
        self.refresh_annotations(cx);
    }
    pub(super) fn editor_mouse_up(
        &mut self,
        event: &gpui_kit::MouseUpEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.drag = None;
        if let Some(edit) = &self.editor.comparison_edit {
            edit.input.update(cx, |input, _| input.end_drag());
        }
        if let Some(input) = &self.editor.editing {
            input.update(cx, |input, _| input.end_drag());
        }
        if self.editor.marquee.take().is_some() || self.editor.group_gesture.take().is_some() {
            self.update_selection_cursor(event.position, window, cx);
            self.refresh_annotations(cx);
            return;
        }
        if let Some(gesture) = self.editor.gesture.take()
            && matches!(gesture.mode, Mode::Draw)
        {
            let mut p = self.image_pos(event.position, window);
            if let Some(bottom) = self.comparison_annotation_bottom() {
                let margin = self.doc().unwrap().marks[gesture.index].style.width / 2.;
                p.y = p.y.min(bottom - margin);
            }
            let (_, scale) = self.transform(window);
            let text = self
                .doc()
                .and_then(|d| d.marks.get(gesture.index))
                .is_some_and(|m| m.tool == Tool::Text);
            if text {
                if !gesture.text_fixed
                    && let Some(mark) = self.doc_mut().and_then(|d| d.marks.get_mut(gesture.index))
                {
                    mark.wrap = None;
                    mark.points[0] = Pos::new(
                        gesture.start.x - mark.text_align().offset(16. * mark.style.scale),
                        gesture.start.y - mark.text_layout().line_height / 2.,
                    );
                }
                if let Some(mark) = self.doc_mut().and_then(|d| d.marks.get_mut(gesture.index)) {
                    mark.points[0].y = gesture.start.y - mark.text_layout().line_height / 2.;
                }
                self.edit_text(gesture.index, window, cx);
                self.editor.editing_created = true;
                self.editor.editing_checkpointed = true;
            } else {
                let tool = self.doc().unwrap().marks[gesture.index].tool;
                let brush = tool == Tool::Redact;
                let click = p.distance(gesture.start) < 2. / scale;
                if brush {
                    let mark = &mut self.doc_mut().unwrap().marks[gesture.index];
                    if mark.points.last().is_some_and(|last| last.distance(p) > 0.) {
                        mark.points.push(p);
                    }
                    self.clear_selection();
                } else {
                    let doc = self.doc_mut().unwrap();
                    let mut mark = doc.marks.remove(gesture.index);
                    if click && mark.tool.is_shape() {
                        let (w, h) = if mark.tool == Tool::Oval {
                            (120. * mark.style.scale, 200. * mark.style.scale)
                        } else {
                            (160. * mark.style.scale, 160. * mark.style.scale)
                        };
                        mark.points = vec![
                            Pos::new(gesture.start.x - w / 2., gesture.start.y - h / 2.),
                            Pos::new(gesture.start.x + w / 2., gesture.start.y + h / 2.),
                        ];
                    } else if click && mark.tool == Tool::Line {
                        mark.points = vec![gesture.start, gesture.start];
                    }
                    if !click || mark.tool.is_shape() || mark.tool == Tool::Line {
                        if !gesture.checkpointed {
                            doc.checkpoint();
                        }
                        doc.marks.insert(gesture.index, mark);
                    }
                }
                if !self.editor.locked && !brush && (tool != Tool::Arrow || !click) {
                    self.editor.tool = Tool::Select;
                }
                self.clear_selection();
            }
        }
        self.refresh_annotations(cx);
    }
    fn create_text(
        &mut self,
        p: Pos,
        wrap: Option<f32>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut mark = Mark::new(
            Tool::Text,
            p,
            self.editor
                .style
                .clone()
                .with_scale(self.annotation_scale(window)),
        );
        mark.wrap = wrap;
        let Some(doc) = self.doc_mut() else {
            return;
        };
        doc.checkpoint();
        let index = doc.marks.len();
        doc.marks.push(mark);
        self.edit_text(index, window, cx);
        self.editor.editing_created = true;
        self.editor.editing_checkpointed = true;
    }
    fn change_style(&mut self, change: impl Fn(&mut Style), cx: &mut Context<Self>) {
        change(&mut self.editor.style);
        let indices = self.selected_indices();
        let mut changed_bounds = false;
        if !indices.is_empty()
            && let Some(doc) = self.doc_mut()
        {
            doc.checkpoint();
            for i in indices {
                if let Some(mark) = doc.marks.get_mut(i) {
                    let before = mark.world_bounds();
                    let scale = mark.style.scale;
                    let mut style = mark.style.clone().with_scale(1.);
                    change(&mut style);
                    mark.style = style.with_scale(scale);
                    mark.set_text(mark.text.clone());
                    changed_bounds |= before != mark.world_bounds();
                }
            }
        }
        if changed_bounds {
            self.editor.group_frame = None;
        }
        self.refresh_annotations(cx);
    }
    fn delete_selected(&mut self, cx: &mut Context<Self>) {
        self.finish_text(cx);
        let indices = self.selected_indices();
        if !indices.is_empty()
            && let Some(doc) = self.doc_mut()
        {
            doc.checkpoint();
            for i in indices.into_iter().rev() {
                doc.marks.remove(i);
            }
        }
        self.clear_selection();
        self.refresh_annotations(cx);
    }
    fn history(&mut self, redo: bool, cx: &mut Context<Self>) {
        self.finish_text(cx);
        self.clear_selection();
        self.editor.gesture = None;
        if let Some(doc) = self.doc_mut() {
            if redo {
                doc.redo();
            } else {
                doc.undo();
            }
        }
        self.refresh_annotations(cx);
    }
    pub(super) fn editor_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.saving.is_some() {
            cx.stop_propagation();
            return;
        }
        let key = event.keystroke.key.as_str();
        let command = event.keystroke.modifiers.platform;
        if self.editor.comparison_edit.is_some() && !(command && key == "s") {
            return;
        }
        if command && key == "s" {
            self.apply_annotation(window, cx);
            cx.stop_propagation();
            return;
        }
        if self.editor.editing.is_some() && command && key == "enter" {
            self.finish_text(cx);
            window.focus(&self.focus_handle, cx);
            self.refresh_annotations(cx);
            cx.stop_propagation();
            return;
        }
        if self.editor.editing.is_some() {
            return;
        }
        if command {
            match key {
                "z" => self.history(event.keystroke.modifiers.shift, cx),
                "d" => self.clone_selected(cx),
                "c" => self.copy_selected(cx),
                "v" => self.paste_selected(window, cx),
                "a" => {
                    self.editor.gesture = None;
                    self.editor.group_gesture = None;
                    self.editor.marquee = None;
                    let indices = self
                        .doc()
                        .map(|d| (0..d.marks.len()).collect())
                        .unwrap_or_default();
                    self.set_selection(indices);
                    self.editor.tool = Tool::Select;
                    if let Some(i) = self.editor.selected {
                        self.editor.style =
                            self.doc().unwrap().marks[i].style.clone().with_scale(1.);
                    }
                    cx.notify();
                }
                _ => return,
            }
            cx.stop_propagation();
            return;
        }
        if !self.editable() {
            return;
        }
        match key {
            "space" => {
                self.editor.space = true;
                cx.notify();
            }
            "backspace" | "delete" => self.delete_selected(cx),
            "v" => self.select_tool(Tool::Select, window, cx),
            "h" => self.select_tool(Tool::Pan, window, cx),
            "t" => self.select_tool(Tool::Text, window, cx),
            "r" => self.select_tool(Tool::Rectangle, window, cx),
            "o" => self.select_tool(Tool::Ellipse, window, cx),
            "k" => self.select_tool(Tool::Line, window, cx),
            "a" => self.select_tool(Tool::Arrow, window, cx),
            "p" => self.select_tool(Tool::Pen, window, cx),
            "e" => self.select_tool(Tool::Eraser, window, cx),
            "x" => self.select_tool(Tool::Redact, window, cx),
            "l" => {
                self.editor.locked = !self.editor.locked;
                cx.notify();
            }
            "enter" => {
                if let Some(index) = self.editor.selected
                    && self.doc().is_some_and(|d| {
                        d.marks[index].tool == Tool::Text || d.marks[index].can_edit_label()
                    })
                {
                    self.edit_text(index, window, cx);
                }
            }
            _ => return,
        }
        cx.stop_propagation();
    }
    pub(super) fn editor_key_up(
        &mut self,
        event: &gpui_kit::KeyUpEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if event.keystroke.key == "space" {
            self.editor.space = false;
            self.drag = None;
            cx.notify();
        }
    }
    fn has_unsaved_changes(&self) -> bool {
        self.images.iter().any(|image| {
            image.document.as_ref().is_some_and(|doc| {
                doc.marks != image.saved_marks || doc.comparison != image.saved_comparison
            })
        })
    }
    fn can_discard(&self) -> bool {
        self.images.iter().any(|image| {
            image.document.as_ref().is_some_and(|doc| {
                doc.marks != image.saved_marks
                    || doc.comparison != image.saved_comparison
                    || doc.can_undo()
                    || doc.can_redo()
            })
        })
    }
    fn apply_annotation(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.saving.is_some() {
            return;
        }
        self.finish_text(cx);
        if !self.has_unsaved_changes() {
            return;
        }
        let exports: Vec<_> = self
            .images
            .iter()
            .filter_map(|image| {
                let doc = image.document.as_ref()?;
                if doc.marks == image.saved_marks && doc.comparison == image.saved_comparison {
                    return None;
                }
                let name = std::path::Path::new(image.name.as_ref())
                    .with_extension("png")
                    .to_string_lossy()
                    .into_owned();
                Some((image.id, name, doc.snapshot()))
            })
            .collect();
        self.saving = Some(SaveBatch {
            pending: exports
                .iter()
                .map(|(id, _, doc)| (*id, doc.edit_state()))
                .collect(),
        });
        self.editor.gesture = None;
        self.editor.group_gesture = None;
        self.editor.marquee = None;
        self.editor.shapes_open = None;
        self.editor.slider_dragging = None;
        self.drag = None;
        self.editor.canvas_hover = false;
        self.editor.brush_cursor = None;
        self.editor.selection_cursor = SelectionCursor::Arrow;
        let generation = self.session_generation;
        cx.notify();
        // Export the whole pending batch before replacing any attachments. Close only
        // after the uploader confirms each replacement, so a rejected save stays editable.
        cx.spawn_in(window, async move |this, window| {
            let result: anyhow::Result<Vec<_>> = window
                .background_spawn(async move {
                    exports
                        .into_iter()
                        .map(|(id, name, doc)| {
                            let (bytes, _) = doc.png(None)?;
                            crate::model::validate_size(
                                bytes.len() as u64,
                                crate::model::MAX_ATTACHMENT_BYTES,
                            )?;
                            Ok((id, StagedAttachment::from_bytes(name, bytes, None)?))
                        })
                        .collect()
                })
                .await;
            let _ = this.update_in(window, |this, window, cx| {
                if this.session_generation != generation || this.saving.is_none() {
                    return;
                }
                match result {
                    Ok(exports) => {
                        for (id, attachment) in exports {
                            cx.emit(AppliedAnnotation { id, attachment });
                        }
                    }
                    Err(error) => {
                        this.saving = None;
                        window.push_notification(Notification::error(error.to_string()), cx);
                        cx.notify();
                    }
                }
            });
        })
        .detach();
    }
    pub(crate) fn annotation_saved(
        &mut self,
        id: u64,
        result: Result<(), &'static str>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(batch) = self.saving.as_mut() else {
            return;
        };
        let Some(index) = batch.pending.iter().position(|(pending, _)| *pending == id) else {
            return;
        };
        if let Err(message) = result {
            self.saving = None;
            window.push_notification(Notification::warning(message), cx);
            cx.notify();
            return;
        }
        let (_, state) = batch.pending.remove(index);
        if let Some(image) = self.images.iter_mut().find(|image| image.id == id) {
            image.saved_marks = state.marks;
            image.saved_comparison = state.comparison;
        }
        if batch.pending.is_empty() {
            if window.has_active_dialog(cx) {
                window.close_dialog(cx);
            }
            self.release_documents(false, cx);
            self.close_confirmed = true;
            crate::window_limits::request_close(window, cx);
        }
        cx.notify();
    }
    fn confirm_revert(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.saving.is_some() || !self.can_discard() {
            return;
        }
        self.finish_text(cx);
        let preview = cx.entity().downgrade();
        let image_id = self.current().map(|image| image.id);
        window.open_alert_dialog(cx, move |alert, _, _| {
            let preview = preview.clone();
            alert.width(px(420.))
                .title("Are you sure?")
                .description("Discard unsaved edits and close the preview? Saved annotations stay editable. Undo/redo history will be erased. This can’t be undone.")
                .show_cancel(true)
                .cancel_text("Cancel")
                .ok_text("Discard")
                .footer(theme::confirmation_footer("ok", "Discard", 88.))
                .on_ok(move |_, window, cx| {
                    let closed = preview.update(cx, |this, cx| {
                        if this.current().map(|image| image.id) != image_id { return false; }
                        this.clear(cx);
                        this.close_confirmed = true;
                        true
                    }).unwrap_or(false);
                    if closed { crate::window_limits::request_close(window, cx); }
                    true
                })
        });
    }
    pub(crate) fn confirm_exit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.close_confirmed || window.has_active_dialog(cx) {
            return;
        }
        self.finish_text(cx);
        if !self.has_unsaved_changes() {
            self.release_documents(false, cx);
            self.close_confirmed = true;
            crate::window_limits::request_close(window, cx);
            return;
        }
        let preview = cx.entity().downgrade();
        window.open_alert_dialog(cx, move |alert, _, cx| {
            let save = preview.clone();
            let discard = preview.clone();
            let saving = preview
                .upgrade()
                .is_some_and(|view| view.read(cx).saving.is_some());
            alert
                .width(px(440.))
                .title("Save changes before closing?")
                .description(
                    "Discard removes unsaved edits and edit history. Saved annotations stay editable.",
                )
                .footer(
                    DialogFooter::new()
                        .justify_between()
                        .child(
                            Button::new("preview-exit-discard")
                                .app_secondary()
                                .w(px(88.))
                                .h(px(32.))
                                .flex_none()
                                .label("Discard")
                                .on_click(move |_, window, cx| {
                                    window.close_dialog(cx);
                                    let _ = discard.update(cx, |this, cx| {
                                        this.clear(cx);
                                        this.close_confirmed = true;
                                    });
                                    crate::window_limits::request_close(window, cx);
                                }),
                        )
                        .child(
                            h_flex()
                                .gap_2()
                                .flex_none()
                                .child(
                                    div().w(px(88.)).h(px(32.)).flex_none().child(
                                        DialogClose::new().child(
                                            Button::new("cancel")
                                                .app_secondary()
                                                .w(px(88.))
                                                .h(px(32.))
                                                .label("Cancel"),
                                        ),
                                    ),
                                )
                                .child(
                                    div().w(px(88.)).h(px(32.)).flex_none().child(
                                        DialogAction::new().child(
                                            Button::new("ok")
                                                .app_primary()
                                                .w(px(88.))
                                                .h(px(32.))
                                                .label("Save")
                                                .disabled(saving),
                                        ),
                                    ),
                                ),
                        ),
                )
                .on_ok(move |_, window, cx| {
                    let _ = save.update(cx, |this, cx| this.apply_annotation(window, cx));
                    true
                })
        });
    }
    pub(crate) fn should_close(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if std::mem::take(&mut self.close_confirmed) {
            return true;
        }
        self.finish_text(cx);
        if !self.has_unsaved_changes() {
            self.release_documents(false, cx);
            return true;
        }
        self.confirm_exit(window, cx);
        false
    }
    pub(super) fn close_editor(
        &mut self,
        _: &crate::application_menu::CloseWindow,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.confirm_exit(window, cx);
    }
    pub(crate) fn dismiss_editor(
        &mut self,
        _: &crate::application_menu::DismissWindow,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if window.has_active_dialog(cx) {
            window.close_dialog(cx);
            return;
        }
        let tool = self.editor.tool;
        let finished_creation = self.editor.editing_created;
        self.finish_text(cx);
        if !finished_creation {
            self.editor.tool = tool;
        }
        self.clear_selection();
        self.editor.shapes_open = None;
        self.editor.gesture = None;
        self.editor.slider_dragging = None;
        self.editor.space = false;
        self.editor.brush_cursor = None;
        self.drag = None;
        self.editor.selection_cursor = SelectionCursor::Arrow;
        // Return keyboard handling to the artwork after leaving an input or selection.
        window.focus(&self.focus_handle, cx);
        self.refresh_annotations(cx);
    }
    pub(super) fn annotation_overlays(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let (origin, scale) = self.transform(window);
        let selected = self.editor.selected.and_then(|i| self.doc()?.marks.get(i));
        // Drawing owns the pointer until release; selection chrome belongs to editing
        // the finished object, not to its live preview.
        let selection = selected.filter(|_| {
            self.editor.editing.is_none()
                && !self
                    .editor
                    .gesture
                    .as_ref()
                    .is_some_and(|gesture| matches!(gesture.mode, Mode::Draw))
        });
        let text_box = selected.filter(|mark| {
            mark.tool == Tool::Text
                && mark.wrap.is_some()
                && (self.editor.editing.is_some()
                    || self.editor.gesture.as_ref().is_some_and(|gesture| {
                        matches!(gesture.mode, Mode::Draw) && gesture.text_fixed
                    }))
        });
        let multiple = self.selected_indices().len() > 1;
        let selection_frame = self.selection_frame();
        let handles = selection
            .map(|mark| {
                if !multiple && matches!(mark.tool, Tool::Arrow | Tool::Line) {
                    vec![mark.points[0], mark.points[1], mark.bend_handle()]
                } else {
                    // Edges remain resize targets without visible midpoint handles.
                    selection_frame.unwrap().corners().to_vec()
                }
            })
            .unwrap_or_default();
        div()
            .absolute()
            .inset_0()
            .when_some(
                self.editor.brush_cursor.filter(|_| {
                    self.editor.tool == Tool::Redact && !self.editor.space && self.drag.is_none()
                }),
                |overlay, p| {
                    let diameter = self.pixelate_width();
                    overlay.child(
                        div()
                            .id("pixelate-brush-cursor")
                            .test_support()
                            .absolute()
                            .left(p.x - px(diameter / 2.))
                            .top(p.y - px(TOOLBAR_HEIGHT + diameter / 2.))
                            .size(px(diameter))
                            .rounded_full()
                            .border_2()
                            .border_color(rgb(0x438fff)),
                    )
                },
            )
            .when_some(selection, |overlay, mark| {
                if !multiple && matches!(mark.tool, Tool::Arrow | Tool::Line) {
                    let mark = mark.clone();
                    return overlay.child(
                        div()
                            .id("annotation-arrow-selection")
                            .test_support()
                            .absolute()
                            .inset_0()
                            .child(
                                canvas(
                                    |_, _, _| (),
                                    move |_, _, window, _| {
                                        let at = |p: Pos| {
                                            point(
                                                px(origin.x + p.x * scale),
                                                px(origin.y + p.y * scale),
                                            )
                                        };
                                        let mut path = gpui_kit::PathBuilder::stroke(px(1.5));
                                        path.move_to(at(mark.points[0]));
                                        if let Some(control) = mark.points.get(2) {
                                            path.curve_to(at(mark.points[1]), at(*control));
                                        } else {
                                            path.line_to(at(mark.points[1]));
                                        }
                                        if let Ok(path) = path.build() {
                                            window.paint_path(path, rgb(0x438fff));
                                        }
                                    },
                                )
                                .size_full(),
                            ),
                    );
                }
                overlay.child(frame_outline(
                    "annotation-selection-box",
                    selection_frame.unwrap(),
                    origin,
                    scale,
                ))
            })
            .when_some(text_box, |overlay, mark| {
                overlay.child(frame_outline(
                    "annotation-text-box-outline",
                    mark.frame(),
                    origin,
                    scale,
                ))
            })
            .when_some(self.editor.marquee.as_ref(), |overlay, marquee| {
                let b = Rect::between(marquee.start, marquee.current);
                overlay.child(
                    div()
                        .id("annotation-marquee")
                        .test_support()
                        .absolute()
                        .left(px(origin.x + b.x * scale))
                        .top(px(origin.y + b.y * scale - TOOLBAR_HEIGHT))
                        .w(px(b.w * scale))
                        .h(px(b.h * scale))
                        .border_1()
                        .border_color(rgb(0x438fff))
                        .bg(gpui_kit::rgba(0x438fff22)),
                )
            })
            .children(handles.into_iter().enumerate().map(|(i, p)| {
                div()
                    .id(("annotation-handle", i))
                    .test_support()
                    .absolute()
                    .left(px(origin.x + p.x * scale - 4.))
                    .top(px(origin.y + p.y * scale - TOOLBAR_HEIGHT - 4.))
                    .size(px(8.))
                    .bg(rgb(0x000000))
                    .border_2()
                    .border_color(rgb(0x438fff))
            }))
            .when_some(self.editor.editing.clone(), |overlay, input| {
                let mark = selected.unwrap();
                input.update(cx, |input, _| input.configure(mark.clone(), origin, scale));
                overlay.child(
                    div()
                        .id("annotation-text-editor")
                        .test_support()
                        .absolute()
                        .inset_0()
                        .child(input),
                )
            })
            .when_some(self.editor.status.clone(), |overlay, status| {
                overlay.child(
                    div()
                        .absolute()
                        .bottom_3()
                        .left(px(64.))
                        .right_3()
                        .p_2()
                        .rounded_md()
                        .bg(cx.theme().background)
                        .text_sm()
                        .child(status),
                )
            })
    }
}

fn projected_mark(
    current: &Mark,
    gesture: &Gesture,
    p: Pos,
    shift: bool,
    fixed_text: bool,
    scale: f32,
) -> Mark {
    let mut result = current.clone();
    apply_mark_gesture(&mut result, gesture, p, shift, fixed_text, scale);
    result
}
fn apply_mark_gesture(
    mark: &mut Mark,
    gesture: &Gesture,
    mut p: Pos,
    shift: bool,
    fixed_text: bool,
    scale: f32,
) {
    match gesture.mode {
        Mode::Draw => {
            if shift && mark.tool.is_shape() {
                let side = (p.x - gesture.start.x)
                    .abs()
                    .max((p.y - gesture.start.y).abs());
                p = Pos::new(
                    gesture.start.x + if p.x < gesture.start.x { -side } else { side },
                    gesture.start.y + if p.y < gesture.start.y { -side } else { side },
                );
            }
            if matches!(mark.tool, Tool::Pen | Tool::Redact) {
                if mark.points.last().unwrap().distance(p) > 1. / scale {
                    mark.points.push(p);
                }
            } else {
                mark.points[1] = p;
                if mark.tool == Tool::Text && fixed_text {
                    mark.wrap = Some((p.x - gesture.start.x).abs().max(12. * mark.style.scale));
                    mark.points[0] = Pos::new(
                        p.x.min(gesture.start.x),
                        gesture.start.y - mark.text_layout().line_height / 2.,
                    );
                }
            }
        }
        Mode::Move => {
            *mark = gesture.original.clone();
            mark.translate(p.x - gesture.start.x, p.y - gesture.start.y);
        }
        Mode::Resize(handle) => selection::resize(mark, &gesture.original, handle, p, shift),
        Mode::Rotate(_) => {
            let c = gesture.original.frame().center();
            let a = (gesture.start.y - c.y).atan2(gesture.start.x - c.x);
            let b = (p.y - c.y).atan2(p.x - c.x);
            let mut angle = gesture.original.rotation + b - a;
            if shift {
                let step = std::f32::consts::PI / 12.;
                angle = (angle / step).round() * step;
            }
            mark.rotation = (angle + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::PI;
        }
        Mode::Bend => {
            *mark = gesture.original.clone();
            let t: f32 = if gesture.original.text.is_empty() {
                0.5
            } else {
                0.25
            };
            let handle = gesture.original.bend_handle();
            let target = Pos::new(
                handle.x + p.x - gesture.start.x,
                handle.y + p.y - gesture.start.y,
            );
            let a = mark.points[0];
            let b = mark.points[1];
            let control = Pos::new(
                (target.x - (1. - t).powi(2) * a.x - t * t * b.x) / (2. * t * (1. - t)),
                (target.y - (1. - t).powi(2) * a.y - t * t * b.y) / (2. * t * (1. - t)),
            );
            if mark.points.len() > 2 {
                mark.points[2] = control;
            } else {
                mark.points.push(control);
            }
            mark.constrain_arrow_label(None);
        }
        Mode::Endpoint(index) => {
            *mark = gesture.original.clone();
            mark.points[index] = p;
            mark.constrain_arrow_label(Some(1 - index));
        }
        Mode::Erase => {}
    }
}

// Editing text shows its wrapping bounds, independently of selection handles.
fn frame_outline(
    id: &'static str,
    frame: crate::annotation::Frame,
    origin: Pos,
    scale: f32,
) -> impl IntoElement {
    let b = frame.world_bounds();
    let corners = frame.corners();
    div()
        .id(id)
        .test_support()
        .absolute()
        .left(px(origin.x + b.x * scale))
        .top(px(origin.y + b.y * scale - TOOLBAR_HEIGHT))
        .w(px(b.w * scale))
        .h(px(b.h * scale))
        .child(
            canvas(
                |_, _, _| (),
                move |_, _, window, _| {
                    let at = |p: Pos| point(px(origin.x + p.x * scale), px(origin.y + p.y * scale));
                    let mut path = gpui_kit::PathBuilder::stroke(px(1.));
                    path.move_to(at(corners[0]));
                    for p in [corners[1], corners[2], corners[3], corners[0]] {
                        path.line_to(at(p));
                    }
                    if let Ok(path) = path.build() {
                        window.paint_path(path, rgb(0x438fff));
                    }
                },
            )
            .size_full(),
        )
}

#[cfg(feature = "walkthrough-screenshots")]
impl ImagePreview {
    #[allow(dead_code)]
    pub fn walkthrough_text_edit(
        &mut self,
        finished: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if finished {
            self.finish_text(cx);
            self.clear_selection();
        } else {
            self.session_generation += 1;
            self.saving = None;
            self.editor = Editor::default();
            let doc = self.doc_mut().unwrap();
            doc.marks[1].style.font_size = 44.;
            doc.marks[1].set_text("Hello".into());
            doc.marks[3].set_text("One layout,\neven while editing.".into());
            self.editor.style = self.doc().unwrap().marks[3].style.clone().with_scale(1.);
            self.edit_text(3, window, cx);
        }
        self.refresh_annotations(cx);
    }
    #[allow(dead_code)]
    pub fn select_walkthrough_group(&mut self, multiple: bool, cx: &mut Context<Self>) {
        self.set_selection(if multiple { vec![1, 2, 3, 4] } else { vec![3] });
        self.editor.tool = Tool::Select;
        self.editor.style = self.doc().unwrap().marks[self.editor.selected.unwrap()]
            .style
            .clone()
            .with_scale(1.);
        self.refresh_annotations(cx);
    }
    #[allow(dead_code)] // Shared with the screenshot binary when the feature is enabled.
    pub fn for_walkthrough(cx: &mut Context<Self>) -> Self {
        use crate::annotation::{Dash, Fill};
        let attachment = StagedAttachment::from_bytes(
            "rocket-cat.png".into(),
            include_bytes!("../../packaging/rocket-cat-transparent.png").to_vec(),
            None,
        )
        .unwrap();
        let id = attachment.id;
        let mut preview = Self::new(vec![attachment], id, cx);
        let mut title = Mark::new(
            Tool::Text,
            Pos::new(-40., -32.),
            Style {
                font_size: 30.,
                color: 0,
                ..Default::default()
            },
        );
        title.text = "Ready for takeoff".into();
        let mut arrow = Mark::new(
            Tool::Arrow,
            Pos::new(-15., 300.),
            Style {
                color: 5,
                width: crate::annotation::STROKE_WIDTHS[0],
                start: crate::annotation::Cap::Triangle,
                ..Default::default()
            },
        );
        arrow.points = vec![
            Pos::new(-15., 300.),
            Pos::new(250., 230.),
            Pos::new(155., 350.),
        ];
        let mut star = Mark::new(
            Tool::Star,
            Pos::new(2., 24.),
            Style {
                color: 6,
                fill: Fill::Hatch,
                width: crate::annotation::STROKE_WIDTHS[0],
                ..Default::default()
            },
        );
        star.points[1] = Pos::new(105., 120.);
        let mut rectangle = Mark::new(
            Tool::Rectangle,
            Pos::new(310., 72.),
            Style {
                color: 5,
                width: crate::annotation::STROKE_WIDTHS[0],
                dash: Dash::Dashed,
                fill: Fill::Soft,
                font_size: 18.,
                ..Default::default()
            },
        );
        rectangle.points[1] = Pos::new(463., 225.);
        rectangle.rotation = -0.16;
        rectangle.text = "A larger shape label".into();
        let mut note = Mark::new(
            Tool::Text,
            Pos::new(-60., 342.),
            Style {
                font_size: 22.,
                color: 5,
                ..Default::default()
            },
        );
        note.text = "A little boost.".into();
        note.wrap = Some(140.);
        let mut pixelation = Mark::new(
            Tool::Redact,
            Pos::new(190., 150.),
            Style {
                width: 96.,
                ..Default::default()
            },
        );
        pixelation.points = vec![
            Pos::new(190., 150.),
            Pos::new(260., 200.),
            Pos::new(380., 190.),
        ];
        let doc = preview.doc_mut().unwrap();
        doc.marks = vec![title, arrow, star, rectangle, note, pixelation];
        let style = doc.marks[3].style.clone().with_scale(1.);
        preview.editor.selected = Some(3);
        preview.editor.style = style;
        preview.zoom = 0.8;
        preview.refresh_annotations(cx);
        preview
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::test::TestWindowExt as _;
    use gpui_kit::{Modifiers, TestAppContext, VisualTestContext};

    fn comparison_attachment() -> StagedAttachment {
        crate::image_composer::combine([
            fixture_attachment_size(400, 300),
            fixture_attachment_size(400, 300),
        ])
        .unwrap()
    }
    #[gpui_kit::test]
    fn comparison_annotations_stop_before_the_caption_row(cx: &mut TestAppContext) {
        for key in ["a", "r", "p"] {
            let (view, cx) = editor_for(cx, comparison_attachment());
            let bottom = cx.update(|_, cx| {
                let comparison = view.read(cx).doc().unwrap().comparison.as_ref().unwrap();
                comparison.panels[0].y + comparison.panels[0].h
            });
            cx.simulate_keystrokes(key);
            drag(
                &view,
                cx,
                Pos::new(100., 100.),
                Pos::new(230., bottom + 70.),
            );
            cx.update(|_, cx| {
                let mark = &view.read(cx).doc().unwrap().marks[0];
                let bounds = mark.visual_bounds();
                assert!(
                    bounds.y + bounds.h <= bottom + 0.01,
                    "{key} went below images: {bounds:?}, bottom={bottom}"
                );
            });
        }
    }
    #[gpui_kit::test]
    fn comparison_caption_controls_do_not_pause_an_active_arrow_drag(cx: &mut TestAppContext) {
        let (view, cx) = editor_for(cx, comparison_attachment());
        cx.simulate_keystrokes("a");
        let start = pointer(&view, cx, Pos::new(100., 100.));
        cx.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
        for p in [
            Pos::new(190., 330.),
            Pos::new(240., 350.),
            Pos::new(270., 400.),
            Pos::new(230., 240.),
        ] {
            cx.update(|window, cx| window.render_frame(cx));
            let position = pointer(&view, cx, p);
            cx.simulate_mouse_move(position, Some(MouseButton::Left), Modifiers::default());
            cx.update(|_, cx| {
                let preview = view.read(cx);
                let mark = &preview.doc().unwrap().marks[0];
                assert_eq!(mark.points[0], Pos::new(100., 100.));
                if p.y > 316. {
                    assert!(
                        mark.points[1].y > 313.
                            && mark.visual_bounds().y + mark.visual_bounds().h <= 316.01,
                        "pointer paused over the caption"
                    );
                } else {
                    assert!(mark.points[1].distance(p) < 0.1);
                }
                assert!(preview.editor.comparison_edit.is_none());
            });
        }
        let end = pointer(&view, cx, Pos::new(230., 240.));
        cx.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
    }
    #[gpui_kit::test]
    fn comparison_shapes_stop_when_moved_or_resized_and_dragging_resumes_above_the_edge(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = editor_for(cx, comparison_attachment());
        cx.simulate_keystrokes("r");
        drag(&view, cx, Pos::new(100., 100.), Pos::new(200., 200.));
        select_at(&view, cx, Pos::new(100., 150.));
        let bottom = 316.;
        drag(
            &view,
            cx,
            Pos::new(150., 150.),
            Pos::new(150., bottom + 80.),
        );
        let moved = cx.update(|_, cx| view.read(cx).doc().unwrap().marks[0].clone());
        assert_eq!(moved.bounds().h, 100.);
        assert!((moved.visual_bounds().y + moved.visual_bounds().h - bottom).abs() < 0.01);
        cx.simulate_keystrokes("cmd-z");
        select_at(&view, cx, Pos::new(100., 150.));
        let start = pointer(&view, cx, Pos::new(200., 200.));
        cx.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
        for p in [
            Pos::new(270., bottom + 30.),
            Pos::new(280., bottom + 80.),
            Pos::new(240., 250.),
        ] {
            let at = pointer(&view, cx, p);
            cx.simulate_mouse_move(at, Some(MouseButton::Left), Modifiers::default());
            cx.update(|_, cx| {
                let mark = &view.read(cx).doc().unwrap().marks[0];
                assert!(mark.visual_bounds().y + mark.visual_bounds().h <= bottom + 0.01);
                assert!(
                    mark.points[0].distance(Pos::new(100., 100.)) < 0.1,
                    "resize must preserve the opposite anchor"
                );
                if p.y == 250. {
                    assert!(
                        (mark.bounds().y + mark.bounds().h - 250.).abs() < 0.1,
                        "drag must resume immediately above the edge"
                    );
                }
            });
        }
        let end = pointer(&view, cx, Pos::new(240., 250.));
        cx.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
    }
    #[gpui_kit::test]
    fn comparison_group_moves_and_resizes_stop_at_the_image_bottom(cx: &mut TestAppContext) {
        let (view, cx) = editor_for(cx, comparison_attachment());
        for (a, b) in [
            (Pos::new(100., 100.), Pos::new(180., 150.)),
            (Pos::new(240., 170.), Pos::new(320., 240.)),
        ] {
            cx.simulate_keystrokes("r");
            drag(&view, cx, a, b);
        }
        cx.simulate_keystrokes("cmd-a");
        drag(&view, cx, Pos::new(200., 160.), Pos::new(200., 430.));
        cx.update(|_, cx| {
            let marks = &view.read(cx).doc().unwrap().marks;
            for mark in marks {
                let b = mark.visual_bounds();
                assert!(b.y + b.h <= 316.01);
            }
            assert!(
                (marks[1].points[0].y - marks[0].points[0].y - 70.).abs() < 0.01,
                "group spacing must stay intact"
            );
        });
        cx.simulate_keystrokes("cmd-z cmd-a");
        drag(&view, cx, Pos::new(320., 240.), Pos::new(400., 420.));
        cx.update(|_, cx| {
            for mark in &view.read(cx).doc().unwrap().marks {
                let b = mark.visual_bounds();
                assert!(b.y + b.h <= 316.01);
            }
        });
    }
    #[gpui_kit::test]
    fn comparison_rejects_drawing_below_images_but_ordinary_images_keep_expansion(
        cx: &mut TestAppContext,
    ) {
        let (view, window) = editor_for(cx, comparison_attachment());
        window.simulate_keystrokes("a");
        drag(&view, window, Pos::new(500., 420.), Pos::new(600., 460.));
        window.update(|_, cx| assert!(view.read(cx).doc().unwrap().marks.is_empty()));
        let (view, window) = editor_window(cx);
        window.simulate_keystrokes("a");
        drag(&view, window, Pos::new(100., 100.), Pos::new(230., 390.));
        window.update(|_, cx| {
            let b = view.read(cx).doc().unwrap().marks[0].visual_bounds();
            assert!(b.y + b.h > 300.);
        });
    }
    #[gpui_kit::test]
    fn comparison_caption_keeps_its_width_while_typing_spaces(cx: &mut TestAppContext) {
        let (_, cx) = editor_for(cx, comparison_attachment());
        let id = ("comparison-caption", 0_usize);
        cx.update(|window, cx| window.double_click(id, cx));
        cx.simulate_input("Original interface");
        let before = cx.update(|window, cx| {
            window.render_frame(cx);
            window.find(id).bounds()
        });
        cx.simulate_keystrokes("end");
        cx.simulate_input(" extra words");
        cx.update(|window, cx| {
            window.render_frame(cx);
            let after = window.find(id).bounds();
            assert_eq!(
                after.origin, before.origin,
                "typing must keep the caption anchor fixed"
            );
            assert_eq!(
                after.size.width, before.size.width,
                "caption width must not follow the text"
            );
        });
    }
    #[gpui_kit::test]
    fn comparison_caption_wraps_and_shift_enter_keeps_a_shared_baseline(cx: &mut TestAppContext) {
        let (view, cx) = editor_for(cx, comparison_attachment());
        cx.update(|window, cx| window.double_click(("comparison-caption", 0_usize), cx));
        cx.simulate_input("Original interface");
        cx.simulate_keystrokes("end shift-enter");
        cx.simulate_input(
            "A detailed explanation that is longer than a single line in this caption column",
        );
        cx.update(|_, cx| {
            let comparison = view.read(cx).doc().unwrap().comparison.as_ref().unwrap();
            assert!(comparison.labels[0].contains('\n'));
            assert_eq!(comparison.caption_rect(0).y, comparison.caption_rect(1).y);
            assert!(comparison.caption_rect(0).h > comparison.label_mark(1).bounds().h);
            assert!(view.read(cx).editor.comparison_edit.is_some());
        });
        cx.simulate_keystrokes("enter");
        cx.update(|_, cx| assert!(view.read(cx).editor.comparison_edit.is_none()));
    }
    #[gpui_kit::test]
    fn space_pan_can_start_on_a_comparison_caption(cx: &mut TestAppContext) {
        let (view, cx) = editor_for(cx, comparison_attachment());
        let p = cx.update(|window, cx| {
            window.render_frame(cx);
            window
                .find(("comparison-caption", 0_usize))
                .bounds()
                .center()
        });
        cx.simulate_keystrokes("space");
        cx.simulate_mouse_down(p, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_move(
            p + point(px(40.), px(20.)),
            Some(MouseButton::Left),
            Modifiers::default(),
        );
        cx.update(|_, cx| {
            assert_eq!(
                view.read(cx).offset,
                point(px(40.), px(20.)),
                "caption controls must let pan gestures through"
            )
        });
    }
    #[gpui_kit::test]
    fn comparison_caption_editing_is_inline_and_part_of_undo_redo(cx: &mut TestAppContext) {
        let (view, cx) = editor_for(cx, comparison_attachment());
        let id = ("comparison-caption", 0_usize);
        let original = cx.update(|window, cx| {
            window.render_frame(cx);
            window.find(id).bounds()
        });
        cx.update(|window, cx| window.double_click(id, cx));
        cx.run_until_parked();
        cx.update(|window, cx| {
            window.render_frame(cx);
            assert_eq!(
                window.find(id).bounds(),
                original,
                "selecting the caption must not resize its column"
            );
            assert!(view.read(cx).editor.comparison_edit.is_some());
        });
        cx.simulate_input("Original interface");
        cx.simulate_keystrokes("escape");
        cx.update(|_, cx| {
            let preview = view.read(cx);
            assert_eq!(
                preview.doc().unwrap().comparison.as_ref().unwrap().labels[0],
                "Original interface"
            );
            assert!(
                preview.doc().unwrap().marks.is_empty(),
                "labels are separate from annotation marks"
            );
            assert!(preview.has_unsaved_changes());
            assert!(preview.editor.comparison_edit.is_none());
        });
        cx.simulate_keystrokes("cmd-z");
        cx.update(|_, cx| {
            assert_eq!(
                view.read(cx)
                    .doc()
                    .unwrap()
                    .comparison
                    .as_ref()
                    .unwrap()
                    .labels[0],
                "Before"
            )
        });
        cx.simulate_keystrokes("cmd-shift-z");
        cx.update(|_, cx| {
            assert_eq!(
                view.read(cx)
                    .doc()
                    .unwrap()
                    .comparison
                    .as_ref()
                    .unwrap()
                    .labels[0],
                "Original interface"
            )
        });
    }
    #[gpui_kit::test]
    fn comparison_captions_block_drawing_and_arrows_can_cross_both_images(cx: &mut TestAppContext) {
        let (view, cx) = editor_for(cx, comparison_attachment());
        for key in ["r", "p", "a"] {
            cx.simulate_keystrokes(key);
            let tab = cx.update(|window, cx| {
                window.render_frame(cx);
                window.find(("comparison-caption", 0_usize)).bounds()
            });
            cx.simulate_mouse_down(tab.center(), MouseButton::Left, Modifiers::default());
            cx.simulate_mouse_move(
                tab.center() + point(px(120.), px(100.)),
                Some(MouseButton::Left),
                Modifiers::default(),
            );
            cx.simulate_mouse_up(
                tab.center() + point(px(120.), px(100.)),
                MouseButton::Left,
                Modifiers::default(),
            );
            cx.update(|_, cx| {
                assert!(
                    view.read(cx).doc().unwrap().marks.is_empty(),
                    "clicking a title must not draw with {key}"
                )
            });
        }
        cx.simulate_keystrokes("a");
        drag(&view, cx, Pos::new(100., 180.), Pos::new(650., 180.));
        cx.update(|_, cx| {
            let doc = view.read(cx).doc().unwrap();
            assert_eq!(doc.marks.len(), 1);
            assert_eq!(doc.marks[0].tool, Tool::Arrow);
            assert!(doc.marks[0].points[0].x < doc.comparison.as_ref().unwrap().panels[1].x);
            assert!(doc.marks[0].points[1].x > doc.comparison.as_ref().unwrap().panels[1].x);
        });
    }
    #[gpui_kit::test]
    fn comparison_rename_saves_and_discard_restores_saved_captions(cx: &mut TestAppContext) {
        let original = comparison_attachment();
        let session = original.image().unwrap().annotation_session.clone();
        let id = original.id;
        let (view, cx) = editor_for(cx, original);
        let applied = Rc::new(RefCell::new(None));
        let capture = applied.clone();
        let _subscription = cx.update(|_, cx| {
            cx.subscribe(&view, move |_, event: &AppliedAnnotation, _| {
                *capture.borrow_mut() = Some(event.attachment.clone())
            })
        });
        cx.update(|window, cx| window.double_click(("comparison-caption", 1_usize), cx));
        cx.simulate_input("Updated");
        cx.simulate_keystrokes("enter");
        cx.update(|window, cx| view.update(cx, |preview, cx| preview.apply_annotation(window, cx)));
        cx.run_until_parked();
        let mut staged = applied
            .borrow_mut()
            .take()
            .expect("label edits must export through ordinary Save");
        staged.id = id;
        if let crate::model::Media::Image(image) = &mut staged.media {
            image.annotation_session = session;
        }
        cx.update(|window, cx| {
            view.update(cx, |preview, cx| {
                preview.annotation_saved(id, Ok(()), window, cx);
                assert!(preview.images.is_empty());
                preview.show(vec![staged.clone()], id, cx);
                assert_eq!(
                    preview.doc().unwrap().comparison.as_ref().unwrap().labels[1],
                    "Updated"
                );
                assert!(!preview.has_unsaved_changes());
                let doc = preview.doc_mut().unwrap();
                doc.checkpoint();
                doc.comparison.as_mut().unwrap().labels[1] = "Unsaved title".into();
                preview.clear(cx);
                preview.show(vec![staged.clone()], id, cx);
                assert_eq!(
                    preview.doc().unwrap().comparison.as_ref().unwrap().labels[1],
                    "Updated"
                );
                assert!(!preview.has_unsaved_changes());
                assert!(!preview.doc().unwrap().can_undo());
            })
        });
    }
    #[gpui_kit::test]
    fn ordinary_images_have_no_comparison_captions(cx: &mut TestAppContext) {
        let (view, cx) = editor_window(cx);
        cx.update(|window, cx| {
            window.render_frame(cx);
            assert!(window.try_find(("comparison-caption", 0_usize)).is_none());
            assert!(view.read(cx).doc().unwrap().comparison.is_none());
        });
    }
    fn fixture_attachment() -> StagedAttachment {
        fixture_attachment_size(400, 300)
    }
    fn fixture_attachment_size(width: u32, height: u32) -> StagedAttachment {
        let mut bytes = std::io::Cursor::new(Vec::new());
        image::RgbaImage::from_pixel(width, height, image::Rgba([40, 45, 55, 255]))
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        StagedAttachment::from_bytes("test.png".into(), bytes.into_inner(), None).unwrap()
    }
    fn editor_window(cx: &mut TestAppContext) -> (Entity<ImagePreview>, &mut VisualTestContext) {
        editor_for(cx, fixture_attachment())
    }
    fn editor_for(
        cx: &mut TestAppContext,
        attachment: StagedAttachment,
    ) -> (Entity<ImagePreview>, &mut VisualTestContext) {
        cx.update(|cx| {
            gpui_kit::init(cx);
            // Pointer assertions must use resting dialog bounds, independent of
            // wall-clock entrance animation progress under parallel test load.
            cx.set_reduce_motion(true);
            super::super::init_keybindings(cx);
            crate::application_menu::init(cx);
        });
        let id = attachment.id;
        let captured = Rc::new(RefCell::new(None));
        let capture = captured.clone();
        let (_, cx) = cx.add_window_view(move |window, cx| {
            let view = cx.new(|cx| ImagePreview::new(vec![attachment], id, cx));
            let focus = view.read(cx).focus_handle.clone();
            window.focus(&focus, cx);
            *capture.borrow_mut() = Some(view.clone());
            gpui_kit::base::Root::new(view, window, cx)
        });
        let view = captured.borrow().clone().unwrap();
        cx.update(|window, cx| window.draw(cx).clear(cx));
        (view, cx)
    }
    fn pointer(view: &Entity<ImagePreview>, cx: &mut VisualTestContext, p: Pos) -> Point<Pixels> {
        cx.update(|window, cx| {
            let (origin, scale) = view.read(cx).transform(window);
            point(px(origin.x + p.x * scale), px(origin.y + p.y * scale))
        })
    }
    fn drag(view: &Entity<ImagePreview>, cx: &mut VisualTestContext, a: Pos, b: Pos) {
        let start = pointer(view, cx, a);
        let end = pointer(view, cx, b);
        cx.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
        cx.update(|_, cx| {
            view.update(cx, |preview, _| {
                if let Some(gesture) = &mut preview.editor.gesture {
                    gesture.started -= Duration::from_millis(160);
                }
            });
        });
        cx.update(|window, cx| {
            use gpui_kit::InputEvent as _;
            window.dispatch_event(
                MouseMoveEvent {
                    position: end,
                    pressed_button: Some(MouseButton::Left),
                    modifiers: Modifiers::default(),
                }
                .to_platform_input(),
                cx,
            );
        });
        cx.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();
    }
    fn select_at(view: &Entity<ImagePreview>, cx: &mut VisualTestContext, p: Pos) {
        cx.simulate_keystrokes("v");
        let at = pointer(view, cx, p);
        cx.simulate_click(at, Modifiers::default());
    }
    #[gpui_kit::test]
    fn drawing_at_different_zooms_keeps_matching_strokes_and_text(cx: &mut TestAppContext) {
        assert_matching_sizes(cx, fixture_attachment());
    }
    #[gpui_kit::test]
    fn fitted_images_keep_matching_strokes_and_text_after_zooming(cx: &mut TestAppContext) {
        assert_matching_sizes(cx, fixture_attachment_size(4000, 3000));
    }
    fn assert_matching_sizes(cx: &mut TestAppContext, attachment: StagedAttachment) {
        let (view, cx) = editor_for(cx, attachment);
        let unit = cx.update(|window, cx| {
            let preview = view.read(cx);
            let unit = preview.annotation_scale(window);
            assert!((unit * preview.fit_scale(window) - 1.).abs() < 0.001);
            unit
        });
        for tool in [
            Tool::Pen,
            Tool::Rectangle,
            Tool::Arrow,
            Tool::Line,
            Tool::Text,
            Tool::Redact,
        ] {
            cx.update(|window, cx| {
                view.update(cx, |preview, cx| {
                    preview.finish_text(cx);
                    preview.doc_mut().unwrap().revert();
                    preview.editor.style = Style::default();
                    preview.editor.locked = true;
                    preview.select_tool(tool, window, cx);
                });
            });
            for actual_scale in [0.25, 1., 2.] {
                let center = cx.update(|window, cx| {
                    view.update(cx, |preview, cx| {
                        preview.set_zoom_at(
                            actual_scale / preview.fit_scale(window),
                            ImagePreview::viewport_center(window),
                            window,
                            cx,
                        );
                        preview.image_pos(ImagePreview::viewport_center(window), window)
                    })
                });
                drag(
                    &view,
                    cx,
                    center,
                    Pos::new(center.x + 40. / actual_scale, center.y + 30. / actual_scale),
                );
                if tool == Tool::Text {
                    cx.simulate_input("Annotation");
                    cx.simulate_keystrokes("escape");
                }
                cx.update(|window, cx| {
                    let preview = view.read(cx);
                    let marks = &preview.doc().unwrap().marks;
                    let first = &marks[0];
                    let latest = marks.last().unwrap();
                    if tool == Tool::Redact {
                        assert!(
                            (latest.style.width * preview.transform(window).1
                                - preview.pixelate_width())
                            .abs()
                                < 0.01,
                            "Pixelation keeps the same screen diameter at every creation zoom"
                        );
                    } else {
                        assert_eq!(
                            first.style.width, latest.style.width,
                            "{tool:?}: drawing after zooming must keep the same border thickness"
                        );
                        assert_eq!(
                            first.style.font_size, latest.style.font_size,
                            "{tool:?}: drawing after zooming must keep the same text size"
                        );
                        assert_eq!(first.style.scale, latest.style.scale);
                        assert_eq!(latest.style.scale, unit);
                    }
                    assert!(preview.selected_indices().is_empty());
                });
            }
            cx.update(|window, cx| {
                view.update(cx, |preview, cx| {
                    if tool != Tool::Redact {
                        preview.set_selection(vec![0, 1, 2]);
                        preview.editor.style =
                            preview.doc().unwrap().marks[0].style.clone().with_scale(1.);
                        preview.editor.tool = Tool::Select;
                        preview.change_style(|s| s.color = 7, cx);
                    }
                });
                window.render_frame(cx);
                if tool != Tool::Redact {
                    window.click(("annotation-size", 2usize), cx);
                    let preview = view.read(cx);
                    for mark in &preview.doc().unwrap().marks {
                        assert!(
                            (mark.style.width / mark.style.scale
                                - crate::annotation::STROKE_WIDTHS[2])
                                .abs()
                                < 0.001
                        );
                    }
                    assert_eq!(
                        window.find(("annotation-size", 2usize)).checked(),
                        Some(true)
                    );
                }
            });
        }
    }

    #[gpui_kit::test]
    fn initial_fit_shrinks_large_images_but_does_not_enlarge_small_images(cx: &mut TestAppContext) {
        let (view, cx) = editor_window(cx);
        cx.update(|window, cx| {
            view.update(cx, |preview, cx| {
                assert_eq!(preview.transform(window).1, 1.);
                let natural_size = preview.images[0].pixel_size;
                preview.images[0].pixel_size = (6000, 3000);
                let fit = preview.transform(window).1;
                assert!(fit < 1.);
                assert_eq!(fit, preview.fit_scale(window));
                preview.actual_size(&ActualSize, window, cx);
                assert!((preview.transform(window).1 - 1.).abs() < 0.001);
                preview.fit(cx);
                assert!((preview.transform(window).1 - fit).abs() < 0.001);
                preview.images[0].pixel_size = natural_size;
            });
        });
    }

    #[gpui_kit::test]
    fn clicking_closed_shapes_places_defaults_without_selection(cx: &mut TestAppContext) {
        let (view, cx) = editor_window(cx);
        let at = Pos::new(200., 150.);
        for locked in [false, true] {
            for zoom in [0.7, 1.8] {
                for tool in crate::annotation::SHAPES
                    .into_iter()
                    .filter(|t| t.is_shape())
                {
                    cx.update(|window, cx| {
                        view.update(cx, |preview, cx| {
                            preview.doc_mut().unwrap().revert();
                            preview.select_tool(tool, window, cx);
                            preview.zoom = zoom;
                            preview.editor.locked = locked;
                            preview.editor.style.color = 5;
                            preview.editor.style.fill = if locked {
                                crate::annotation::Fill::Solid
                            } else {
                                crate::annotation::Fill::None
                            };
                        });
                    });
                    let position = pointer(&view, cx, at);
                    cx.simulate_click(position, Modifiers::default());
                    cx.update(|_, cx| {
                        view.update(cx, |preview, _| {
                            assert!(preview.selected_indices().is_empty());
                            assert_eq!(
                                preview.editor.tool,
                                if locked { tool } else { Tool::Select }
                            );
                            let doc = preview.doc_mut().unwrap();
                            assert_eq!(doc.marks.len(), 1, "{tool:?}");
                            let mark = doc.marks[0].clone();
                            assert!(mark.bounds().center().distance(at) < 0.001, "{tool:?}");
                            assert!(
                                (mark.bounds().w / mark.style.scale
                                    - if tool == Tool::Oval { 120. } else { 160. })
                                .abs()
                                    < 0.001
                            );
                            assert!(
                                (mark.bounds().h / mark.style.scale
                                    - if tool == Tool::Oval { 200. } else { 160. })
                                .abs()
                                    < 0.001
                            );
                            if tool == Tool::Oval {
                                assert!(mark.bounds().h > mark.bounds().w);
                            }
                            assert_eq!(mark.style.color, 5);
                            assert_eq!(
                                mark.style.fill,
                                if locked {
                                    crate::annotation::Fill::Solid
                                } else {
                                    crate::annotation::Fill::None
                                }
                            );
                            doc.undo();
                            assert!(doc.marks.is_empty());
                            assert!(!doc.can_undo());
                            doc.redo();
                            assert_eq!(doc.marks, vec![mark]);
                        });
                    });
                }
            }
        }
    }
    #[gpui_kit::test]
    fn arrow_click_preserves_redo_and_drag_creates_one_undo_step(cx: &mut TestAppContext) {
        let (view, cx) = editor_window(cx);
        for locked in [false, true] {
            cx.update(|window, cx| {
                view.update(cx, |preview, cx| {
                    preview.doc_mut().unwrap().revert();
                    preview.select_tool(Tool::Rectangle, window, cx);
                });
            });
            drag(&view, cx, Pos::new(100., 100.), Pos::new(250., 200.));
            cx.update(|window, cx| {
                view.update(cx, |preview, cx| {
                    preview.history(false, cx);
                    preview.select_tool(Tool::Arrow, window, cx);
                    preview.editor.locked = locked;
                });
            });
            let at = pointer(&view, cx, Pos::new(150., 150.));
            cx.simulate_click(at, Modifiers::default());
            cx.update(|window, cx| {
                view.update(cx, |preview, cx| {
                    let doc = preview.doc_mut().unwrap();
                    assert!(doc.marks.is_empty());
                    assert!(!doc.can_undo());
                    assert!(doc.can_redo(), "An ignored arrow click must preserve redo");
                    doc.redo();
                    assert_eq!(doc.marks[0].tool, Tool::Rectangle);
                    doc.undo();
                    assert!(preview.selected_indices().is_empty());
                    assert_eq!(preview.editor.tool, Tool::Arrow);
                    preview.select_tool(Tool::Arrow, window, cx);
                });
            });
            drag(&view, cx, Pos::new(100., 100.), Pos::new(250., 200.));
            cx.update(|_, cx| {
                view.update(cx, |preview, _| {
                    let doc = preview.doc_mut().unwrap();
                    let arrow = doc.marks[0].clone();
                    assert_eq!(arrow.tool, Tool::Arrow);
                    assert!(!doc.can_redo());
                    doc.undo();
                    assert!(doc.marks.is_empty());
                    assert!(!doc.can_undo());
                    doc.redo();
                    assert_eq!(doc.marks, vec![arrow]);
                });
            });
        }
    }
    #[gpui_kit::test]
    fn line_click_makes_overlapping_endpoints_that_pull_apart_on_first_selection_drag(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = editor_window(cx);
        let at = Pos::new(150., 130.);
        let end = Pos::new(290., 200.);
        for locked in [false, true] {
            cx.update(|window, cx| {
                view.update(cx, |preview, cx| {
                    preview.doc_mut().unwrap().revert();
                    preview.select_tool(Tool::Line, window, cx);
                    preview.editor.locked = locked;
                });
            });
            let position = pointer(&view, cx, at);
            cx.simulate_click(position, Modifiers::default());
            cx.update(|_, cx| {
                let preview = view.read(cx);
                assert!(preview.selected_indices().is_empty());
                let dot = &preview.doc().unwrap().marks[0];
                assert_eq!(dot.points.len(), 2);
                assert!(dot.is_line_dot());
                assert!(dot.points[0].distance(at) < 0.001);
            });
            cx.simulate_keystrokes("v");
            drag(&view, cx, at, end);
            cx.update(|window, cx| {
                window.render_frame(cx);
                let preview = view.read(cx);
                let line = &preview.doc().unwrap().marks[0];
                assert!(line.points[0].distance(at) < 0.001);
                assert!(line.points[1].distance(end) < 0.001);
                assert!(!line.is_line_dot());
                assert_eq!(preview.selected_indices(), vec![0]);
                assert!(window.try_find("annotation-selection-box").is_none());
            });
            cx.update(|_, cx| {
                view.update(cx, |preview, cx| {
                    preview.history(false, cx);
                    assert!(preview.doc().unwrap().marks[0].is_line_dot());
                });
            });
            // Clicking to select first must leave the overlapping endpoint draggable too.
            let position = pointer(&view, cx, at);
            cx.simulate_click(position, Modifiers::default());
            drag(&view, cx, at, end);
            cx.update(|_, cx| {
                view.update(cx, |preview, cx| {
                    let doc = preview.doc().unwrap();
                    assert!(doc.marks[0].points[0].distance(at) < 0.001);
                    assert!(doc.marks[0].points[1].distance(end) < 0.001);
                    preview.history(false, cx);
                    assert!(preview.doc().unwrap().marks[0].is_line_dot());
                    preview.history(false, cx);
                    assert!(preview.doc().unwrap().marks.is_empty());
                    assert!(!preview.doc().unwrap().can_undo());
                });
            });
        }
    }
    #[gpui_kit::test]
    fn live_drawing_hides_selection_chrome_then_finished_objects_can_be_selected(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = editor_window(cx);
        for locked in [false, true] {
            for tool in [
                Tool::Rectangle,
                Tool::Ellipse,
                Tool::Pen,
                Tool::Arrow,
                Tool::Line,
            ] {
                cx.update(|window, cx| {
                    view.update(cx, |preview, cx| {
                        preview.select_tool(tool, window, cx);
                        preview.doc_mut().unwrap().revert();
                        preview.editor.locked = locked;
                        preview.refresh_annotations(cx);
                    });
                });
                let start = pointer(&view, cx, Pos::new(100., 100.));
                let end = pointer(&view, cx, Pos::new(220., 180.));
                cx.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
                cx.update(|window, cx| {
                    use gpui_kit::InputEvent as _;
                    window.dispatch_event(
                        MouseMoveEvent {
                            position: end,
                            pressed_button: Some(MouseButton::Left),
                            modifiers: Modifiers::default(),
                        }
                        .to_platform_input(),
                        cx,
                    );
                    window.render_frame(cx);
                    let preview = view.read(cx);
                    assert!(matches!(
                        preview.editor.gesture.as_ref().unwrap().mode,
                        Mode::Draw
                    ));
                    assert_eq!(preview.doc().unwrap().marks.len(), 1);
                    assert!(preview.doc().unwrap().marks[0].bounds().w > 100.);
                    assert!(
                        window.try_find("annotation-selection-box").is_none(),
                        "{tool:?}"
                    );
                    assert!(
                        window.try_find("annotation-arrow-selection").is_none(),
                        "{tool:?}"
                    );
                    assert!(
                        window.try_find(("annotation-handle", 0usize)).is_none(),
                        "{tool:?}"
                    );
                });
                cx.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
                cx.update(|window, cx| {
                    assert!(view.read(cx).editor.selected.is_none());
                    assert_eq!(
                        view.read(cx).editor.tool,
                        if locked { tool } else { Tool::Select }
                    );
                    window.render_frame(cx);
                    assert!(window.try_find("annotation-selection-box").is_none());
                    assert!(window.try_find("annotation-arrow-selection").is_none());
                    assert!(window.try_find(("annotation-handle", 0usize)).is_none());
                });
                cx.update(|window, cx| {
                    view.update(cx, |preview, cx| {
                        preview.select_tool(Tool::Select, window, cx)
                    });
                });
                let border = if tool == Tool::Ellipse {
                    pointer(&view, cx, Pos::new(100., 140.))
                } else {
                    start
                };
                cx.simulate_click(border, Modifiers::default());
                cx.update(|window, cx| {
                    assert_eq!(
                        view.read(cx).editor.selected,
                        Some(0),
                        "{tool:?}, locked {locked}"
                    );
                    window.render_frame(cx);
                    if matches!(tool, Tool::Arrow | Tool::Line) {
                        window.find("annotation-arrow-selection");
                        assert!(window.try_find("annotation-selection-box").is_none());
                        window.find(("annotation-handle", 2usize));
                    } else {
                        window.find("annotation-selection-box");
                        window.find(("annotation-handle", 3usize));
                        assert!(window.try_find(("annotation-handle", 4usize)).is_none());
                    }
                });
            }
        }
    }

    #[gpui_kit::test]
    fn locked_drawing_styles_prepare_the_next_object_without_changing_finished_objects(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = editor_window(cx);
        for (key, tool) in [("r", Tool::Rectangle), ("p", Tool::Pen), ("a", Tool::Arrow)] {
            cx.update(|_, cx| {
                view.update(cx, |preview, cx| {
                    preview.doc_mut().unwrap().revert();
                    preview.clear_selection();
                    preview.editor.style = Style::default();
                    preview.editor.locked = false;
                    preview.refresh_annotations(cx);
                });
            });
            cx.simulate_keystrokes(key);
            cx.update(|window, cx| window.click("annotation-lock", cx));
            drag(&view, cx, Pos::new(100., 100.), Pos::new(180., 180.));
            let finished = cx.update(|window, cx| {
                let preview = view.read(cx);
                assert_eq!(preview.editor.tool, tool);
                assert!(preview.selected_indices().is_empty());
                assert!(window.try_find("annotation-selection-box").is_none());
                preview.doc().unwrap().marks[0].clone()
            });
            cx.update(|window, cx| {
                window.click(("annotation-color", 7usize), cx);
                window.click(("annotation-size", 0usize), cx);
                window.click(("annotation-dash", 2usize), cx);
                if tool.is_shape() {
                    window.click(("annotation-fill", 2usize), cx);
                }
                if tool == Tool::Arrow {
                    window.click("annotation-start-cap", cx);
                    window.click("annotation-end-cap", cx);
                }
            });
            let opacity =
                cx.update(|window, _| window.find("annotation-opacity").bounds().center());
            cx.simulate_click(opacity, Modifiers::default());
            let next_style = cx.update(|_, cx| {
                let preview = view.read(cx);
                assert_eq!(preview.doc().unwrap().marks, vec![finished.clone()]);
                assert!(preview.selected_indices().is_empty());
                assert_eq!(preview.editor.style.color, 7);
                assert_eq!(
                    preview.editor.style.width,
                    crate::annotation::STROKE_WIDTHS[0]
                );
                assert_eq!(preview.editor.style.dash, crate::annotation::Dash::Dotted);
                assert!((preview.editor.style.opacity - 0.5).abs() < 0.05);
                preview.editor.style.clone()
            });
            drag(&view, cx, Pos::new(220., 100.), Pos::new(300., 180.));
            cx.update(|_, cx| {
                let preview = view.read(cx);
                let doc = preview.doc().unwrap();
                assert_eq!(doc.marks.len(), 2);
                assert_eq!(doc.marks[0], finished);
                assert_eq!(doc.marks[1].style, next_style);
                assert!(preview.selected_indices().is_empty());
            });
            cx.simulate_keystrokes("cmd-z");
            cx.update(|_, cx| {
                assert_eq!(view.read(cx).doc().unwrap().marks, vec![finished.clone()])
            });
            cx.simulate_keystrokes("cmd-z");
            cx.update(|_, cx| {
                assert!(
                    view.read(cx).doc().unwrap().marks.is_empty(),
                    "Changing defaults must not add artwork undo steps"
                )
            });
        }
    }

    #[gpui_kit::test]
    fn finished_new_text_is_unselected_and_next_text_uses_the_updated_defaults(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = editor_window(cx);
        for locked in [false, true] {
            cx.update(|_, cx| {
                view.update(cx, |preview, cx| {
                    preview.doc_mut().unwrap().revert();
                    preview.editor.locked = locked;
                    preview.editor.style = Style::default();
                    preview.refresh_annotations(cx);
                });
            });
            cx.simulate_keystrokes("t");
            let at = pointer(&view, cx, Pos::new(100., 100.));
            cx.simulate_click(at, Modifiers::default());
            cx.run_until_parked();
            cx.simulate_input("First text");
            cx.update(|_, cx| {
                view.update(cx, |preview, cx| {
                    preview.finish_text(cx);
                    assert_eq!(
                        preview.editor.tool,
                        if locked { Tool::Text } else { Tool::Select }
                    );
                    assert!(preview.editor.selected.is_none());
                    preview.refresh_annotations(cx);
                });
            });
            let finished = cx.update(|_, cx| view.read(cx).doc().unwrap().marks[0].clone());
            cx.update(|window, cx| window.click(("annotation-tool", 2usize), cx));
            cx.update(|window, cx| {
                window.click(("annotation-color", 11usize), cx);
                window.click(("annotation-size", 2usize), cx);
                assert_eq!(view.read(cx).doc().unwrap().marks[0], finished);
            });
            let at = pointer(&view, cx, Pos::new(200., 200.));
            cx.simulate_click(at, Modifiers::default());
            cx.run_until_parked();
            cx.simulate_input("Second text");
            cx.simulate_keystrokes("escape");
            cx.update(|_, cx| {
                let preview = view.read(cx);
                let doc = preview.doc().unwrap();
                assert_eq!(doc.marks[0], finished);
                assert_eq!(doc.marks[1].text, "Second text");
                assert_eq!(doc.marks[1].style.color, 11);
                assert_eq!(
                    doc.marks[1].style.font_size,
                    crate::annotation::TEXT_SIZES[2]
                );
                assert!(preview.editor.selected.is_none());
                assert_eq!(
                    preview.editor.tool,
                    if locked { Tool::Text } else { Tool::Select }
                );
            });
        }
    }

    #[gpui_kit::test]
    fn marquee_selects_objects_then_moves_resizes_rotates_and_undoes_them_together(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = editor_window(cx);
        cx.update(|_, cx| {
            view.update(cx, |preview, cx| {
                let mut rectangle = Mark::new(
                    Tool::Rectangle,
                    Pos::new(80., 80.),
                    Style {
                        fill: crate::annotation::Fill::Solid,
                        width: 8.,
                        ..Default::default()
                    },
                );
                rectangle.points[1] = Pos::new(140., 140.);
                let mut text = Mark::new(
                    Tool::Text,
                    Pos::new(170., 90.),
                    Style {
                        font_size: 20.,
                        ..Default::default()
                    },
                );
                text.text = "Hi".into();
                let mut arrow = Mark::new(
                    Tool::Arrow,
                    Pos::new(80., 190.),
                    Style {
                        width: 8.,
                        ..Default::default()
                    },
                );
                arrow.points[1] = Pos::new(190., 190.);
                let brush = Mark::new(
                    Tool::Redact,
                    Pos::new(100., 100.),
                    Style {
                        width: 32.,
                        ..Default::default()
                    },
                );
                preview.doc_mut().unwrap().marks = vec![rectangle, text, arrow, brush];
                preview.refresh_annotations(cx);
            })
        });
        let before = cx.update(|_, cx| view.read(cx).doc().unwrap().marks.clone());
        drag(&view, cx, Pos::new(40., 40.), Pos::new(240., 230.));
        cx.update(|window, cx| {
            let preview = view.read(cx);
            assert_eq!(preview.selected_indices(), vec![0, 1, 2]);
            assert!(
                !preview.doc().unwrap().can_undo(),
                "Selection itself does not change artwork history"
            );
            window.find("annotation-selection-box");
            window.find(("annotation-handle", 3usize));
            assert!(window.try_find(("annotation-handle", 4usize)).is_none());
            assert!(window.try_find("annotation-marquee").is_none());
        });
        drag(&view, cx, Pos::new(110., 110.), Pos::new(130., 130.));
        cx.update(|_, cx| {
            let doc = view.read(cx).doc().unwrap();
            for (i, original) in before.iter().enumerate().take(3) {
                assert!((doc.marks[i].bounds().x - original.bounds().x - 20.).abs() < 0.01);
                assert!((doc.marks[i].bounds().y - original.bounds().y - 20.).abs() < 0.01);
            }
            assert_eq!(doc.marks[3], before[3]);
        });
        let frame = cx.update(|_, cx| view.read(cx).selection_frame().unwrap());
        let corner = frame.corners()[2];
        let fixed = frame.corners()[0];
        let end = Pos::new(
            fixed.x + (corner.x - fixed.x) * 1.25,
            fixed.y + (corner.y - fixed.y) * 1.25,
        );
        drag(&view, cx, corner, end);
        cx.update(|_, cx| {
            let preview = view.read(cx);
            assert!(preview.selection_frame().unwrap().corners()[0].distance(fixed) < 0.01);
            assert!((preview.doc().unwrap().marks[0].bounds().w - 75.).abs() < 0.01);
            assert!((preview.doc().unwrap().marks[1].style.font_size - 25.).abs() < 0.01);
            assert!((preview.doc().unwrap().marks[2].style.width - 10.).abs() < 0.01);
        });
        let (frame, scale, before_rotation) = cx.update(|window, cx| {
            let preview = view.read(cx);
            (
                preview.selection_frame().unwrap(),
                preview.transform(window).1,
                preview.doc().unwrap().marks.clone(),
            )
        });
        let corner = frame.bounds.corners()[1];
        let start = frame.to_world(Pos::new(corner.x + 12. / scale, corner.y - 12. / scale));
        let end = start.rotate(frame.center(), std::f32::consts::FRAC_PI_2);
        let at = pointer(&view, cx, start);
        cx.simulate_mouse_move(at, None, Modifiers::default());
        cx.update(|_, cx| assert_eq!(view.read(cx).selection_cursor(), SelectionCursor::Rotate(0)));
        drag(&view, cx, start, end);
        cx.update(|_, cx| {
            let preview = view.read(cx);
            let doc = preview.doc().unwrap();
            for i in [0, 1] {
                assert!((doc.marks[i].rotation - std::f32::consts::FRAC_PI_2).abs() < 0.01);
            }
            let arrow = &doc.marks[2];
            assert!((arrow.points[0].x - arrow.points[1].x).abs() < 0.01);
            assert_eq!(
                arrow.rotation, 0.,
                "Arrow endpoints stay directly editable after group rotation"
            );
            assert_eq!(doc.marks[3], before[3]);
        });
        cx.simulate_keystrokes("cmd-z");
        cx.update(|_, cx| assert_eq!(view.read(cx).doc().unwrap().marks, before_rotation));
        cx.simulate_keystrokes("cmd-shift-z");
        let off = pointer(&view, cx, Pos::new(30., 30.));
        cx.simulate_click(off, Modifiers::default());
        cx.update(|_, cx| assert!(view.read(cx).selected_indices().is_empty()));
    }

    #[gpui_kit::test]
    fn group_color_opacity_stacking_duplicate_and_delete_apply_to_every_selected_object(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = editor_window(cx);
        cx.update(|_, cx| {
            view.update(cx, |preview, cx| {
                for (color, x) in [(4, 80.), (5, 160.), (6, 240.)] {
                    let mut mark = Mark::new(
                        Tool::Rectangle,
                        Pos::new(x, 100.),
                        Style {
                            color,
                            fill: crate::annotation::Fill::Solid,
                            width: 8.,
                            ..Default::default()
                        },
                    );
                    mark.points[1] = Pos::new(x + 70., 200.);
                    preview.doc_mut().unwrap().marks.push(mark);
                }
                preview.refresh_annotations(cx);
            })
        });
        drag(&view, cx, Pos::new(40., 50.), Pos::new(230., 220.));
        cx.update(|_, cx| assert_eq!(view.read(cx).selected_indices(), vec![0, 1]));
        cx.update(|window, cx| window.click(("annotation-color", 11usize), cx));
        cx.update(|_, cx| {
            let doc = view.read(cx).doc().unwrap();
            assert_eq!(
                doc.marks.iter().map(|m| m.style.color).collect::<Vec<_>>(),
                vec![11, 11, 6]
            );
        });
        let opacity = cx.update(|window, _| window.find("annotation-opacity").bounds().center());
        cx.simulate_click(opacity, Modifiers::default());
        cx.update(|_, cx| {
            let doc = view.read(cx).doc().unwrap();
            assert!((doc.marks[0].style.opacity - 0.5).abs() < 0.01);
            assert_eq!(doc.marks[0].style.opacity, doc.marks[1].style.opacity);
            assert_eq!(doc.marks[2].style.opacity, 1.);
        });
        cx.update(|_, cx| {
            view.update(cx, |preview, cx| {
                preview.reorder_selection(StackOrder::Front, cx)
            })
        });
        cx.update(|_, cx| {
            let preview = view.read(cx);
            assert_eq!(preview.selected_indices(), vec![1, 2]);
            assert_eq!(preview.doc().unwrap().marks[0].style.color, 6);
        });
        cx.update(|window, cx| window.click("annotation-duplicate", cx));
        cx.update(|_, cx| {
            assert_eq!(view.read(cx).selected_indices(), vec![3, 4]);
            assert_eq!(view.read(cx).doc().unwrap().marks.len(), 5);
        });
        cx.simulate_keystrokes("backspace");
        cx.update(|_, cx| assert_eq!(view.read(cx).doc().unwrap().marks.len(), 3));
        cx.simulate_keystrokes("cmd-z");
        cx.update(|_, cx| assert_eq!(view.read(cx).doc().unwrap().marks.len(), 5));
    }

    #[gpui_kit::test]
    fn shift_constrains_drawing_and_corners_without_toggling_selection(cx: &mut TestAppContext) {
        let (view, cx) = editor_window(cx);
        let shift = Modifiers {
            shift: true,
            ..Default::default()
        };
        for tool in [Tool::Rectangle, Tool::Ellipse, Tool::Triangle] {
            cx.update(|window, cx| {
                view.update(cx, |preview, cx| {
                    preview.doc_mut().unwrap().revert();
                    preview.select_tool(tool, window, cx);
                })
            });
            let start = pointer(&view, cx, Pos::new(100., 100.));
            let horizontal = pointer(&view, cx, Pos::new(230., 100.));
            let end = pointer(&view, cx, Pos::new(230., 150.));
            cx.simulate_mouse_down(start, MouseButton::Left, shift);
            cx.simulate_mouse_move(horizontal, Some(MouseButton::Left), shift);
            cx.update(|_, cx| {
                let b = view.read(cx).doc().unwrap().marks[0].bounds();
                assert!((b.w - b.h).abs() < 0.01);
                assert!(b.h > 100.);
            });
            cx.simulate_mouse_move(end, Some(MouseButton::Left), Modifiers::default());
            cx.update(|_, cx| {
                let b = view.read(cx).doc().unwrap().marks[0].bounds();
                assert!((b.w - 130.).abs() < 0.01 && (b.h - 50.).abs() < 0.01);
            });
            cx.simulate_mouse_move(end, Some(MouseButton::Left), shift);
            cx.simulate_mouse_up(end, MouseButton::Left, shift);
            select_at(&view, cx, Pos::new(160., 100.));
            let before = cx.update(|_, cx| view.read(cx).doc().unwrap().marks[0].clone());
            let corner = before.bounds().corners()[2];
            let start = pointer(&view, cx, corner);
            let end = pointer(&view, cx, Pos::new(corner.x + 100., corner.y + 40.));
            cx.simulate_mouse_down(start, MouseButton::Left, shift);
            cx.update(|_, cx| {
                let preview = view.read(cx);
                assert_eq!(preview.editor.selected, Some(0));
                assert!(matches!(
                    preview.editor.gesture.as_ref().unwrap().mode,
                    Mode::Resize(Handle::Corner(2))
                ));
            });
            for modifiers in [shift, Modifiers::default(), shift] {
                cx.simulate_mouse_move(end, Some(MouseButton::Left), modifiers);
                cx.update(|_, cx| {
                    let b = view.read(cx).doc().unwrap().marks[0].bounds();
                    assert_eq!((b.w - b.h).abs() < 0.01, modifiers.shift);
                });
            }
            cx.simulate_mouse_up(end, MouseButton::Left, shift);
            cx.simulate_keystrokes("cmd-z");
            cx.update(|_, cx| assert_eq!(view.read(cx).doc().unwrap().marks[0], before));
        }
    }

    #[gpui_kit::test]
    fn shift_rotation_snaps_objects_and_groups_in_fifteen_degree_steps(cx: &mut TestAppContext) {
        let (view, cx) = editor_window(cx);
        let shift = Modifiers {
            shift: true,
            ..Default::default()
        };
        for count in [1, 2] {
            let (frame, scale, before) = cx.update(|window, cx| {
                view.update(cx, |preview, cx| {
                    preview.doc_mut().unwrap().revert();
                    for i in 0..count {
                        let mut mark = Mark::new(
                            Tool::Rectangle,
                            Pos::new(80. + i as f32 * 140., 100.),
                            Style::default(),
                        );
                        mark.points[1] = Pos::new(180. + i as f32 * 140., 210.);
                        mark.rotation = 0.13;
                        preview.doc_mut().unwrap().marks.push(mark);
                    }
                    preview.editor.tool = Tool::Select;
                    preview.set_selection((0..count).collect());
                    preview.refresh_annotations(cx);
                    (
                        preview.selection_frame().unwrap(),
                        preview.transform(window).1,
                        preview.doc().unwrap().marks.clone(),
                    )
                })
            });
            let corner = frame.bounds.corners()[1];
            let start = frame.to_world(Pos::new(corner.x + 12. / scale, corner.y - 12. / scale));
            let end = start.rotate(frame.center(), 0.42);
            let from = pointer(&view, cx, start);
            let to = pointer(&view, cx, end);
            cx.simulate_mouse_down(from, MouseButton::Left, shift);
            for modifiers in [shift, Modifiers::default(), shift] {
                cx.simulate_mouse_move(to, Some(MouseButton::Left), modifiers);
                cx.update(|_, cx| {
                    let angle = view
                        .read(cx)
                        .selection_frame()
                        .unwrap()
                        .rotation
                        .to_degrees();
                    if modifiers.shift {
                        assert!((angle - 30.).abs() < 0.01, "{count}: {angle}");
                    } else {
                        assert!((angle / 15. - (angle / 15.).round()).abs() > 0.05);
                    }
                });
            }
            cx.simulate_mouse_up(to, MouseButton::Left, shift);
            cx.simulate_keystrokes("cmd-z");
            cx.update(|_, cx| assert_eq!(view.read(cx).doc().unwrap().marks, before));
        }
    }

    #[gpui_kit::test]
    fn toolbar_drawing_selection_delete_and_undo_work_through_native_events(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = editor_window(cx);
        cx.update(|window, _| {
            let history = window.find("annotation-history-bar").bounds();
            let tools = window.find("annotation-tool-bar").bounds();
            assert!((history.bottom() - tools.top()).as_f32().abs() < 0.01);
            assert!((history.left() - tools.left() - px(10.)).as_f32().abs() < 0.01);
        });
        cx.update(|window, cx| window.click(("annotation-tool", 3usize), cx));
        cx.update(|_, cx| {
            assert_eq!(view.read(cx).editor.tool, Tool::Rectangle);
            assert!(view.read(cx).doc().unwrap().marks.is_empty());
        });
        drag(&view, cx, Pos::new(100., 100.), Pos::new(230., 210.));
        cx.update(|_, cx| {
            let preview = view.read(cx);
            assert_eq!(preview.editor.tool, Tool::Select);
            assert_eq!(preview.doc().unwrap().marks.len(), 1);
            assert!((preview.doc().unwrap().marks[0].bounds().w - 130.).abs() < 0.1);
        });
        cx.simulate_keystrokes("backspace");
        cx.update(|_, cx| {
            assert_eq!(
                view.read(cx).doc().unwrap().marks.len(),
                1,
                "A completed, unselected object must not be deleted by typing Delete"
            )
        });
        select_at(&view, cx, Pos::new(100., 150.));
        cx.simulate_keystrokes("backspace");
        cx.update(|_, cx| assert!(view.read(cx).doc().unwrap().marks.is_empty()));
        cx.simulate_keystrokes("cmd-z");
        cx.update(|_, cx| assert_eq!(view.read(cx).doc().unwrap().marks.len(), 1));
        cx.simulate_keystrokes("cmd-shift-z");
        cx.update(|_, cx| assert!(view.read(cx).doc().unwrap().marks.is_empty()));
    }
    #[gpui_kit::test]
    fn double_click_selects_all_text_for_replacement_and_arrow_keys_place_the_caret(
        cx: &mut TestAppContext,
    ) {
        use gpui_kit::EntityInputHandler as _;
        for tool in [Tool::Text, Tool::Rectangle, Tool::Arrow, Tool::Line] {
            let (view, cx) = editor_window(cx);
            let original = "Original 🙂\nsecond line";
            let at = cx.update(|_, cx| {
                view.update(cx, |preview, cx| {
                    let mut mark = Mark::new(tool, Pos::new(70., 100.), Style::default());
                    mark.points[1] = Pos::new(330., 240.);
                    if tool == Tool::Text {
                        mark.wrap = Some(220.);
                    }
                    mark.set_text(original.into());
                    mark.rotation = 0.15;
                    let b = mark.label_edit_bounds().unwrap();
                    let caret = mark.text_layout().caret(2, b.w, mark.text_align());
                    let at = mark.frame().to_world(Pos::new(
                        b.x + caret.x - 2.,
                        b.y + caret.y + mark.text_layout().line_height / 2.,
                    ));
                    preview.doc_mut().unwrap().marks = vec![mark];
                    preview.refresh_annotations(cx);
                    at
                })
            });
            double_click(&view, cx, at);
            let input = cx.update(|window, cx| {
                let input = view
                    .read(cx)
                    .editor
                    .editing
                    .clone()
                    .expect("Double-click edits existing text");
                input.update(cx, |input, cx| {
                    assert_eq!(
                        input.selected_text_range(false, window, cx).unwrap().range,
                        0..original.encode_utf16().count(),
                        "{tool:?}: double-click must highlight the entire value"
                    )
                });
                assert!(
                    !view.read(cx).doc().unwrap().can_undo(),
                    "Highlighting alone is not an artwork edit"
                );
                input
            });
            cx.simulate_input("Replacement");
            cx.update(|_, cx| assert_eq!(input.read(cx).value().as_ref(), "Replacement"));
            // The native typing helper sends one input event per character.
            for _ in "Replacement".chars() {
                cx.simulate_keystrokes("cmd-z");
            }
            cx.update(|_, cx| assert_eq!(input.read(cx).value().as_ref(), original));
            cx.simulate_keystrokes("right");
            cx.update(|window, cx| {
                input.update(cx, |input, cx| {
                    let end = original.encode_utf16().count();
                    assert_eq!(
                        input.selected_text_range(false, window, cx).unwrap().range,
                        end..end
                    );
                })
            });
            cx.simulate_input("!");
            cx.update(|_, cx| assert_eq!(input.read(cx).value().as_ref(), format!("{original}!")));
            // Double-clicking while already editing also selects the whole object text.
            double_click(&view, cx, at);
            cx.update(|window, cx| {
                input.update(cx, |input, cx| {
                    assert_eq!(
                        input.selected_text_range(false, window, cx).unwrap().range,
                        0..format!("{original}!").encode_utf16().count()
                    );
                })
            });
            cx.simulate_keystrokes("left");
            cx.simulate_input("^");
            cx.simulate_keystrokes("cmd-enter");
            cx.update(|_, cx| {
                assert_eq!(
                    view.read(cx).doc().unwrap().marks[0].text,
                    format!("^{original}!")
                )
            });
            cx.simulate_keystrokes("cmd-z");
            cx.update(|_, cx| assert_eq!(view.read(cx).doc().unwrap().marks[0].text, original));
        }
    }

    #[gpui_kit::test]
    fn double_click_text_is_unwrapped_and_dragged_text_wraps(cx: &mut TestAppContext) {
        let (view, cx) = editor_window(cx);
        let p = pointer(&view, cx, Pos::new(120., 130.));
        cx.update(|window, cx| {
            use gpui_kit::InputEvent as _;
            window.dispatch_event(
                MouseDownEvent {
                    position: p,
                    button: MouseButton::Left,
                    modifiers: Modifiers::default(),
                    click_count: 2,
                    first_mouse: false,
                }
                .to_platform_input(),
                cx,
            );
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            let preview = view.read(cx);
            assert!(
                preview
                    .editor
                    .editing
                    .as_ref()
                    .unwrap()
                    .read(cx)
                    .focus_handle(cx)
                    .is_focused(window),
                "input focused immediately after mouse down"
            );
        });
        cx.simulate_mouse_up(p, MouseButton::Left, Modifiers::default());
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.update(|window, cx| {
            let preview = view.read(cx);
            let input = preview
                .editor
                .editing
                .as_ref()
                .expect("double-click starts editing");
            assert!(
                input.read(cx).focus_handle(cx).is_focused(window),
                "text input must retain focus"
            );
        });
        cx.simulate_input("A long annotation that keeps going");
        cx.update(|_, cx| {
            let preview = view.read(cx);
            assert_eq!(
                preview
                    .editor
                    .editing
                    .as_ref()
                    .unwrap()
                    .read(cx)
                    .value()
                    .as_ref(),
                "A long annotation that keeps going",
                "native input must receive typing"
            );
        });
        cx.run_until_parked();
        cx.simulate_keystrokes("escape");
        cx.update(|_, cx| {
            let preview = view.read(cx);
            assert!(preview.editor.editing.is_none());
            let mark = &preview.doc().unwrap().marks[0];
            assert_eq!(mark.text, "A long annotation that keeps going");
            assert_eq!(mark.wrap, None);
        });
        cx.update(|window, cx| window.click(("annotation-tool", 2usize), cx));
        drag(&view, cx, Pos::new(100., 220.), Pos::new(240., 220.));
        cx.update(|_, cx| {
            let preview = view.read(cx);
            let mark = preview.doc().unwrap().marks.last().unwrap();
            assert_eq!(mark.tool, Tool::Text);
            assert!((mark.wrap.unwrap() - 140.).abs() < 0.1);
            assert!(preview.editor.editing.is_some());
        });
    }
    #[gpui_kit::test]
    fn double_clicking_a_filled_shape_attaches_wrapped_text_and_keeps_it_in_history(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = editor_window(cx);
        cx.simulate_keystrokes("r");
        cx.update(|window, cx| window.click(("annotation-fill", 2usize), cx));
        drag(&view, cx, Pos::new(100., 100.), Pos::new(320., 270.));
        let at = pointer(&view, cx, Pos::new(100., 180.));
        cx.simulate_click(at, Modifiers::default());
        double_click(&view, cx, Pos::new(100., 180.));
        cx.simulate_input("hello shape, wrap this text inside the fill");
        cx.run_until_parked();
        cx.update(|window, cx| {
            let preview = view.read(cx);
            let input = preview
                .editor
                .editing
                .as_ref()
                .expect("double-clicking starts a label");
            assert!(input.read(cx).focus_handle(cx).is_focused(window));
            assert_eq!(preview.doc().unwrap().marks.len(), 1);
            assert_eq!(
                preview.editor.tool,
                Tool::Select,
                "Editing a label keeps the selection tool"
            );
            let mark = &preview.doc().unwrap().marks[0];
            assert_eq!(mark.text, "hello shape, wrap this text inside the fill");
            assert!(mark.label_bounds().is_some());
            assert!(mark.text_font_size() >= 12.);
            assert!(
                mark.text_font_size() < 48.,
                "The entire label fits without clipping"
            );
            assert_eq!(
                Mark::new(Tool::Text, Pos::default(), Style::default()).text_font_size(),
                crate::annotation::TEXT_SIZES[1]
            );
        });
        cx.simulate_keystrokes("escape");
        cx.simulate_keystrokes("cmd-z");
        cx.update(|_, cx| {
            let doc = view.read(cx).doc().unwrap();
            assert_eq!(doc.marks.len(), 1);
            assert!(doc.marks[0].text.is_empty());
        });
        cx.simulate_keystrokes("cmd-shift-z");
        let mark = cx.update(|_, cx| view.read(cx).doc().unwrap().marks[0].clone());
        let inside = pointer(&view, cx, Pos::new(180., 180.));
        cx.simulate_click(inside, Modifiers::default());
        drag(&view, cx, Pos::new(180., 180.), Pos::new(200., 200.));
        cx.update(|_, cx| {
            let preview = view.read(cx);
            let moved = &preview.doc().unwrap().marks[0];
            assert_eq!(moved.text, mark.text);
            assert!((moved.bounds().x - mark.bounds().x - 20.).abs() < 0.01);
        });
    }
    #[gpui_kit::test]
    fn typing_shape_text_grows_geometry_without_shrinking_letters(cx: &mut TestAppContext) {
        let (view, cx) = editor_window(cx);
        cx.simulate_keystrokes("r");
        drag(&view, cx, Pos::new(100., 80.), Pos::new(260., 180.));
        double_click(&view, cx, Pos::new(100., 130.));
        let (before, size) = cx.update(|_, cx| {
            let mark = &view.read(cx).doc().unwrap().marks[0];
            (mark.bounds(), mark.text_font_size())
        });
        cx.simulate_input(&"Text stays legible while the shape grows. ".repeat(3));
        cx.update(|_, cx| {
            let mark = &view.read(cx).doc().unwrap().marks[0];
            assert_eq!(
                mark.text_font_size(),
                size,
                "Typing must not shrink letters"
            );
            assert!(mark.bounds().h > before.h, "Text must grow its shape");
        });
    }
    #[gpui_kit::test]
    fn double_clicking_an_arrow_edits_its_label_instead_of_creating_loose_text(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = editor_window(cx);
        cx.simulate_keystrokes("a");
        drag(&view, cx, Pos::new(80., 100.), Pos::new(300., 200.));
        double_click(&view, cx, Pos::new(190., 150.));
        cx.simulate_input("Arrow label");
        cx.simulate_keystrokes("escape");
        cx.update(|_, cx| {
            let doc = view.read(cx).doc().unwrap();
            assert_eq!(doc.marks.len(), 1, "The label belongs to its arrow");
            assert_eq!(doc.marks[0].text, "Arrow label");
        });
    }
    #[gpui_kit::test]
    fn editing_caret_and_finished_text_share_layout_at_every_alignment_zoom_and_rotation(
        cx: &mut TestAppContext,
    ) {
        use gpui_kit::EntityInputHandler as _;
        let (view, cx) = editor_window(cx);
        for tool in [Tool::Text, Tool::Rectangle, Tool::Arrow] {
            for align in [
                crate::annotation::Align::Start,
                crate::annotation::Align::Middle,
                crate::annotation::Align::End,
            ] {
                for (zoom, rotation) in [(0.7, 0.), (1.4, 0.65)] {
                    cx.update(|window, cx| {
                        view.update(cx, |preview, cx| {
                            preview.editor = Editor::default();
                            let mut mark = Mark::new(
                                tool,
                                Pos::new(90., 85.),
                                Style {
                                    text_align: Some(align),
                                    ..Default::default()
                                },
                            );
                            mark.points[1] = Pos::new(310., 245.);
                            if tool == Tool::Text {
                                mark.wrap = Some(200.);
                            }
                            mark.set_text("Line one\nTwo".into());
                            mark.rotation = rotation;
                            preview.doc_mut().unwrap().revert();
                            preview.doc_mut().unwrap().marks = vec![mark];
                            preview.zoom = zoom;
                            preview.edit_text(0, window, cx);
                            preview.refresh_annotations(cx);
                        });
                    });
                    cx.run_until_parked();
                    cx.update(|window, cx| {
                        window.render_frame(cx);
                        let preview = view.read(cx);
                        let mark = preview.doc().unwrap().marks[0].clone();
                        let (origin, scale) = preview.transform(window);
                        let b = mark.label_edit_bounds().unwrap();
                        let caret = mark.text_layout().caret(0, b.w, mark.text_align());
                        let expected = mark.frame().map_bounds(Rect {
                            x: b.x + caret.x,
                            y: b.y + caret.y,
                            w: 1. / scale,
                            h: crate::annotation::line_height(mark.text_font_size()),
                        });
                        let input = preview.editor.editing.as_ref().unwrap().clone();
                        let native = input.update(cx, |input, cx| {
                            input
                                .bounds_for_range(0..0, Bounds::default(), window, cx)
                                .unwrap()
                        });
                        assert!(
                            (native.origin.x.as_f32() - origin.x - expected.x * scale).abs() < 0.02
                        );
                        assert!(
                            (native.origin.y.as_f32() - origin.y - expected.y * scale).abs() < 0.02
                        );
                        assert!((native.size.height.as_f32() - expected.h * scale).abs() < 0.02);
                        view.update(cx, |preview, cx| {
                            preview.finish_text(cx);
                            assert_eq!(
                                preview.doc().unwrap().marks[0],
                                mark,
                                "Finishing must not move the text"
                            );
                            assert!(
                                !preview.doc().unwrap().can_undo(),
                                "Simply opening text editing is not an artwork change"
                            );
                        });
                    });
                }
            }
        }
    }
    #[gpui_kit::test]
    fn native_text_input_handles_plain_clipboard_composition_graphemes_and_local_undo(
        cx: &mut TestAppContext,
    ) {
        use gpui_kit::EntityInputHandler as _;
        let (view, cx) = editor_window(cx);
        double_click(&view, cx, Pos::new(100., 100.));
        cx.simulate_input("abc");
        cx.simulate_keystrokes("left");
        cx.simulate_input("X");
        cx.update(|_, cx| {
            assert_eq!(
                view.read(cx)
                    .editor
                    .editing
                    .as_ref()
                    .unwrap()
                    .read(cx)
                    .value()
                    .as_ref(),
                "abXc"
            )
        });
        cx.simulate_keystrokes("backspace end");
        let input = cx.update(|_, cx| view.read(cx).editor.editing.clone().unwrap());
        cx.update(|_, cx| input.update(cx, |input, cx| input.set_selected_range(0..3, cx)));
        cx.simulate_keystrokes("cmd-c end cmd-v");
        cx.update(|_, cx| assert_eq!(input.read(cx).value().as_ref(), "abcabc"));
        cx.simulate_keystrokes("cmd-z");
        cx.update(|_, cx| assert_eq!(input.read(cx).value().as_ref(), "abc"));
        cx.simulate_keystrokes("cmd-shift-z");
        cx.update(|_, cx| input.update(cx, |input, cx| input.set_selected_range(3..6, cx)));
        cx.simulate_keystrokes("cmd-x");
        cx.update(|_, cx| assert_eq!(input.read(cx).value().as_ref(), "abc"));
        cx.update(|window, cx| {
            input.update(cx, |input, cx| {
                input.replace_and_mark_text_in_range(Some(0..3), "に", Some(1..1), window, cx);
                assert_eq!(input.marked_text_range(window, cx), Some(0..1));
                input.replace_and_mark_text_in_range(None, "日本", Some(2..2), window, cx);
                input.replace_text_in_range(None, "日本語", window, cx);
                assert_eq!(input.value().as_ref(), "日本語");
                assert_eq!(
                    input.selected_text_range(false, window, cx).unwrap().range,
                    3..3
                );
                assert_eq!(input.marked_text_range(window, cx), None);
            });
        });
        cx.simulate_keystrokes("cmd-z");
        cx.update(|window, cx| {
            assert_eq!(input.read(cx).value().as_ref(), "abc");
            input.update(cx, |input, cx| {
                input.replace_text_in_range(Some(0..3), "e\u{301}🇺🇸👩‍👩‍👦", window, cx)
            });
        });
        cx.simulate_keystrokes("backspace");
        cx.update(|_, cx| assert_eq!(input.read(cx).value().as_ref(), "e\u{301}🇺🇸"));
        cx.simulate_keystrokes("backspace backspace");
        cx.update(|_, cx| assert!(input.read(cx).value().is_empty()));
        cx.simulate_keystrokes("escape");
        cx.update(|_, cx| assert!(view.read(cx).doc().unwrap().marks.is_empty()));
    }
    #[gpui_kit::test]
    fn selected_shape_enter_alignment_controls_and_command_enter_keep_text_editing_consistent(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = editor_window(cx);
        cx.simulate_keystrokes("r");
        drag(&view, cx, Pos::new(100., 80.), Pos::new(300., 230.));
        select_at(&view, cx, Pos::new(100., 130.));
        cx.simulate_keystrokes("enter");
        cx.simulate_input("First line");
        cx.simulate_keystrokes("enter");
        cx.simulate_input("Second line");
        cx.update(|window, cx| {
            window.click(("annotation-text-align", 2usize), cx);
            window.click(("annotation-text-vertical", 0usize), cx);
            let preview = view.read(cx);
            let mark = &preview.doc().unwrap().marks[0];
            assert_eq!(mark.text_align(), crate::annotation::Align::End);
            assert_eq!(mark.style.vertical_align, crate::annotation::Align::Start);
            assert_eq!(mark.text, "First line\nSecond line");
            assert!(
                preview
                    .editor
                    .editing
                    .as_ref()
                    .unwrap()
                    .read(cx)
                    .focus_handle(cx)
                    .is_focused(window)
            );
        });
        cx.simulate_keystrokes("cmd-enter");
        cx.update(|_, cx| {
            let preview = view.read(cx);
            assert!(preview.editor.editing.is_none());
            assert_eq!(preview.doc().unwrap().marks.len(), 1);
        });
    }
    #[gpui_kit::test]
    fn transparent_labels_move_hosts_and_empty_interiors_respect_focus(cx: &mut TestAppContext) {
        let (view, cx) = editor_window(cx);
        let original = cx.update(|_, cx| {
            view.update(cx, |preview, cx| {
                let mut bottom = Mark::new(
                    Tool::Rectangle,
                    Pos::new(10., 10.),
                    Style {
                        fill: crate::annotation::Fill::Solid,
                        color: 7,
                        ..Default::default()
                    },
                );
                bottom.points[1] = Pos::new(390., 290.);
                let mut top = Mark::new(Tool::Rectangle, Pos::new(70., 70.), Style::default());
                top.points[1] = Pos::new(330., 260.);
                top.set_text("Hi".into());
                preview.doc_mut().unwrap().marks = vec![bottom, top.clone()];
                preview.refresh_annotations(cx);
                top
            })
        });
        drag(&view, cx, Pos::new(110., 110.), Pos::new(130., 120.));
        cx.update(|_, cx| {
            let preview = view.read(cx);
            assert_eq!(preview.selected_indices(), vec![0]);
            assert_eq!(preview.doc().unwrap().marks[1], original);
        });
        cx.simulate_keystrokes("escape");
        let text = original.label_edit_bounds().unwrap().center();
        drag(&view, cx, text, Pos::new(text.x + 25., text.y + 10.));
        let moved = cx.update(|_, cx| {
            let preview = view.read(cx);
            assert_eq!(preview.selected_indices(), vec![1]);
            let mut expected = original.clone();
            expected.translate(25., 10.);
            assert_eq!(preview.doc().unwrap().marks[1], expected);
            expected
        });
        let blank = Pos::new(moved.bounds().x + 30., moved.bounds().y + 30.);
        drag(&view, cx, blank, Pos::new(blank.x + 15., blank.y + 10.));
        let focused = cx.update(|_, cx| {
            let mut expected = moved.clone();
            expected.translate(15., 10.);
            assert_eq!(view.read(cx).doc().unwrap().marks[1], expected);
            expected
        });
        cx.simulate_keystrokes("escape");
        let blank = Pos::new(focused.bounds().x + 30., focused.bounds().y + 30.);
        drag(&view, cx, blank, Pos::new(blank.x + 10., blank.y + 10.));
        cx.update(|_, cx| assert_eq!(view.read(cx).doc().unwrap().marks[1], focused));
    }

    #[gpui_kit::test]
    fn clone_and_clipboard_preserve_groups_and_paste_at_the_cursor(cx: &mut TestAppContext) {
        let (view, cx) = editor_window(cx);
        let (originals, bounds) = cx.update(|_, cx| {
            view.update(cx, |preview, cx| {
                let mut shape = Mark::new(Tool::Rectangle, Pos::new(50., 70.), Style::default());
                shape.points[1] = Pos::new(150., 140.);
                shape.rotation = 0.3;
                shape.set_text("Hi".into());
                let mut line = Mark::new(Tool::Line, Pos::new(170., 100.), Style::default());
                line.points = vec![
                    Pos::new(170., 100.),
                    Pos::new(300., 160.),
                    Pos::new(220., 90.),
                ];
                line.set_text("Line".into());
                let bounds = shape.visual_bounds().union(line.visual_bounds());
                let marks = vec![shape, line];
                preview.doc_mut().unwrap().marks = marks.clone();
                preview.set_selection(vec![0, 1]);
                preview.refresh_annotations(cx);
                (marks, bounds)
            })
        });
        cx.simulate_keystrokes("cmd-d");
        cx.update(|window, cx| {
            let preview = view.read(cx);
            assert_eq!(preview.selected_indices(), vec![2, 3]);
            for (mark, original) in preview.doc().unwrap().marks[2..].iter().zip(&originals) {
                let mut expected = original.clone();
                expected.translate(bounds.w, 0.);
                assert_eq!(
                    *mark, expected,
                    "Clone shifts the full selection width without moving down"
                );
            }
            window.click("annotation-duplicate", cx);
        });
        cx.update(|_, cx| assert_eq!(view.read(cx).doc().unwrap().marks.len(), 6));
        cx.simulate_keystrokes("cmd-z cmd-z cmd-a cmd-c");
        cx.update(|_, cx| {
            assert_eq!(view.read(cx).doc().unwrap().marks, originals);
            assert!(
                view.read(cx).doc().unwrap().can_redo(),
                "Copy must not alter history"
            );
            assert_eq!(
                cx.read_from_clipboard().unwrap().text().as_deref(),
                Some("Hi\nLine")
            );
            view.update(cx, |preview, cx| {
                preview.zoom = 0.75;
                preview.offset = point(px(19.), px(-12.));
                cx.notify();
            });
        });
        cx.simulate_keystrokes("escape");
        let cursor = Pos::new(240., 190.);
        let at = pointer(&view, cx, cursor);
        cx.simulate_mouse_move(at, None, Modifiers::default());
        cx.simulate_keystrokes("cmd-v");
        cx.update(|_, cx| {
            let preview = view.read(cx);
            let marks = &preview.doc().unwrap().marks;
            assert_eq!(marks.len(), 4);
            assert_eq!(preview.selected_indices(), vec![2, 3]);
            let center = bounds.center();
            for (mark, original) in marks[2..].iter().zip(&originals) {
                let mut expected = original.clone();
                expected.translate(cursor.x - center.x, cursor.y - center.y);
                for (actual, expected) in mark.points.iter().zip(&expected.points) {
                    assert!(actual.distance(*expected) < 0.01);
                }
                assert_eq!(mark.style, expected.style);
                assert_eq!(mark.text, expected.text);
                assert_eq!(mark.rotation, expected.rotation);
            }
        });
        cx.simulate_keystrokes("cmd-z");
        cx.update(|_, cx| assert_eq!(view.read(cx).doc().unwrap().marks, originals));
        cx.simulate_keystrokes("cmd-shift-z");
        cx.update(|_, cx| assert_eq!(view.read(cx).doc().unwrap().marks.len(), 4));
        // A thin vertical line still needs its full visible width when cloned.
        let (line, width) = cx.update(|_, cx| {
            view.update(cx, |preview, cx| {
                let mut line = Mark::new(Tool::Line, Pos::new(160., 80.), Style::default());
                line.points[1] = Pos::new(160., 240.);
                let width = line.visual_bounds().w;
                preview.doc_mut().unwrap().marks = vec![line.clone()];
                preview.set_selection(vec![0]);
                preview.refresh_annotations(cx);
                (line, width)
            })
        });
        cx.simulate_keystrokes("cmd-d");
        cx.update(|_, cx| {
            let marks = &view.read(cx).doc().unwrap().marks;
            assert_eq!(marks.len(), 2);
            assert!(width >= line.style.width);
            assert!((marks[1].points[0].x - line.points[0].x - width).abs() < 0.01);
            assert_eq!(marks[1].points[0].y, line.points[0].y);
            cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string_with_metadata(
                "Unrelated clipboard text".into(),
                "{}".into(),
            ));
        });
        cx.simulate_keystrokes("cmd-v");
        cx.update(|_, cx| assert_eq!(view.read(cx).doc().unwrap().marks.len(), 2));
    }

    #[gpui_kit::test]
    fn arrow_and_line_labels_move_their_hosts_and_bending_stays_attached(cx: &mut TestAppContext) {
        for tool in [Tool::Arrow, Tool::Line] {
            let (view, cx) = editor_window(cx);
            let before = cx.update(|_, cx| {
                view.update(cx, |preview, cx| {
                    let mut mark = Mark::new(
                        tool,
                        Pos::new(60., 150.),
                        Style {
                            width: 8.,
                            ..Default::default()
                        },
                    );
                    mark.points[1] = Pos::new(340., 150.);
                    mark.set_text("Label".into());
                    preview.doc_mut().unwrap().marks = vec![mark.clone()];
                    preview.set_selection(vec![0]);
                    preview.refresh_annotations(cx);
                    mark
                })
            });
            drag(&view, cx, Pos::new(200., 150.), Pos::new(256., 150.));
            let moved = cx.update(|_, cx| {
                let preview = view.read(cx);
                let mark = &preview.doc().unwrap().marks[0];
                assert!(preview.editor.editing.is_none());
                assert_eq!(
                    mark.points[0],
                    Pos::new(before.points[0].x + 56., before.points[0].y)
                );
                assert_eq!(
                    mark.points[1],
                    Pos::new(before.points[1].x + 56., before.points[1].y)
                );
                assert_eq!(mark.label_position, before.label_position);
                assert_eq!(mark.text, before.text);
                mark.clone()
            });
            let handle = moved.bend_handle();
            drag(&view, cx, handle, Pos::new(handle.x, handle.y + 40.));
            cx.update(|_, cx| {
                let mark = &view.read(cx).doc().unwrap().marks[0];
                assert!(mark.label_bounds().unwrap().center().y > 150.);
                assert_eq!(mark.text, "Label");
            });
            cx.simulate_keystrokes("cmd-z");
            cx.update(|_, cx| assert_eq!(view.read(cx).doc().unwrap().marks[0], moved));
            cx.simulate_keystrokes("cmd-z");
            cx.update(|_, cx| assert_eq!(view.read(cx).doc().unwrap().marks[0], before));
        }
    }
    #[gpui_kit::test]
    fn arrow_and_line_labels_accept_enter_and_resize_without_splitting_words(
        cx: &mut TestAppContext,
    ) {
        for tool in [Tool::Arrow, Tool::Line] {
            let (view, cx) = editor_window(cx);
            cx.update(|_, cx| {
                view.update(cx, |preview, cx| {
                    let mut mark = Mark::new(tool, Pos::new(60., 150.), Style::default());
                    mark.points[1] = Pos::new(340., 150.);
                    preview.doc_mut().unwrap().marks = vec![mark];
                    preview.refresh_annotations(cx);
                })
            });
            double_click(&view, cx, Pos::new(200., 150.));
            cx.simulate_input("Hello");
            cx.simulate_keystrokes("enter");
            cx.simulate_input("there");
            cx.simulate_keystrokes("cmd-enter");
            let before = cx.update(|_, cx| {
                let mark = view.read(cx).doc().unwrap().marks[0].clone();
                assert_eq!(mark.text, "Hello\nthere");
                assert_eq!(
                    mark.text_layout().height,
                    mark.text_layout().line_height * 2.
                );
                mark
            });
            select_at(&view, cx, before.points[0]);
            drag(&view, cx, before.points[1], Pos::new(65., 150.));
            cx.update(|_, cx| {
                let mark = &view.read(cx).doc().unwrap().marks[0];
                assert_eq!(mark.points[0], before.points[0]);
                assert_eq!(mark.style.font_size, before.style.font_size);
                assert!(mark.points[1].x > 65.);
                assert_eq!(
                    mark.text_layout().height,
                    mark.text_layout().line_height * 2.
                );
            });
            cx.simulate_keystrokes("cmd-z");
            cx.update(|_, cx| assert_eq!(&view.read(cx).doc().unwrap().marks[0], &before));
        }
    }
    #[gpui_kit::test]
    fn fixed_width_text_shows_bounds_during_creation_and_editing_but_handles_only_when_selected(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = editor_window(cx);
        cx.simulate_keystrokes("t");
        let start = pointer(&view, cx, Pos::new(100., 130.));
        let end = pointer(&view, cx, Pos::new(300., 130.));
        cx.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
        cx.update(|_, cx| {
            view.update(cx, |preview, _| {
                preview.editor.gesture.as_mut().unwrap().started -= Duration::from_millis(160);
            });
        });
        cx.simulate_mouse_move(end, Some(MouseButton::Left), Modifiers::default());
        let drawing_bounds = cx.update(|window, cx| {
            window.render_frame(cx);
            let bounds = window.find("annotation-text-box-outline").bounds();
            let (_, scale) = view.read(cx).transform(window);
            assert!((f32::from(bounds.size.width) - 200. * scale).abs() < 0.01);
            assert!(window.try_find("annotation-selection-box").is_none());
            assert!(window.try_find(("annotation-handle", 0usize)).is_none());
            bounds
        });
        cx.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();
        cx.update(|window, cx| {
            window.render_frame(cx);
            assert_eq!(
                window.find("annotation-text-box-outline").bounds(),
                drawing_bounds
            );
            assert!(view.read(cx).editor.editing.is_some());
            assert!(window.try_find(("annotation-handle", 0usize)).is_none());
        });
        cx.simulate_input("First line");
        cx.simulate_keystrokes("enter");
        cx.simulate_input("Second line");
        cx.update(|window, cx| {
            window.render_frame(cx);
            let bounds = window.find("annotation-text-box-outline").bounds();
            assert_eq!(bounds.size.width, drawing_bounds.size.width);
            assert!(bounds.size.height > drawing_bounds.size.height);
            assert!(window.try_find(("annotation-handle", 0usize)).is_none());
        });
        cx.simulate_keystrokes("escape");
        cx.update(|window, cx| {
            window.render_frame(cx);
            assert!(window.try_find("annotation-text-box-outline").is_none());
            assert!(window.try_find("annotation-selection-box").is_none());
        });
        select_at(&view, cx, Pos::new(105., 130.));
        let before = cx.update(|window, cx| {
            window.render_frame(cx);
            assert!(window.try_find("annotation-text-box-outline").is_none());
            window.find("annotation-selection-box");
            for i in 0usize..4 {
                window.find(("annotation-handle", i));
            }
            view.read(cx).doc().unwrap().marks[0].clone()
        });
        let edge = before.bounds().corners()[1];
        let edge = Pos::new(edge.x, before.bounds().center().y);
        drag(&view, cx, edge, Pos::new(edge.x + 60., edge.y));
        cx.update(|_, cx| {
            let mark = &view.read(cx).doc().unwrap().marks[0];
            assert!((mark.wrap.unwrap() - 260.).abs() < 0.01);
            assert_eq!(mark.text_font_size(), before.text_font_size());
        });
        double_click(&view, cx, Pos::new(105., 130.));
        cx.update(|window, cx| {
            window.render_frame(cx);
            window.find("annotation-text-box-outline");
            assert!(window.try_find("annotation-selection-box").is_none());
            assert!(window.try_find(("annotation-handle", 0usize)).is_none());
        });
        // Blurring an existing input removes the editing outline too.
        let blank = pointer(&view, cx, Pos::new(380., 280.));
        cx.simulate_click(blank, Modifiers::default());
        cx.update(|window, cx| {
            window.render_frame(cx);
            assert!(view.read(cx).editor.editing.is_none());
            assert!(window.try_find("annotation-text-box-outline").is_none());
            assert!(window.try_find(("annotation-handle", 0usize)).is_none());
        });
    }

    #[gpui_kit::test]
    fn free_form_text_stays_unboxed_until_directly_resized(cx: &mut TestAppContext) {
        let (view, cx) = editor_window(cx);
        cx.simulate_keystrokes("t");
        let at = pointer(&view, cx, Pos::new(110., 110.));
        cx.simulate_click(at, Modifiers::default());
        cx.simulate_input("Free form");
        cx.update(|window, cx| {
            window.render_frame(cx);
            assert!(view.read(cx).doc().unwrap().marks[0].wrap.is_none());
            assert!(window.try_find("annotation-text-box-outline").is_none());
            assert!(window.try_find(("annotation-handle", 0usize)).is_none());
        });
        cx.simulate_keystrokes("escape");
        double_click(&view, cx, Pos::new(115., 110.));
        cx.update(|window, cx| {
            window.render_frame(cx);
            assert!(view.read(cx).editor.editing.is_some());
            assert!(window.try_find("annotation-text-box-outline").is_none());
            assert!(window.try_find(("annotation-handle", 0usize)).is_none());
        });
        cx.simulate_keystrokes("escape");
        select_at(&view, cx, Pos::new(115., 110.));
        let before = cx.update(|window, cx| {
            window.render_frame(cx);
            window.find("annotation-selection-box");
            window.find(("annotation-handle", 2usize));
            view.read(cx).doc().unwrap().marks[0].clone()
        });
        let corner = before.frame().corners()[2];
        drag(&view, cx, corner, Pos::new(corner.x + 90., corner.y + 40.));
        cx.update(|_, cx| {
            let mark = &view.read(cx).doc().unwrap().marks[0];
            assert!(mark.wrap.is_some());
            assert!(mark.text_font_size() > before.text_font_size());
            assert_eq!(mark.text_layout().height, mark.text_layout().line_height);
        });
        double_click(&view, cx, Pos::new(115., 110.));
        cx.update(|window, cx| {
            window.render_frame(cx);
            window.find("annotation-text-box-outline");
            assert!(window.try_find(("annotation-handle", 0usize)).is_none());
        });
        cx.simulate_keystrokes("escape cmd-z");
        cx.update(|_, cx| assert_eq!(&view.read(cx).doc().unwrap().marks[0], &before));
    }

    #[gpui_kit::test]
    fn text_creation_distinguishes_clicks_deliberate_widths_and_accidental_drags(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = editor_window(cx);
        for (a, b, delayed, fixed) in [
            (Pos::new(100., 150.), Pos::new(260., 150.), true, true),
            (Pos::new(260., 150.), Pos::new(100., 180.), true, true),
            (Pos::new(100., 100.), Pos::new(105., 240.), true, false),
            (Pos::new(100., 150.), Pos::new(260., 180.), false, false),
        ] {
            cx.update(|window, cx| {
                view.update(cx, |preview, cx| {
                    preview.select_tool(Tool::Text, window, cx);
                    preview.doc_mut().unwrap().revert();
                    preview.refresh_annotations(cx);
                })
            });
            let start = pointer(&view, cx, a);
            let end = pointer(&view, cx, b);
            cx.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
            cx.update(|window, cx| {
                use gpui_kit::InputEvent as _;
                if delayed {
                    view.update(cx, |preview, _| {
                        preview.editor.gesture.as_mut().unwrap().started -=
                            Duration::from_millis(160)
                    });
                }
                window.dispatch_event(
                    MouseMoveEvent {
                        position: end,
                        pressed_button: Some(MouseButton::Left),
                        modifiers: Modifiers::default(),
                    }
                    .to_platform_input(),
                    cx,
                );
            });
            cx.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
            cx.simulate_input("wrap these words");
            cx.simulate_keystrokes("cmd-enter");
            let before = cx.update(|_, cx| {
                let mark = view.read(cx).doc().unwrap().marks[0].clone();
                assert_eq!(mark.wrap.is_some(), fixed);
                if fixed {
                    assert!((mark.wrap.unwrap() - 160.).abs() < 0.01);
                }
                assert!((mark.bounds().x - if fixed { a.x.min(b.x) } else { a.x }).abs() < 0.01);
                assert_eq!(mark.text_font_size(), crate::annotation::TEXT_SIZES[1]);
                mark
            });
            let bounds = before.bounds();
            select_at(&view, cx, bounds.center());
            let edge = Pos::new(bounds.x + bounds.w, bounds.y + bounds.h / 2.);
            drag(&view, cx, edge, Pos::new(edge.x + 60., edge.y));
            cx.update(|_, cx| {
                let mark = &view.read(cx).doc().unwrap().marks[0];
                assert!((mark.wrap.unwrap() - bounds.w - 60.).abs() < 0.01);
                assert_eq!(mark.text_font_size(), before.text_font_size());
            });
        }
    }
    fn double_click(view: &Entity<ImagePreview>, cx: &mut VisualTestContext, p: Pos) {
        let position = pointer(view, cx, p);
        cx.update(|window, cx| {
            use gpui_kit::InputEvent as _;
            window.dispatch_event(
                MouseDownEvent {
                    position,
                    button: MouseButton::Left,
                    modifiers: Modifiers::default(),
                    click_count: 2,
                    first_mouse: false,
                }
                .to_platform_input(),
                cx,
            );
        });
        cx.run_until_parked();
        cx.simulate_mouse_move(
            point(position.x + px(1.), position.y + px(1.)),
            Some(MouseButton::Left),
            Modifiers::default(),
        );
        cx.simulate_mouse_up(position, MouseButton::Left, Modifiers::default());
    }
    #[gpui_kit::test]
    fn shape_labels_edit_from_the_border_or_selected_transparent_interior(cx: &mut TestAppContext) {
        let (view, cx) = editor_window(cx);
        cx.simulate_keystrokes("r");
        cx.update(|window, cx| window.click(("annotation-fill", 1usize), cx));
        drag(&view, cx, Pos::new(100., 100.), Pos::new(320., 270.));
        double_click(&view, cx, Pos::new(100., 180.));
        cx.simulate_input("A shape label");
        cx.run_until_parked();
        cx.simulate_keystrokes("escape");
        // A selected transparent interior takes priority over the shape underneath,
        // both when creating a label and when reopening an existing one.
        cx.update(|_, cx| {
            view.update(cx, |preview, cx| {
                let mut underneath = preview.doc().unwrap().marks[0].clone();
                underneath.text.clear();
                preview.doc_mut().unwrap().marks.insert(0, underneath);
                preview.doc_mut().unwrap().marks[1].style.fill = crate::annotation::Fill::None;
                preview.refresh_annotations(cx);
            })
        });
        for existing in ["", "A shape label"] {
            cx.update(|_, cx| {
                view.update(cx, |preview, cx| {
                    preview.doc_mut().unwrap().marks[1].text = existing.into();
                    preview.refresh_annotations(cx);
                })
            });
            let at = pointer(&view, cx, Pos::new(100., 180.));
            cx.simulate_click(at, Modifiers::default());
            cx.update(|_, cx| assert_eq!(view.read(cx).editor.selected, Some(1)));
            double_click(&view, cx, Pos::new(200., 180.));
            cx.update(|window, cx| {
                let preview = view.read(cx);
                assert_eq!(preview.editor.selected, Some(1));
                let input = preview.editor.editing.as_ref().unwrap();
                assert!(input.read(cx).focus_handle(cx).is_focused(window));
                assert_eq!(input.read(cx).value().as_ref(), existing);
                assert_eq!(preview.doc().unwrap().marks.len(), 2);
            });
            cx.simulate_keystrokes("right");
            cx.simulate_input(" more text");
            cx.run_until_parked();
            cx.simulate_keystrokes("escape");
            cx.update(|_, cx| {
                assert_eq!(
                    view.read(cx).doc().unwrap().marks[1].text,
                    format!("{existing} more text")
                )
            });
        }
    }
    #[gpui_kit::test]
    fn selected_shapes_keep_tool_shortcuts_without_starting_text(cx: &mut TestAppContext) {
        let (view, cx) = editor_window(cx);
        for (tool, fill, size) in [
            (Tool::Rectangle, crate::annotation::Fill::Solid, 200.),
            (Tool::Ellipse, crate::annotation::Fill::Soft, 200.),
            (Tool::Triangle, crate::annotation::Fill::Hatch, 200.),
            (Tool::Rectangle, crate::annotation::Fill::None, 200.),
            (Tool::XBox, crate::annotation::Fill::Solid, 200.),
            (Tool::CheckBox, crate::annotation::Fill::Solid, 200.),
            (Tool::Ellipse, crate::annotation::Fill::Hatch, 10.),
        ] {
            cx.update(|window, cx| {
                view.update(cx, |preview, cx| {
                    let mut mark = Mark::new(
                        tool,
                        Pos::new(100., 100.),
                        Style {
                            fill,
                            ..Default::default()
                        },
                    );
                    mark.points[1] = Pos::new(100. + size, 100. + size);
                    preview.doc_mut().unwrap().marks = vec![mark];
                    preview.editor.tool = Tool::Select;
                    preview.editor.selected = Some(0);
                    window.focus(&preview.focus_handle, cx);
                    preview.refresh_annotations(cx);
                })
            });
            cx.simulate_keystrokes("h");
            cx.update(|_, cx| {
                let preview = view.read(cx);
                assert_eq!(preview.editor.tool, Tool::Pan);
                assert!(preview.editor.editing.is_none());
                assert!(preview.doc().unwrap().marks[0].text.is_empty());
            });
        }
    }
    #[gpui_kit::test]
    fn transformed_drawing_and_arrow_bend_use_image_coordinates(cx: &mut TestAppContext) {
        let (view, cx) = editor_window(cx);
        cx.update(|window, cx| {
            view.update(cx, |preview, cx| {
                preview.zoom = 1.6;
                preview.offset = point(px(20.), px(-10.));
                preview.select_tool(Tool::Arrow, window, cx);
            })
        });
        drag(&view, cx, Pos::new(130., 100.), Pos::new(230., 200.));
        select_at(&view, cx, Pos::new(130., 100.));
        drag(&view, cx, Pos::new(180., 150.), Pos::new(220., 125.));
        cx.update(|_, cx| {
            let preview = view.read(cx);
            let mark = &preview.doc().unwrap().marks[0];
            assert!((mark.points[0].x - 130.).abs() < 0.1);
            assert_eq!(mark.points.len(), 3);
            assert!(mark.arrow_mid().distance(Pos::new(220., 125.)) < 0.1);
        });
    }
    #[gpui_kit::test]
    fn rapid_typing_in_long_text_stays_interactive(cx: &mut TestAppContext) {
        let (view, cx) = editor_window(cx);
        cx.update(|window, cx| {
            view.update(cx, |preview, cx| {
                preview.create_text(Pos::new(50., 70.), Some(320.), window, cx);
            });
        });
        cx.run_until_parked();
        let prefix = "Typing should stay responsive even with a long paragraph. ".repeat(4);
        cx.update(|window, cx| {
            use gpui_kit::EntityInputHandler as _;
            let input = view.read(cx).editor.editing.clone().unwrap();
            input.update(cx, |input, cx| {
                input.replace_text_in_range(None, &prefix, window, cx)
            });
            window.render_frame(cx);
        });
        let suffix = " More text.";
        let mut timings = Vec::new();
        let mut input_time = Duration::ZERO;
        let mut render_time = Duration::ZERO;
        for ch in suffix.chars() {
            let started = Instant::now();
            cx.simulate_input(&ch.to_string());
            input_time += started.elapsed();
            let rendering = Instant::now();
            cx.update(|window, cx| window.render_frame(cx));
            render_time += rendering.elapsed();
            timings.push(started.elapsed());
        }
        eprintln!("Typing input {input_time:?}; frame rendering {render_time:?}");
        let total: Duration = timings.iter().sum();
        timings.sort();
        eprintln!(
            "Long-text typing: {} keys, total {total:?}, median {:?}, p95 {:?}",
            timings.len(),
            timings[timings.len() / 2],
            timings[timings.len() * 95 / 100],
        );
        cx.update(|_, cx| {
            let preview = view.read(cx);
            assert_eq!(
                preview.doc().unwrap().marks[0].text,
                format!("{prefix}{suffix}")
            );
            assert_eq!(
                preview
                    .editor
                    .editing
                    .as_ref()
                    .unwrap()
                    .read(cx)
                    .value()
                    .as_ref(),
                format!("{prefix}{suffix}")
            );
        });
        assert!(
            total < Duration::from_millis(500),
            "Rapid typing blocks the editor: {total:?}"
        );
    }

    #[gpui_kit::test]
    fn large_image_pointer_updates_stay_interactive(cx: &mut TestAppContext) {
        let (view, cx) = editor_window(cx);
        cx.update(|window, cx| {
            view.update(cx, |preview, cx| {
                preview.doc_mut().unwrap().base = Arc::new(image::RgbaImage::from_pixel(
                    2400,
                    1600,
                    image::Rgba([50, 60, 70, 255]),
                ));
                preview.images[0].pixel_size = (2400, 1600);
                preview.select_tool(Tool::Rectangle, window, cx);
            })
        });
        let a = pointer(&view, cx, Pos::new(700., 600.));
        cx.simulate_mouse_down(a, MouseButton::Left, Modifiers::default());
        let started = Instant::now();
        for step in 0..12 {
            let p = pointer(&view, cx, Pos::new(1200. + step as f32 * 8., 1000.));
            cx.update(|window, cx| {
                view.update(cx, |preview, cx| {
                    preview.editor_mouse_move(
                        &MouseMoveEvent {
                            position: p,
                            pressed_button: Some(MouseButton::Left),
                            modifiers: Modifiers::default(),
                        },
                        window,
                        cx,
                    );
                })
            });
        }
        let elapsed = started.elapsed();
        eprintln!("12 pointer updates on a 2400×1600 image: {elapsed:?}");
        assert!(
            elapsed < Duration::from_millis(500),
            "Live drawing blocks the UI: {elapsed:?}"
        );
    }
    #[gpui_kit::test]
    fn shape_pickers_share_the_restrained_set_and_use_the_clicked_location(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = editor_window(cx);
        cx.update(|window, cx| {
            window.find(("annotation-tool", 3usize));
            window.click(("annotation-tool", 4usize), cx);
        });
        cx.update(|_, cx| assert_eq!(view.read(cx).editor.tool, Tool::Ellipse));
        cx.simulate_keystrokes("r");
        drag(&view, cx, Pos::new(100., 100.), Pos::new(230., 210.));
        cx.update(|window, cx| {
            window.click("annotation-shapes-overflow", cx);
            let menu = window.find("annotation-shape-menu").bounds();
            let trigger = window.find("annotation-shapes-overflow").bounds();
            assert!(
                menu.bottom() < trigger.top(),
                "Overflow opens above the bottom toolbar"
            );
            for i in 0usize..17 {
                window.find(("annotation-shape", i));
            }
            assert!(window.try_find(("annotation-shape", 17usize)).is_none());
            window.click(("annotation-shape", 4usize), cx);
        });
        cx.update(|_, cx| {
            let preview = view.read(cx);
            assert_eq!(preview.editor.tool, Tool::Hexagon);
            assert_eq!(
                preview.doc().unwrap().marks[0].tool,
                Tool::Rectangle,
                "Bottom picker chooses the next drawing tool"
            );
        });
        drag(&view, cx, Pos::new(100., 100.), Pos::new(230., 210.));
        select_at(&view, cx, Pos::new(165., 100.));
        cx.update(|window, cx| {
            window.click("annotation-shape-picker", cx);
            let menu = window.find("annotation-shape-menu").bounds();
            let trigger = window.find("annotation-shape-picker").bounds();
            assert!(
                (menu.center().y - trigger.center().y).as_f32().abs() < 1.,
                "Properties picker is centered beside its Shape button"
            );
            assert!(
                (trigger.left() - menu.right() - px(8.)).as_f32().abs() < 1.,
                "Properties picker sits just to the left of its button"
            );
        });
        let viewport = cx.update(|window, _| window.viewport_size());
        cx.simulate_resize(gpui_kit::size(px(800.), px(560.)));
        cx.update(|window, cx| {
            window.render_frame(cx);
            let menu = window.find("annotation-shape-menu").bounds();
            assert!(menu.top() >= px(TOOLBAR_HEIGHT + 8.));
            assert!(menu.bottom() <= window.viewport_size().height - px(8.));
        });
        cx.simulate_resize(viewport);
        cx.update(|window, cx| {
            window.render_frame(cx);
            window.click(("annotation-shape", 9usize), cx);
        });
        cx.update(|_, cx| {
            let preview = view.read(cx);
            assert_eq!(preview.editor.selected, Some(1));
            assert_eq!(preview.doc().unwrap().marks.len(), 2);
            assert_eq!(
                preview.doc().unwrap().marks[1].tool,
                Tool::Heart,
                "Properties picker changes the selected object"
            );
        });
        cx.simulate_keystrokes("cmd-z");
        cx.update(|_, cx| assert_eq!(view.read(cx).doc().unwrap().marks[1].tool, Tool::Hexagon));
        cx.update(|window, cx| {
            window.click("annotation-shapes-overflow", cx);
            window.click(("annotation-shape", 16usize), cx);
        });
        drag(&view, cx, Pos::new(110., 130.), Pos::new(240., 210.));
        cx.update(|_, cx| {
            let line = &view.read(cx).doc().unwrap().marks[2];
            assert_eq!(line.tool, Tool::Line);
            assert_eq!(line.style.start, crate::annotation::Cap::None);
            assert_eq!(line.style.end, crate::annotation::Cap::None);
        });
    }
    #[gpui_kit::test]
    fn zoom_menu_preserves_zoom_until_a_choice_and_supports_actual_size_and_fit(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = editor_window(cx);
        let before = cx.update(|_, cx| view.read(cx).zoom);
        cx.update(|window, cx| {
            window.click("preview-fit", cx);
            assert_eq!(
                view.read(cx).zoom,
                before,
                "Opening the menu must not change the zoom"
            );
            assert!(
                window.find("popup-menu").bounds().bottom()
                    <= window.find("preview-fit").bounds().top()
            );
            window.within("popup-menu").click(2usize, cx);
        });
        cx.update(|window, cx| {
            assert!(
                (view.read(cx).transform(window).1 - 1.).abs() < 0.001,
                "100% is one image pixel per logical pixel"
            )
        });
        cx.simulate_keystrokes("shift-1");
        cx.update(|_, cx| assert_eq!(view.read(cx).zoom, 1.));
        cx.simulate_keystrokes("shift-0");
        cx.update(|window, cx| assert!((view.read(cx).transform(window).1 - 1.).abs() < 0.001));
        cx.update(|window, cx| window.click("preview-zoom-in", cx));
        cx.update(|window, cx| {
            assert!((view.read(cx).transform(window).1 - ZOOM_STEP).abs() < 0.001)
        });
        cx.simulate_keystrokes("cmd--");
        cx.update(|window, cx| assert!((view.read(cx).transform(window).1 - 1.).abs() < 0.001));
        cx.update(|window, cx| {
            window.click("preview-fit", cx);
            window.within("popup-menu").click(3usize, cx);
        });
        cx.update(|_, cx| assert_eq!(view.read(cx).zoom, 1.));
    }
    #[gpui_kit::test]
    fn context_palette_is_visible_for_shapes_and_hidden_for_eraser(cx: &mut TestAppContext) {
        let (view, cx) = editor_window(cx);
        cx.update(|window, cx| window.click(("annotation-tool", 3usize), cx));
        cx.update(|window, cx| {
            window.render_frame(cx);
            let swatch = window.find(("annotation-color", 0usize)).bounds();
            assert!(swatch.top() >= px(TOOLBAR_HEIGHT));
            assert!(
                swatch.bottom() < window.viewport_size().height / 2.,
                "Palette belongs at the upper right: {swatch:?}"
            );
            assert!(window.try_find("annotation-shape-menu").is_none());
        });
        cx.update(|window, cx| window.click(("annotation-tool", 9usize), cx));
        cx.update(|window, cx| {
            window.render_frame(cx);
            assert!(window.try_find(("annotation-color", 0usize)).is_none());
            assert!(window.try_find("annotation-opacity").is_none());
            assert!(window.try_find(("annotation-shape", 0usize)).is_none());
            assert_eq!(view.read(cx).editor.tool, Tool::Eraser);
        });
    }
    #[gpui_kit::test]
    fn exit_discard_restores_saved_marks_for_every_image_and_clears_history(
        cx: &mut TestAppContext,
    ) {
        let original = fixture_attachment();
        let other = fixture_attachment();
        let id = original.id;
        let mut saved_mark = Mark::new(Tool::Rectangle, Pos::new(10., 10.), Style::default());
        saved_mark.points[1] = Pos::new(90., 90.);
        let (view, cx) = editor_for(cx, original.clone());
        cx.update(|_, cx| {
            view.update(cx, |preview, cx| {
                preview.show(vec![original.clone(), other.clone()], id, cx)
            })
        });
        cx.update(|window, _| window.activate_window());
        cx.simulate_keystrokes("r");
        drag(&view, cx, Pos::new(100., 100.), Pos::new(230., 210.));
        cx.update(|_, cx| {
            view.update(cx, |preview, _| {
                let image = &mut preview.images[1];
                image.saved_marks = vec![saved_mark.clone()];
                let doc = image.document.as_mut().unwrap();
                doc.marks = image.saved_marks.clone();
                doc.checkpoint();
                doc.marks[0].points[0].x += 40.;
            })
        });
        let before = cx.update(|_, cx| view.read(cx).doc().unwrap().marks.clone());
        cx.simulate_keystrokes("escape escape");
        cx.update(|window, cx| {
            assert!(view.read(cx).focus_handle.is_focused(window));
            assert!(!window.has_active_dialog(cx));
            let preview = view.read(cx);
            assert_eq!(preview.doc().unwrap().marks, before);
            assert!(preview.editor.selected.is_none());
        });
        cx.update(|window, cx| window.click(("annotation-tool", 3usize), cx));
        cx.simulate_keystrokes("escape");
        cx.update(|_, cx| {
            assert_eq!(
                view.read(cx).editor.tool,
                Tool::Rectangle,
                "Escape must not switch tools"
            )
        });
        cx.simulate_keystrokes("cmd-w");
        cx.run_until_parked();
        cx.update(|window, cx| {
            assert!(window.has_active_dialog(cx));
            assert_eq!(view.read(cx).doc().unwrap().marks, before);
            window.click("cancel", cx);
        });
        cx.update(|window, cx| {
            assert!(!window.has_active_dialog(cx));
            assert_eq!(view.read(cx).doc().unwrap().marks, before);
            view.update(cx, |preview, cx| {
                assert!(
                    !preview.should_close(window, cx),
                    "The native close button must also ask first"
                )
            });
            window.render_frame(cx);
        });
        cx.simulate_keystrokes("escape");
        cx.update(|window, cx| {
            assert!(
                !window.has_active_dialog(cx),
                "Escape cancels the exit dialog"
            );
            assert_eq!(view.read(cx).doc().unwrap().marks, before);
            view.update(cx, |preview, cx| assert!(!preview.should_close(window, cx)));
            window.render_frame(cx);
            window.click("preview-exit-discard", cx);
        });
        cx.update(|window, cx| {
            assert!(view.read(cx).images.is_empty());
            view.update(cx, |preview, cx| {
                assert!(
                    preview.should_close(window, cx),
                    "Confirmation approves the native close exactly once"
                );
                preview.show(vec![original, other], id, cx);
                assert!(
                    preview.images[0]
                        .document
                        .as_ref()
                        .unwrap()
                        .marks
                        .is_empty()
                );
                assert_eq!(
                    preview.images[1].document.as_ref().unwrap().marks,
                    vec![saved_mark]
                );
                for image in &preview.images {
                    let doc = image.document.as_ref().unwrap();
                    assert_eq!(doc.marks, image.saved_marks);
                    assert!(!doc.can_undo());
                    assert!(!doc.can_redo());
                }
                assert!(preview.editor.selected.is_none());
                assert_eq!(preview.editor.tool, Tool::Select);
                assert!(!preview.close_confirmed);
            });
        });
    }
    #[gpui_kit::test]
    fn exit_dialog_save_exports_then_closes_only_after_the_image_is_saved(cx: &mut TestAppContext) {
        let (view, cx) = editor_window(cx);
        cx.simulate_keystrokes("r");
        drag(&view, cx, Pos::new(100., 100.), Pos::new(230., 210.));
        let exported = Rc::new(RefCell::new(Vec::new()));
        let capture = exported.clone();
        let _subscription = cx.update(|_, cx| {
            cx.subscribe(&view, move |_, event: &AppliedAnnotation, _| {
                capture.borrow_mut().push(event.id);
            })
        });
        cx.simulate_keystrokes("cmd-w");
        cx.run_until_parked();
        cx.update(|window, cx| {
            assert!(window.has_active_dialog(cx));
            assert_eq!(window.find("ok").label(), Some("Save"));
            assert_eq!(window.find("preview-exit-discard").label(), Some("Discard"));
            assert_eq!(window.find("cancel").label(), Some("Cancel"));
            window.click("ok", cx);
            assert!(!window.has_active_dialog(cx));
            let preview = view.read(cx);
            assert!(preview.saving.is_some());
            assert_eq!(preview.doc().unwrap().marks.len(), 1);
            assert!(!preview.close_confirmed);
        });
        cx.run_until_parked();
        assert_eq!(exported.borrow().len(), 1);
        cx.update(|window, cx| {
            view.update(cx, |preview, cx| {
                preview.annotation_saved(exported.borrow()[0], Ok(()), window, cx);
                assert!(preview.images.is_empty());
                assert!(preview.close_confirmed);
                assert!(!window.has_active_dialog(cx));
            });
        });
    }

    #[gpui_kit::test]
    fn escape_leaves_typed_text_in_the_preview_and_clears_focus(cx: &mut TestAppContext) {
        let (view, cx) = editor_window(cx);
        cx.update(|window, _| window.activate_window());
        cx.simulate_keystrokes("t");
        let at = pointer(&view, cx, Pos::new(100., 130.));
        cx.simulate_click(at, Modifiers::default());
        cx.run_until_parked();
        cx.simulate_keystrokes("draft");
        cx.simulate_keystrokes("escape escape");
        cx.update(|window, cx| {
            assert!(view.read(cx).focus_handle.is_focused(window));
            assert!(!window.has_active_dialog(cx));
            let preview = view.read(cx);
            assert_eq!(preview.doc().unwrap().marks.len(), 1);
            assert_eq!(preview.doc().unwrap().marks[0].text, "draft");
            assert!(preview.editor.editing.is_none());
            assert!(preview.editor.selected.is_none());
            assert_eq!(preview.editor.tool, Tool::Select);
        });
    }
    #[gpui_kit::test]
    fn an_apply_finishing_after_exit_cannot_restore_discarded_edits(cx: &mut TestAppContext) {
        let original = fixture_attachment();
        let id = original.id;
        let (view, cx) = editor_for(cx, original.clone());
        cx.simulate_keystrokes("r");
        drag(&view, cx, Pos::new(100., 100.), Pos::new(230., 210.));
        let applied = Rc::new(std::cell::Cell::new(false));
        let capture = applied.clone();
        let _subscription = cx.update(|_, cx| {
            cx.subscribe(&view, move |_, _: &AppliedAnnotation, _| capture.set(true))
        });
        cx.update(|window, cx| {
            view.update(cx, |preview, cx| {
                preview.apply_annotation(window, cx);
                preview.clear(cx);
                preview.show(vec![original], id, cx);
            })
        });
        cx.run_until_parked();
        assert!(!applied.get());
        cx.update(|_, cx| assert!(view.read(cx).doc().unwrap().marks.is_empty()));
    }
    #[gpui_kit::test]
    fn save_and_discard_start_inactive_and_clean_exit_needs_no_confirmation(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = editor_window(cx);
        cx.update(|window, cx| {
            assert_eq!(window.find("annotation-apply").label(), Some("Save"));
            assert_eq!(window.find("annotation-revert").label(), Some("Discard"));
            assert!(!view.read(cx).has_unsaved_changes());
            assert!(!view.read(cx).can_discard());
            window.click("annotation-apply", cx);
            window.click("annotation-revert", cx);
            assert!(view.read(cx).saving.is_none());
            assert!(!window.has_active_dialog(cx));
            view.update(cx, |preview, cx| {
                assert_eq!(preview.images.len(), 1);
                preview.confirm_exit(window, cx);
                assert!(preview.images.is_empty());
                assert!(!window.has_active_dialog(cx));
                assert!(preview.should_close(window, cx));
            });
        });
    }

    #[gpui_kit::test]
    fn save_button_keeps_its_size_and_fits_loading_content(cx: &mut TestAppContext) {
        let (view, cx) = editor_window(cx);
        let idle = cx.update(|window, _| window.find("annotation-apply").bounds());
        cx.simulate_keystrokes("r");
        drag(&view, cx, Pos::new(100., 100.), Pos::new(230., 210.));
        cx.update(|window, cx| {
            window.render_frame(cx);
            assert_eq!(window.find("annotation-apply").bounds(), idle);
            window.click("annotation-apply", cx);
            window.render_frame(cx);
            assert!(view.read(cx).saving.is_some());
            let button = window.find("annotation-apply");
            assert_eq!(button.bounds(), idle, "Saving must not resize or move Save");
            assert_eq!(button.label(), Some("Saving…"));

            let label = button.label().unwrap();
            let mut text_style = window.text_style();
            text_style.font_family = cx.theme().font_family.clone();
            let label_width = window
                .text_system()
                .shape_line(
                    label.into(),
                    window.rem_size() * 0.875,
                    &[text_style.to_run(label.len())],
                    None,
                )
                .width;
            // App button padding (2 × 0.75 rem), medium spinner (1 rem), gap (0.5 rem).
            let required_width = label_width + window.rem_size() * 3.;
            assert!(
                button.bounds().size.width >= required_width,
                "Save needs {required_width:?} for its spinner and full label, got {:?}",
                button.bounds().size.width
            );
        });
    }

    #[gpui_kit::test]
    fn rejected_saves_keep_edits_open_and_allow_retry_without_duplicate_exports(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = editor_window(cx);
        cx.simulate_keystrokes("r");
        drag(&view, cx, Pos::new(100., 100.), Pos::new(230., 210.));
        let exported = Rc::new(RefCell::new(Vec::new()));
        let capture = exported.clone();
        let _subscription = cx.update(|_, cx| {
            cx.subscribe(&view, move |_, event: &AppliedAnnotation, _| {
                capture.borrow_mut().push(event.id)
            })
        });
        cx.update(|window, cx| {
            window.click("annotation-apply", cx);
            window.render_frame(cx);
            assert!(view.read(cx).saving.is_some());
            assert_eq!(window.find("annotation-apply").label(), Some("Saving…"));
            window.click("annotation-apply", cx);
            window.click("annotation-revert", cx);
            assert!(!window.has_active_dialog(cx));
        });
        let before = cx.update(|_, cx| view.read(cx).doc().unwrap().marks.clone());
        drag(&view, cx, Pos::new(150., 150.), Pos::new(210., 180.));
        cx.update(|_, cx| {
            assert_eq!(
                view.read(cx).doc().unwrap().marks,
                before,
                "The saving shield must block canvas edits until the snapshot is accepted"
            )
        });
        cx.run_until_parked();
        assert_eq!(
            exported.borrow().len(),
            1,
            "Repeated Save must not start another export"
        );
        let id = exported.borrow()[0];
        cx.update(|window, cx| {
            view.update(cx, |preview, cx| {
                preview.annotation_saved(id, Err("Finish the current upload first."), window, cx);
                assert!(preview.saving.is_none());
                assert!(preview.has_unsaved_changes());
                assert!(preview.doc().unwrap().can_undo());
                assert_eq!(preview.doc().unwrap().marks.len(), 1);
                assert!(!preview.close_confirmed);
            });
            window.render_frame(cx);
            window.click("annotation-apply", cx);
        });
        cx.run_until_parked();
        assert_eq!(exported.borrow().len(), 2);
        cx.update(|window, cx| {
            view.update(cx, |preview, cx| {
                preview.annotation_saved(id, Ok(()), window, cx);
                assert!(preview.images.is_empty());
                assert!(!window.has_active_dialog(cx));
            });
        });
    }

    #[gpui_kit::test]
    fn failed_png_export_clears_loading_and_keeps_the_preview_editable(cx: &mut TestAppContext) {
        let (view, cx) = editor_window(cx);
        cx.update(|window, cx| {
            view.update(cx, |preview, cx| {
                let doc = preview.doc_mut().unwrap();
                doc.checkpoint();
                let mut mark = Mark::new(Tool::Rectangle, Pos::default(), Style::default());
                mark.points[1] = Pos::new(1_000_000., 1_000_000.);
                doc.marks.push(mark);
                preview.apply_annotation(window, cx);
                assert!(preview.saving.is_some());
            });
        });
        cx.run_until_parked();
        cx.update(|_, cx| {
            let preview = view.read(cx);
            assert!(preview.saving.is_none());
            assert!(preview.has_unsaved_changes());
            assert!(preview.doc().unwrap().can_undo());
            assert_eq!(preview.images.len(), 1);
            assert!(!preview.close_confirmed);
        });
    }

    #[gpui_kit::test]
    fn save_waits_for_every_changed_image_before_closing(cx: &mut TestAppContext) {
        let first = fixture_attachment();
        let second = fixture_attachment();
        let first_id = first.id;
        let second_id = second.id;
        let (view, cx) = editor_for(cx, first.clone());
        cx.update(|_, cx| {
            view.update(cx, |preview, cx| {
                preview.show(vec![first, second], first_id, cx);
                for image in &mut preview.images {
                    let doc = image.document.as_mut().unwrap();
                    doc.checkpoint();
                    let mut mark = Mark::new(Tool::Rectangle, Pos::new(20., 20.), Style::default());
                    mark.points[1] = Pos::new(100., 100.);
                    doc.marks.push(mark);
                }
                preview.refresh_annotations(cx);
            });
        });
        let exported = Rc::new(RefCell::new(Vec::new()));
        let capture = exported.clone();
        let _subscription = cx.update(|_, cx| {
            cx.subscribe(&view, move |_, event: &AppliedAnnotation, _| {
                capture.borrow_mut().push(event.id)
            })
        });
        cx.update(|window, cx| window.click("annotation-apply", cx));
        cx.run_until_parked();
        assert_eq!(*exported.borrow(), vec![first_id, second_id]);
        cx.update(|window, cx| {
            view.update(cx, |preview, cx| {
                preview.annotation_saved(first_id, Ok(()), window, cx);
                assert_eq!(preview.images.len(), 2);
                assert!(preview.saving.is_some());
                assert_eq!(
                    preview.images[0].saved_marks,
                    preview.images[0].document.as_ref().unwrap().marks
                );
                preview.annotation_saved(second_id, Ok(()), window, cx);
                assert!(preview.images.is_empty());
                assert!(preview.saving.is_none());
                assert!(preview.close_confirmed);
                assert!(!window.has_active_dialog(cx));
            });
        });
    }

    #[gpui_kit::test]
    fn saving_then_discarding_keeps_saved_annotations_editable(cx: &mut TestAppContext) {
        let original = fixture_attachment();
        let source = original.image().unwrap().preview.clone();
        let session = original.image().unwrap().annotation_session.clone();
        let id = original.id;
        let (view, cx) = editor_for(cx, original);
        let applied = Rc::new(RefCell::new(None));
        let capture = applied.clone();
        let _subscription = cx.update(|_, cx| {
            cx.subscribe(&view, move |_, event: &AppliedAnnotation, _| {
                *capture.borrow_mut() = Some(event.attachment.clone())
            })
        });
        cx.update(|_, cx| {
            view.update(cx, |preview, cx| {
                let mut mark = Mark::new(Tool::Rectangle, Pos::new(-40., 40.), Style::default());
                mark.points[1] = Pos::new(120., 140.);
                let doc = preview.doc_mut().unwrap();
                doc.checkpoint();
                doc.marks.push(mark);
                preview.refresh_annotations(cx);
            })
        });
        cx.update(|window, cx| {
            window.render_frame(cx);
            assert!(window.try_find("annotation-save").is_none());
            window.click("annotation-apply", cx);
        });
        cx.run_until_parked();
        let mut staged = applied
            .borrow_mut()
            .take()
            .expect("Save exports the staged image");
        assert_eq!(staged.name, "test.png");
        assert!(staged.image().unwrap().pixel_size.0 > 400);
        staged.id = id;
        if let crate::model::Media::Image(image) = &mut staged.media {
            image.annotation_session = session;
        }
        cx.update(|window, cx| {
            view.update(cx, |preview, cx| {
                assert!(
                    preview.saving.is_some(),
                    "Wait for the uploader's acknowledgement"
                );
                preview.annotation_saved(id, Ok(()), window, cx);
                assert!(
                    preview.images.is_empty(),
                    "Successful Save closes and releases the preview"
                );
                assert!(!window.has_active_dialog(cx));
                assert!(preview.should_close(window, cx));
                preview.show(vec![staged.clone()], id, cx);
                assert!(!preview.has_unsaved_changes());
                assert!(preview.doc().unwrap().can_undo());
                assert_eq!(preview.doc().unwrap().marks.len(), 1);
                assert_eq!(preview.images[0].pixel_size, (400, 300));
                assert!(
                    Arc::ptr_eq(&preview.images[0].image, &source),
                    "An open session must draw the original, not the already-flattened replacement"
                );
            })
        });
        cx.update(|window, cx| window.click("annotation-undo", cx));
        cx.update(|_, cx| {
            assert!(view.read(cx).doc().unwrap().marks.is_empty());
            assert!(
                view.read(cx).has_unsaved_changes(),
                "Undoing saved work needs another save"
            );
        });
        cx.update(|_, cx| {
            view.update(cx, |preview, cx| {
                preview.show(vec![staged.clone()], id, cx);
            })
        });
        cx.update(|window, cx| window.click("annotation-redo", cx));
        cx.update(|window, cx| {
            assert_eq!(view.read(cx).doc().unwrap().marks.len(), 1);
            assert!(!view.read(cx).has_unsaved_changes());
            window.click("annotation-apply", cx);
            assert!(
                view.read(cx).saving.is_none(),
                "Saved work cannot be applied twice"
            );
            view.update(cx, |preview, cx| {
                assert!(
                    preview.should_close(window, cx),
                    "Saved previews close without confirmation"
                );
                assert!(!window.has_active_dialog(cx));
                preview.show(vec![staged.clone()], id, cx);
                assert!(preview.doc().unwrap().can_undo());
            });
        });
        // Save a moved shape again, then make a third, unsaved change.
        cx.update(|window, cx| {
            view.update(cx, |preview, cx| {
                let doc = preview.doc_mut().unwrap();
                doc.checkpoint();
                for point in &mut doc.marks[0].points {
                    point.x += 20.;
                }
                preview.apply_annotation(window, cx);
            });
        });
        cx.run_until_parked();
        let mut latest = applied
            .borrow_mut()
            .take()
            .expect("Second save exports edits");
        latest.id = id;
        if let crate::model::Media::Image(image) = &mut latest.media {
            image.annotation_session = staged.image().unwrap().annotation_session.clone();
        }
        staged = latest;
        let saved_marks = cx.update(|window, cx| {
            view.update(cx, |preview, cx| {
                let saved = preview.doc().unwrap().marks.clone();
                preview.annotation_saved(id, Ok(()), window, cx);
                preview.show(vec![staged.clone()], id, cx);
                let doc = preview.doc_mut().unwrap();
                doc.checkpoint();
                doc.marks[0].points[0].x += 50.;
                preview.refresh_annotations(cx);
                assert!(preview.has_unsaved_changes());
                saved
            })
        });
        cx.update(|window, cx| {
            window.click("annotation-revert", cx);
            assert!(window.has_active_dialog(cx));
            assert_eq!(view.read(cx).doc().unwrap().marks.len(), 1);
        });
        cx.update(|window, cx| window.click("cancel", cx));
        cx.update(|_, cx| assert_eq!(view.read(cx).doc().unwrap().marks.len(), 1));
        cx.update(|window, cx| {
            let revert = window.find("annotation-revert").bounds();
            let apply = window.find("annotation-apply").bounds();
            assert!(revert.bottom() <= px(TOOLBAR_HEIGHT));
            assert_eq!(revert.center().y, apply.center().y);
            assert!(revert.right() <= apply.left());
            window.click("annotation-revert", cx);
        });
        cx.update(|window, cx| window.click("ok", cx));
        cx.update(|window, cx| {
            let preview = view.read(cx);
            assert!(
                preview.images.is_empty(),
                "Confirmed Discard closes the preview"
            );
            assert!(preview.close_confirmed);
            assert!(!window.has_active_dialog(cx));
        });
        cx.update(|_, cx| {
            view.update(cx, |preview, cx| {
                preview.clear(cx);
                preview.show(vec![staged], id, cx);
                let doc = preview.doc().unwrap();
                assert_eq!(
                    doc.base.dimensions(),
                    (400, 300),
                    "Discard must keep the original source, not decode flattened staged pixels"
                );
                assert_eq!(
                    doc.marks, saved_marks,
                    "Discard must restore the latest save"
                );
                assert!(Arc::ptr_eq(&preview.images[0].image, &source));
                assert!(!doc.can_undo());
                assert!(!doc.can_redo());
                assert!(!preview.has_unsaved_changes());
                assert!(!preview.can_discard());
            });
        });
        // Reopened saved shapes still respond to selection and dragging.
        drag(&view, cx, Pos::new(140., 100.), Pos::new(170., 130.));
        let expected_png = cx.update(|_, cx| {
            let preview = view.read(cx);
            let doc = preview.doc().unwrap();
            assert_eq!(preview.editor.selected, Some(0));
            assert_ne!(doc.marks, saved_marks);
            assert!(preview.has_unsaved_changes());
            assert!(doc.can_undo());
            doc.png(None).unwrap().0
        });
        cx.update(|window, cx| window.click("annotation-apply", cx));
        cx.run_until_parked();
        let exported = applied
            .borrow_mut()
            .take()
            .expect("Save after Discard succeeds");
        assert_eq!(exported.image().unwrap().bytes.as_ref(), expected_png);
        cx.update(|window, cx| {
            view.update(cx, |preview, cx| {
                preview.annotation_saved(id, Ok(()), window, cx);
                assert!(preview.images.is_empty());
            });
        });
    }
    #[gpui_kit::test]
    fn pixelation_paints_fixed_strokes_with_a_zoom_independent_brush(cx: &mut TestAppContext) {
        let (view, cx) = editor_window(cx);
        cx.simulate_keystrokes("x");
        let hover = pointer(&view, cx, Pos::new(100., 140.));
        cx.simulate_mouse_move(hover, None, Modifiers::default());
        cx.update(|window, cx| {
            window.render_frame(cx);
            let cursor = window.find("pixelate-brush-cursor").bounds();
            assert!(
                (cursor.size.width.as_f32() - 64.).abs() < 0.6,
                "Cursor {cursor:?}, expected diameter {}",
                64.
            );
            assert!((cursor.center().x - hover.x).abs() < px(0.6));
            assert!((cursor.center().y - hover.y).abs() < px(0.6));
            assert!(window.try_find(("annotation-color", 0usize)).is_none());
            assert!(window.try_find("pixelate-size").is_some());
            assert!(window.try_find(("pixelate-size", 1usize)).is_none());
            let preview = view.read(cx);
            assert!(preview.pixelate_preview(window).is_some());
            assert!(preview.doc().unwrap().marks.is_empty());
            assert!(!preview.doc().unwrap().can_undo());
            assert!(!preview.has_unsaved_changes());
            assert!(
                window.find(("annotation-tool", 8usize)).bounds().left()
                    > window.find("annotation-shapes-overflow").bounds().right()
            );
        });
        drag(&view, cx, Pos::new(100., 140.), Pos::new(210., 140.));
        let painted = cx.update(|_, cx| {
            let preview = view.read(cx);
            assert_eq!(
                preview.editor.tool,
                Tool::Redact,
                "Brush stays active between strokes"
            );
            assert!(preview.editor.selected.is_none());
            let doc = preview.doc().unwrap();
            assert!(doc.hit(Pos::new(150., 140.), 2.).is_none());
            doc.marks[0].clone()
        });
        cx.simulate_keystrokes("v");
        drag(&view, cx, Pos::new(150., 140.), Pos::new(200., 180.));
        cx.update(|window, cx| {
            window.render_frame(cx);
            assert!(window.try_find("annotation-selection-box").is_none());
            assert!(window.try_find(("annotation-handle", 0usize)).is_none());
            assert_eq!(view.read(cx).doc().unwrap().marks[0], painted);
        });
        cx.simulate_keystrokes("e");
        drag(&view, cx, Pos::new(150., 140.), Pos::new(180., 140.));
        cx.update(|_, cx| assert_eq!(view.read(cx).doc().unwrap().marks[0], painted));
        cx.simulate_keystrokes("x");
        let p = pointer(&view, cx, Pos::new(80., 80.));
        cx.simulate_click(p, Modifiers::default());
        cx.update(|_, cx| {
            assert_eq!(
                view.read(cx).doc().unwrap().marks.len(),
                2,
                "A single click paints a circular dab"
            )
        });
        cx.simulate_keystrokes("cmd-z");
        cx.update(|_, cx| assert_eq!(view.read(cx).doc().unwrap().marks.len(), 1));
        cx.simulate_keystrokes("cmd-shift-z");
        cx.update(|_, cx| assert_eq!(view.read(cx).doc().unwrap().marks.len(), 2));
        let slider_at = |cx: &mut gpui_kit::VisualTestContext, fraction: f32| {
            cx.update(|window, cx| {
                window.render_frame(cx);
                let b = window.find("pixelate-size").bounds();
                point(
                    b.left() + px(9.) + (b.size.width - px(18.)) * fraction,
                    b.center().y,
                )
            })
        };
        let from = slider_at(cx, 0.25);
        let at = slider_at(cx, 0.75);
        cx.simulate_mouse_down(from, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_move(at, Some(MouseButton::Left), Modifiers::default());
        cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::default());
        cx.update(|_, cx| {
            assert!((view.read(cx).pixelate_width() - 90.75).abs() < 0.01);
            assert_eq!(view.read(cx).doc().unwrap().marks[0], painted);
        });
        for zoom in [0.2, 0.6, 2.4, 0.6] {
            cx.update(|_, cx| {
                view.update(cx, |preview, cx| {
                    preview.zoom = zoom;
                    cx.notify();
                });
            });
            let center = cx.update(|_, cx| {
                let doc = view.read(cx).doc().unwrap();
                Pos::new(doc.base.width() as f32 / 2., doc.base.height() as f32 / 2.)
            });
            let hover = pointer(&view, cx, center);
            cx.simulate_mouse_move(hover, None, Modifiers::default());
            cx.update(|window, cx| {
                window.render_frame(cx);
                let expected = 90.75;
                let preview = view.read(cx);
                assert!(
                    (preview.pixelate_preview(window).unwrap().1 * preview.transform(window).1
                        - expected)
                        .abs()
                        < 0.01,
                    "Hover coverage must match the screen-space brush diameter after zooming"
                );
                assert!(
                    (window
                        .find("pixelate-brush-cursor")
                        .bounds()
                        .size
                        .width
                        .as_f32()
                        - expected)
                        .abs()
                        < 0.6
                );
            });
        }
        cx.update(|window, cx| window.hover(("annotation-tool", 8usize), cx));
        cx.update(|window, cx| {
            window.render_frame(cx);
            assert!(
                window.try_find("pixelate-brush-cursor").is_none(),
                "Hide the brush over controls"
            );
        });
        // Brush-size changes affect new strokes only and never consume artwork history.
        drag(&view, cx, Pos::new(100., 180.), Pos::new(140., 180.));
        cx.update(|window, cx| {
            let preview = view.read(cx);
            let doc = preview.doc().unwrap();
            assert_eq!(doc.marks.len(), 3);
            assert!((doc.marks[2].style.width * preview.transform(window).1 - 90.75).abs() < 0.01);
            assert_eq!(doc.marks[0], painted);
        });
        cx.simulate_keystrokes("cmd-z");
        let at = slider_at(cx, 0.);
        cx.simulate_click(at, Modifiers::default());
        drag(&view, cx, Pos::new(100., 180.), Pos::new(140., 180.));
        cx.update(|window, cx| {
            let preview = view.read(cx);
            assert_eq!(preview.pixelate_width(), 30.);
            let doc = preview.doc().unwrap();
            assert_eq!(
                doc.marks.len(),
                3,
                "The minimum still paints a visible stroke"
            );
            assert!((doc.marks[2].style.width * preview.transform(window).1 - 30.).abs() < 0.01);
            assert!(
                !doc.can_redo(),
                "Painting a minimum-size stroke replaces redo history"
            );
        });
        cx.simulate_keystrokes("cmd-z");
        let at = slider_at(cx, 1.);
        cx.simulate_click(at, Modifiers::default());
        cx.update(|_, cx| assert_eq!(view.read(cx).pixelate_width(), 138.));
        cx.simulate_keystrokes("cmd-shift-z");
        cx.update(|_, cx| assert_eq!(view.read(cx).doc().unwrap().marks.len(), 3));
    }
    #[gpui_kit::test]
    fn stroke_presets_follow_tldraw_steps(cx: &mut TestAppContext) {
        let (view, cx) = editor_window(cx);
        cx.simulate_keystrokes("p");
        let selected = |window: &mut Window, cx: &mut App| {
            window.render_frame(cx);
            (0..4)
                .filter(|i| window.find(("annotation-size", *i)).checked() == Some(true))
                .collect::<Vec<_>>()
        };
        cx.update(|window, cx| {
            assert_eq!(
                selected(window, cx),
                vec![1],
                "Default medium size is selected"
            )
        });
        for (index, width) in crate::annotation::STROKE_WIDTHS.into_iter().enumerate() {
            cx.update(|window, cx| window.click(("annotation-size", index), cx));
            cx.update(|window, cx| {
                assert_eq!(view.read(cx).editor.style.width, width);
                assert_eq!(selected(window, cx), vec![index]);
            });
        }
        cx.update(|window, cx| {
            view.update(cx, |preview, cx| {
                preview.editor.style.width = 3.8;
                preview.refresh_annotations(cx);
            });
            assert_eq!(
                selected(window, cx),
                vec![1],
                "Custom strokes still highlight one preset"
            );
            assert_eq!(
                view.read(cx).editor.style.width,
                3.8,
                "Showing a preset never changes the artwork"
            );
        });
        cx.simulate_keystrokes("t");
        cx.update(|window, cx| {
            view.update(cx, |preview, cx| {
                preview.editor.style.font_size = 43.;
                preview.refresh_annotations(cx);
            });
            assert_eq!(
                selected(window, cx),
                vec![3],
                "Continuous text sizes keep a preset selected too"
            );
            assert_eq!(view.read(cx).editor.style.font_size, 43.);
        });
    }
    #[gpui_kit::test]
    fn transparent_interiors_pass_through_until_selected_then_move_the_shape(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = editor_window(cx);
        cx.update(|_, cx| {
            view.update(cx, |preview, cx| {
                let mut below = Mark::new(
                    Tool::Rectangle,
                    Pos::new(110., 110.),
                    Style {
                        fill: crate::annotation::Fill::Solid,
                        ..Default::default()
                    },
                );
                below.points[1] = Pos::new(170., 170.);
                let mut above = Mark::new(Tool::Rectangle, Pos::new(80., 80.), Style::default());
                above.points[1] = Pos::new(230., 200.);
                preview.doc_mut().unwrap().marks = vec![below, above];
                preview.refresh_annotations(cx);
            })
        });
        let at = pointer(&view, cx, Pos::new(140., 140.));
        cx.simulate_click(at, Modifiers::default());
        cx.update(|_, cx| {
            assert_eq!(
                view.read(cx).editor.selected,
                Some(0),
                "Unselected transparent shapes pass through"
            )
        });
        cx.simulate_keystrokes("escape");
        let at = pointer(&view, cx, Pos::new(200., 150.));
        cx.simulate_click(at, Modifiers::default());
        cx.update(|_, cx| assert!(view.read(cx).editor.selected.is_none()));
        let border = pointer(&view, cx, Pos::new(80., 140.));
        cx.simulate_click(border, Modifiers::default());
        cx.update(|_, cx| assert_eq!(view.read(cx).editor.selected, Some(1)));
        cx.simulate_mouse_move(at, None, Modifiers::default());
        cx.update(|_, cx| {
            assert_eq!(view.read(cx).selection_cursor(), SelectionCursor::Move);
            assert_eq!(
                view.read(cx).selection_cursor().style(),
                CursorStyle::Arrow,
                "Movable interiors keep the regular pointer"
            );
        });
        drag(&view, cx, Pos::new(200., 150.), Pos::new(220., 170.));
        cx.update(|_, cx| {
            let doc = view.read(cx).doc().unwrap();
            assert!((doc.marks[1].bounds().x - 100.).abs() < 0.01);
            assert!((doc.marks[1].bounds().y - 100.).abs() < 0.01);
            assert_eq!(doc.marks[0].bounds().x, 110.);
        });
        let empty = pointer(&view, cx, Pos::new(40., 40.));
        cx.simulate_click(empty, Modifiers::default());
        let at = pointer(&view, cx, Pos::new(220., 170.));
        cx.simulate_click(at, Modifiers::default());
        cx.update(|_, cx| {
            assert!(
                view.read(cx).editor.selected.is_none(),
                "Clicking off ends transparent-interior dragging"
            )
        });
    }
    #[gpui_kit::test]
    fn additive_selection_and_select_all_exclude_pixelation_and_do_not_add_undo_steps(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = editor_window(cx);
        cx.update(|_, cx| {
            view.update(cx, |preview, cx| {
                for x in [80., 210.] {
                    let mut mark = Mark::new(
                        Tool::Rectangle,
                        Pos::new(x, 100.),
                        Style {
                            fill: crate::annotation::Fill::Solid,
                            ..Default::default()
                        },
                    );
                    mark.points[1] = Pos::new(x + 70., 180.);
                    preview.doc_mut().unwrap().marks.push(mark);
                }
                preview.doc_mut().unwrap().marks.push(Mark::new(
                    Tool::Redact,
                    Pos::new(40., 40.),
                    Style::default(),
                ));
                preview.refresh_annotations(cx);
            })
        });
        let first = pointer(&view, cx, Pos::new(110., 140.));
        let second = pointer(&view, cx, Pos::new(240., 140.));
        cx.simulate_click(first, Modifiers::default());
        cx.simulate_click(
            second,
            Modifiers {
                shift: true,
                ..Default::default()
            },
        );
        cx.update(|_, cx| assert_eq!(view.read(cx).selected_indices(), vec![0, 1]));
        cx.simulate_click(
            first,
            Modifiers {
                shift: true,
                ..Default::default()
            },
        );
        cx.update(|_, cx| assert_eq!(view.read(cx).selected_indices(), vec![1]));
        cx.simulate_keystrokes("cmd-a");
        cx.update(|_, cx| {
            let preview = view.read(cx);
            assert_eq!(preview.selected_indices(), vec![0, 1]);
            assert!(!preview.doc().unwrap().can_undo());
        });
        cx.simulate_mouse_down(first, MouseButton::Right, Modifiers::default());
        cx.simulate_mouse_up(first, MouseButton::Right, Modifiers::default());
        cx.update(|_, cx| {
            assert_eq!(
                view.read(cx).selected_indices(),
                vec![0, 1],
                "Right-clicking a member preserves the selection"
            )
        });
    }
    fn cursor_frame(
        view: &Entity<ImagePreview>,
        cx: &mut VisualTestContext,
        rotation: f32,
        grouped: bool,
    ) -> (crate::annotation::Frame, f32) {
        cx.update(|window, cx| {
            view.update(cx, |preview, cx| {
                let mut mark = Mark::new(Tool::Rectangle, Pos::new(130., 100.), Style::default());
                mark.points[1] = Pos::new(230., 180.);
                mark.rotation = rotation;
                let frame = mark.frame();
                let mut inner = mark.clone();
                let center = frame.center();
                inner.points = vec![
                    Pos::new(center.x - 15., center.y - 10.),
                    Pos::new(center.x + 15., center.y + 10.),
                ];
                preview.doc_mut().unwrap().marks = if grouped {
                    vec![mark, inner]
                } else {
                    vec![mark]
                };
                preview.editor.tool = Tool::Select;
                preview.set_selection(if grouped { vec![0, 1] } else { vec![0] });
                if grouped {
                    preview.editor.group_frame = Some(frame);
                }
                preview.refresh_annotations(cx);
                (
                    preview.selection_frame().unwrap(),
                    preview.transform(window).1,
                )
            })
        })
    }
    #[gpui_kit::test]
    fn crossing_a_corner_swaps_the_resize_cursor_and_can_cross_back(cx: &mut TestAppContext) {
        let (view, cx) = editor_window(cx);
        for rotation in [0_f32, 17_f32.to_radians()] {
            for grouped in [false, true] {
                let (frame, _) = cursor_frame(&view, cx, rotation, grouped);
                let fixed = frame.bounds.corners()[2];
                let from = frame.bounds.corners()[0];
                let to = Pos::new(fixed.x + (fixed.x - from.x), from.y);
                drag(&view, cx, frame.to_world(from), frame.to_world(to));
                cx.update(|_, cx| {
                    let preview = view.read(cx);
                    let expected = ((135. + rotation.to_degrees()).round().rem_euclid(180.)) as u16;
                    assert_eq!(
                        preview.selection_cursor(),
                        SelectionCursor::Resize(expected)
                    );
                    assert!(preview.doc().unwrap().marks[0].flip_x);
                });
                drag(&view, cx, frame.to_world(to), frame.to_world(from));
                cx.update(|_, cx| {
                    let preview = view.read(cx);
                    let expected = ((45. + rotation.to_degrees()).round().rem_euclid(180.)) as u16;
                    assert_eq!(
                        preview.selection_cursor(),
                        SelectionCursor::Resize(expected)
                    );
                    assert!(!preview.doc().unwrap().marks[0].flip_x);
                    assert!(
                        (preview.doc().unwrap().marks[0].bounds().w - frame.bounds.w).abs() < 0.01
                    );
                });
            }
        }
    }

    #[gpui_kit::test]
    fn rotating_a_group_keeps_multiline_arrow_labels_clear_and_is_undoable(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = editor_window(cx);
        let (frame, scale, before) = cx.update(|window, cx| {
            view.update(cx, |preview, cx| {
                let mut arrow = Mark::new(
                    Tool::Arrow,
                    Pos::new(65., 220.),
                    Style {
                        width: 5.,
                        start: crate::annotation::Cap::Circle,
                        ..Default::default()
                    },
                );
                arrow.style.font_size = 28.;
                arrow.points[1] = Pos::new(335., 220.);
                arrow.set_text(
                    "Hello, this is\nthe end\nof glory\nand I'm holding\nof\na\nwith you".into(),
                );
                let mut rectangle =
                    Mark::new(Tool::Rectangle, Pos::new(180., 210.), Style::default());
                rectangle.points[1] = Pos::new(220., 230.);
                preview.doc_mut().unwrap().marks = vec![arrow, rectangle];
                preview.set_selection(vec![0, 1]);
                preview.editor.tool = Tool::Select;
                preview.refresh_annotations(cx);
                (
                    preview.selection_frame().unwrap(),
                    preview.transform(window).1,
                    preview.doc().unwrap().marks.clone(),
                )
            })
        });
        let corner = frame.bounds.corners()[1];
        let start = frame.to_world(Pos::new(corner.x + 12. / scale, corner.y - 12. / scale));
        drag(
            &view,
            cx,
            start,
            start.rotate(frame.center(), std::f32::consts::FRAC_PI_4),
        );
        cx.update(|_, cx| {
            let mark = &view.read(cx).doc().unwrap().marks[0];
            assert_eq!(mark.style.font_size, 28.);
            assert!(
                mark.points[0].y != mark.points[1].y,
                "The group must actually rotate"
            );
            let label = mark.label_bounds().unwrap();
            for tip in &mark.points[..2] {
                assert!(
                    !label.contains(*tip, 20.),
                    "Rotating a group must preserve visible arrow ends"
                );
            }
        });
        cx.simulate_keystrokes("cmd-z");
        cx.update(|_, cx| assert_eq!(view.read(cx).doc().unwrap().marks, before));
    }

    #[gpui_kit::test]
    fn groups_resize_through_opposite_edges_with_readable_text_and_undo(cx: &mut TestAppContext) {
        let (view, cx) = editor_window(cx);
        for rotation in [0_f32, 0.5] {
            for (handle, sx, sy) in [
                (Handle::Edge(1), -1.3, 1.3),
                (Handle::Edge(0), 1.3, -1.3),
                (Handle::Corner(2), -1.3, 1.3),
                (Handle::Corner(2), -1.3, -1.3),
            ] {
                cx.update(|_, cx| {
                    view.update(cx, |preview, cx| {
                        preview.doc_mut().unwrap().revert();
                        let mut rectangle =
                            Mark::new(Tool::Triangle, Pos::new(80., 100.), Style::default());
                        rectangle.points[1] = Pos::new(240., 200.);
                        rectangle.set_text("Hi".into());
                        let mut text =
                            Mark::new(Tool::Text, Pos::new(140., 130.), Style::default());
                        text.wrap = Some(100.);
                        text.set_text("Text".into());
                        text.rotation = -0.2;
                        let mut pen = Mark::new(Tool::Pen, Pos::new(90., 210.), Style::default());
                        pen.points = vec![
                            Pos::new(90., 210.),
                            Pos::new(120., 240.),
                            Pos::new(160., 220.),
                        ];
                        let mut arrow =
                            Mark::new(Tool::Arrow, Pos::new(180., 210.), Style::default());
                        // Leave ample label room here; content-constrained resizing has its own tests.
                        arrow.points = vec![
                            Pos::new(180., 210.),
                            Pos::new(320., 260.),
                            Pos::new(250., 180.),
                        ];
                        arrow.set_text("Arrow".into());
                        preview.doc_mut().unwrap().marks = vec![rectangle, text, pen, arrow];
                        preview.editor.tool = Tool::Select;
                        preview.set_selection(vec![0, 1, 2, 3]);
                        preview.refresh_annotations(cx);
                    })
                });
                if rotation != 0. {
                    let (frame, scale) = cx.update(|window, cx| {
                        (
                            view.read(cx).selection_frame().unwrap(),
                            view.read(cx).transform(window).1,
                        )
                    });
                    let c = frame.bounds.corners()[1];
                    let start = frame.to_world(Pos::new(c.x + 12. / scale, c.y - 12. / scale));
                    drag(&view, cx, start, start.rotate(frame.center(), rotation));
                }
                let (frame, before) = cx.update(|_, cx| {
                    (
                        view.read(cx).selection_frame().unwrap(),
                        view.read(cx).doc().unwrap().marks.clone(),
                    )
                });
                let fixed = handle.anchor(frame.bounds);
                let from = handle.point(frame.bounds);
                let to = Pos::new(
                    fixed.x + (from.x - fixed.x) * sx,
                    fixed.y + (from.y - fixed.y) * sy,
                );
                let map = |p: Pos| {
                    let q = frame.to_local(p);
                    frame.to_world(Pos::new(
                        fixed.x + (q.x - fixed.x) * sx,
                        fixed.y + (q.y - fixed.y) * sy,
                    ))
                };
                drag(&view, cx, frame.to_world(from), frame.to_world(to));
                let after = cx.update(|_, cx| {
                    let preview = view.read(cx);
                    let doc = preview.doc().unwrap();
                    for (old, mark) in before.iter().zip(&doc.marks) {
                        assert_eq!(mark.text, old.text);
                        assert!(
                            mark.style.font_size > 0.
                                && mark.bounds().w > 0.
                                && mark.bounds().h > 0.
                        );
                        if matches!(mark.tool, Tool::Arrow | Tool::Pen) {
                            for (a, b) in old.points.iter().zip(&mark.points) {
                                let expected = map(old.frame().to_world(*a));
                                let actual = mark.frame().to_world(*b);
                                assert!(
                                    expected.distance(actual) < 0.02,
                                    "{:?} {rotation} {handle:?}: {expected:?} != {actual:?}",
                                    mark.tool
                                );
                            }
                        } else {
                            let corners = mark.frame().corners();
                            for corner in old.frame().corners() {
                                let expected = map(corner);
                                assert!(
                                    corners.iter().any(|p| p.distance(expected) < 0.02),
                                    "{:?} {rotation} {handle:?}: missing corner {expected:?}",
                                    mark.tool
                                );
                            }
                        }
                        if mark.tool == Tool::Text {
                            assert!(!mark.flip_x && !mark.flip_y);
                        }
                    }
                    doc.marks.clone()
                });
                cx.simulate_keystrokes("cmd-z");
                cx.update(|_, cx| assert_eq!(view.read(cx).doc().unwrap().marks, before));
                cx.simulate_keystrokes("cmd-shift-z");
                cx.update(|_, cx| assert_eq!(view.read(cx).doc().unwrap().marks, after));
            }
        }
    }

    #[gpui_kit::test]
    fn rotation_cursor_artwork_follows_each_corner(cx: &mut TestAppContext) {
        let (view, cx) = editor_window(cx);
        for (rotation, grouped) in [0., 17_f32.to_radians(), -39_f32.to_radians()]
            .into_iter()
            .flat_map(|r| [(r, false), (r, true)])
        {
            let (frame, scale) = cursor_frame(&view, cx, rotation, grouped);
            let mut images = Vec::new();
            for (index, corner) in frame.bounds.corners().into_iter().enumerate() {
                let center = frame.bounds.center();
                let point = frame.to_world(Pos::new(
                    corner.x + (corner.x - center.x).signum() * 12. / scale,
                    corner.y + (corner.y - center.y).signum() * 12. / scale,
                ));
                let at = pointer(&view, cx, point);
                cx.simulate_mouse_move(at, None, Modifiers::default());
                cx.update(|_, cx| {
                    let angle = (rotation.to_degrees() + [270., 0., 90., 180.][index])
                        .round()
                        .rem_euclid(360.) as u16;
                    assert_eq!(
                        view.read(cx).selection_cursor(),
                        SelectionCursor::Rotate(angle)
                    );
                    images.push(
                        cursor::cursor_image(view.read(cx).selection_cursor())
                            .expect("Rotation cursor")
                            .data()
                            .to_vec(),
                    );
                });
            }
            for i in 0..4 {
                for j in i + 1..4 {
                    assert!(
                        images[i] != images[j],
                        "Corners {i} and {j} must curve around different sides of the selection"
                    );
                }
            }
        }
    }
    #[gpui_kit::test]
    fn rotation_cursor_artwork_updates_in_the_same_drag_event(cx: &mut TestAppContext) {
        let (view, cx) = editor_window(cx);
        for grouped in [false, true] {
            let (frame, scale) = cursor_frame(&view, cx, 0., grouped);
            let corner = frame.bounds.corners()[1];
            let start = frame.to_world(Pos::new(corner.x + 12. / scale, corner.y - 12. / scale));
            let end = start.rotate(frame.center(), 17_f32.to_radians());
            let from = pointer(&view, cx, start);
            let to = pointer(&view, cx, end);
            cx.simulate_mouse_move(from, None, Modifiers::default());
            let before = cx.update(|_, cx| {
                cursor::cursor_image(view.read(cx).selection_cursor())
                    .unwrap()
                    .data()
                    .to_vec()
            });
            cx.simulate_mouse_down(from, MouseButton::Left, Modifiers::default());
            cx.update(|window, cx| {
                use gpui_kit::InputEvent as _;
                window.dispatch_event(
                    MouseMoveEvent {
                        position: to,
                        pressed_button: Some(MouseButton::Left),
                        modifiers: Modifiers::default(),
                    }
                    .to_platform_input(),
                    cx,
                );
            });
            cx.update(|_, cx| {
                let preview = view.read(cx);
                assert!(
                    (preview.selection_frame().unwrap().rotation.to_degrees() - 17.).abs() < 0.01
                );
                let after = cursor::cursor_image(preview.selection_cursor()).unwrap();
                assert!(
                    before != after.data(),
                    "The cursor must follow the rotation applied by this pointer event"
                );
            });
            cx.simulate_mouse_up(to, MouseButton::Left, Modifiers::default());
        }
    }

    #[gpui_kit::test]
    fn resize_cursor_artwork_tracks_the_actual_rotated_edge(cx: &mut TestAppContext) {
        let (view, cx) = editor_window(cx);
        for (degrees, grouped) in [0_f32, 17., 39., 73., 101., -22., 270.]
            .into_iter()
            .flat_map(|d| [(d, false), (d, true)])
        {
            let rotation = degrees.to_radians();
            let (frame, _) = cursor_frame(&view, cx, rotation, grouped);
            let b = frame.bounds;
            for (local, base) in [
                (Pos::new(b.x + b.w, b.y + b.h / 2.), 0_f32),
                (Pos::new(b.x + b.w / 2., b.y), 90.),
                (b.corners()[0], 45.),
                (b.corners()[1], 135.),
            ] {
                let at = pointer(&view, cx, frame.to_world(local));
                cx.simulate_mouse_move(at, None, Modifiers::default());
                let image = cx.update(|_, cx| {
                    cursor::cursor_image(view.read(cx).selection_cursor()).expect("Resize cursor")
                });
                // Measure the rendered silhouette's major axis, not just its semantic tag.
                let mut xx = 0.;
                let mut yy = 0.;
                let mut xy = 0.;
                for (i, p) in image.data().as_chunks::<4>().0.iter().enumerate() {
                    let x = (i % 48) as f32 - 23.5;
                    let y = (i / 48) as f32 - 23.5;
                    let weight = f32::from(p[3]);
                    xx += x * x * weight;
                    yy += y * y * weight;
                    xy += x * y * weight;
                }
                let actual = 0.5 * (2. * xy).atan2(xx - yy);
                let expected = rotation + base.to_radians();
                let error = (actual - expected + std::f32::consts::FRAC_PI_2)
                    .rem_euclid(std::f32::consts::PI)
                    - std::f32::consts::FRAC_PI_2;
                assert!(
                    error.abs() < 1.5_f32.to_radians(),
                    "At object angle {degrees} and handle angle {base}, cursor points at {} degrees",
                    actual.to_degrees()
                );
            }
        }
    }

    #[gpui_kit::test]
    fn hover_cursors_survive_repaints_and_reset_over_floating_controls_without_clicking(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = editor_window(cx);
        cx.simulate_keystrokes("r");
        drag(&view, cx, Pos::new(100., 100.), Pos::new(200., 180.));
        select_at(&view, cx, Pos::new(100., 140.));
        let scale = cx.update(|window, cx| view.read(cx).transform(window).1);
        for (p, expected) in [
            (Pos::new(200., 140.), SelectionCursor::Resize(0)),
            (Pos::new(150., 180.), SelectionCursor::Resize(90)),
            (Pos::new(200., 180.), SelectionCursor::Resize(45)),
            (Pos::new(200., 100.), SelectionCursor::Resize(135)),
            (
                Pos::new(200. + 12. / scale, 100. - 12. / scale),
                SelectionCursor::Rotate(0),
            ),
            (Pos::new(150., 140.), SelectionCursor::Move),
        ] {
            for dx in [0., 0.1, 0.] {
                let at = pointer(&view, cx, Pos::new(p.x + dx, p.y));
                cx.simulate_mouse_move(at, None, Modifiers::default());
                cx.update(|window, cx| {
                    window.render_frame(cx);
                    window.render_frame(cx);
                    let preview = view.read(cx);
                    assert_eq!(preview.selection_cursor(), expected);
                    assert!(preview.editor.canvas_hover);
                    assert!(
                        preview.editor.gesture.is_none(),
                        "Hover never starts a drag"
                    );
                });
            }
        }
        cx.update(|window, cx| window.hover(("annotation-size", 1usize), cx));
        cx.update(|window, cx| {
            window.render_frame(cx);
            assert!(!view.read(cx).editor.canvas_hover);
            assert_eq!(view.read(cx).selection_cursor(), SelectionCursor::Arrow);
        });
    }
    #[gpui_kit::test]
    fn borders_resize_and_cursors_match_each_handle(cx: &mut TestAppContext) {
        let (view, cx) = editor_window(cx);
        cx.simulate_keystrokes("r");
        drag(&view, cx, Pos::new(100., 100.), Pos::new(200., 180.));
        select_at(&view, cx, Pos::new(100., 140.));
        for (p, cursor) in [
            (Pos::new(200., 140.), SelectionCursor::Resize(0)),
            (Pos::new(150., 180.), SelectionCursor::Resize(90)),
            (Pos::new(200., 180.), SelectionCursor::Resize(45)),
            (Pos::new(200., 100.), SelectionCursor::Resize(135)),
            (Pos::new(150., 140.), SelectionCursor::Move),
        ] {
            let at = pointer(&view, cx, p);
            cx.simulate_mouse_move(at, None, Modifiers::default());
            cx.update(|_, cx| assert_eq!(view.read(cx).selection_cursor(), cursor));
        }
        drag(&view, cx, Pos::new(200., 140.), Pos::new(260., 140.));
        cx.update(|_, cx| {
            let b = view.read(cx).doc().unwrap().marks[0].bounds();
            assert!((b.x - 100.).abs() < 0.1);
            assert!((b.w - 160.).abs() < 0.1);
            assert!((b.h - 80.).abs() < 0.1);
        });
        drag(&view, cx, Pos::new(180., 180.), Pos::new(180., 240.));
        cx.update(|_, cx| {
            assert!((view.read(cx).doc().unwrap().marks[0].bounds().h - 140.).abs() < 0.1)
        });
    }
    #[gpui_kit::test]
    fn shapes_text_and_pen_rotate_from_outside_corners_with_undo(cx: &mut TestAppContext) {
        let (view, cx) = editor_window(cx);
        for tool in [Tool::Rectangle, Tool::Text, Tool::Pen] {
            cx.update(|_, cx| {
                view.update(cx, |preview, cx| {
                    let mut mark = Mark::new(
                        tool,
                        Pos::new(100., 100.),
                        Style {
                            font_size: 20.,
                            ..Default::default()
                        },
                    );
                    mark.points[1] = Pos::new(220., 180.);
                    if tool == Tool::Text {
                        mark.text = "Rotate me".into();
                    }
                    preview.doc_mut().unwrap().marks = vec![mark];
                    preview.editor.selected = Some(0);
                    preview.editor.tool = Tool::Select;
                    preview.refresh_annotations(cx);
                })
            });
            let (frame, scale) = cx.update(|window, cx| {
                (
                    view.read(cx).doc().unwrap().marks[0].frame(),
                    view.read(cx).transform(window).1,
                )
            });
            let corner = frame.bounds.corners()[1];
            let start = frame.to_world(Pos::new(corner.x + 12. / scale, corner.y - 12. / scale));
            let end = start.rotate(frame.center(), std::f32::consts::FRAC_PI_2);
            let at = pointer(&view, cx, start);
            cx.simulate_mouse_move(at, None, Modifiers::default());
            cx.update(|_, cx| {
                assert_eq!(
                    view.read(cx).selection_cursor(),
                    SelectionCursor::Rotate(0),
                    "{tool:?}"
                )
            });
            drag(&view, cx, start, end);
            cx.update(|window, cx| {
                let mark = &view.read(cx).doc().unwrap().marks[0];
                assert!(
                    (mark.rotation - std::f32::consts::FRAC_PI_2).abs() < 0.001,
                    "{tool:?}"
                );
                assert_eq!(mark.bounds(), frame.bounds);
                window.render_frame(cx);
                window.find("annotation-selection-box");
                window.find(("annotation-handle", 3usize));
                assert!(window.try_find(("annotation-handle", 4usize)).is_none());
            });
            cx.simulate_keystrokes("cmd-z");
            cx.update(|_, cx| assert_eq!(view.read(cx).doc().unwrap().marks[0].rotation, 0.));
            cx.simulate_keystrokes("cmd-shift-z");
            cx.update(|_, cx| {
                assert!(
                    (view.read(cx).doc().unwrap().marks[0].rotation - std::f32::consts::FRAC_PI_2)
                        .abs()
                        < 0.001
                )
            });
        }
    }
    #[gpui_kit::test]
    fn dragging_a_handle_through_floating_controls_tracks_every_pointer_event(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = editor_window(cx);
        cx.simulate_keystrokes("a");
        drag(&view, cx, Pos::new(100., 100.), Pos::new(230., 200.));
        select_at(&view, cx, Pos::new(100., 100.));
        let tip = pointer(&view, cx, Pos::new(230., 200.));
        cx.simulate_mouse_down(tip, MouseButton::Left, Modifiers::default());
        for control in [
            ("annotation-color", 0usize).into(),
            gpui_kit::ElementId::from("annotation-opacity"),
            ("annotation-tool", 7usize).into(),
            gpui_kit::ElementId::from("preview-zoom-in"),
        ] {
            let position = cx.update(|window, _| window.find(control.clone()).bounds().center());
            let expected = cx.update(|window, cx| view.read(cx).image_pos(position, window));
            cx.simulate_mouse_move(position, Some(MouseButton::Left), Modifiers::default());
            cx.update(|_, cx| {
                let preview = view.read(cx);
                let actual = preview.doc().unwrap().marks[0].points[1];
                assert!(
                    actual.distance(expected) < 0.1,
                    "Handle freezes over {control:?}: expected {expected:?}, got {actual:?}"
                );
                assert_eq!(preview.editor.tool, Tool::Select);
                assert_eq!(preview.editor.selected, Some(0));
            });
        }
        let release = cx.update(|window, _| window.find("preview-zoom-in").bounds().center());
        cx.simulate_mouse_up(release, MouseButton::Left, Modifiers::default());
        let after = cx.update(|_, cx| {
            let preview = view.read(cx);
            assert!(preview.editor.gesture.is_none());
            assert_eq!(
                preview.zoom, 1.,
                "Releasing over a control must not activate it"
            );
            preview.doc().unwrap().marks[0].clone()
        });
        cx.simulate_mouse_move(tip, None, Modifiers::default());
        cx.update(|_, cx| assert_eq!(view.read(cx).doc().unwrap().marks[0], after));
    }
    #[gpui_kit::test]
    fn arrows_only_show_the_path_highlight_and_three_handles(cx: &mut TestAppContext) {
        let (view, cx) = editor_window(cx);
        cx.simulate_keystrokes("a");
        cx.update(|_, cx| assert_eq!(view.read(cx).editor.tool, Tool::Arrow));
        drag(&view, cx, Pos::new(100., 100.), Pos::new(230., 200.));
        select_at(&view, cx, Pos::new(100., 100.));
        cx.update(|window, cx| {
            assert_eq!(view.read(cx).editor.selected, Some(0));
            assert_eq!(view.read(cx).doc().unwrap().marks[0].tool, Tool::Arrow);
            window.render_frame(cx);
            assert!(window.try_find("annotation-selection-box").is_none());
            window.find("annotation-arrow-selection");
            for i in 0usize..3 {
                assert!(window.try_find(("annotation-handle", i)).is_some());
            }
            assert!(window.try_find(("annotation-handle", 3usize)).is_none());
        });
    }
    #[gpui_kit::test]
    fn a_continuous_pen_stroke_moves_and_resizes_as_one_object(cx: &mut TestAppContext) {
        let (view, cx) = editor_window(cx);
        cx.simulate_keystrokes("p");
        let start = pointer(&view, cx, Pos::new(80., 120.));
        cx.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
        for p in [
            Pos::new(120., 100.),
            Pos::new(150., 130.),
            Pos::new(180., 150.),
        ] {
            let position = pointer(&view, cx, p);
            cx.update(|window, cx| {
                use gpui_kit::InputEvent as _;
                window.dispatch_event(
                    MouseMoveEvent {
                        position,
                        pressed_button: Some(MouseButton::Left),
                        modifiers: Modifiers::default(),
                    }
                    .to_platform_input(),
                    cx,
                );
            });
        }
        let end = pointer(&view, cx, Pos::new(180., 150.));
        cx.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
        cx.update(|_, cx| {
            let doc = view.read(cx).doc().unwrap();
            assert_eq!(doc.marks.len(), 1);
            assert_eq!(doc.marks[0].tool, Tool::Pen);
            assert!(doc.marks[0].points.len() >= 4);
        });
        drag(&view, cx, Pos::new(150., 130.), Pos::new(170., 150.));
        let before = cx.update(|_, cx| view.read(cx).doc().unwrap().marks[0].clone());
        assert!((before.bounds().x - 100.).abs() < 0.01);
        assert!((before.bounds().y - 120.).abs() < 0.01);
        drag(&view, cx, Pos::new(200., 170.), Pos::new(280., 220.));
        cx.update(|_, cx| {
            let doc = view.read(cx).doc().unwrap();
            assert_eq!(doc.marks.len(), 1);
            assert_eq!(doc.marks[0].points.len(), before.points.len());
            assert!((doc.marks[0].bounds().w - 180.).abs() < 0.1);
            assert!((doc.marks[0].bounds().h - 100.).abs() < 0.1);
        });
        cx.simulate_keystrokes("cmd-z");
        cx.update(|_, cx| assert_eq!(view.read(cx).doc().unwrap().marks[0], before));
    }
    #[gpui_kit::test]
    fn right_click_selects_the_object_and_stack_menu_reorders_it(cx: &mut TestAppContext) {
        let (view, cx) = editor_window(cx);
        cx.update(|_, cx| {
            view.update(cx, |preview, cx| {
                for (color, left) in [(11, 80.), (4, 160.)] {
                    let mut mark = Mark::new(
                        Tool::Rectangle,
                        Pos::new(left, 100.),
                        Style {
                            color,
                            fill: crate::annotation::Fill::Solid,
                            ..Default::default()
                        },
                    );
                    mark.points[1] = Pos::new(left + 140., 240.);
                    preview.doc_mut().unwrap().marks.push(mark);
                }
                preview.refresh_annotations(cx);
            })
        });
        let p = pointer(&view, cx, Pos::new(110., 170.));
        cx.simulate_mouse_down(p, MouseButton::Right, Modifiers::default());
        cx.simulate_mouse_up(p, MouseButton::Right, Modifiers::default());
        cx.run_until_parked();
        cx.update(|window, cx| {
            window.render_frame(cx);
            assert_eq!(view.read(cx).editor.selected, Some(0));
            assert!(window.try_find("popup-menu").is_some());
            window.within("popup-menu").click(0usize, cx);
        });
        cx.run_until_parked();
        cx.update(|_, cx| {
            let preview = view.read(cx);
            assert_eq!(preview.editor.selected, Some(1));
            assert_eq!(preview.doc().unwrap().marks.last().unwrap().style.color, 11);
        });
        cx.simulate_keystrokes("cmd-z");
        cx.update(|_, cx| assert_eq!(view.read(cx).doc().unwrap().marks[0].style.color, 11));
    }
}

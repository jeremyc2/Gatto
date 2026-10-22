//! Native plain-text input over the artwork's own line boxes and rotated frame.
//! GPUI supplies platform text/IME events; the annotation renderer remains the
//! only painter of glyphs, avoiding a second layout or an opaque editing field.
use super::*;
use gpui_kit::{ClipboardItem, ElementInputHandler, EntityInputHandler, UTF16Selection};
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;

gpui_kit::actions!(
    annotation_text,
    [
        MoveLeft,
        MoveRight,
        CutText,
        CopyText,
        PasteText,
        SelectAllText,
        UndoText,
        RedoText
    ]
);
pub(super) fn init(cx: &mut gpui_kit::App) {
    cx.bind_keys([
        KeyBinding::new("left", MoveLeft, Some("AnnotationText")),
        KeyBinding::new("right", MoveRight, Some("AnnotationText")),
        KeyBinding::new("cmd-c", CopyText, Some("AnnotationText")),
        KeyBinding::new("cmd-x", CutText, Some("AnnotationText")),
        KeyBinding::new("cmd-v", PasteText, Some("AnnotationText")),
        KeyBinding::new("cmd-a", SelectAllText, Some("AnnotationText")),
        KeyBinding::new("cmd-z", UndoText, Some("AnnotationText")),
        KeyBinding::new("cmd-shift-z", RedoText, Some("AnnotationText")),
    ]);
}

#[derive(Clone)]
struct Snapshot {
    value: SharedString,
    anchor: usize,
    head: usize,
}

pub(super) struct TextInput {
    value: SharedString,
    focus: FocusHandle,
    anchor: usize,
    head: usize,
    marked: Option<Range<usize>>,
    selecting: bool,
    mark: Mark,
    origin: Pos,
    scale: f32,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    preferred_x: Option<f32>,
}
impl EventEmitter<InputEvent> for TextInput {}
impl Focusable for TextInput {
    fn focus_handle(&self, _: &gpui_kit::App) -> FocusHandle {
        self.focus.clone()
    }
}

impl TextInput {
    fn command(&mut self, key: &str, shift: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.key(
            &KeyDownEvent {
                keystroke: gpui_kit::Keystroke {
                    key: key.into(),
                    modifiers: gpui_kit::Modifiers {
                        platform: true,
                        shift,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                is_held: false,
                prefer_character_input: false,
            },
            window,
            cx,
        );
    }
    pub(super) fn menu(
        &self,
        menu: gpui_kit::component::menu::PopupMenu,
    ) -> gpui_kit::component::menu::PopupMenu {
        menu.menu_with_disabled("Undo", Box::new(UndoText), self.undo.is_empty())
            .menu_with_disabled("Redo", Box::new(RedoText), self.redo.is_empty())
            .separator()
            .menu_with_disabled("Cut", Box::new(CutText), self.range().is_empty())
            .menu_with_disabled("Copy", Box::new(CopyText), self.range().is_empty())
            .menu("Paste", Box::new(PasteText))
            .separator()
            .menu_with_disabled("Select all", Box::new(SelectAllText), self.value.is_empty())
    }
    fn bound_key(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.key(
            &KeyDownEvent {
                keystroke: gpui_kit::Keystroke {
                    key: key.into(),
                    modifiers: window.modifiers(),
                    ..Default::default()
                },
                is_held: false,
                prefer_character_input: false,
            },
            window,
            cx,
        );
    }
    pub fn new(mark: Mark, cx: &mut Context<Self>) -> Self {
        let value: SharedString = mark.text.clone().into();
        let end = value.len();
        Self {
            value,
            focus: cx.focus_handle(),
            anchor: end,
            head: end,
            marked: None,
            selecting: false,
            mark,
            origin: Pos::default(),
            scale: 1.,
            undo: Vec::new(),
            redo: Vec::new(),
            preferred_x: None,
        }
    }
    pub fn value(&self) -> &SharedString {
        &self.value
    }
    pub fn configure(&mut self, mut mark: Mark, origin: Pos, scale: f32) {
        mark.text = self.value.to_string();
        self.mark = mark;
        self.origin = origin;
        self.scale = scale;
    }
    pub fn focus_and_select_all(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.anchor = 0;
        self.head = self.value.len();
        window.focus(&self.focus, cx);
        cx.notify();
    }
    fn range(&self) -> Range<usize> {
        self.anchor.min(self.head)..self.anchor.max(self.head)
    }
    fn snapshot(&self) -> Snapshot {
        Snapshot {
            value: self.value.clone(),
            anchor: self.anchor,
            head: self.head,
        }
    }
    fn restore(&mut self, snapshot: Snapshot, cx: &mut Context<Self>) {
        self.value = snapshot.value;
        self.anchor = snapshot.anchor;
        self.head = snapshot.head;
        self.mark.text = self.value.to_string();
        self.marked = None;
        cx.emit(InputEvent::Change);
        cx.notify();
    }
    fn replace(
        &mut self,
        range: Range<usize>,
        value: &str,
        checkpoint: bool,
        cx: &mut Context<Self>,
    ) {
        let normalized = value.replace("\r\n", "\n").replace('\r', "\n");
        let value = normalized.as_str();
        if checkpoint {
            self.undo.push(self.snapshot());
            self.redo.clear();
        }
        let mut text = self.value.to_string();
        text.replace_range(range.clone(), value);
        self.value = text.into();
        self.anchor = range.start + value.len();
        self.head = self.anchor;
        self.mark.text = self.value.to_string();
        self.preferred_x = None;
        cx.emit(InputEvent::Change);
        cx.notify();
    }
    pub fn set_selected_range(&mut self, range: Range<usize>, cx: &mut Context<Self>) {
        self.anchor = self.boundary(range.start);
        self.head = self.boundary(range.end);
        cx.notify();
    }
    fn boundary(&self, index: usize) -> usize {
        let mut index = index.min(self.value.len());
        while !self.value.is_char_boundary(index) {
            index -= 1;
        }
        index
    }
    fn previous(&self, index: usize) -> usize {
        self.value
            .grapheme_indices(true)
            .rev()
            .find(|(i, _)| *i < index)
            .map_or(0, |(i, _)| i)
    }
    fn next(&self, index: usize) -> usize {
        self.value
            .grapheme_indices(true)
            .find(|(i, _)| *i > index)
            .map_or(self.value.len(), |(i, _)| i)
    }
    fn local(&self, p: Point<Pixels>) -> Pos {
        let b = self.mark.label_edit_bounds().unwrap();
        let p = self.mark.frame().to_local(Pos::new(
            (p.x.as_f32() - self.origin.x) / self.scale,
            (p.y.as_f32() - self.origin.y) / self.scale,
        ));
        Pos::new(p.x - b.x, p.y - b.y)
    }
    fn screen(&self, p: Pos) -> Point<Pixels> {
        let b = self.mark.label_edit_bounds().unwrap();
        let p = self.mark.frame().to_world(Pos::new(b.x + p.x, b.y + p.y));
        point(
            px(self.origin.x + p.x * self.scale),
            px(self.origin.y + p.y * self.scale),
        )
    }
    fn index_at(&self, p: Point<Pixels>) -> usize {
        let layout = self.mark.text_layout();
        self.boundary(layout.index_at(
            self.local(p),
            self.mark.label_edit_bounds().unwrap().w,
            self.mark.text_align(),
        ))
    }
    pub(super) fn contains(&self, p: Point<Pixels>) -> bool {
        let b = self.mark.label_edit_bounds().unwrap();
        Rect {
            x: 0.,
            y: 0.,
            w: b.w,
            h: b.h,
        }
        .contains(self.local(p), 3. / self.scale)
    }
    pub(super) fn is_selecting(&self) -> bool {
        self.selecting
    }
    pub(super) fn end_drag(&mut self) {
        self.selecting = false;
    }
    pub(super) fn drag_selection(&mut self, event: &MouseMoveEvent, cx: &mut Context<Self>) {
        if self.selecting && event.dragging() {
            self.head = self.index_at(event.position);
            cx.stop_propagation();
            cx.notify();
        }
    }
    fn copy(&self, cx: &mut Context<Self>) {
        if !self.range().is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.value[self.range()].to_owned(),
            ));
        }
    }
    fn key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let key = event.keystroke.key.as_str();
        let modifiers = event.keystroke.modifiers;
        let command = modifiers.platform;
        let range = self.range();
        let layout = self.mark.text_layout();
        let b = self.mark.label_edit_bounds().unwrap();
        let caret = layout.caret(self.head, b.w, self.mark.text_align());
        let mut target = None;
        match key {
            "a" if command => {
                self.anchor = 0;
                self.head = self.value.len();
            }
            "c" if command => self.copy(cx),
            "x" if command => {
                self.copy(cx);
                if !range.is_empty() {
                    self.replace(range, "", true, cx);
                }
            }
            "v" if command => {
                if let Some(value) = cx.read_from_clipboard().and_then(|item| item.text()) {
                    self.replace(
                        range,
                        &value.replace("\r\n", "\n").replace('\r', "\n"),
                        true,
                        cx,
                    );
                }
            }
            "z" if command => {
                let snapshot = if modifiers.shift {
                    self.redo.pop()
                } else {
                    self.undo.pop()
                };
                if let Some(snapshot) = snapshot {
                    let current = self.snapshot();
                    if modifiers.shift {
                        self.undo.push(current);
                    } else {
                        self.redo.push(current);
                    }
                    self.restore(snapshot, cx);
                }
            }
            "backspace" | "delete" => {
                let deleting = if !range.is_empty() {
                    range
                } else if key == "backspace" {
                    let start = if command {
                        layout.index_at(Pos::new(0., caret.y), b.w, self.mark.text_align())
                    } else if modifiers.alt {
                        self.value
                            .unicode_word_indices()
                            .rev()
                            .find(|(i, _)| *i < self.head)
                            .map_or(0, |(i, _)| i)
                    } else {
                        self.previous(self.head)
                    };
                    start..self.head
                } else {
                    self.head..self.next(self.head)
                };
                if !deleting.is_empty() {
                    self.replace(deleting, "", true, cx);
                }
            }
            "enter" if !command => {
                self.replace(range, "\n", true, cx);
            }
            "left" | "right" => {
                let left = key == "left";
                target = Some(if command {
                    layout.index_at(
                        Pos::new(if left { -1e6 } else { 1e6 }, caret.y),
                        b.w,
                        self.mark.text_align(),
                    )
                } else if modifiers.alt {
                    if left {
                        self.value
                            .unicode_word_indices()
                            .rev()
                            .find(|(i, _)| *i < self.head)
                            .map_or(0, |(i, _)| i)
                    } else {
                        self.value
                            .unicode_word_indices()
                            .find(|(i, word)| i + word.len() > self.head)
                            .map_or(self.value.len(), |(i, word)| i + word.len())
                    }
                } else if !modifiers.shift && !range.is_empty() {
                    if left { range.start } else { range.end }
                } else if left {
                    self.previous(self.head)
                } else {
                    self.next(self.head)
                });
                self.preferred_x = None;
            }
            "up" | "down" => {
                let up = key == "up";
                target = Some(if command {
                    if up { 0 } else { self.value.len() }
                } else {
                    let x = *self.preferred_x.get_or_insert(caret.x);
                    let y = caret.y
                        + if up {
                            -layout.line_height
                        } else {
                            layout.line_height
                        };
                    if y < 0. {
                        0
                    } else if y >= layout.height {
                        self.value.len()
                    } else {
                        layout.index_at(Pos::new(x, y), b.w, self.mark.text_align())
                    }
                });
            }
            "home" => target = Some(0),
            "end" => target = Some(self.value.len()),
            _ => {
                return;
            }
        }
        if let Some(target) = target {
            self.head = self.boundary(target);
            if !modifiers.shift {
                self.anchor = self.head;
            }
        }
        window.focus(&self.focus, cx);
        cx.stop_propagation();
        cx.notify();
    }
    fn mouse_down(&mut self, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if !self.contains(event.position) {
            return;
        }
        window.focus(&self.focus, cx);
        let i = self.index_at(event.position);
        if event.click_count >= 3 {
            self.anchor = self.value[..i].rfind('\n').map_or(0, |j| j + 1);
            self.head = self.value[i..]
                .find('\n')
                .map_or(self.value.len(), |j| i + j);
        } else if event.click_count == 2 {
            self.anchor = 0;
            self.head = self.value.len();
        } else {
            self.head = i;
            if !event.modifiers.shift {
                self.anchor = i;
            }
        }
        // A double-click selects the full value; pointer jitter must not shorten it.
        self.selecting = event.click_count != 2;
        self.preferred_x = None;
        cx.stop_propagation();
        cx.notify();
    }
    fn paint_rect(&self, rect: Rect, color: gpui_kit::Hsla, window: &mut Window) {
        let mut path = gpui_kit::PathBuilder::fill();
        let corners = rect.corners();
        path.move_to(self.screen(corners[0]));
        for p in [corners[1], corners[2], corners[3], corners[0]] {
            path.line_to(self.screen(p));
        }
        if let Ok(path) = path.build() {
            window.paint_path(path, color);
        }
    }
    fn utf16(&self, offset: usize) -> usize {
        self.value[..self.boundary(offset)].encode_utf16().count()
    }
    fn utf8(&self, offset: usize) -> usize {
        let mut count = 0;
        for (i, c) in self.value.char_indices() {
            if count >= offset {
                return i;
            }
            count += c.len_utf16();
        }
        self.value.len()
    }
    fn byte_range(&self, range: Range<usize>) -> Range<usize> {
        self.utf8(range.start)..self.utf8(range.end)
    }
    fn utf16_range(&self, range: Range<usize>) -> Range<usize> {
        self.utf16(range.start)..self.utf16(range.end)
    }
}

impl Render for TextInput {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let input = cx.entity();
        let input_for_paint = input.clone();
        div()
            .absolute()
            .inset_0()
            .track_focus(&self.focus)
            .key_context("AnnotationText")
            .when(self.contains(window.mouse_position()), |d| d.cursor_text())
            .on_key_down(cx.listener(Self::key))
            .on_action(
                cx.listener(|this, _: &CutText, window, cx| this.command("x", false, window, cx)),
            )
            .on_action(
                cx.listener(|this, _: &CopyText, window, cx| this.command("c", false, window, cx)),
            )
            .on_action(
                cx.listener(|this, _: &PasteText, window, cx| this.command("v", false, window, cx)),
            )
            .on_action(cx.listener(|this, _: &SelectAllText, window, cx| {
                this.command("a", false, window, cx)
            }))
            .on_action(
                cx.listener(|this, _: &UndoText, window, cx| this.command("z", false, window, cx)),
            )
            .on_action(
                cx.listener(|this, _: &RedoText, window, cx| this.command("z", true, window, cx)),
            )
            .on_action(
                cx.listener(|this, _: &MoveLeft, window, cx| this.bound_key("left", window, cx)),
            )
            .on_action(
                cx.listener(|this, _: &MoveRight, window, cx| this.bound_key("right", window, cx)),
            )
            .on_mouse_down(MouseButton::Left, cx.listener(Self::mouse_down))
            .on_mouse_move(
                cx.listener(|this, event: &MouseMoveEvent, _, cx| this.drag_selection(event, cx)),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, _| this.selecting = false),
            )
            .child(
                canvas(
                    |_, _, _| (),
                    move |bounds, _, window, cx| {
                        let this = input_for_paint.read(cx);
                        window.handle_input(
                            &this.focus,
                            ElementInputHandler::new(bounds, input.clone()),
                            cx,
                        );
                        if !this.focus.is_focused(window) {
                            return;
                        }
                        let layout = this.mark.text_layout();
                        let width = this.mark.label_edit_bounds().unwrap().w;
                        let align = this.mark.text_align();
                        for rect in layout
                            .selection(this.range(), width, align)
                            .into_iter()
                            .filter(|_| !this.range().is_empty())
                        {
                            this.paint_rect(rect, gpui_kit::rgba(0x438fff66).into(), window);
                        }
                        if let Some(marked) = &this.marked {
                            for rect in layout.selection(marked.clone(), width, align) {
                                this.paint_rect(
                                    Rect {
                                        y: rect.y + rect.h - 1. / this.scale,
                                        h: 1. / this.scale,
                                        ..rect
                                    },
                                    rgb(0x6caaff).into(),
                                    window,
                                );
                            }
                        }
                        let caret = layout.caret(this.head, width, align);
                        this.paint_rect(
                            Rect {
                                x: caret.x,
                                y: caret.y + layout.line_height * 0.05,
                                w: 1.5 / this.scale,
                                h: layout.line_height * 0.9,
                            },
                            rgb(0x6caaff).into(),
                            window,
                        );
                    },
                )
                .size_full(),
            )
    }
}

impl EntityInputHandler for TextInput {
    fn text_for_range(
        &mut self,
        range: Range<usize>,
        actual: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.byte_range(range);
        *actual = Some(self.utf16_range(range.clone()));
        Some(self.value[range].to_owned())
    }
    fn selected_text_range(
        &mut self,
        _: bool,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.utf16_range(self.range()),
            reversed: self.head < self.anchor,
        })
    }
    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        self.marked.clone().map(|range| self.utf16_range(range))
    }
    fn unmark_text(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.marked = None;
        cx.notify();
    }
    fn replace_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range
            .map(|range| self.byte_range(range))
            .or(self.marked.clone())
            .unwrap_or_else(|| self.range());
        self.replace(range, text, self.marked.is_none(), cx);
        self.marked = None;
    }
    fn replace_and_mark_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        selected: Option<Range<usize>>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range
            .map(|range| self.byte_range(range))
            .or(self.marked.clone())
            .unwrap_or_else(|| self.range());
        let start = range.start;
        self.replace(range, text, self.marked.is_none(), cx);
        self.marked = (!text.is_empty()).then_some(start..start + text.len());
        if let Some(selected) = selected {
            let base = self.utf16(start);
            self.anchor = self.utf8(base + selected.start);
            self.head = self.utf8(base + selected.end);
        }
    }
    fn bounds_for_range(
        &mut self,
        range: Range<usize>,
        _: Bounds<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let range = self.byte_range(range);
        let layout = self.mark.text_layout();
        let b = self.mark.label_edit_bounds()?;
        let a = layout.caret(range.start, b.w, self.mark.text_align());
        let z = layout.caret(range.end, b.w, self.mark.text_align());
        let rect = Rect {
            x: a.x,
            y: a.y,
            w: if a.y == z.y {
                (z.x - a.x).abs().max(1. / self.scale)
            } else {
                b.w
            },
            h: layout.line_height,
        };
        let screen = Rect::enclosing(rect.corners().map(|p| {
            let p = self.screen(p);
            Pos::new(p.x.as_f32(), p.y.as_f32())
        }));
        Some(Bounds::new(
            point(px(screen.x), px(screen.y)),
            size(px(screen.w), px(screen.h)),
        ))
    }
    fn character_index_for_point(
        &mut self,
        p: Point<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<usize> {
        Some(self.utf16(self.index_at(p)))
    }
    fn set_selected_text_range(
        &mut self,
        range: Range<usize>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_selected_range(self.byte_range(range), cx);
    }
    fn text_length_utf16(&mut self, _: &mut Window, _: &mut Context<Self>) -> Option<usize> {
        Some(self.value.encode_utf16().count())
    }
}

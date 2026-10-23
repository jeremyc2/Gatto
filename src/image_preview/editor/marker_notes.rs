//! A slide-notes style editor. Text is document metadata, never canvas text.
use super::*;
use gpui_kit::component::input::{Textarea, TextareaState};

pub(super) const HEIGHT: f32 = 108.;
pub(super) struct NotesEdit {
    index: usize,
    number: u32,
    input: Entity<TextareaState>,
    checkpointed: bool,
    _subscription: Subscription,
}

impl ImagePreview {
    pub(super) fn selected_marker(&self) -> Option<(usize, &Mark)> {
        let indices = self.selected_indices();
        if indices.len() != 1 {
            return None;
        }
        let index = indices[0];
        let mark = self.doc()?.marks.get(index)?;
        (mark.tool == Tool::Marker).then_some((index, mark))
    }

    pub(in crate::image_preview) fn marker_notes_height(&self) -> f32 {
        if self.selected_marker().is_some() {
            HEIGHT
        } else {
            0.
        }
    }

    pub(in crate::image_preview) fn sync_marker_notes(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((index, mark)) = self.selected_marker() else {
            self.editor.notes = None;
            return;
        };
        let number = mark.number.unwrap_or(1);
        if self
            .editor
            .notes
            .as_ref()
            .is_some_and(|notes| notes.index == index && notes.number == number)
        {
            return;
        }
        let value = mark.description.clone();
        let input = cx.new(|cx| {
            TextareaState::new(window, cx)
                .placeholder("Add a description…")
                .default_value(value)
        });
        let subscription = cx.subscribe(&input, |this, input, event, cx| {
            if matches!(event, InputEvent::Blur) {
                if let Some(notes) = &mut this.editor.notes {
                    notes.checkpointed = false;
                }
                return;
            }
            if !matches!(event, InputEvent::Change) {
                return;
            }
            let value = input.read(cx).value().to_string();
            let Some(notes) = &this.editor.notes else {
                return;
            };
            let index = notes.index;
            let number = notes.number;
            let checkpointed = notes.checkpointed;
            let Some(mark) = this.doc().and_then(|doc| doc.marks.get(index)) else {
                return;
            };
            if mark.tool != Tool::Marker || mark.number != Some(number) || mark.description == value
            {
                return;
            }
            if !checkpointed {
                this.doc_mut().unwrap().checkpoint();
                this.editor.notes.as_mut().unwrap().checkpointed = true;
            }
            this.doc_mut().unwrap().marks[index].description = value;
            cx.notify();
        });
        self.editor.notes = Some(NotesEdit {
            index,
            number,
            input,
            checkpointed: false,
            _subscription: subscription,
        });
    }

    pub(super) fn marker_notes_focused(&self, window: &Window, cx: &App) -> bool {
        self.editor
            .notes
            .as_ref()
            .is_some_and(|notes| notes.input.read(cx).focus_handle(cx).is_focused(window))
    }

    pub(super) fn focus_marker_notes(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.sync_marker_notes(window, cx);
        if let Some(notes) = &self.editor.notes {
            window.focus(&notes.input.read(cx).focus_handle(cx), cx);
        }
    }

    pub(in crate::image_preview) fn marker_notes(
        &self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .absolute()
            .bottom_0()
            .left_0()
            .right_0()
            .children(self.editor.notes.as_ref().map(|notes| {
                div()
                    .id("marker-description-pane")
                    .h(px(HEIGHT))
                    .w_full()
                    .bg(cx.theme().background)
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .p_3()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .cursor(CursorStyle::Arrow)
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
                    .on_mouse_move(cx.listener(Self::editor_controls_hover))
                    .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
                    .child(
                        Textarea::new(&notes.input)
                            .w_full()
                            .h(px(80.))
                            .aria_label(format!("Description for marker {}", notes.number))
                            .accessibility_id("marker-description-input"),
                    )
            }))
    }
}

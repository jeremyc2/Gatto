//! Object clipboard contents preserve editable geometry independently of text editing.
use super::*;
use gpui_kit::ClipboardItem;

#[derive(serde::Serialize, serde::Deserialize)]
struct AnnotationClipboard {
    kind: String,
    marks: Vec<Mark>,
}
const CLIPBOARD_KIND: &str = "gatto.annotations.v1";

fn bounds(marks: &[Mark]) -> Option<Rect> {
    marks.iter().map(Mark::visual_bounds).reduce(Rect::union)
}

fn valid(mark: &Mark) -> bool {
    let style = &mark.style;
    (mark.tool.is_shape() || matches!(mark.tool, Tool::Text | Tool::Pen | Tool::Arrow | Tool::Line))
        && mark.points.len() >= 2
        && mark
            .points
            .iter()
            .all(|p| p.x.is_finite() && p.y.is_finite())
        && style.color < crate::annotation::COLORS.len()
        && [style.width, style.font_size, style.scale]
            .into_iter()
            .all(|v| v.is_finite() && v > 0.)
        && style.opacity.is_finite()
        && (0. ..=1.).contains(&style.opacity)
        && mark.rotation.is_finite()
        && mark.label_position.is_finite()
        && mark.wrap.is_none_or(|v| v.is_finite() && v > 0.)
        && mark
            .label_base_height
            .is_none_or(|v| v.is_finite() && v >= 0.)
}

impl ImagePreview {
    fn selected_marks(&self) -> Vec<Mark> {
        self.doc().map_or_else(Vec::new, |doc| {
            self.selected_indices()
                .into_iter()
                .map(|i| doc.marks[i].clone())
                .collect()
        })
    }
    fn insert_copies(&mut self, mut marks: Vec<Mark>, delta: Pos, cx: &mut Context<Self>) {
        if self.saving.is_some() || marks.is_empty() {
            return;
        }
        self.finish_text(cx);
        let bottom = self.comparison_annotation_bottom();
        let Some(doc) = self.doc_mut() else {
            return;
        };
        doc.checkpoint();
        let start = doc.marks.len();
        for mark in &mut marks {
            mark.translate(delta.x, delta.y);
        }
        if let Some(bottom) = bottom
            && let Some(bounds) = bounds(&marks)
        {
            let overflow = (bounds.y + bounds.h - bottom).max(0.);
            if overflow > 0. {
                for mark in &mut marks {
                    mark.translate(0., -overflow);
                }
            }
        }
        doc.marks.extend(marks);
        let indices = (start..doc.marks.len()).collect();
        let style = doc.marks.last().unwrap().style.clone().with_scale(1.);
        self.editor.gesture = None;
        self.set_selection(indices);
        self.editor.tool = Tool::Select;
        self.editor.style = style;
        self.refresh_annotations(cx);
    }
    pub(super) fn clone_selected(&mut self, cx: &mut Context<Self>) {
        let marks = self.selected_marks();
        let Some(bounds) = bounds(&marks) else {
            return;
        };
        self.insert_copies(marks, Pos::new(bounds.w, 0.), cx);
    }
    pub(super) fn copy_selected(&mut self, cx: &mut Context<Self>) {
        let marks = self.selected_marks();
        if marks.is_empty() {
            return;
        }
        let text = marks
            .iter()
            .filter(|m| !m.text.is_empty())
            .map(|m| m.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        cx.write_to_clipboard(ClipboardItem::new_string_with_json_metadata(
            if text.is_empty() {
                "Gatto annotations".into()
            } else {
                text
            },
            AnnotationClipboard {
                kind: CLIPBOARD_KIND.into(),
                marks,
            },
        ));
    }
    pub(super) fn paste_selected(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.editable() || self.saving.is_some() {
            return;
        }
        let Some(item) = cx.read_from_clipboard() else {
            return;
        };
        let Some(metadata) = item.metadata().filter(|v| v.len() <= 8 * 1024 * 1024) else {
            return;
        };
        let Ok(payload) = serde_json::from_str::<AnnotationClipboard>(metadata) else {
            return;
        };
        if payload.kind != CLIPBOARD_KIND || !payload.marks.iter().all(valid) {
            return;
        }
        let Some(bounds) = bounds(&payload.marks) else {
            return;
        };
        let position = if self.editor.canvas_hover {
            window.mouse_position()
        } else {
            Self::viewport_center(window)
        };
        let cursor = self.image_pos(position, window);
        let center = bounds.center();
        self.insert_copies(
            payload.marks,
            Pos::new(cursor.x - center.x, cursor.y - center.y),
            cx,
        );
    }
}

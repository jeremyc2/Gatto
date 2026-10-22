use super::*;
use crate::annotation::Frame;

#[derive(Clone)]
pub(super) struct GroupGesture {
    pub start: Pos,
    pub frame: Frame,
    pub originals: std::rc::Rc<Vec<(usize, Mark)>>,
    pub mode: Mode,
    pub checkpointed: bool,
}
pub(super) struct Marquee {
    pub start: Pos,
    pub current: Pos,
    pub previous: Vec<usize>,
}

impl ImagePreview {
    pub(super) fn selected_indices(&self) -> Vec<usize> {
        let Some(primary) = self.editor.selected else {
            return Vec::new();
        };
        if self.editor.group.contains(&primary) {
            self.editor.group.clone()
        } else {
            vec![primary]
        }
    }
    pub(super) fn clear_selection(&mut self) {
        self.editor.selected = None;
        self.editor.group.clear();
        self.editor.group_frame = None;
        self.editor.group_gesture = None;
        self.editor.marquee = None;
    }
    pub(super) fn set_selection(&mut self, mut indices: Vec<usize>) {
        indices.sort_unstable();
        indices.dedup();
        indices.retain(|i| {
            self.doc()
                .and_then(|d| d.marks.get(*i))
                .is_some_and(|m| m.tool != Tool::Redact)
        });
        self.editor.selected = indices.last().copied();
        self.editor.group = if indices.len() > 1 {
            indices
        } else {
            Vec::new()
        };
        self.editor.group_frame = None;
    }
    pub(super) fn selection_frame(&self) -> Option<Frame> {
        let indices = self.selected_indices();
        if indices.len() == 1 {
            return self.doc()?.marks.get(indices[0]).map(Mark::frame);
        }
        if indices.is_empty() {
            return None;
        }
        if let Some(frame) = self.editor.group_frame {
            return Some(frame);
        }
        let mut bounds = self.doc()?.marks[indices[0]].world_bounds();
        for i in indices.into_iter().skip(1) {
            bounds = bounds.union(self.doc()?.marks[i].world_bounds());
        }
        Some(Frame {
            bounds,
            rotation: 0.,
        })
    }
    pub(super) fn begin_selection(
        &mut self,
        event: &MouseDownEvent,
        p: Pos,
        scale: f32,
        cx: &mut Context<Self>,
    ) {
        self.editor.gesture = None;
        self.editor.group_gesture = None;
        self.editor.marquee = None;
        let target = self.selection_target(p, scale);
        let handle = target.as_ref().is_some_and(|(_, mode)| {
            matches!(
                mode,
                Mode::Resize(_) | Mode::Rotate(_) | Mode::Endpoint(_) | Mode::Bend
            )
        });
        if event.modifiers.shift
            && !handle
            && let Some(index) = self.doc().and_then(|d| d.hit(p, 7. / scale))
        {
            let mut indices = self.selected_indices();
            if indices.contains(&index) {
                indices.retain(|i| *i != index);
            } else {
                indices.push(index);
            }
            self.set_selection(indices);
            if let Some(i) = self.editor.selected {
                self.editor.style = self.doc().unwrap().marks[i].style.clone().with_scale(1.);
            }
            cx.notify();
            return;
        }
        let Some((index, mode)) = target else {
            let previous = if event.modifiers.shift {
                self.selected_indices()
            } else {
                Vec::new()
            };
            self.clear_selection();
            self.editor.marquee = Some(Marquee {
                start: p,
                current: p,
                previous,
            });
            cx.notify();
            return;
        };
        let indices = self.selected_indices();
        if indices.len() > 1 && indices.contains(&index) {
            let frame = self.selection_frame().unwrap();
            let originals = std::rc::Rc::new(
                indices
                    .iter()
                    .map(|i| (*i, self.doc().unwrap().marks[*i].clone()))
                    .collect(),
            );
            self.editor.group_gesture = Some(GroupGesture {
                start: p,
                frame,
                originals,
                mode,
                checkpointed: false,
            });
        } else {
            self.set_selection(vec![index]);
            let doc = self.doc_mut().unwrap();
            let original = doc.marks[index].clone();
            self.editor.style = original.style.clone().with_scale(1.);
            self.editor.gesture = Some(Gesture {
                start: p,
                index,
                original,
                mode,
                checkpointed: false,
                started: Instant::now(),
                text_fixed: false,
            });
        }
        cx.notify();
    }
    pub(super) fn update_group_gesture(
        &mut self,
        event: &MouseMoveEvent,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if let Some(marquee) = &self.editor.marquee {
            if !event.dragging() {
                return true;
            }
            let p = self.image_pos(event.position, window);
            let start = marquee.start;
            let mut selected = marquee.previous.clone();
            let b = Rect::between(start, p);
            if start.distance(p) * self.transform(window).1 >= 3. {
                selected.extend(
                    self.doc()
                        .unwrap()
                        .marks
                        .iter()
                        .enumerate()
                        .filter(|(_, mark)| mark.tool != Tool::Redact && mark.intersects(b))
                        .map(|(i, _)| i),
                );
            }
            self.set_selection(selected);
            self.editor.marquee.as_mut().unwrap().current = p;
            if let Some(i) = self.editor.selected {
                self.editor.style = self.doc().unwrap().marks[i].style.clone().with_scale(1.);
            }
            cx.notify();
            return true;
        }
        let Some(gesture) = self.editor.group_gesture.clone() else {
            return false;
        };
        if !event.dragging() {
            return true;
        }
        let p = self.image_pos(event.position, window);
        if !gesture.checkpointed {
            if gesture.start.distance(p) * self.transform(window).1 < 0.5 {
                return true;
            }
            self.doc_mut().unwrap().checkpoint();
            self.editor.group_gesture.as_mut().unwrap().checkpointed = true;
        }
        let mut frame = gesture.frame;
        let mut ratio = 1.;
        let mut angle = 0.;
        let (mut flip_x, mut flip_y) = (false, false);
        match gesture.mode {
            Mode::Move => {
                frame.bounds.x += p.x - gesture.start.x;
                frame.bounds.y += p.y - gesture.start.y;
            }
            Mode::Rotate(_) => {
                let c = frame.center();
                angle = (p.y - c.y).atan2(p.x - c.x)
                    - (gesture.start.y - c.y).atan2(gesture.start.x - c.x);
                if event.modifiers.shift {
                    let step = std::f32::consts::PI / 12.;
                    angle = ((frame.rotation + angle) / step).round() * step - frame.rotation;
                }
                frame.rotation += angle;
            }
            Mode::Resize(handle) => {
                let b = frame.bounds;
                let to = frame.to_local(p);
                let (next, x, y) = handle.resized(b, to, true);
                flip_x = x;
                flip_y = y;
                ratio = next.w / b.w;
                frame.bounds = next;
                let center = frame.center();
                let world = gesture.frame.to_world(center);
                frame.bounds.x += world.x - center.x;
                frame.bounds.y += world.y - center.y;
            }
            _ => return true,
        }
        let map = |p: Pos| {
            let q = gesture.frame.to_local(p);
            let c = gesture.frame.center();
            let delta = Pos::new(
                (q.x - c.x) * ratio * if flip_x { -1. } else { 1. },
                (q.y - c.y) * ratio * if flip_y { -1. } else { 1. },
            )
            .rotate(Pos::default(), frame.rotation);
            Pos::new(frame.center().x + delta.x, frame.center().y + delta.y)
        };
        let doc = self.doc_mut().unwrap();
        let mut expanded_label = false;
        for (i, original) in gesture.originals.iter() {
            let mark = &mut doc.marks[*i];
            *mark = original.clone();
            if matches!(gesture.mode, Mode::Resize(_)) && mark.tool != Tool::Text {
                mark.style = mark.style.clone().with_scale(mark.style.scale * ratio);
            }
            if matches!(mark.tool, Tool::Arrow | Tool::Line) {
                for p in &mut mark.points {
                    *p = map(*p);
                }
                if !mark.text.trim().is_empty() {
                    let before = mark.points.clone();
                    mark.constrain_arrow_label(None);
                    expanded_label |= mark.points != before;
                }
            } else {
                let old = original.bounds();
                mark.resize(
                    old,
                    Rect {
                        x: old.x,
                        y: old.y,
                        w: old.w * ratio,
                        h: old.h * ratio,
                    },
                    false,
                );
                mark.flip_geometry(flip_x, flip_y);
                let center = mark.bounds().center();
                let target = map(old.center());
                mark.translate(target.x - center.x, target.y - center.y);
                mark.rotation = if flip_x != flip_y {
                    2. * gesture.frame.rotation - original.rotation + angle
                } else {
                    original.rotation + angle
                };
            }
        }
        self.editor.group_frame = (!expanded_label).then_some(frame);
        self.editor.style = self.doc().unwrap().marks[self.editor.selected.unwrap()]
            .style
            .clone()
            .with_scale(1.);
        self.refresh_annotations(cx);
        true
    }
}

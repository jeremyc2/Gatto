use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Handle {
    Corner(usize),
    Edge(usize),
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub(in crate::image_preview) enum SelectionCursor {
    #[default]
    Arrow,
    Move,
    /// Clockwise screen angle in degrees; resize axes repeat every half turn.
    Resize(u16),
    Rotate(u16),
}
impl SelectionCursor {
    pub fn style(self) -> CursorStyle {
        match self {
            Self::Arrow | Self::Move => CursorStyle::Arrow,
            Self::Rotate(_) => CursorStyle::Crosshair,
            Self::Resize(angle) => match ((angle + 22) / 45) % 4 {
                0 => CursorStyle::ResizeLeftRight,
                1 => CursorStyle::ResizeUpLeftDownRight,
                2 => CursorStyle::ResizeUpDown,
                _ => CursorStyle::ResizeUpRightDownLeft,
            },
        }
    }
}
impl Handle {
    pub(super) fn anchor(self, b: Rect) -> Pos {
        match self {
            Self::Corner(i) => b.corners()[(i + 2) % 4],
            Self::Edge(0) => Pos::new(b.x + b.w / 2., b.y + b.h),
            Self::Edge(1) => Pos::new(b.x, b.y + b.h / 2.),
            Self::Edge(2) => Pos::new(b.x + b.w / 2., b.y),
            Self::Edge(_) => Pos::new(b.x + b.w, b.y + b.h / 2.),
        }
    }
    fn cursor(self, rotation: f32) -> SelectionCursor {
        let base = match self {
            Self::Corner(i) => {
                if i % 2 == 0 {
                    45.
                } else {
                    135.
                }
            }
            Self::Edge(i) => {
                if i % 2 == 0 {
                    90.
                } else {
                    0.
                }
            }
        };
        SelectionCursor::Resize((base + rotation.to_degrees()).round().rem_euclid(180.) as u16)
    }
    pub(super) fn point(self, b: Rect) -> Pos {
        match self {
            Self::Corner(i) => b.corners()[i],
            Self::Edge(i) => Self::Edge((i + 2) % 4).anchor(b),
        }
    }
    pub(super) fn flips(self, b: Rect, p: Pos) -> (bool, bool) {
        let fixed = self.anchor(b);
        let from = self.point(b);
        (
            (p.x - fixed.x) * (from.x - fixed.x) < 0.,
            (p.y - fixed.y) * (from.y - fixed.y) < 0.,
        )
    }
    pub(super) fn flipped(self, x: bool, y: bool) -> Self {
        match self {
            Self::Corner(mut i) => {
                if x {
                    i = [1, 0, 3, 2][i];
                }
                if y {
                    i = [3, 2, 1, 0][i];
                }
                Self::Corner(i)
            }
            Self::Edge(i) if (i % 2 == 0 && y) || (i % 2 == 1 && x) => Self::Edge((i + 2) % 4),
            _ => self,
        }
    }
    /// Normalize crossed dimensions while retaining the fixed opposite anchor.
    pub(super) fn resized(self, b: Rect, p: Pos, aspect: bool) -> (Rect, bool, bool) {
        let fixed = self.anchor(b);
        let from = self.point(b);
        let (flip_x, flip_y) = self.flips(b, p);
        let mut next = b;
        if aspect {
            let (dx, dy) = (from.x - fixed.x, from.y - fixed.y);
            let ratio = match self {
                Self::Corner(_) => {
                    ((p.x - fixed.x).abs() * dx.abs() + (p.y - fixed.y).abs() * dy.abs())
                        / (dx * dx + dy * dy).max(1.)
                }
                Self::Edge(0 | 2) => (p.y - fixed.y).abs() / b.h,
                Self::Edge(_) => (p.x - fixed.x).abs() / b.w,
            }
            .clamp(0.05, 100.);
            next.w = b.w * ratio;
            next.h = b.h * ratio;
        } else {
            match self {
                Self::Corner(_) => {
                    next.w = (p.x - fixed.x).abs().max(1.);
                    next.h = (p.y - fixed.y).abs().max(1.);
                }
                Self::Edge(0 | 2) => next.h = (p.y - fixed.y).abs().max(1.),
                Self::Edge(_) => next.w = (p.x - fixed.x).abs().max(1.),
            }
        }
        let anchor = self.flipped(flip_x, flip_y).anchor(next);
        next.x += fixed.x - anchor.x;
        next.y += fixed.y - anchor.y;
        (next, flip_x, flip_y)
    }
}

impl ImagePreview {
    pub(super) fn selection_target(&self, p: Pos, scale: f32) -> Option<(usize, Mode)> {
        let tolerance = 7. / scale;
        if self.selected_indices().len() > 1
            && let Some(frame) = self.selection_frame()
        {
            if let Some(mode) = frame_target(frame, p, scale) {
                return self.editor.selected.map(|i| (i, mode));
            }
            if frame.bounds.contains(frame.to_local(p), 0.) {
                return self.editor.selected.map(|i| (i, Mode::Move));
            }
            return self.doc()?.hit(p, tolerance).map(|i| (i, Mode::Move));
        }
        if let Some(index) = self.editor.selected
            && let Some(mark) = self.doc()?.marks.get(index)
        {
            if matches!(mark.tool, Tool::Arrow | Tool::Line) {
                if mark.is_line_dot()
                    && p.distance(mark.points[0]) < tolerance + mark.style.width / 2.
                {
                    return Some((index, Mode::Endpoint(1)));
                }
                for (i, at) in [mark.points[0], mark.points[1]].into_iter().enumerate() {
                    if p.distance(at) < tolerance {
                        return Some((index, Mode::Endpoint(i)));
                    }
                }
                if p.distance(mark.bend_handle()) < tolerance {
                    return Some((index, Mode::Bend));
                }
                if mark.hit_text(p, tolerance) {
                    return Some((index, Mode::Move));
                }
            } else {
                if let Some(mode) = frame_target(mark.frame(), p, scale) {
                    return Some((index, mode));
                }
                if mark.contains_body(p) {
                    return Some((index, Mode::Move));
                }
            }
        }
        self.doc()?.hit(p, tolerance).map(|i| {
            let mode = if self.doc().unwrap().marks[i].is_line_dot() {
                Mode::Endpoint(1)
            } else {
                Mode::Move
            };
            (i, mode)
        })
    }
    fn mode_cursor(&self, mode: Mode, index: usize) -> SelectionCursor {
        let rotation = if self.selected_indices().len() > 1 {
            self.selection_frame().map_or(0., |f| f.rotation)
        } else {
            self.doc()
                .and_then(|d| d.marks.get(index))
                .map_or(0., |m| m.rotation)
        };
        match mode {
            Mode::Resize(handle) => handle.cursor(rotation),
            // The cursor artwork bends around the top-right corner at zero degrees.
            Mode::Rotate(corner) => SelectionCursor::Rotate(
                (rotation.to_degrees() + (corner as f32 - 1.) * 90.)
                    .round()
                    .rem_euclid(360.) as u16,
            ),
            _ => SelectionCursor::Move,
        }
    }
    fn resize_cursor_mode(&self, mode: Mode, frame: crate::annotation::Frame, p: Pos) -> Mode {
        if let Mode::Resize(handle) = mode {
            let (x, y) = handle.flips(frame.bounds, frame.to_local(p));
            Mode::Resize(handle.flipped(x, y))
        } else {
            mode
        }
    }
    pub(in crate::image_preview) fn selection_cursor(&self) -> SelectionCursor {
        self.editor.selection_cursor
    }
    pub(super) fn update_selection_cursor(
        &mut self,
        p: Point<Pixels>,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        if self.editor.tool != Tool::Select
            || self.editor.editing.is_some()
            || self.editor.space
            || self.drag.is_some()
        {
            return;
        }
        let cursor = if self.editor.marquee.is_some() {
            SelectionCursor::Arrow
        } else if let Some(gesture) = &self.editor.group_gesture {
            let mode =
                self.resize_cursor_mode(gesture.mode, gesture.frame, self.image_pos(p, window));
            self.mode_cursor(mode, self.editor.selected.unwrap())
        } else if let Some(gesture) = &self.editor.gesture {
            let mode = self.resize_cursor_mode(
                gesture.mode,
                gesture.original.frame(),
                self.image_pos(p, window),
            );
            self.mode_cursor(mode, gesture.index)
        } else {
            let (_, scale) = self.transform(window);
            self.selection_target(self.image_pos(p, window), scale)
                .map_or(SelectionCursor::Arrow, |(index, mode)| {
                    self.mode_cursor(mode, index)
                })
        };
        // GPUI can reset the OS cursor when the hovered hitbox changes, even if
        // the semantic cursor stays the same. Reapply it on every pointer move.
        let changed = self.editor.selection_cursor != cursor;
        self.editor.selection_cursor = cursor;
        self.sync_selection_cursor(window, cursor.style(), cx);
        if changed {
            cx.notify();
        }
    }
}

fn frame_target(frame: crate::annotation::Frame, p: Pos, scale: f32) -> Option<Mode> {
    let tolerance = 7. / scale;
    let q = frame.to_local(p);
    let b = frame.bounds;
    let nearest = b
        .corners()
        .into_iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| q.distance(*a).total_cmp(&q.distance(*b)))
        .unwrap();
    if q.distance(nearest.1) <= tolerance {
        return Some(Mode::Resize(Handle::Corner(nearest.0)));
    }
    let outside_x = if nearest.0 == 0 || nearest.0 == 3 {
        q.x < b.x
    } else {
        q.x > b.x + b.w
    };
    let outside_y = if nearest.0 < 2 {
        q.y < b.y
    } else {
        q.y > b.y + b.h
    };
    if outside_x && outside_y && q.distance(nearest.1) <= 22. / scale {
        return Some(Mode::Rotate(nearest.0));
    }
    let edges = [
        (q.y - b.y).abs(),
        (q.x - b.x - b.w).abs(),
        (q.y - b.y - b.h).abs(),
        (q.x - b.x).abs(),
    ];
    if b.contains(q, tolerance)
        && let Some((edge, _)) = edges
            .into_iter()
            .enumerate()
            .filter(|(_, d)| *d <= tolerance)
            .min_by(|a, b| a.1.total_cmp(&b.1))
    {
        return Some(Mode::Resize(Handle::Edge(edge)));
    }
    None
}

pub(super) fn resize(mark: &mut Mark, original: &Mark, handle: Handle, p: Pos, shift: bool) {
    let frame = original.frame();
    let old = frame.bounds;
    let p = frame.to_local(p);
    let fixed = handle.anchor(old);
    let wrap = original.tool == Tool::Text && matches!(handle, Handle::Edge(1 | 3));
    let aspect =
        (original.tool == Tool::Text && !wrap) || (shift && matches!(handle, Handle::Corner(_)));
    let (mut next, flip_x, flip_y) = handle.resized(old, p, aspect);
    let minimum = original.constrain_text_bounds(next);
    if aspect && original.tool != Tool::Text {
        if minimum.w > next.w || minimum.h > next.h {
            // Find the smallest proportional frame that fits the label. Using
            // its height at a very narrow width would make shrinking jump larger.
            let fits = |ratio: f32| {
                let bounds = Rect {
                    w: old.w * ratio,
                    h: old.h * ratio,
                    ..next
                };
                let minimum = original.constrain_text_bounds(bounds);
                minimum.w <= bounds.w && minimum.h <= bounds.h
            };
            let mut low = next.w / old.w;
            let gain = (minimum.w / next.w).max(minimum.h / next.h);
            let mut high = if low < 1. && fits(1.) { 1. } else { low * gain };
            for _ in 0..12 {
                let mid = (low + high) / 2.;
                if fits(mid) {
                    high = mid;
                } else {
                    low = mid;
                }
            }
            next.w = old.w * high;
            next.h = old.h * high;
        }
    } else {
        next = minimum;
    }
    *mark = original.clone();
    mark.resize(old, next, wrap);
    if mark.tool == Tool::Text && mark.wrap.is_none() {
        // Any direct text resize establishes a wrapping width. Group scaling
        // keeps its separate behavior and does not convert free-form text.
        mark.wrap = Some(mark.bounds().w);
    }
    mark.flip_geometry(flip_x, flip_y);
    // Crossing swaps the moving handle; pin the newly opposite anchor after reflow.
    let new_anchor = handle.flipped(flip_x, flip_y).anchor(mark.bounds());
    mark.translate(fixed.x - new_anchor.x, fixed.y - new_anchor.y);
    let center = mark.bounds().center();
    let world_center = frame.to_world(center);
    mark.translate(world_center.x - center.x, world_center.y - center.y);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shift_corner_resizing_preserves_proportions_anchors_and_label_minimums() {
        for tool in crate::annotation::SHAPES
            .into_iter()
            .filter(|t| t.is_shape())
            .chain([Tool::Pen])
        {
            for rotation in [0., 0.7] {
                let mut original = Mark::new(tool, Pos::new(80., 90.), Style::default());
                original.points[1] = Pos::new(280., 210.);
                if tool.is_shape() {
                    original.set_text("Keep this label".into());
                }
                original.rotation = rotation;
                let frame = original.frame();
                for handle in (0..4).map(Handle::Corner) {
                    let fixed = handle.anchor(frame.bounds);
                    let from = handle.point(frame.bounds);
                    for (sx, sy) in [(1.7, 0.6), (0.01, 0.01), (-1.5, 0.7), (-1.5, -0.7)] {
                        let to = Pos::new(
                            fixed.x + (from.x - fixed.x) * sx,
                            fixed.y + (from.y - fixed.y) * sy,
                        );
                        let mut mark = original.clone();
                        resize(&mut mark, &original, handle, frame.to_world(to), true);
                        let b = mark.bounds();
                        assert!(
                            (b.w / b.h - frame.bounds.w / frame.bounds.h).abs() < 0.001,
                            "{tool:?}"
                        );
                        if sx == 0.01 {
                            assert!(
                                b.w <= frame.bounds.w + 0.01 && b.h <= frame.bounds.h + 0.01,
                                "Shrinking must not jump larger: {tool:?}"
                            );
                        }
                        let (x, y) = handle.flips(frame.bounds, to);
                        let anchor = handle.flipped(x, y).anchor(b);
                        assert!(
                            mark.frame()
                                .to_world(anchor)
                                .distance(frame.to_world(fixed))
                                < 0.01
                        );
                        assert_eq!(mark.text, original.text);
                        assert_eq!(mark.text_font_size(), original.text_font_size());
                        if tool.is_shape() {
                            assert!(b.h + 0.01 >= mark.text_layout().height + 32.);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn every_direct_text_resize_converts_free_form_to_fixed_width_without_spurious_wrapping() {
        for scale in [0.4, 1., 3.25] {
            for rotation in [0., 0.7] {
                let mut original = Mark::new(
                    Tool::Text,
                    Pos::new(100., 100.),
                    Style::default().with_scale(scale),
                );
                original.set_text("Fractional text width".into());
                original.rotation = rotation;
                let frame = original.frame();
                for handle in (0..4).map(Handle::Corner).chain((0..4).map(Handle::Edge)) {
                    let fixed = handle.anchor(frame.bounds);
                    let from = handle.point(frame.bounds);
                    let to = Pos::new(
                        fixed.x + (from.x - fixed.x) * 1.4,
                        fixed.y + (from.y - fixed.y) * 1.4,
                    );
                    let mut mark = original.clone();
                    resize(&mut mark, &original, handle, frame.to_world(to), false);
                    assert!(mark.wrap.is_some(), "{handle:?}");
                    assert_eq!(mark.text_layout().height, mark.text_layout().line_height);
                    assert!(
                        mark.frame()
                            .to_world(handle.anchor(mark.bounds()))
                            .distance(frame.to_world(fixed))
                            < 0.01
                    );
                    if matches!(handle, Handle::Edge(1 | 3)) {
                        assert_eq!(mark.text_font_size(), original.text_font_size());
                    } else {
                        assert!(mark.text_font_size() > original.text_font_size());
                    }
                }
                assert!(original.wrap.is_none());
            }
        }
    }

    #[test]
    fn handles_cross_opposite_edges_without_mirroring_labels_or_losing_the_anchor() {
        let tools = crate::annotation::SHAPES
            .into_iter()
            .filter(|t| t.is_shape())
            .chain([Tool::Text, Tool::Pen]);
        for tool in tools {
            for rotation in [0., 0.7] {
                let mut original = Mark::new(tool, Pos::new(80., 90.), Style::default());
                original.points[1] = Pos::new(380., 310.);
                original.wrap = (tool == Tool::Text).then_some(300.);
                original.set_text("Readable text".into());
                original.rotation = rotation;
                let frame = original.frame();
                for handle in (0..4).map(Handle::Corner).chain((0..4).map(Handle::Edge)) {
                    let fixed = handle.anchor(frame.bounds);
                    let from = handle.point(frame.bounds);
                    for (sx, sy) in [(-1.5, 1.5), (1.5, -1.5), (-1.5, -1.5)] {
                        let to = Pos::new(
                            fixed.x + (from.x - fixed.x) * sx,
                            fixed.y + (from.y - fixed.y) * sy,
                        );
                        let mut mark = original.clone();
                        resize(&mut mark, &original, handle, frame.to_world(to), false);
                        let (x, y) = (sx < 0. && from.x != fixed.x, sy < 0. && from.y != fixed.y);
                        let after = mark
                            .frame()
                            .to_world(handle.flipped(x, y).anchor(mark.bounds()));
                        assert!(
                            after.distance(frame.to_world(fixed)) < 0.01,
                            "{tool:?} {handle:?} {sx},{sy}: anchor moved"
                        );
                        assert!(
                            mark.bounds().w > 0.
                                && mark.bounds().h > 0.
                                && mark.style.font_size > 0.
                        );
                        assert_eq!(mark.text, original.text);
                        let center = frame.to_local(mark.frame().center());
                        if from.x != fixed.x {
                            assert_eq!(
                                (center.x - fixed.x).signum(),
                                ((from.x - fixed.x) * sx).signum(),
                                "{tool:?} {handle:?}: horizontal crossing stopped"
                            );
                        }
                        if from.y != fixed.y {
                            assert_eq!(
                                (center.y - fixed.y).signum(),
                                ((from.y - fixed.y) * sy).signum(),
                                "{tool:?} {handle:?}: vertical crossing stopped"
                            );
                        }
                        if tool.is_shape() {
                            assert_eq!((mark.flip_x, mark.flip_y), (x, y));
                        } else if tool == Tool::Text {
                            assert!(!mark.flip_x && !mark.flip_y);
                            assert!(mark.text_layout().width > 0.);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn labeled_shapes_stop_at_their_content_minimum_from_every_resize_handle() {
        for tool in crate::annotation::SHAPES
            .into_iter()
            .filter(|t| t.is_shape())
        {
            for rotation in [0., 0.7] {
                let mut original = Mark::new(tool, Pos::new(80., 90.), Style::default());
                original.points[1] = Pos::new(380., 310.);
                original.set_text("Keep all of these words visible while resizing.".into());
                original.rotation = rotation;
                for handle in (0..4).map(Handle::Corner).chain((0..4).map(Handle::Edge)) {
                    let frame = original.frame();
                    let fixed = handle.anchor(original.bounds());
                    let from = match handle {
                        Handle::Corner(i) => original.bounds().corners()[i],
                        Handle::Edge(i) => Handle::Edge((i + 2) % 4).anchor(original.bounds()),
                    };
                    let p = fixed.lerp(from, 0.01);
                    let mut mark = original.clone();
                    resize(&mut mark, &original, handle, frame.to_world(p), false);
                    assert_eq!(mark.text_font_size(), original.text_font_size());
                    assert!(mark.label_bounds().is_some());
                    assert!(mark.bounds().h + 0.01 >= mark.text_layout().height + 32.);
                    assert!(
                        mark.frame()
                            .to_world(handle.anchor(mark.bounds()))
                            .distance(frame.to_world(fixed))
                            < 0.01,
                        "{tool:?}, {rotation}, {handle:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn rotated_border_and_corner_resizing_keeps_the_opposite_anchor_fixed() {
        for tool in [Tool::Rectangle, Tool::Text, Tool::Pen] {
            for rotation in [0., 0.4, -1.2, std::f32::consts::PI] {
                for handle in (0..4).map(Handle::Corner).chain((0..4).map(Handle::Edge)) {
                    let mut mark = Mark::new(
                        tool,
                        Pos::new(60., 80.),
                        Style {
                            font_size: 24.,
                            ..Default::default()
                        },
                    );
                    mark.points[1] = Pos::new(240., 200.);
                    mark.text = "Keep this corner fixed while resizing".into();
                    mark.wrap = (tool == Tool::Text).then_some(180.);
                    mark.rotation = rotation;
                    let frame = mark.frame();
                    let b = frame.bounds;
                    let fixed = frame.to_world(handle.anchor(b));
                    let p = match handle {
                        Handle::Corner(i) => b.corners()[i],
                        Handle::Edge(i) => Handle::Edge((i + 2) % 4).anchor(b),
                    };
                    let p = Pos::new(
                        b.center().x + (p.x - b.center().x) * 1.6,
                        b.center().y + (p.y - b.center().y) * 1.6,
                    );
                    let old = mark.clone();
                    resize(&mut mark, &old, handle, frame.to_world(p), false);
                    let after = mark.frame().to_world(handle.anchor(mark.bounds()));
                    assert!(
                        fixed.distance(after) < 0.001,
                        "{tool:?}, {rotation}, {handle:?}: {fixed:?} moved to {after:?}"
                    );
                    if tool == Tool::Text && matches!(handle, Handle::Edge(1 | 3)) {
                        assert_eq!(mark.style.font_size, old.style.font_size);
                        assert!(mark.wrap.unwrap() > old.wrap.unwrap());
                    }
                }
            }
        }
    }
}

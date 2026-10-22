//! Text hosts share measured line boxes, alignment, geometry, and export.
mod layout;
use super::*;
use layout::Layout;
#[cfg(test)]
pub use layout::line_height;

pub const TEXT_SIZES: [f32; 4] = [18., 24., 36., 44.];
pub const LABEL_SIZES: [f32; 4] = [18., 22., 26., 32.];
pub const ARROW_TEXT_SIZES: [f32; 4] = [18., 20., 24., 28.];
const PADDING: f32 = 16.;
const ARROW_PADDING: f32 = 4.25;
const ARROW_VISIBLE_SHAFT: f32 = 12.;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Align {
    Start,
    #[default]
    Middle,
    End,
}
impl Align {
    pub fn offset(self, available: f32) -> f32 {
        available
            * match self {
                Self::Start => 0.,
                Self::Middle => 0.5,
                Self::End => 1.,
            }
    }
}

impl Mark {
    pub fn text_font_size(&self) -> f32 {
        self.style.font_size
    }
    pub fn text_align(&self) -> Align {
        self.style.text_align.unwrap_or(if self.tool == Tool::Text {
            Align::Start
        } else {
            Align::Middle
        })
    }
    pub fn can_edit_label(&self) -> bool {
        self.tool.is_shape() || matches!(self.tool, Tool::Arrow | Tool::Line)
    }
    pub fn hit_text(&self, p: Pos, tolerance: f32) -> bool {
        if self.text.trim().is_empty() {
            return false;
        }
        let Some(bounds) = self.label_edit_bounds() else {
            return false;
        };
        let p = self.frame().to_local(p);
        self.text_layout().hit(
            &self.text,
            Pos::new(p.x - bounds.x, p.y - bounds.y),
            bounds.w,
            self.text_align(),
            3. * self.style.scale + tolerance.min(2. * self.style.scale),
        )
    }
    pub(crate) fn text_layout(&self) -> Arc<Layout> {
        self.layout_for(self.text_wrap())
    }
    pub(super) fn has_bitmap_text(&self) -> bool {
        !self.text.is_empty() && self.text_layout().has_bitmaps()
    }
    pub(super) fn paint_bitmap_text(&self, target: &mut Pixmap, transform: Transform) {
        if let Some(bounds) = self.label_edit_bounds() {
            self.text_layout().paint_bitmaps(
                target,
                bounds.w,
                self.text_align(),
                Transform::from_translate(bounds.x, bounds.y).post_concat(transform),
            );
        }
    }
    fn layout_for(&self, wrap: Option<f32>) -> Arc<Layout> {
        Layout::get_scaled(&self.text, self.text_font_size(), wrap, self.style.scale)
    }
    fn text_wrap(&self) -> Option<f32> {
        if self.tool.is_shape() {
            Some((self.bounds().w - PADDING * self.style.scale * 2.).max(self.minimum_text_width()))
        } else if matches!(self.tool, Tool::Arrow | Tool::Line) {
            let available = (self.arrow_length() - self.arrow_label_clearance())
                .max(self.arrow_label_word_width());
            // Natural-width labels must not enter the fixed-width rounding path:
            // flooring a fractional glyph advance can split even a short word.
            (self.layout_for(None).width > available).then_some(available)
        } else {
            self.wrap
        }
    }
    fn arrow_cap_clearance(&self, cap: Cap) -> f32 {
        let cap = if self.tool == Tool::Line {
            Cap::None
        } else {
            cap
        };
        let depth = match cap {
            Cap::None => self.style.width / 2.,
            Cap::Circle => self.style.width * CAP_RADIUS,
            Cap::Triangle => self.style.width * CAP_LENGTH * CAP_ANGLE.cos(),
            Cap::Arrow => self.style.width * (CAP_LENGTH * CAP_ANGLE.cos() + 0.5),
        };
        depth + (ARROW_PADDING + ARROW_VISIBLE_SHAFT) * self.style.scale + self.style.width / 2.
    }
    fn arrow_label_clearance(&self) -> f32 {
        self.arrow_cap_clearance(self.style.start) + self.arrow_cap_clearance(self.style.end)
    }
    fn arrow_label_word_width(&self) -> f32 {
        let width = self
            .text
            .split_whitespace()
            .map(|word| {
                Layout::get_scaled(word, self.text_font_size(), None, self.style.scale).width
            })
            .fold(0., f32::max);
        // Leave a measured annotation unit for CoreText's fixed-width rounding.
        (width / self.style.scale).ceil().max(16.) * self.style.scale + self.style.scale
    }
    fn arrow_length(&self) -> f32 {
        let mut previous = self.points[0];
        let a = previous;
        let b = self.points[1];
        let c = self.points.get(2).copied().unwrap_or(a.lerp(b, 0.5));
        (1..=64)
            .map(|i| {
                let t = i as f32 / 64.;
                let next = a.lerp(c, t).lerp(c.lerp(b, t), t);
                let length = previous.distance(next);
                previous = next;
                length
            })
            .sum()
    }
    /// Preserve whole words and visible shaft beyond both caps, at every path angle.
    pub(crate) fn constrain_arrow_label(&mut self, fixed_endpoint: Option<usize>) {
        if !matches!(self.tool, Tool::Arrow | Tool::Line) || self.text.trim().is_empty() {
            return;
        }
        let minimum = self.arrow_label_word_width() + self.arrow_label_clearance();
        let length = self.arrow_length();
        let anchor =
            fixed_endpoint.map_or_else(|| self.arrow_at(self.label_position), |i| self.points[i]);
        if length < 0.001 {
            let start = if fixed_endpoint == Some(1) {
                anchor.x - minimum
            } else if fixed_endpoint == Some(0) {
                anchor.x
            } else {
                anchor.x - minimum / 2.
            };
            self.points = vec![
                Pos::new(start, anchor.y),
                Pos::new(start + minimum, anchor.y),
            ];
        } else if length < minimum {
            let scale = minimum / length;
            for p in &mut self.points {
                *p = Pos::new(
                    anchor.x + (p.x - anchor.x) * scale,
                    anchor.y + (p.y - anchor.y) * scale,
                );
            }
        }
        if self.safe_arrow_label_position().is_some() {
            return;
        }
        // A multiline label is an upright rectangle, not a distance along the
        // path. Check its actual footprint, including the gap and stroke radius.
        // Re-measure wrapping as the arrow grows; retain the dragged endpoint's
        // opposite anchor, font size, and relative curve geometry.
        let original = self.points.clone();
        let resize = |mark: &mut Self, factor: f32| {
            for (p, original) in mark.points.iter_mut().zip(&original) {
                *p = Pos::new(
                    anchor.x + (original.x - anchor.x) * factor,
                    anchor.y + (original.y - anchor.y) * factor,
                );
            }
        };
        let mut low = 1.;
        let mut high = 2.;
        loop {
            resize(self, high);
            if self.safe_arrow_label_position().is_some() {
                break;
            }
            low = high;
            high *= 2.;
        }
        for _ in 0..12 {
            let middle = (low + high) / 2.;
            resize(self, middle);
            if self.safe_arrow_label_position().is_some() {
                high = middle;
            } else {
                low = middle;
            }
        }
        resize(self, high);
    }
    fn minimum_text_width(&self) -> f32 {
        (self.text_font_size() / self.style.scale * 0.65 + 2.).ceil() * self.style.scale
    }

    /// Line boxes keep empty/trailing lines and have the same origin while editing.
    pub fn label_edit_bounds(&self) -> Option<Rect> {
        if self.tool == Tool::Text {
            return Some(self.bounds());
        }
        let b = self.label_bounds()?;
        let height = self.text_layout().height;
        Some(Rect {
            x: b.x,
            y: b.y + self.style.vertical_align.offset((b.h - height).max(0.)),
            w: b.w,
            h: height,
        })
    }
    pub fn label_bounds(&self) -> Option<Rect> {
        if self.tool.is_shape() {
            let b = self.bounds();
            Some(Rect {
                x: b.x + PADDING * self.style.scale,
                y: b.y + PADDING * self.style.scale,
                w: (b.w - PADDING * self.style.scale * 2.).max(self.minimum_text_width()),
                h: (b.h - PADDING * self.style.scale * 2.).max(self.text_layout().height),
            })
        } else if matches!(self.tool, Tool::Arrow | Tool::Line) {
            let layout = self.text_layout();
            let center = self.arrow_at(self.clamped_label_position());
            Some(Rect {
                x: center.x - layout.width.max(16. * self.style.scale) / 2.,
                y: center.y - layout.height / 2.,
                w: layout.width.max(16. * self.style.scale),
                h: layout.height,
            })
        } else {
            None
        }
    }
    pub fn set_text(&mut self, text: String) {
        if text.is_empty() && self.text.is_empty() {
            return;
        }
        let old = (self.tool == Tool::Text && self.wrap.is_none()).then(|| self.bounds());
        self.text = text;
        if self.tool.is_shape() {
            let b = self.bounds();
            let first = self.label_base_height.is_none();
            let base = *self.label_base_height.get_or_insert(b.h);
            let (mut width, mut height) = if self.text.trim().is_empty() {
                (b.w, base)
            } else {
                let layout = self.text_layout();
                (
                    (layout.width + PADDING * self.style.scale * 2.).max(b.w),
                    (layout.height + PADDING * self.style.scale * 2.).max(base),
                )
            };
            if first
                && !self.text.trim().is_empty()
                && b.w < 51. * self.style.scale
                && b.h < 51. * self.style.scale
            {
                width = width.max(height).max(51. * self.style.scale);
                height = width;
                self.label_base_height = Some(height);
            }
            self.set_shape_bounds(Rect {
                w: width,
                h: height,
                ..b
            });
        } else if let Some(old) = old {
            let next = self.bounds();
            let delta = self.text_align().offset(old.w - next.w);
            let from = Pos::new(old.x, old.y).rotate(old.center(), self.rotation);
            let to = Pos::new(next.x - delta, next.y).rotate(next.center(), self.rotation);
            self.translate(from.x - to.x, from.y - to.y);
        }
        self.constrain_arrow_label(None);
    }
    /// Geometric resize keeps text size and enforces the measured content minimum.
    pub fn constrain_text_bounds(&self, mut b: Rect) -> Rect {
        if self.tool.is_shape() && !self.text.trim().is_empty() {
            b.w =
                b.w.max(self.minimum_text_width() + PADDING * self.style.scale * 2.);
            let layout = self.layout_for(Some(b.w - PADDING * self.style.scale * 2.));
            b.w = b.w.max(layout.width + PADDING * self.style.scale * 2.);
            b.h = b.h.max(layout.height + PADDING * self.style.scale * 2.);
        }
        b
    }
    fn set_shape_bounds(&mut self, b: Rect) {
        let frame = self.frame();
        let fixed = frame.to_world(Pos::new(b.x, b.y));
        self.points[0] = Pos::new(b.x, b.y);
        self.points[1] = Pos::new(b.x + b.w, b.y + b.h);
        let after = self.frame().to_world(self.points[0]);
        self.translate(fixed.x - after.x, fixed.y - after.y);
    }
    pub(super) fn label_color(&self) -> [u8; 3] {
        if self.tool.is_shape() {
            [255; 3]
        } else {
            COLORS[self.style.color]
        }
    }
    pub(super) fn label_path(&self) -> Option<Path> {
        if self.text.is_empty() || !self.can_edit_label() {
            return None;
        }
        let b = self.label_edit_bounds()?;
        self.text_layout()
            .path(b.w, self.text_align())?
            .transform(Transform::from_translate(b.x, b.y))
    }
    pub(super) fn standalone_path(&self) -> Option<Path> {
        let b = self.bounds();
        self.text_layout()
            .path(b.w, self.text_align())?
            .transform(Transform::from_translate(b.x, b.y))
    }
    pub fn arrow_at(&self, fraction: f32) -> Pos {
        let mut lengths = [0.; 65];
        let a = self.points[0];
        let b = self.points[1];
        let c = self.points.get(2).copied().unwrap_or(a.lerp(b, 0.5));
        let at = |t| a.lerp(c, t).lerp(c.lerp(b, t), t);
        let mut previous = a;
        for (i, length) in lengths.iter_mut().enumerate().skip(1) {
            let next = at(i as f32 / 64.);
            *length = previous.distance(next);
            previous = next;
        }
        for i in 1..=64 {
            lengths[i] += lengths[i - 1];
        }
        let target = lengths[64] * fraction.clamp(0., 1.);
        let i = (1..=64).find(|i| lengths[*i] >= target).unwrap_or(64);
        let t = ((i - 1) as f32
            + (target - lengths[i - 1]) / (lengths[i] - lengths[i - 1]).max(0.001))
            / 64.;
        at(t)
    }
    pub fn clamped_label_position(&self) -> f32 {
        self.safe_arrow_label_position().unwrap_or(0.5)
    }
    fn safe_arrow_label_position(&self) -> Option<f32> {
        let layout = self.text_layout();
        let start_clearance =
            self.arrow_cap_clearance(self.style.start) - ARROW_PADDING * self.style.scale;
        let end_clearance =
            self.arrow_cap_clearance(self.style.end) - ARROW_PADDING * self.style.scale;
        let safe = |fraction| {
            let center = self.arrow_at(fraction);
            let width = layout.width.max(16. * self.style.scale);
            let b = Rect {
                x: center.x - width / 2. - ARROW_PADDING * self.style.scale,
                y: center.y - layout.height / 2. - ARROW_PADDING * self.style.scale,
                w: width + ARROW_PADDING * self.style.scale * 2.,
                h: layout.height + ARROW_PADDING * self.style.scale * 2.,
            };
            !b.contains(self.points[0], start_clearance)
                && !b.contains(self.points[1], end_clearance)
        };
        let target = self.label_position.clamp(0., 1.);
        if safe(target) {
            return Some(target);
        }
        // A minimum-length label may have less than one sample step of spare
        // room. Test the center between its cap clearances before sampling.
        let balanced = (0.5
            + (start_clearance - end_clearance) / (2. * self.arrow_length().max(0.001)))
        .clamp(0., 1.);
        let seed = if safe(balanced) {
            Some(balanced)
        } else {
            (0..=64).map(|i| i as f32 / 64.).find(|t| safe(*t))
        };
        let mut inside = seed?;
        let mut outside = target;
        for _ in 0..16 {
            let middle = (outside + inside) / 2.;
            if safe(middle) {
                inside = middle;
            } else {
                outside = middle;
            }
        }
        Some(inside)
    }
    pub(super) fn arrow_label_cutout(&self) -> Option<Rect> {
        if self.text.trim().is_empty() || !matches!(self.tool, Tool::Arrow | Tool::Line) {
            return None;
        }
        let b = self.label_bounds()?;
        Some(Rect {
            x: b.x - ARROW_PADDING * self.style.scale,
            y: b.y - ARROW_PADDING * self.style.scale,
            w: b.w + ARROW_PADDING * self.style.scale * 2.,
            h: b.h + ARROW_PADDING * self.style.scale * 2.,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn transparent_shape_labels_hit_letters_instead_of_the_whole_interior() {
        let mut mark = Mark::new(Tool::Rectangle, Pos::new(20., 20.), Style::default());
        mark.points[1] = Pos::new(280., 220.);
        mark.set_text("Hi\n\nHi      there".into());
        for rotation in [0., 0.5] {
            mark.rotation = rotation;
            let b = mark.label_edit_bounds().unwrap();
            let frame = mark.frame();
            assert!(
                !mark.hit(frame.to_world(Pos::new(b.x + 4., b.y + 4.)), 0.),
                "Empty label padding must not grab an unfocused transparent shape"
            );
            assert!(
                !mark.hit(
                    frame.to_world(Pos::new(
                        b.x + b.w / 2.,
                        b.y + mark.text_layout().line_height * 1.5
                    )),
                    0.
                ),
                "Blank lines must pass through"
            );
            let caret = mark.text_layout().caret(1, b.w, mark.text_align());
            assert!(
                mark.hit(
                    frame.to_world(Pos::new(
                        b.x + caret.x - 2.,
                        b.y + caret.y + mark.text_layout().line_height / 2.
                    )),
                    0.
                ),
                "Letters and their surrounding space must move the host"
            );
            let gap = mark.text_layout().caret(9, b.w, mark.text_align());
            assert!(
                !mark.hit(
                    frame.to_world(Pos::new(
                        b.x + gap.x,
                        b.y + gap.y + mark.text_layout().line_height / 2.
                    )),
                    0.
                ),
                "A long whitespace gap must pass through"
            );
        }
    }

    #[test]
    fn multiline_arrow_labels_keep_shaft_clearance_at_every_angle_and_when_shortened() {
        for tool in [Tool::Arrow, Tool::Line] {
            for cap in [Cap::None, Cap::Circle, Cap::Triangle, Cap::Arrow] {
                for angle in [0_f32, 30., 60., 90., 135., 210., 300.] {
                    for curved in [false, true] {
                        let mut mark = Mark::new(
                            tool,
                            Pos::new(200., 200.),
                            Style {
                                width: 5.,
                                start: cap,
                                end: cap,
                                ..Default::default()
                            },
                        );
                        mark.style.font_size = 28.;
                        let direction =
                            Pos::new(angle.to_radians().cos(), angle.to_radians().sin());
                        mark.points[1] =
                            Pos::new(200. + direction.x * 120., 200. + direction.y * 120.);
                        if curved {
                            mark.points.push(Pos::new(
                                200. + direction.x * 60. - direction.y * 80.,
                                200. + direction.y * 60. + direction.x * 80.,
                            ));
                        }
                        mark.label_position = 0.98;
                        mark.set_text(
                            "Hello, this is\nthe end\nof glory\nand I'm holding\nof\na\nwith you"
                                .into(),
                        );
                        for endpoint in [0, 1] {
                            let mut resized = mark.clone();
                            let fixed = resized.points[1 - endpoint];
                            resized.points[endpoint] = fixed.lerp(resized.points[endpoint], 0.05);
                            resized.constrain_arrow_label(Some(1 - endpoint));
                            assert_eq!(resized.points[1 - endpoint], fixed);
                            assert_eq!(resized.style.font_size, 28.);
                            let cutout = resized.arrow_label_cutout().unwrap();
                            for (tip, cap) in [
                                (resized.points[0], resized.style.start),
                                (resized.points[1], resized.style.end),
                            ] {
                                assert!(
                                    !cutout.contains(
                                        tip,
                                        resized.arrow_cap_clearance(cap)
                                            - ARROW_PADDING * resized.style.scale
                                    ),
                                    "{tool:?}, {cap:?}, {angle}, curved={curved}: label hides the shaft"
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn arrow_labels_wrap_between_words_using_cap_space_and_keep_explicit_newlines() {
        for tool in [Tool::Arrow, Tool::Line] {
            let mut mark = Mark::new(tool, Pos::new(0., 0.), Style::default());
            mark.style.font_size = 24.;
            mark.points[1] = Pos::new(160., 0.);
            mark.style.start = Cap::None;
            mark.style.end = Cap::None;
            mark.set_text("Hello world".into());
            let single = mark.layout_for(None);
            assert_eq!(mark.text_layout().height, single.height);
            if tool == Tool::Arrow {
                mark.style.width = 12.;
                mark.style.start = Cap::Triangle;
                mark.style.end = Cap::Triangle;
            } else {
                mark.points[1].x = 90.;
            }
            mark.set_text(mark.text.clone());
            assert_eq!(mark.text_layout().height, single.line_height * 2.);
            assert!(
                mark.text_layout().width
                    <= mark.arrow_length() - mark.arrow_label_clearance() + 0.01
            );
            mark.points[1].x = 900.;
            assert_eq!(
                mark.text_layout().height,
                single.height,
                "Lengthening must unwrap the label"
            );
            mark.set_text("Hello\nworld".into());
            assert_eq!(mark.text_layout().height, single.line_height * 2.);
        }
    }
    #[test]
    fn vertical_arrow_labels_keep_manual_lines_clear_of_the_caps_when_shortened() {
        for tool in [Tool::Arrow, Tool::Line] {
            let mut mark = Mark::new(tool, Pos::new(100., 100.), Style::default());
            mark.style.font_size = 28.;
            mark.points[1] = Pos::new(100., 110.);
            mark.set_text("Hello\nworld\nagain".into());
            let label = mark.label_bounds().unwrap();
            assert_eq!(
                mark.text_layout().height,
                mark.text_layout().line_height * 3.
            );
            for (tip, cap) in [
                (mark.points[0], mark.style.start),
                (mark.points[1], mark.style.end),
            ] {
                assert!(
                    !label.contains(tip, mark.arrow_cap_clearance(cap)),
                    "{tool:?}: manual lines overlap a cap"
                );
            }
        }
    }
    #[test]
    fn arrow_labels_preserve_whole_words_when_typing_or_shortening_past_their_minimum() {
        for tool in [Tool::Arrow, Tool::Line] {
            for cap in [Cap::None, Cap::Arrow, Cap::Triangle, Cap::Circle] {
                let mut mark = Mark::new(tool, Pos::new(100., 100.), Style::default());
                mark.style.font_size = 36.;
                mark.style.start = Cap::None;
                mark.style.end = cap;
                mark.set_text("Hello".into());
                assert_eq!(mark.text_layout().height, mark.layout_for(None).height);
                assert!(
                    mark.arrow_length()
                        >= mark.arrow_label_word_width() + mark.arrow_label_clearance() - 0.01
                );
                let original = mark.clone();
                for endpoint in [0, 1] {
                    let mut resized = original.clone();
                    let fixed = resized.points[1 - endpoint];
                    resized.points[endpoint] = fixed.lerp(resized.points[endpoint], 0.1);
                    resized.constrain_arrow_label(Some(1 - endpoint));
                    assert_eq!(resized.points[1 - endpoint], fixed);
                    assert_eq!(resized.style.font_size, original.style.font_size);
                    assert_eq!(
                        resized.text_layout().height,
                        resized.layout_for(None).height
                    );
                    let label = resized.label_bounds().unwrap();
                    for (tip, cap) in [
                        (resized.points[0], resized.style.start),
                        (resized.points[1], resized.style.end),
                    ] {
                        assert!(
                            !label.contains(tip, resized.arrow_cap_clearance(cap)),
                            "{tool:?}, {cap:?}: label overlaps a tip after resizing"
                        );
                    }
                }
            }
        }
    }
    #[test]
    fn arrow_labels_keep_short_words_on_one_line_when_the_shaft_has_space() {
        for tool in [Tool::Arrow, Tool::Line] {
            for size in [18., 20., 24., 28., 36., 44., 52., 64.] {
                for curved in [false, true] {
                    let mut mark = Mark::new(
                        tool,
                        Pos::new(100., 400.),
                        Style {
                            font_size: size,
                            ..Default::default()
                        },
                    );
                    mark.style.font_size = size;
                    mark.points[1] = Pos::new(1100., 500.);
                    if curved {
                        mark.points.push(Pos::new(350., -200.));
                    }
                    mark.text = "Hello".into();
                    let unwrapped = mark.layout_for(None);
                    assert_eq!(
                        mark.layout_for(Some(unwrapped.width.ceil() + 1.)).height,
                        unwrapped.height,
                        "Rounding the measured width up must leave the word intact"
                    );
                    assert_eq!(
                        mark.text_layout().height,
                        unwrapped.height,
                        "{tool:?}, font {size}, curved {curved}: a short word must not split"
                    );
                }
            }
        }
    }
    #[test]
    fn wrapping_uses_the_actual_shaped_width_of_kerned_text() {
        let single = Layout::get("AV AV", 44., None);
        let wrapped = Layout::get("AV AV", 44., Some(single.width.ceil() + 1.));
        assert_eq!(
            single.height, wrapped.height,
            "Text which fits at its shaped width must stay on one line"
        );
        let long_word = Layout::get(&"W".repeat(100), 22., Some(64.));
        assert!(
            long_word.width <= 64.1,
            "Long words must break rather than stretching the wrapping width"
        );
    }
    #[test]
    fn automatic_text_preserves_its_alignment_anchor_when_content_changes() {
        for align in [Align::Start, Align::Middle, Align::End] {
            for rotation in [0., 0.8] {
                let mut mark = Mark::new(
                    Tool::Text,
                    Pos::new(100., 100.),
                    Style {
                        text_align: Some(align),
                        ..Default::default()
                    },
                );
                mark.rotation = rotation;
                let anchor = |mark: &Mark| {
                    let b = mark.bounds();
                    mark.frame()
                        .to_world(Pos::new(b.x + align.offset(b.w), b.y))
                };
                let before = anchor(&mark);
                for text in ["A", "A longer line of text", "One\nTwo\nThree", "Short"] {
                    mark.set_text(text.into());
                    assert!(
                        anchor(&mark).distance(before) < 0.001,
                        "{align:?}, {rotation}, {text}"
                    );
                }
            }
        }
    }
    #[test]
    fn labels_keep_their_size_and_grow_then_release_extra_height() {
        for tool in SHAPES.into_iter().filter(|t| t.is_shape()) {
            let mut mark = Mark::new(tool, Pos::new(10., 10.), Style::default());
            mark.points[1] = Pos::new(170., 110.);
            let size = mark.text_font_size();
            mark.set_text("A label that wraps and keeps its chosen letter size. ".repeat(8));
            assert_eq!(mark.text_font_size(), size);
            assert!(mark.bounds().h > 100.);
            assert!(mark.label_path().is_some(), "{tool:?}");
            let small = mark.constrain_text_bounds(Rect {
                x: 10.,
                y: 10.,
                w: 10.,
                h: 10.,
            });
            assert!(small.h > 10. && small.w > 10.);
            mark.set_text(String::new());
            assert_eq!(mark.bounds().h, 100.);
        }
    }
    #[test]
    fn explicit_empty_lines_and_alignment_share_a_stable_line_box() {
        let mut mark = Mark::new(Tool::Text, Pos::default(), Style::default());
        mark.wrap = Some(220.);
        mark.text = "Wide text\nI\n".into();
        assert_eq!(mark.bounds().h, line_height(mark.style.font_size) * 3.);
        let mut lefts = Vec::new();
        for align in [Align::Start, Align::Middle, Align::End] {
            mark.style.text_align = Some(align);
            lefts.push(mark.standalone_path().unwrap().bounds().left());
        }
        assert!(lefts[0] < lefts[1] && lefts[1] < lefts[2]);
    }
}

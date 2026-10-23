pub mod comparison;
pub use comparison::Comparison;
mod geometry;
mod marker;
mod pixelation;
mod preview;
mod raster;
mod shapes;
mod stack;
mod text;
pub use geometry::Frame;
pub use preview::{PreviewCache, fill_icon, pixelate_icon, scribble_icon, shape_icon};
pub use shapes::SHAPES;
#[cfg(test)]
pub use text::line_height;
pub use text::{ARROW_TEXT_SIZES, Align, LABEL_SIZES, TEXT_SIZES};

use anyhow::{Context as _, Result, bail};
use std::sync::{Arc, Mutex, OnceLock};
use tiny_skia::{
    FillRule, LineCap, Paint, Path, PathBuilder, Pixmap, Stroke, StrokeDash, Transform,
};

pub const STROKE_WIDTHS: [f32; 4] = [2., 3.5, 5., 10.];
const CAP_LENGTH: f32 = 4.;
const CAP_ANGLE: f32 = 0.5;
const CAP_RADIUS: f32 = 1.8;

pub const COLORS: [[u8; 3]; 12] = [
    [244, 244, 245],
    [151, 156, 179],
    [224, 157, 242],
    [181, 62, 201],
    [78, 113, 255],
    [69, 169, 239],
    [255, 198, 52],
    [255, 112, 15],
    [8, 151, 113],
    [62, 194, 87],
    [255, 133, 143],
    [231, 48, 48],
];
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Fill {
    #[default]
    None,
    Soft,
    Solid,
    Hatch,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Dash {
    #[default]
    Solid,
    Dashed,
    Dotted,
}
impl Dash {
    fn pattern(self, width: f32) -> Option<StrokeDash> {
        match self {
            Self::Solid => None,
            Self::Dashed => StrokeDash::new(vec![width * 3., width * 2.], 0.),
            Self::Dotted => StrokeDash::new(vec![0.01, width * 2.5], 0.),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Pos {
    pub x: f32,
    pub y: f32,
}
impl Pos {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
    pub fn distance(self, b: Self) -> f32 {
        (self.x - b.x).hypot(self.y - b.y)
    }
    pub fn lerp(self, b: Self, t: f32) -> Self {
        Self::new(self.x + (b.x - self.x) * t, self.y + (b.y - self.y) * t)
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}
impl Rect {
    pub fn between(a: Pos, b: Pos) -> Self {
        Self {
            x: a.x.min(b.x),
            y: a.y.min(b.y),
            w: (a.x - b.x).abs().max(1.),
            h: (a.y - b.y).abs().max(1.),
        }
    }
    pub fn contains(self, p: Pos, pad: f32) -> bool {
        p.x >= self.x - pad
            && p.x <= self.x + self.w + pad
            && p.y >= self.y - pad
            && p.y <= self.y + self.h + pad
    }
    pub fn corners(self) -> [Pos; 4] {
        [
            Pos::new(self.x, self.y),
            Pos::new(self.x + self.w, self.y),
            Pos::new(self.x + self.w, self.y + self.h),
            Pos::new(self.x, self.y + self.h),
        ]
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Tool {
    #[default]
    Select,
    Pan,
    Text,
    Marker,
    Rectangle,
    Ellipse,
    Diamond,
    Triangle,
    Star,
    Hexagon,
    Rhombus,
    Oval,
    ArrowLeft,
    ArrowUp,
    ArrowDown,
    ArrowRight,
    Cloud,
    XBox,
    CheckBox,
    Heart,
    Arrow,
    Line,
    Pen,
    Redact,
    Eraser,
}
#[derive(Clone, Copy, Debug)]
pub enum StackOrder {
    Front,
    Forward,
    Backward,
    Back,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Cap {
    None,
    #[default]
    Arrow,
    Triangle,
    Circle,
}
impl Cap {
    pub fn next(self) -> Self {
        match self {
            Self::None => Self::Arrow,
            Self::Arrow => Self::Triangle,
            Self::Triangle => Self::Circle,
            Self::Circle => Self::None,
        }
    }
}
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Style {
    /// Image units per annotation unit, fixed when the object is created.
    pub scale: f32,
    pub color: usize,
    pub fill: Fill,
    pub opacity: f32,
    pub width: f32,
    pub dash: Dash,
    pub start: Cap,
    pub end: Cap,
    pub font_size: f32,
    pub text_align: Option<Align>,
    pub vertical_align: Align,
}
impl Default for Style {
    fn default() -> Self {
        Self {
            scale: 1.,
            color: 0,
            fill: Fill::None,
            opacity: 1.,
            width: STROKE_WIDTHS[1],
            dash: Dash::Solid,
            start: Cap::None,
            end: Cap::Arrow,
            font_size: TEXT_SIZES[1],
            text_align: None,
            vertical_align: Align::Middle,
        }
    }
}
impl Style {
    /// Convert preset dimensions without changing their meaning for this object.
    pub fn with_scale(mut self, scale: f32) -> Self {
        let ratio = scale / self.scale;
        self.width *= ratio;
        self.font_size *= ratio;
        self.scale = scale;
        self
    }
}
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Mark {
    #[serde(default)]
    pub number: Option<u32>,
    #[serde(default)]
    pub description: String,
    pub tool: Tool,
    pub points: Vec<Pos>,
    pub text: String,
    pub wrap: Option<f32>,
    pub style: Style,
    pub rotation: f32,
    pub label_position: f32,
    pub label_base_height: Option<f32>,
    pub flip_x: bool,
    pub flip_y: bool,
}
impl Mark {
    pub fn new(tool: Tool, p: Pos, mut style: Style) -> Self {
        if tool.is_shape() || matches!(tool, Tool::Arrow | Tool::Line) {
            let tier = STROKE_WIDTHS
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    (style.width / style.scale - **a)
                        .abs()
                        .total_cmp(&(style.width / style.scale - **b).abs())
                })
                .unwrap()
                .0;
            style.font_size = style.scale
                * if tool.is_shape() {
                    LABEL_SIZES[tier]
                } else {
                    ARROW_TEXT_SIZES[tier]
                };
        }
        if tool == Tool::Marker {
            // A sequence badge stays legible even after using a translucent drawing tool.
            style.opacity = 1.;
        }
        if tool == Tool::Line {
            style.start = Cap::None;
            style.end = Cap::None;
        }
        Self {
            number: None,
            description: String::new(),
            tool,
            points: vec![p, p],
            text: String::new(),
            wrap: None,
            style,
            rotation: 0.,
            label_position: 0.5,
            label_base_height: None,
            flip_x: false,
            flip_y: false,
        }
    }
    pub fn bounds(&self) -> Rect {
        if self.tool == Tool::Text {
            let layout = self.text_layout();
            return Rect {
                x: self.points[0].x,
                y: self.points[0].y,
                w: self
                    .wrap
                    .unwrap_or(layout.width + self.style.scale)
                    .max(16. * self.style.scale),
                h: layout.height,
            };
        }
        if self.tool == Tool::Marker {
            let r = self.marker_radius();
            return Rect {
                x: self.points[0].x - r,
                y: self.points[0].y - r,
                w: r * 2.,
                h: r * 2.,
            };
        }
        let mut a = self.points[0];
        let mut b = a;
        for p in &self.points {
            a.x = a.x.min(p.x);
            a.y = a.y.min(p.y);
            b.x = b.x.max(p.x);
            b.y = b.y.max(p.y);
        }
        Rect::between(a, b)
    }
    pub fn translate(&mut self, dx: f32, dy: f32) {
        for p in &mut self.points {
            p.x += dx;
            p.y += dy;
        }
    }
    pub fn frame(&self) -> Frame {
        Frame {
            bounds: self.bounds(),
            rotation: self.rotation,
        }
    }
    pub fn world_bounds(&self) -> Rect {
        let frame = self.frame();
        let b = frame.world_bounds();
        if !self.text.is_empty()
            && let Some(label) = self.label_bounds()
        {
            b.union(frame.map_bounds(label))
        } else {
            b
        }
    }
    /// Full visible extent for placing copies beside each other, including caps
    /// and stroke thickness on vertical lines whose geometric width is zero.
    pub fn visual_bounds(&self) -> Rect {
        let frame = self.frame();
        let map = |r: tiny_skia::Rect| {
            frame.map_bounds(Rect {
                x: r.x(),
                y: r.y(),
                w: r.width(),
                h: r.height(),
            })
        };
        let mut bounds = if matches!(self.tool, Tool::Arrow | Tool::Line) {
            self.body_path()
                .map_or_else(|| self.world_bounds(), |p| map(p.bounds()))
        } else {
            self.world_bounds()
        };
        if self.tool == Tool::Arrow {
            for (tip, toward, cap) in [
                (
                    self.points[0],
                    self.points.get(2).copied().unwrap_or(self.points[1]),
                    self.style.start,
                ),
                (
                    self.points[1],
                    self.points.get(2).copied().unwrap_or(self.points[0]),
                    self.style.end,
                ),
            ] {
                if let Some(path) = cap_path(tip, toward, cap, self.style.width) {
                    bounds = bounds.union(map(path.bounds()));
                }
            }
        }
        if let Some(label) = self.label_bounds().filter(|_| !self.text.is_empty()) {
            bounds = bounds.union(frame.map_bounds(label));
        }
        let text_edge = if self.text.is_empty() {
            0.
        } else {
            (self.text_font_size() / 24.).max(self.style.scale)
        };
        let padding = if self.tool == Tool::Text {
            text_edge
        } else {
            (self.style.width / 2.).max(text_edge)
        };
        Rect {
            x: bounds.x - padding,
            y: bounds.y - padding,
            w: bounds.w + padding * 2.,
            h: bounds.h + padding * 2.,
        }
    }
    pub fn contains_body(&self, p: Pos) -> bool {
        let p = self.frame().to_local(p);
        if !self.bounds().contains(p, 0.) {
            return false;
        }
        if self.tool.is_shape() {
            return self
                .body_path()
                .is_some_and(|path| point_inside(p, &shapes::edges(&path)));
        }
        matches!(self.tool, Tool::Text | Tool::Pen)
    }
    pub fn resize(&mut self, old: Rect, new: Rect, wrap_only: bool) {
        if self.tool == Tool::Marker {
            let ratio = (new.w / old.w).max(0.05);
            self.style = self.style.clone().with_scale(self.style.scale * ratio);
            self.points = vec![Pos::new(new.x + new.w / 2., new.y + new.w / 2.); 2];
            self.rotation = 0.;
        } else if self.tool == Tool::Text {
            self.points[0] = Pos::new(new.x, new.y);
            if wrap_only {
                self.wrap = Some(new.w.max(12. * self.style.scale));
            } else {
                let ratio = (new.w / old.w).max(0.05);
                let font_size = (self.style.font_size * ratio).clamp(1., 4096.);
                let ratio = font_size / self.style.font_size;
                self.style = self.style.clone().with_scale(self.style.scale * ratio);
                self.wrap = self.wrap.map(|w| w * ratio);
            }
        } else {
            let new = self.constrain_text_bounds(new);
            for p in &mut self.points {
                p.x = new.x + (p.x - old.x) / old.w * new.w;
                p.y = new.y + (p.y - old.y) / old.h * new.h;
            }
            if self.tool.is_shape() {
                self.label_base_height = Some(new.h);
            }
        }
    }
    pub fn arrow_mid(&self) -> Pos {
        if self.points.len() > 2 {
            self.points[0]
                .lerp(self.points[2], 0.5)
                .lerp(self.points[2].lerp(self.points[1], 0.5), 0.5)
        } else {
            self.points[0].lerp(self.points[1], 0.5)
        }
    }
    pub fn bend_handle(&self) -> Pos {
        if self.text.is_empty() {
            return self.arrow_mid();
        }
        let t = 0.25;
        let a = self.points[0];
        let b = self.points[1];
        let c = self.points.get(2).copied().unwrap_or(a.lerp(b, 0.5));
        a.lerp(c, t).lerp(c.lerp(b, t), t)
    }
    pub fn is_line_dot(&self) -> bool {
        self.tool == Tool::Line && self.points.iter().all(|p| *p == self.points[0])
    }
    pub fn hit(&self, p: Pos, tolerance: f32) -> bool {
        if self.hit_text(p, tolerance) {
            return true;
        }
        let p = self.frame().to_local(p);
        let b = self.bounds();
        if !b.contains(p, tolerance + self.style.width) {
            return false;
        }
        if self.tool.is_shape() {
            let Some(path) = self.path() else {
                return false;
            };
            let edges = shapes::edges(&path);
            let stroke_hit = edges
                .iter()
                .any(|(a, b)| segment_distance(p, *a, *b) < tolerance + self.style.width / 2.);
            let inside = self
                .body_path()
                .is_some_and(|path| point_inside(p, &shapes::edges(&path)));
            return stroke_hit || (self.style.fill != Fill::None && inside);
        }
        match self.tool {
            Tool::Marker => self.points[0].distance(p) <= self.marker_radius() + tolerance,
            Tool::Text => false,
            Tool::Redact => false,
            Tool::Arrow | Tool::Line => {
                let mut previous = self.points[0];
                (1..=32).any(|i| {
                    let t = i as f32 / 32.;
                    let next = if self.points.len() > 2 {
                        self.points[0]
                            .lerp(self.points[2], t)
                            .lerp(self.points[2].lerp(self.points[1], t), t)
                    } else {
                        self.points[0].lerp(self.points[1], t)
                    };
                    let hit =
                        segment_distance(p, previous, next) < tolerance + self.style.width / 2.;
                    previous = next;
                    hit
                })
            }
            Tool::Pen => self
                .points
                .windows(2)
                .any(|s| segment_distance(p, s[0], s[1]) < tolerance + self.style.width / 2.),
            _ => false,
        }
    }
    /// Body geometry for filling and stroking, excluding independently painted box symbols.
    fn body_path(&self) -> Option<Path> {
        if self.is_line_dot() {
            if !self.text.trim().is_empty() {
                return None;
            }
            PathBuilder::from_circle(self.points[0].x, self.points[0].y, self.style.width / 2.)
        } else if matches!(self.tool, Tool::XBox | Tool::CheckBox) {
            shapes::shape_path(Tool::Rectangle, self.bounds(), self.style.width * 2.)
        } else {
            self.path()
        }
    }
    fn path(&self) -> Option<Path> {
        if matches!(self.tool, Tool::Arrow | Tool::Line) {
            return arrow_shaft(self);
        }
        let mut p = PathBuilder::new();
        let b = self.bounds();
        if self.tool.is_shape() {
            return self.shape_geometry(shapes::shape_path(self.tool, b, self.style.width * 2.)?);
        }
        match self.tool {
            Tool::Redact => return None,
            Tool::Text => {
                return self.standalone_path();
            }
            _ => {
                p.move_to(self.points[0].x, self.points[0].y);
                if self.tool == Tool::Arrow && self.points.len() > 2 {
                    p.quad_to(
                        self.points[2].x,
                        self.points[2].y,
                        self.points[1].x,
                        self.points[1].y,
                    );
                } else {
                    for point in &self.points[1..] {
                        p.line_to(point.x, point.y);
                    }
                }
            }
        }
        p.finish()
    }
}
fn point_inside(p: Pos, edges: &[(Pos, Pos)]) -> bool {
    edges
        .iter()
        .filter(|(a, b)| {
            (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x
        })
        .count()
        % 2
        == 1
}
fn arrow_shaft(mark: &Mark) -> Option<Path> {
    let a = mark.points[0];
    let b = mark.points[1];
    let c = mark.points.get(2).copied().unwrap_or(a.lerp(b, 0.5));
    let at = |t| a.lerp(c, t).lerp(c.lerp(b, t), t);
    let mut lengths = [0.; 65];
    let mut previous = a;
    for i in 1..=64 {
        let p = at(i as f32 / 64.);
        lengths[i] = lengths[i - 1] + previous.distance(p);
        previous = p;
    }
    // A round shaft end must sit inside a filled triangle, never at its pointed vertex.
    let inset = |cap| {
        if cap == Cap::Triangle && mark.tool == Tool::Arrow {
            mark.style.width * 1.25
        } else {
            0.
        }
    };
    let start = inset(mark.style.start);
    let end = inset(mark.style.end);
    let fit = (lengths[64] * 0.95 / (start + end).max(0.001)).min(1.);
    let parameter = |distance: f32| {
        let i = (1..=64).find(|i| lengths[*i] >= distance).unwrap_or(64);
        ((i - 1) as f32 + (distance - lengths[i - 1]) / (lengths[i] - lengths[i - 1]).max(0.001))
            / 64.
    };
    let t0 = parameter(start * fit);
    let t1 = parameter(lengths[64] - end * fit);
    let from = at(t0);
    let to = at(t1);
    let control = Pos::new(
        from.x + (t1 - t0) * ((1. - t0) * (c.x - a.x) + t0 * (b.x - c.x)),
        from.y + (t1 - t0) * ((1. - t0) * (c.y - a.y) + t0 * (b.y - c.y)),
    );
    let mut path = PathBuilder::new();
    if let Some(cutout) = mark.arrow_label_cutout() {
        let mut previous = from;
        let mut last = None::<Pos>;
        for i in 1..=128 {
            let next = at(t0 + (t1 - t0) * i as f32 / 128.);
            let dx = next.x - previous.x;
            let dy = next.y - previous.y;
            let mut enter = 0f32;
            let mut exit = 1f32;
            let pad = mark.style.width / 2.;
            let mut intersects = true;
            for (origin, delta, low, high) in [
                (previous.x, dx, cutout.x - pad, cutout.x + cutout.w + pad),
                (previous.y, dy, cutout.y - pad, cutout.y + cutout.h + pad),
            ] {
                if delta.abs() < 0.0001 {
                    intersects &= origin >= low && origin <= high;
                } else {
                    let a = (low - origin) / delta;
                    let b = (high - origin) / delta;
                    enter = enter.max(a.min(b));
                    exit = exit.min(a.max(b));
                }
            }
            let mut segment = |a: Pos, b: Pos| {
                if a.distance(b) > 0.0001 {
                    if last.is_none_or(|last| last.distance(a) > 0.0001) {
                        path.move_to(a.x, a.y);
                    }
                    path.line_to(b.x, b.y);
                    last = Some(b);
                }
            };
            if intersects && enter < exit {
                segment(previous, previous.lerp(next, enter));
                segment(previous.lerp(next, exit), next);
            } else {
                segment(previous, next);
            }
            previous = next;
        }
        return path.finish();
    }
    path.move_to(from.x, from.y);
    path.quad_to(control.x, control.y, to.x, to.y);
    path.finish()
}
fn segment_distance(p: Pos, a: Pos, b: Pos) -> f32 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let t = ((p.x - a.x) * dx + (p.y - a.y) * dy) / (dx * dx + dy * dy).max(0.001);
    p.distance(a.lerp(b, t.clamp(0., 1.)))
}

/// Shares the original image and edit history within an editing session.
/// Save and clean close retain it; Discard restores its saved editable marks.
/// Staged image bytes remain independent.
#[derive(Clone, Default)]
pub struct Session(Arc<Mutex<Option<RetainedDocument>>>);
pub struct RetainedDocument {
    pub saved_next_marker: u32,
    pub saved_comparison: Option<Comparison>,
    pub document: Document,
    pub image: Arc<gpui_kit::Image>,
    pub saved_marks: Vec<Mark>,
}
impl Session {
    pub fn open(&self, bytes: &[u8], image: Arc<gpui_kit::Image>) -> Result<RetainedDocument> {
        let stored = self
            .0
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take();
        stored.map_or_else(
            || {
                Ok(RetainedDocument {
                    document: Document::decode(bytes)?,
                    image,
                    saved_next_marker: 1,
                    saved_marks: Vec::new(),
                    saved_comparison: None,
                })
            },
            Ok,
        )
    }
    pub(crate) fn retain(&self, document: Document, image: Arc<gpui_kit::Image>, saved: EditState) {
        *self.0.lock().unwrap_or_else(|error| error.into_inner()) = Some(RetainedDocument {
            document,
            image,
            saved_next_marker: saved.next_marker,
            saved_marks: saved.marks,
            saved_comparison: saved.comparison,
        });
    }
}

#[derive(Clone)]
pub struct Document {
    pub next_marker: u32,
    pub comparison: Option<Comparison>,
    pub base: Arc<image::RgbaImage>,
    pub marks: Vec<Mark>,
    /// Preset unit established from the first fitted view, retained across saves.
    pub annotation_scale: OnceLock<f32>,
    undo: Vec<EditState>,
    redo: Vec<EditState>,
    mosaic: Arc<OnceLock<Vec<[u8; 3]>>>,
}
#[derive(Clone)]
pub(crate) struct EditState {
    pub next_marker: u32,
    pub marks: Vec<Mark>,
    pub comparison: Option<Comparison>,
}
impl Document {
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        Ok(Self::from_image(image::load_from_memory(bytes)?.to_rgba8()))
    }
    pub(crate) fn from_image(image: image::RgbaImage) -> Self {
        Self {
            base: Arc::new(image),
            marks: Vec::new(),
            annotation_scale: OnceLock::new(),
            next_marker: 1,
            comparison: None,
            undo: Vec::new(),
            redo: Vec::new(),
            mosaic: Arc::default(),
        }
    }
    pub fn snapshot(&self) -> Self {
        Self {
            base: self.base.clone(),
            marks: self.marks.clone(),
            annotation_scale: self.annotation_scale.clone(),
            next_marker: self.next_marker,
            comparison: self.comparison.clone(),
            undo: Vec::new(),
            redo: Vec::new(),
            mosaic: self.mosaic.clone(),
        }
    }
    fn mosaic_colors(&self) -> &[[u8; 3]] {
        self.mosaic.get_or_init(|| {
            let mut colors = Vec::new();
            for y in (0..self.base.height()).step_by(16) {
                for x in (0..self.base.width()).step_by(16) {
                    let mut sum = [0u32; 3];
                    let mut count = 0;
                    for dy in [1, 5, 9, 13] {
                        for dx in [1, 5, 9, 13] {
                            let p = self.base.get_pixel(
                                (x + dx).min(self.base.width() - 1),
                                (y + dy).min(self.base.height() - 1),
                            );
                            for c in 0..3 {
                                sum[c] += u32::from(p[c]) * u32::from(p[3]) / 255;
                            }
                            count += 1;
                        }
                    }
                    colors.push([
                        (sum[0] / count) as u8,
                        (sum[1] / count) as u8,
                        (sum[2] / count) as u8,
                    ]);
                }
            }
            colors
        })
    }
    pub fn checkpoint(&mut self) {
        self.undo.push(self.edit_state());
        self.redo.clear();
    }
    #[cfg(test)]
    pub fn revert(&mut self) {
        self.next_marker = 1;
        self.marks.clear();
        self.undo.clear();
        self.redo.clear();
    }
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
    pub(crate) fn edit_state(&self) -> EditState {
        EditState {
            next_marker: self.next_marker,
            marks: self.marks.clone(),
            comparison: self.comparison.clone(),
        }
    }
    fn restore_state(&mut self, state: EditState) {
        self.next_marker = state.next_marker;
        self.marks = state.marks;
        self.comparison = state.comparison;
    }
    pub fn undo(&mut self) {
        if let Some(state) = self.undo.pop() {
            self.redo.push(self.edit_state());
            self.restore_state(state);
        }
    }
    pub fn redo(&mut self) {
        if let Some(state) = self.redo.pop() {
            self.undo.push(self.edit_state());
            self.restore_state(state);
        }
    }
    pub fn hit(&self, p: Pos, tolerance: f32) -> Option<usize> {
        self.marks
            .iter()
            .rposition(|mark| mark.tool != Tool::Redact && mark.hit(p, tolerance))
    }
    fn stack_target(&self, index: usize, order: StackOrder) -> Option<usize> {
        let selected = self.marks.get(index)?;
        if selected.tool == Tool::Redact {
            return None;
        }
        let same_layer = |i: usize| self.marks[i].tool != Tool::Redact;
        let overlaps = |i: usize| {
            let a = selected.world_bounds();
            let b = self.marks[i].world_bounds();
            a.x <= b.x + b.w && a.x + a.w >= b.x && a.y <= b.y + b.h && a.y + a.h >= b.y
        };
        match order {
            StackOrder::Front => (0..self.marks.len()).rfind(|i| same_layer(*i)),
            StackOrder::Back => (0..self.marks.len()).find(|i| same_layer(*i)),
            StackOrder::Forward => {
                (index + 1..self.marks.len()).find(|i| same_layer(*i) && overlaps(*i))
            }
            StackOrder::Backward => (0..index).rev().find(|i| same_layer(*i) && overlaps(*i)),
        }
    }
    pub fn can_reorder(&self, index: usize, order: StackOrder) -> bool {
        self.stack_target(index, order)
            .is_some_and(|target| target != index)
    }
    pub fn reorder(&mut self, index: usize, order: StackOrder) -> usize {
        let Some(target) = self
            .stack_target(index, order)
            .filter(|target| *target != index)
        else {
            return index;
        };
        self.checkpoint();
        let mark = self.marks.remove(index);
        self.marks.insert(target, mark);
        target
    }
    pub fn bounds(&self) -> Rect {
        let mut x = 0f32;
        let mut y = 0f32;
        let mut right = self.base.width() as f32;
        let mut bottom = self.base.height() as f32;
        if let Some(comparison) = &self.comparison {
            let band = comparison.caption_band();
            right = right.max(band.x + band.w);
            bottom = bottom.max(band.y + band.h + 16.);
        }
        for mark in &self.marks {
            if mark.tool == Tool::Redact {
                continue;
            }
            if mark.tool == Tool::Text && mark.text.is_empty() {
                continue;
            }
            let b = mark.world_bounds();
            let padding = if mark.tool == Tool::Text {
                (mark.style.font_size / 12.).max(4.)
            } else if mark.tool == Tool::Arrow {
                mark.style.width * 5.
            } else {
                mark.style.width.max(4.)
            };
            x = x.min(b.x - padding);
            y = y.min(b.y - padding);
            right = right.max(b.x + b.w + padding);
            bottom = bottom.max(b.y + b.h + padding);
        }
        Rect {
            x: x.floor(),
            y: y.floor(),
            w: right.ceil() - x.floor(),
            h: bottom.ceil() - y.floor(),
        }
    }
    pub fn png(&self, max_edge: Option<f32>) -> Result<(Vec<u8>, Rect)> {
        if self.comparison.is_some() && max_edge.is_none() {
            let (bytes, _, bounds) = self.comparison_png(comparison::SAVE_PNG_BUDGET)?;
            return Ok((bytes, bounds));
        }
        let (pixels, bounds) = self.render(max_edge)?;
        Ok((pixels.encode_png()?, bounds))
    }

    /// Render from the retained original on every attempt. Compressible screenshots
    /// keep 4K detail; noisy photos shrink only when their actual PNG requires it.
    pub(crate) fn comparison_png(&self, budget: usize) -> Result<(Vec<u8>, (u32, u32), Rect)> {
        use image::ImageEncoder as _;
        let mut limit = (comparison::MAX_WIDTH, comparison::MAX_HEIGHT);
        for _ in 0..12 {
            let (mut pixels, bounds) = self.render_within(limit.0, limit.1)?;
            let size = (pixels.width(), pixels.height());
            // Transfer the renderer buffer without cloning it; PNG stores straight
            // alpha, whereas the rasterizer uses premultiplied channels.
            for pixel in pixels.data_mut().as_chunks_mut::<4>().0 {
                if pixel[3] != 255 {
                    let color = tiny_skia::PremultipliedColorU8::from_rgba(
                        pixel[0], pixel[1], pixel[2], pixel[3],
                    )
                    .context("Invalid comparison pixels")?
                    .demultiply();
                    pixel.copy_from_slice(&[
                        color.red(),
                        color.green(),
                        color.blue(),
                        color.alpha(),
                    ]);
                }
            }
            let mut bytes = Vec::new();
            image::codecs::png::PngEncoder::new_with_quality(
                &mut bytes,
                image::codecs::png::CompressionType::Default,
                image::codecs::png::FilterType::Adaptive,
            )
            .write_image(
                &pixels.take(),
                size.0,
                size.1,
                image::ExtendedColorType::Rgba8,
            )?;
            if bytes.len() <= budget {
                return Ok((bytes, size, bounds));
            }
            // Pixel count is proportional to scale squared. Margin avoids repeated
            // near-limit encodes when filtering changes the compression ratio.
            let factor = ((budget as f64 / bytes.len() as f64).sqrt() * 0.95).min(0.9);
            limit = (
                ((size.0 as f64 * factor).floor() as u32).max(1),
                ((size.1 as f64 * factor).floor() as u32).max(1),
            );
        }
        bail!("Could not fit the comparison within its PNG size budget.")
    }

    /// Canonical export pixels, including pixelation and expanded annotation bounds.
    fn render(&self, max_edge: Option<f32>) -> Result<(Pixmap, Rect)> {
        let bounds = self.bounds();
        let scale = max_edge.map_or(1., |max| (max / bounds.w.max(bounds.h)).min(1.));
        let w = (bounds.w * scale).ceil() as u32;
        let h = (bounds.h * scale).ceil() as u32;
        self.render_at_scale(bounds, scale, (w, h))
    }

    /// Fit the entire edited document inside a rectangle without upscaling.
    /// Render at the target size rather than allocating a full-resolution export.
    fn render_within(&self, max_width: u32, max_height: u32) -> Result<(Pixmap, Rect)> {
        let bounds = self.bounds();
        let scale = (f64::from(max_width) / f64::from(bounds.w))
            .min(f64::from(max_height) / f64::from(bounds.h))
            .min(1.);
        let w = (f64::from(bounds.w) * scale)
            .ceil()
            .min(f64::from(max_width)) as u32;
        let h = (f64::from(bounds.h) * scale)
            .ceil()
            .min(f64::from(max_height)) as u32;
        self.render_at_scale(bounds, scale as f32, (w, h))
    }

    fn render_at_scale(
        &self,
        bounds: Rect,
        scale: f32,
        (w, h): (u32, u32),
    ) -> Result<(Pixmap, Rect)> {
        if w == 0 || h == 0 || w > 16384 || h > 16384 || u64::from(w) * u64::from(h) > 64_000_000 {
            bail!(
                "The annotated image is too large to save (maximum 64 megapixels and 16,384 pixels per side)."
            );
        }
        let source = if scale < 1. {
            std::borrow::Cow::Owned(image::imageops::resize(
                self.base.as_ref(),
                (self.base.width() as f32 * scale).ceil().max(1.) as u32,
                (self.base.height() as f32 * scale).ceil().max(1.) as u32,
                if self.comparison.is_some() {
                    image::imageops::FilterType::Lanczos3
                } else {
                    image::imageops::FilterType::Triangle
                },
            ))
        } else {
            std::borrow::Cow::Borrowed(self.base.as_ref())
        };
        let mut base =
            Pixmap::new(source.width(), source.height()).context("Could not allocate image")?;
        for (target, pixel) in base
            .data_mut()
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .zip(source.pixels())
        {
            let a = u16::from(pixel[3]);
            for c in 0..3 {
                target[c] = (u16::from(pixel[c]) * a / 255) as u8;
            }
            target[3] = pixel[3];
        }
        // Reuse the premultiplied source allocation when the content fits it.
        let mut pixmap =
            if bounds.x == 0. && bounds.y == 0. && w == base.width() && h == base.height() {
                base
            } else {
                let mut pixels = Pixmap::new(w, h).context("Could not allocate annotated image")?;
                pixels.draw_pixmap(
                    (-bounds.x * scale).round() as i32,
                    (-bounds.y * scale).round() as i32,
                    base.as_ref(),
                    &tiny_skia::PixmapPaint::default(),
                    Transform::identity(),
                    None,
                );
                pixels
            };
        let transform =
            Transform::from_row(scale, 0., 0., scale, -bounds.x * scale, -bounds.y * scale);
        pixelation::export(self, &mut pixmap, bounds, scale);
        for mark in self.marks.iter().filter(|mark| mark.tool != Tool::Redact) {
            let transform = mark.frame().transform().post_concat(transform);
            mark.paint_onto(&mut pixmap, transform)?;
        }
        if let Some(comparison) = &self.comparison {
            comparison.paint(&mut pixmap, transform)?;
        }
        Ok((pixmap, bounds))
    }
}
impl Mark {
    /// Composite object opacity once, on a surface bounded by the caller.
    pub(crate) fn paint_onto(&self, target: &mut Pixmap, transform: Transform) -> Result<()> {
        let mut layer = Pixmap::new(target.width(), target.height())
            .context("Could not allocate annotation")?;
        self.paint_layer(&mut layer, transform);
        target.draw_pixmap(
            0,
            0,
            layer.as_ref(),
            &tiny_skia::PixmapPaint {
                opacity: self.style.opacity,
                ..Default::default()
            },
            Transform::identity(),
            None,
        );
        Ok(())
    }

    /// Paint all parts at full object opacity; callers composite the completed layer once.
    fn paint_layer(&self, layer: &mut Pixmap, transform: Transform) {
        if self.tool == Tool::Marker {
            self.paint_marker(layer, transform);
            return;
        }
        let path = self.body_path();
        let mut paint = Paint::default();
        let color = COLORS[self.style.color];
        paint.set_color_rgba8(color[0], color[1], color[2], 255);
        let mut stroke = Stroke {
            width: self.style.width,
            line_cap: LineCap::Round,
            line_join: tiny_skia::LineJoin::Round,
            ..Stroke::default()
        };
        stroke.dash = self.style.dash.pattern(self.style.width);
        if self.tool == Tool::Text {
            if let Some(path) = &path {
                let mut outline = Paint::default();
                let edge = if u32::from(color[0]) + u32::from(color[1]) + u32::from(color[2]) < 280
                {
                    255
                } else {
                    0
                };
                outline.set_color_rgba8(edge, edge, edge, 255);
                layer.stroke_path(
                    path,
                    &outline,
                    &Stroke {
                        width: (self.style.font_size / 12.).max(2. * self.style.scale),
                        line_join: tiny_skia::LineJoin::Round,
                        ..Stroke::default()
                    },
                    transform,
                    None,
                );
                layer.fill_path(path, &paint, FillRule::Winding, transform, None);
            }
        } else if self.is_line_dot() {
            if let Some(path) = &path {
                layer.fill_path(path, &paint, FillRule::Winding, transform, None);
            }
        } else {
            if self.tool.is_shape() && self.style.fill != Fill::None {
                let body = self.body_path().unwrap();
                let mut fill_paint = paint.clone();
                let alpha = match self.style.fill {
                    Fill::Solid => 255,
                    _ => 40,
                };
                let fill = self.style.fill_color();
                fill_paint.set_color_rgba8(fill[0], fill[1], fill[2], alpha);
                layer.fill_path(&body, &fill_paint, FillRule::Winding, transform, None);
                if self.style.fill == Fill::Hatch
                    && let Some(hatch) = shapes::hatch_path(&body, self.bounds(), self.style.scale)
                {
                    fill_paint.set_color_rgba8(color[0], color[1], color[2], 150);
                    layer.stroke_path(
                        &hatch,
                        &fill_paint,
                        &Stroke {
                            width: (self.style.width * 0.4).max(self.style.scale),
                            ..Default::default()
                        },
                        transform,
                        None,
                    );
                }
            }
            if let Some(path) = &path {
                layer.stroke_path(path, &paint, &stroke, transform, None);
            }
            if self.tool == Tool::Arrow {
                let control = self.points.get(2).copied();
                draw_cap(
                    layer,
                    self.points[0],
                    control.unwrap_or(self.points[1]),
                    self.style.start,
                    &paint,
                    self.style.width,
                    transform,
                );
                draw_cap(
                    layer,
                    self.points[1],
                    control.unwrap_or(self.points[0]),
                    self.style.end,
                    &paint,
                    self.style.width,
                    transform,
                );
            }
        }
        if let Some(symbol) = self.box_symbol() {
            layer.fill_path(&symbol, &paint, FillRule::Winding, transform, None);
        }
        if let Some(label) = self.label_path() {
            let color = self.label_color();
            let edge = if color.iter().map(|c| u32::from(*c)).sum::<u32>() < 280 {
                255
            } else {
                0
            };
            let mut ink = Paint::default();
            ink.set_color_rgba8(edge, edge, edge, 255);
            layer.stroke_path(
                &label,
                &ink,
                &Stroke {
                    width: (self.text_font_size() / 12.).max(2. * self.style.scale),
                    line_join: tiny_skia::LineJoin::Round,
                    ..Default::default()
                },
                transform,
                None,
            );
            ink.set_color_rgba8(color[0], color[1], color[2], 255);
            layer.fill_path(&label, &ink, FillRule::Winding, transform, None);
        }
        self.paint_bitmap_text(layer, transform);
    }
}

fn cap_path(tip: Pos, toward: Pos, cap: Cap, width: f32) -> Option<Path> {
    if cap == Cap::None {
        return None;
    }
    let angle = (tip.y - toward.y).atan2(tip.x - toward.x);
    let length = width * CAP_LENGTH;
    let mut p = PathBuilder::new();
    if cap == Cap::Circle {
        p.push_circle(tip.x, tip.y, width * CAP_RADIUS);
    } else {
        // Filled caps must wind clockwise, matching the shaft's stroked outline.
        // Opposite contours cancel where they overlap under nonzero filling.
        let wing = if cap == Cap::Triangle {
            CAP_ANGLE
        } else {
            -CAP_ANGLE
        };
        p.move_to(
            tip.x - length * (angle + wing).cos(),
            tip.y - length * (angle + wing).sin(),
        );
        p.line_to(tip.x, tip.y);
        p.line_to(
            tip.x - length * (angle - wing).cos(),
            tip.y - length * (angle - wing).sin(),
        );
        if cap == Cap::Triangle {
            p.close();
        }
    }
    p.finish()
}
fn draw_cap(
    pixmap: &mut Pixmap,
    tip: Pos,
    toward: Pos,
    cap: Cap,
    paint: &Paint<'_>,
    width: f32,
    t: Transform,
) {
    if let Some(path) = cap_path(tip, toward, cap, width) {
        if cap == Cap::Arrow {
            pixmap.stroke_path(
                &path,
                paint,
                &Stroke {
                    width,
                    line_cap: LineCap::Round,
                    line_join: tiny_skia::LineJoin::Round,
                    ..Default::default()
                },
                t,
                None,
            );
        } else {
            pixmap.fill_path(&path, paint, FillRule::Winding, t, None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reused_source_raster_matches_compositing_over_transparent_for_every_alpha() {
        let pixels = image::RgbaImage::from_fn(256, 2, |x, y| {
            image::Rgba([x as u8, (255 - x) as u8, (y * 91) as u8, x as u8])
        });
        let document = Document::from_image(pixels);
        for max_edge in [None, Some(128.)] {
            let (actual, _) = document.render(max_edge).unwrap();
            let source = if max_edge.is_some() {
                image::imageops::resize(
                    &*document.base,
                    128,
                    1,
                    image::imageops::FilterType::Triangle,
                )
            } else {
                (*document.base).clone()
            };
            let mut base = Pixmap::new(source.width(), source.height()).unwrap();
            for (target, pixel) in base
                .data_mut()
                .as_chunks_mut::<4>()
                .0
                .iter_mut()
                .zip(source.pixels())
            {
                for channel in 0..3 {
                    target[channel] = (u16::from(pixel[channel]) * u16::from(pixel[3]) / 255) as u8;
                }
                target[3] = pixel[3];
            }
            let mut expected = Pixmap::new(source.width(), source.height()).unwrap();
            expected.draw_pixmap(
                0,
                0,
                base.as_ref(),
                &tiny_skia::PixmapPaint::default(),
                Transform::identity(),
                None,
            );
            assert!(
                actual == expected,
                "reusing the source changed alpha compositing"
            );
        }
    }
    #[cfg(target_os = "macos")]
    #[test]
    fn bitmap_emoji_remain_visible_selectable_and_exported() {
        for text in ["😀", "👩🏽‍💻", "🇺🇸"] {
            let mut doc = document();
            doc.base = Arc::new(image::RgbaImage::new(240, 160));
            let mut mark = Mark::new(Tool::Text, Pos::new(40., 40.), Style::default());
            mark.style.font_size = 44.;
            mark.set_text(text.into());
            let center = mark.bounds().center();
            assert!(mark.hit(center, 0.), "{text} must remain selectable");
            doc.marks.push(mark);
            let (png, _) = doc.png(None).unwrap();
            let output = image::load_from_memory(&png).unwrap().to_rgba8();
            assert!(
                output.pixels().filter(|p| p[3] > 0).count() > 100,
                "{text} must be exported"
            );
            assert!(
                output
                    .pixels()
                    .any(|p| p[3] > 128 && p[0].abs_diff(p[2]) > 30),
                "{text} must retain its native colors"
            );
        }
    }
    fn document() -> Document {
        Document {
            annotation_scale: OnceLock::new(),
            next_marker: 1,
            comparison: None,
            base: Arc::new(image::RgbaImage::from_fn(80, 60, |x, y| {
                image::Rgba([(x * 3) as u8, (y * 4) as u8, 80, 255])
            })),
            marks: vec![],
            undo: vec![],
            redo: vec![],
            mosaic: Arc::default(),
        }
    }
    #[test]
    fn export_expands_in_every_direction_and_keeps_transparent_margins() {
        let mut d = document();
        let mut m = Mark::new(Tool::Rectangle, Pos::new(-20., -10.), Style::default());
        m.points[1] = Pos::new(100., 90.);
        d.marks.push(m);
        let (png, b) = d.png(None).unwrap();
        assert!(b.x < 0. && b.y < 0. && b.w > 120. && b.h > 100.);
        let image = image::load_from_memory(&png).unwrap().to_rgba8();
        assert_eq!(image.get_pixel(0, 0)[3], 0);
        assert_eq!(
            image.get_pixel((-b.x + 40.) as u32, (-b.y + 30.) as u32)[3],
            255
        );
    }
    #[test]
    fn redaction_replaces_pixels_and_ignores_style_opacity() {
        let mut d = document();
        let mut m = Mark::new(
            Tool::Redact,
            Pos::new(16., 16.),
            Style {
                opacity: 0.1,
                width: 16.,
                ..Default::default()
            },
        );
        m.points = vec![Pos::new(16., 16.), Pos::new(48., 16.), Pos::new(48., 40.)];
        d.marks.push(m);
        let (png, _) = d.png(None).unwrap();
        let output = image::load_from_memory(&png).unwrap().to_rgba8();
        assert_eq!(output.get_pixel(20, 16), output.get_pixel(28, 16));
        assert_ne!(output.get_pixel(20, 16), d.base.get_pixel(20, 16));
        assert_eq!(output.get_pixel(1, 1), d.base.get_pixel(1, 1));
        assert_eq!(
            output.get_pixel(9, 9),
            d.base.get_pixel(9, 9),
            "Brush has round ends"
        );
        assert_eq!(
            output.get_pixel(16, 40),
            d.base.get_pixel(16, 40),
            "Do not fill a stroke's bounding rectangle"
        );
        assert_eq!(output.get_pixel(20, 16)[3], 255);
    }
    #[test]
    fn history_restores_deleted_annotations_and_discards_redo_on_edit() {
        let mut d = document();
        d.checkpoint();
        d.marks
            .push(Mark::new(Tool::Pen, Pos::new(2., 2.), Style::default()));
        d.undo();
        assert!(d.marks.is_empty());
        d.redo();
        assert_eq!(d.marks.len(), 1);
        d.checkpoint();
        d.marks.clear();
        d.undo();
        assert_eq!(d.marks.len(), 1);
        d.checkpoint();
        assert!(!d.can_redo());
    }
    #[test]
    fn text_wrap_changes_height_but_not_font_size() {
        let mut m = Mark::new(Tool::Text, Pos::default(), Style::default());
        m.text = "A long annotation that wraps onto multiple lines".into();
        let old = m.bounds();
        m.resize(old, Rect { w: 100., ..old }, true);
        assert!(m.bounds().h > old.h);
        assert_eq!(m.style.font_size, TEXT_SIZES[1]);
        assert_eq!(m.wrap, Some(100.));
    }
    #[test]
    fn line_hit_testing_does_not_select_empty_bounding_box() {
        let mut m = Mark::new(Tool::Arrow, Pos::new(0., 0.), Style::default());
        m.points[1] = Pos::new(100., 100.);
        assert!(!m.hit(Pos::new(5., 95.), 4.));
        assert!(m.hit(Pos::new(50., 50.), 4.));
    }
    #[test]
    fn ellipse_and_capsule_have_distinct_outlines() {
        let mut ellipse = Mark::new(
            Tool::Ellipse,
            Pos::new(0., 0.),
            Style {
                fill: Fill::Solid,
                width: 1.,
                ..Default::default()
            },
        );
        ellipse.points[1] = Pos::new(120., 60.);
        let mut oval = ellipse.clone();
        oval.tool = Tool::Oval;
        assert!(!ellipse.hit(Pos::new(40., 1.), 0.1));
        assert!(
            oval.hit(Pos::new(40., 1.), 0.1),
            "Capsules have straight sides between semicircular ends"
        );
    }
    #[test]
    fn every_shape_and_fill_exports_and_has_selectable_geometry() {
        for tool in SHAPES {
            for fill in [Fill::None, Fill::Soft, Fill::Solid, Fill::Hatch] {
                let mut d = document();
                let mut mark = Mark::new(
                    tool,
                    Pos::new(10., 10.),
                    Style {
                        fill,
                        color: 4,
                        ..Default::default()
                    },
                );
                mark.points[1] = Pos::new(65., 50.);
                assert!(mark.path().is_some(), "{tool:?}");
                let path = mark.path().unwrap();
                let (_, edge) = shapes::edges(&path)[0];
                assert!(mark.hit(edge, 3.), "{tool:?}");
                d.marks.push(mark);
                let (bytes, _) = d.png(None).unwrap();
                assert_eq!(
                    image::guess_format(&bytes).unwrap(),
                    image::ImageFormat::Png
                );
            }
        }
    }
    #[test]
    fn undo_reaches_the_beginning_and_revert_erases_both_history_stacks() {
        let mut doc = document();
        for step in 0..150 {
            doc.checkpoint();
            if step == 0 {
                doc.marks
                    .push(Mark::new(Tool::Pen, Pos::new(10., 10.), Style::default()));
            }
            doc.marks[0].points[1].x = step as f32;
        }
        for _ in 0..150 {
            doc.undo();
        }
        assert!(doc.marks.is_empty());
        assert!(!doc.can_undo());
        for _ in 0..150 {
            doc.redo();
        }
        assert_eq!(doc.marks[0].points[1].x, 149.);
        doc.revert();
        assert!(doc.marks.is_empty());
        assert!(!doc.can_undo());
        assert!(!doc.can_redo());
        doc.undo();
        doc.redo();
        assert!(doc.marks.is_empty());
    }
    #[test]
    fn pixelation_stays_under_annotations_and_never_expands_the_image() {
        let mut doc = document();
        let mut shape = Mark::new(
            Tool::Rectangle,
            Pos::new(20., 15.),
            Style {
                color: 11,
                fill: Fill::Solid,
                width: 8.,
                ..Default::default()
            },
        );
        shape.points[1] = Pos::new(60., 45.);
        let mut pixelation = Mark::new(
            Tool::Redact,
            Pos::new(-80., -90.),
            Style {
                width: 256.,
                ..Default::default()
            },
        );
        pixelation.points[1] = Pos::new(160., 170.);
        doc.marks = vec![shape, pixelation];
        assert_eq!(
            doc.hit(Pos::new(40., 30.), 1.),
            Some(0),
            "Selection follows the visual order, with image effects below annotations"
        );
        let (png, bounds) = doc.png(None).unwrap();
        assert_eq!((bounds.x, bounds.y, bounds.w, bounds.h), (0., 0., 80., 60.));
        let pixels = image::load_from_memory(&png).unwrap().to_rgba8();
        assert_eq!(pixels.get_pixel(40, 30).0, {
            let fill = doc
                .marks
                .iter()
                .find(|m| m.style.color == 11)
                .unwrap()
                .style
                .fill_color();
            [fill[0], fill[1], fill[2], 255]
        });
        assert_ne!(pixels.get_pixel(2, 2), doc.base.get_pixel(2, 2));
        doc.reorder(0, StackOrder::Front);
        let (again, _) = doc.png(None).unwrap();
        assert_eq!(
            png, again,
            "Changing mark order cannot put the image effect over a shape"
        );
    }
    #[test]
    fn stacking_changes_export_and_is_undoable_with_overlap_aware_steps() {
        let mut doc = document();
        for color in [11, 4, 6] {
            let mut mark = Mark::new(
                Tool::Rectangle,
                Pos::new(20., 15.),
                Style {
                    color,
                    fill: Fill::Solid,
                    width: 8.,
                    ..Default::default()
                },
            );
            mark.points[1] = Pos::new(60., 45.);
            doc.marks.push(mark);
        }
        let mut distant = Mark::new(Tool::Rectangle, Pos::new(70., 50.), Style::default());
        distant.points[1] = Pos::new(75., 55.);
        doc.marks.push(distant);
        let index = doc.reorder(0, StackOrder::Forward);
        assert_eq!(index, 1);
        assert_eq!(doc.marks[index].style.color, 11);
        let index = doc.reorder(index, StackOrder::Front);
        assert_eq!(index, 3);
        let (png, _) = doc.png(None).unwrap();
        let output = image::load_from_memory(&png).unwrap().to_rgba8();
        assert_eq!(output.get_pixel(40, 30).0, {
            let fill = doc
                .marks
                .iter()
                .find(|m| m.style.color == 11)
                .unwrap()
                .style
                .fill_color();
            [fill[0], fill[1], fill[2], 255]
        });
        let index = doc.reorder(index, StackOrder::Backward);
        assert_eq!(
            index, 1,
            "One step skips the distant mark and crosses the next overlapping mark"
        );
        let index = doc.reorder(index, StackOrder::Back);
        assert_eq!(index, 0);
        doc.undo();
        assert_eq!(doc.marks[1].style.color, 11);
        doc.redo();
        assert_eq!(doc.marks[0].style.color, 11);
    }
}

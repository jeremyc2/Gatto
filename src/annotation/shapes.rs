use super::*;

pub const SHAPES: [Tool; 17] = [
    Tool::Rectangle,
    Tool::Ellipse,
    Tool::Triangle,
    Tool::Diamond,
    Tool::Hexagon,
    Tool::Oval,
    Tool::Rhombus,
    Tool::Star,
    Tool::Cloud,
    Tool::Heart,
    Tool::XBox,
    Tool::CheckBox,
    Tool::ArrowLeft,
    Tool::ArrowUp,
    Tool::ArrowDown,
    Tool::ArrowRight,
    Tool::Line,
];
impl Tool {
    pub fn is_geometry(self) -> bool {
        SHAPES.contains(&self)
    }
    pub fn is_shape(self) -> bool {
        self.is_geometry() && self != Tool::Line
    }
}

impl Style {
    /// One palette choice supplies a lighter stroke and a darker, matching fill.
    pub(super) fn fill_color(&self) -> [u8; 3] {
        COLORS[self.color].map(|channel| (u16::from(channel) * 4 / 5) as u8)
    }
}

impl Mark {
    pub(super) fn shape_geometry(&self, path: Path) -> Option<Path> {
        if !self.flip_x && !self.flip_y {
            return Some(path);
        }
        let b = self.bounds();
        path.transform(Transform::from_row(
            if self.flip_x { -1. } else { 1. },
            0.,
            0.,
            if self.flip_y { -1. } else { 1. },
            if self.flip_x { b.x * 2. + b.w } else { 0. },
            if self.flip_y { b.y * 2. + b.h } else { 0. },
        ))
    }
    /// Geometry can reflect across a resize anchor; labels always stay readable.
    pub fn flip_geometry(&mut self, x: bool, y: bool) {
        if !x && !y {
            return;
        }
        if self.tool.is_shape() {
            self.flip_x ^= x;
            self.flip_y ^= y;
        } else if self.tool != Tool::Text {
            let c = self.bounds().center();
            for p in &mut self.points {
                if x {
                    p.x = c.x * 2. - p.x;
                }
                if y {
                    p.y = c.y * 2. - p.y;
                }
            }
        }
    }
    /// Symbols share the border's weight and dash pattern, above the shape fill.
    pub(super) fn box_symbol(&self) -> Option<Path> {
        let path = self.shape_geometry(box_symbol_path(
            self.tool,
            self.bounds(),
            self.style.width * 2.,
        )?)?;
        let dashed = self
            .style
            .dash
            .pattern(self.style.width)
            .and_then(|dash| path.dash(&dash, 1.));
        dashed.as_ref().unwrap_or(&path).stroke(
            &Stroke {
                width: self.style.width,
                line_cap: LineCap::Round,
                line_join: tiny_skia::LineJoin::Round,
                ..Default::default()
            },
            1.,
        )
    }
}

fn box_symbol_path(tool: Tool, b: Rect, roundness: f32) -> Option<Path> {
    let mut p = PathBuilder::new();
    let inset = if tool == Tool::XBox {
        roundness.min(b.w.min(b.h) * 0.12) * 0.25
    } else {
        0.
    };
    let at = |x, y| {
        (
            b.x + inset + x * (b.w - inset * 2.),
            b.y + inset + y * (b.h - inset * 2.),
        )
    };
    let lines: &[&[(f32, f32)]] = match tool {
        Tool::XBox => &[&[(0., 0.), (1., 1.)], &[(1., 0.), (0., 1.)]],
        Tool::CheckBox => &[&[(0.3, 0.52), (0.47, 0.75), (0.76, 0.28)]],
        _ => return None,
    };
    for line in lines {
        for (i, &(x, y)) in line.iter().enumerate() {
            let (x, y) = at(x, y);
            if i == 0 {
                p.move_to(x, y);
            } else {
                p.line_to(x, y);
            }
        }
    }
    p.finish()
}

pub(super) fn shape_path(tool: Tool, b: Rect, roundness: f32) -> Option<Path> {
    let mut p = PathBuilder::new();
    let at = |x: f32, y: f32| (b.x + x * b.w, b.y + y * b.h);
    let polygon = |p: &mut PathBuilder, points: &[(f32, f32)]| {
        let area: f32 = points
            .iter()
            .zip(points.iter().cycle().skip(1))
            .map(|(&(x, y), &(nx, ny))| x * ny - nx * y)
            .sum();
        let points: Vec<_> = (0..points.len())
            .map(|i| {
                let index = if area >= 0. { i } else { points.len() - 1 - i };
                let (x, y) = at(points[index].0, points[index].1);
                Pos::new(x, y)
            })
            .collect();
        let count = points.len();
        let radius = roundness.min(b.w.min(b.h) * 0.12);
        for i in 0..count {
            let corner = points[i];
            let previous = points[(i + count - 1) % count];
            let next = points[(i + 1) % count];
            let inset = radius
                .min(corner.distance(previous) * 0.45)
                .min(corner.distance(next) * 0.45);
            let before = corner.lerp(previous, inset / corner.distance(previous).max(0.001));
            let after = corner.lerp(next, inset / corner.distance(next).max(0.001));
            if i == 0 {
                p.move_to(before.x, before.y);
            } else {
                p.line_to(before.x, before.y);
            }
            p.quad_to(corner.x, corner.y, after.x, after.y);
        }
        p.close();
    };
    match tool {
        Tool::Line => {
            p.move_to(b.x, b.y + b.h);
            p.line_to(b.x + b.w, b.y);
        }
        Tool::Rectangle | Tool::XBox | Tool::CheckBox => {
            polygon(&mut p, &[(0., 0.), (1., 0.), (1., 1.), (0., 1.)]);
            if let Some(symbol) = box_symbol_path(tool, b, roundness) {
                p.push_path(&symbol);
            }
        }
        Tool::Ellipse => p.push_oval(tiny_skia::Rect::from_xywh(b.x, b.y, b.w, b.h)?),
        Tool::Oval => {
            let r = (b.w.min(b.h) / 2.).max(0.5);
            let k = r * 0.5522848;
            p.move_to(b.x + r, b.y);
            p.line_to(b.x + b.w - r, b.y);
            p.cubic_to(
                b.x + b.w - r + k,
                b.y,
                b.x + b.w,
                b.y + r - k,
                b.x + b.w,
                b.y + r,
            );
            p.line_to(b.x + b.w, b.y + b.h - r);
            p.cubic_to(
                b.x + b.w,
                b.y + b.h - r + k,
                b.x + b.w - r + k,
                b.y + b.h,
                b.x + b.w - r,
                b.y + b.h,
            );
            p.line_to(b.x + r, b.y + b.h);
            p.cubic_to(
                b.x + r - k,
                b.y + b.h,
                b.x,
                b.y + b.h - r + k,
                b.x,
                b.y + b.h - r,
            );
            p.line_to(b.x, b.y + r);
            p.cubic_to(b.x, b.y + r - k, b.x + r - k, b.y, b.x + r, b.y);
            p.close();
        }
        Tool::Triangle => polygon(&mut p, &[(0.5, 0.), (1., 1.), (0., 1.)]),
        Tool::Diamond => polygon(&mut p, &[(0.5, 0.), (1., 0.5), (0.5, 1.), (0., 0.5)]),
        Tool::Star | Tool::Hexagon => {
            let count = match tool {
                Tool::Star => 10,
                _ => 6,
            };
            let points = (0..count)
                .map(|i| {
                    let angle = -std::f32::consts::FRAC_PI_2
                        + std::f32::consts::TAU * i as f32 / count as f32;
                    let radius = if tool == Tool::Star && i % 2 == 1 {
                        0.23
                    } else {
                        0.5
                    };
                    (0.5 + radius * angle.cos(), 0.5 + radius * angle.sin())
                })
                .collect::<Vec<_>>();
            polygon(&mut p, &points);
        }
        Tool::Rhombus => polygon(&mut p, &[(0.22, 0.), (1., 0.), (0.78, 1.), (0., 1.)]),
        Tool::ArrowLeft | Tool::ArrowUp | Tool::ArrowDown | Tool::ArrowRight => {
            let points = [
                (0., 0.3),
                (0.6, 0.3),
                (0.6, 0.),
                (1., 0.5),
                (0.6, 1.),
                (0.6, 0.7),
                (0., 0.7),
            ]
            .map(|(x, y)| match tool {
                Tool::ArrowLeft => (1. - x, y),
                Tool::ArrowUp => (y, 1. - x),
                Tool::ArrowDown => (y, x),
                _ => (x, y),
            });
            polygon(&mut p, &points);
        }
        Tool::Heart => {
            let (x, y) = at(0.5, 1.);
            p.move_to(x, y);
            for (a, c, d) in [
                ((0.1, 0.72), (-0.15, 0.3), (0.12, 0.08)),
                ((0.25, -0.05), (0.43, 0.), (0.5, 0.18)),
                ((0.57, 0.), (0.75, -0.05), (0.88, 0.08)),
                ((1.15, 0.3), (0.9, 0.72), (0.5, 1.)),
            ] {
                let (ax, ay) = at(a.0, a.1);
                let (cx, cy) = at(c.0, c.1);
                let (dx, dy) = at(d.0, d.1);
                p.cubic_to(ax, ay, cx, cy, dx, dy);
            }
            p.close();
        }
        Tool::Cloud => {
            let (x, y) = at(0.18, 0.92);
            p.move_to(x, y);
            for (a, c, d) in [
                ((-0.07, 0.92), (-0.06, 0.53), (0.12, 0.49)),
                ((0.02, 0.2), (0.26, 0.02), (0.43, 0.19)),
                ((0.58, -0.09), (0.92, 0.04), (0.89, 0.35)),
                ((1.1, 0.4), (1.07, 0.78), (0.91, 0.8)),
                ((0.86, 1.02), (0.61, 1.02), (0.52, 0.9)),
                ((0.4, 1.05), (0.24, 1.04), (0.18, 0.92)),
            ] {
                let (ax, ay) = at(a.0, a.1);
                let (cx, cy) = at(c.0, c.1);
                let (dx, dy) = at(d.0, d.1);
                p.cubic_to(ax, ay, cx, cy, dx, dy);
            }
            p.close();
        }
        _ => return None,
    }
    p.finish()
}

pub(super) fn edges(path: &Path) -> Vec<(Pos, Pos)> {
    let mut result = Vec::new();
    let mut previous = Pos::default();
    let mut start = previous;
    let at = |p: tiny_skia::Point| Pos::new(p.x, p.y);
    for segment in path.segments() {
        match segment {
            tiny_skia::PathSegment::MoveTo(p) => {
                previous = at(p);
                start = previous;
            }
            tiny_skia::PathSegment::LineTo(p) => {
                let p = at(p);
                result.push((previous, p));
                previous = p;
            }
            tiny_skia::PathSegment::Close => {
                result.push((previous, start));
                previous = start;
            }
            tiny_skia::PathSegment::QuadTo(c, p) => {
                let origin = previous;
                let c = at(c);
                let p = at(p);
                for i in 1..=32 {
                    let t = i as f32 / 32.;
                    let next = origin.lerp(c, t).lerp(c.lerp(p, t), t);
                    result.push((previous, next));
                    previous = next;
                }
            }
            tiny_skia::PathSegment::CubicTo(a, b, p) => {
                let origin = previous;
                let a = at(a);
                let b = at(b);
                let p = at(p);
                for i in 1..=32 {
                    let t = i as f32 / 32.;
                    let next = origin
                        .lerp(a, t)
                        .lerp(a.lerp(b, t), t)
                        .lerp(a.lerp(b, t).lerp(b.lerp(p, t), t), t);
                    result.push((previous, next));
                    previous = next;
                }
            }
        }
    }
    result
}
pub(super) fn hatch_path(path: &Path, bounds: Rect, scale: f32) -> Option<Path> {
    let edges = edges(path);
    let spacing = 10. * scale;
    let mut builder = PathBuilder::new();
    for i in ((bounds.y - bounds.x - bounds.w) / spacing).floor() as i32
        ..=((bounds.y + bounds.h - bounds.x) / spacing).ceil() as i32
    {
        let intercept = i as f32 * spacing;
        let mut intersections = Vec::new();
        for (a, b) in &edges {
            let da = a.y - a.x - intercept;
            let db = b.y - b.x - intercept;
            if (da <= 0. && db > 0.) || (db <= 0. && da > 0.) {
                intersections.push(a.lerp(*b, da / (da - db)));
            }
        }
        intersections.sort_by(|a, b| a.x.total_cmp(&b.x));
        for pair in intersections.as_chunks::<2>().0 {
            builder.move_to(pair[0].x, pair[0].y);
            builder.line_to(pair[1].x, pair[1].y);
        }
    }
    builder.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reflecting_geometry_keeps_all_label_and_text_glyphs_readable() {
        for tool in SHAPES
            .into_iter()
            .filter(|t| t.is_shape())
            .chain([Tool::Text])
        {
            let mut original = Mark::new(tool, Pos::new(20., 20.), Style::default());
            original.points[1] = Pos::new(240., 180.);
            original.set_text("Read me".into());
            for (x, y) in [(true, false), (false, true), (true, true)] {
                let mut mark = original.clone();
                mark.flip_geometry(x, y);
                assert_eq!(mark.label_path(), original.label_path(), "{tool:?}");
                if tool == Tool::Text {
                    assert_eq!(mark.standalone_path(), original.standalone_path());
                }
            }
        }
    }
    #[test]
    fn polygon_corners_are_softened_and_rectangle_hits_follow_the_curve() {
        for tool in SHAPES.into_iter().filter(|t| {
            t.is_shape() && !matches!(t, Tool::Ellipse | Tool::Oval | Tool::Cloud | Tool::Heart)
        }) {
            let mut mark = Mark::new(tool, Pos::new(20., 20.), Style::default());
            mark.points[1] = Pos::new(180., 140.);
            assert!(
                mark.body_path()
                    .unwrap()
                    .segments()
                    .any(|p| matches!(p, tiny_skia::PathSegment::QuadTo(_, _))),
                "{tool:?}"
            );
        }
        let mut rectangle = Mark::new(Tool::Rectangle, Pos::new(20., 20.), Style::default());
        rectangle.points[1] = Pos::new(180., 140.);
        assert!(!rectangle.contains_body(Pos::new(20.1, 20.1)));
        assert!(rectangle.contains_body(Pos::new(100., 20.1)));
        assert!(rectangle.contains_body(Pos::new(100., 80.)));
    }
}

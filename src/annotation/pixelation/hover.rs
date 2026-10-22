//! Temporary mosaic coverage, painted below annotations without creating a mark or texture.
use super::*;
use gpui_kit::{Bounds, Window, point, px, rgb, size};

#[derive(Clone)]
pub(in crate::annotation) struct HoverPreview {
    source: Document,
    levels: Arc<OnceLock<Vec<Level>>>,
}

#[derive(Clone)]
struct Level {
    columns: usize,
    rows: usize,
    block: f32,
    colors: Vec<[u8; 3]>,
}

impl HoverPreview {
    pub fn new(doc: &Document) -> Self {
        let mut source = doc.snapshot();
        source.marks.clear();
        Self {
            source,
            levels: Arc::default(),
        }
    }

    pub fn matches(&self, doc: &Document) -> bool {
        Arc::ptr_eq(&self.source.base, &doc.base)
    }

    fn levels(&self) -> &[Level] {
        self.levels.get_or_init(|| {
            let mut levels = vec![Level {
                columns: self.source.base.width().div_ceil(16) as usize,
                rows: self.source.base.height().div_ceil(16) as usize,
                block: 16.,
                colors: self.source.mosaic_colors().to_vec(),
            }];
            // Combine only subpixel cells when zoomed out, avoiding thousands of
            // invisible primitives. Levels are cached once and shared across frames.
            while levels.last().unwrap().columns > 1 || levels.last().unwrap().rows > 1 {
                let previous = levels.last().unwrap();
                let (columns, rows) = (previous.columns.div_ceil(2), previous.rows.div_ceil(2));
                let mut colors = Vec::with_capacity(columns * rows);
                for y in 0..rows {
                    for x in 0..columns {
                        let mut sum = [0u32; 3];
                        let mut count = 0;
                        for yy in y * 2..(y * 2 + 2).min(previous.rows) {
                            for xx in x * 2..(x * 2 + 2).min(previous.columns) {
                                let color = previous.colors[yy * previous.columns + xx];
                                for c in 0..3 {
                                    sum[c] += u32::from(color[c]);
                                }
                                count += 1;
                            }
                        }
                        colors.push(sum.map(|c| (c / count) as u8));
                    }
                }
                levels.push(Level {
                    columns,
                    rows,
                    block: previous.block * 2.,
                    colors,
                });
            }
            levels
        })
    }

    pub fn paint(&self, center: Pos, radius: f32, origin: Pos, scale: f32, window: &mut Window) {
        if radius <= 0. || scale <= 0. {
            return;
        }
        let image = Bounds::new(
            point(px(origin.x), px(origin.y)),
            size(
                px(self.source.base.width() as f32 * scale),
                px(self.source.base.height() as f32 * scale),
            ),
        );
        let brush = Bounds::new(
            point(px(center.x - radius), px(center.y - radius)),
            size(px(radius * 2.), px(radius * 2.)),
        );
        let visible = image
            .intersect(&brush)
            .intersect(&window.content_mask().bounds);
        if visible.size.width <= px(0.) || visible.size.height <= px(0.) {
            return;
        }
        let levels = self.levels();
        let level = levels
            .iter()
            .find(|level| level.block * scale >= 1.)
            .unwrap_or_else(|| levels.last().unwrap());
        let cell = level.block * scale;
        let left = ((visible.left().as_f32() - origin.x) / cell)
            .floor()
            .max(0.) as usize;
        let top = ((visible.top().as_f32() - origin.y) / cell).floor().max(0.) as usize;
        let right =
            (((visible.right().as_f32() - origin.x) / cell).ceil() as usize).min(level.columns);
        let bottom =
            (((visible.bottom().as_f32() - origin.y) / cell).ceil() as usize).min(level.rows);
        let segments = ((radius * 2.).sqrt() * 8.).ceil().clamp(32., 256.) as usize;
        let circle: Vec<_> = (0..segments)
            .map(|i| {
                let angle = i as f32 * std::f32::consts::TAU / segments as f32;
                Pos::new(
                    center.x + radius * angle.cos(),
                    center.y + radius * angle.sin(),
                )
            })
            .collect();
        // All cells share one source-effect layer; avoid a scene-order lookup per cell.
        window.paint_layer(visible, |window| {
            for y in top..bottom {
                let mut x = left;
                while x < right {
                    let color = level.colors[y * level.columns + x];
                    let start = x;
                    x += 1;
                    // Flat screenshot backgrounds often span many cells of the same color.
                    while x < right && level.colors[y * level.columns + x] == color {
                        x += 1;
                    }
                    let rect = Rect {
                        x: (origin.x + start as f32 * cell).max(visible.left().as_f32()),
                        y: (origin.y + y as f32 * cell).max(visible.top().as_f32()),
                        w: 0.,
                        h: 0.,
                    };
                    let rect = Rect {
                        w: (origin.x + x as f32 * cell).min(visible.right().as_f32()) - rect.x,
                        h: (origin.y + (y + 1) as f32 * cell).min(visible.bottom().as_f32())
                            - rect.y,
                        ..rect
                    };
                    let nearest = Pos::new(
                        center.x.clamp(rect.x, rect.x + rect.w),
                        center.y.clamp(rect.y, rect.y + rect.h),
                    );
                    if nearest.distance(center) > radius {
                        continue;
                    }
                    let ink = rgb((u32::from(color[0]) << 16)
                        | (u32::from(color[1]) << 8)
                        | u32::from(color[2]));
                    if rect.corners().iter().all(|p| p.distance(center) <= radius) {
                        window.paint_quad(gpui_kit::fill(
                            Bounds::new(
                                point(px(rect.x), px(rect.y)),
                                size(px(rect.w), px(rect.h)),
                            ),
                            ink,
                        ));
                    } else {
                        let polygon = clip_circle(&circle, rect);
                        if polygon.len() < 3 {
                            continue;
                        }
                        let mut path = gpui_kit::PathBuilder::fill();
                        path.move_to(point(px(polygon[0].x), px(polygon[0].y)));
                        for p in &polygon[1..] {
                            path.line_to(point(px(p.x), px(p.y)));
                        }
                        path.close();
                        if let Ok(path) = path.build() {
                            window.paint_path(path, ink);
                        }
                    }
                }
            }
        });
    }
}

fn clip_circle(circle: &[Pos], rect: Rect) -> Vec<Pos> {
    let mut polygon = circle.to_vec();
    for (axis, boundary, lower) in [
        (0, rect.x, true),
        (0, rect.x + rect.w, false),
        (1, rect.y, true),
        (1, rect.y + rect.h, false),
    ] {
        let input = std::mem::take(&mut polygon);
        let Some(mut previous) = input.last().copied() else {
            break;
        };
        let coordinate = |p: Pos| if axis == 0 { p.x } else { p.y };
        let inside = |p: Pos| {
            if lower {
                coordinate(p) >= boundary
            } else {
                coordinate(p) <= boundary
            }
        };
        for current in input {
            if inside(current) != inside(previous) {
                let t = (boundary - coordinate(previous))
                    / (coordinate(current) - coordinate(previous));
                polygon.push(previous.lerp(current, t));
            }
            if inside(current) {
                polygon.push(current);
            }
            previous = current;
        }
    }
    polygon
}

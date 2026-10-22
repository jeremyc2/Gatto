//! Fixed source-image brush effects. Only newly covered tiles change while painting.
mod hover;
pub(super) use hover::HoverPreview;

use super::*;
use std::collections::{BTreeMap, BTreeSet};

const TILE: u32 = 256;

#[derive(Default, Clone)]
pub(super) struct PixelationPreview {
    strokes: Vec<Mark>,
    tiles: BTreeMap<(u32, u32), Tile>,
    retired: Vec<Arc<gpui_kit::RenderImage>>,
}
#[derive(Clone)]
struct Tile {
    pixels: image::RgbaImage,
    texture: Option<Arc<gpui_kit::RenderImage>>,
}
impl PixelationPreview {
    pub fn update(&mut self, doc: &Document) {
        let strokes: Vec<_> = doc
            .marks
            .iter()
            .filter(|m| m.tool == Tool::Redact)
            .collect();
        let append_only = strokes.len() >= self.strokes.len()
            && self
                .strokes
                .iter()
                .zip(&strokes)
                .enumerate()
                .all(|(i, (old, new))| {
                    old == *new
                        || (i + 1 == self.strokes.len()
                            && old.style == new.style
                            && new.points.starts_with(&old.points))
                });
        if !append_only {
            self.strokes.clear();
            self.retire_tiles();
        }
        let mut dirty = BTreeSet::new();
        let mut gutter_dirty = BTreeSet::new();
        let (w, h) = doc.base.dimensions();
        let columns = w.div_ceil(16) as usize;
        for (i, stroke) in strokes.iter().enumerate() {
            let start = if let Some(old) = self.strokes.get(i) {
                if old == *stroke {
                    continue;
                }
                old.points.len().saturating_sub(1)
            } else {
                0
            };
            let colors = doc.mosaic_colors();
            brush_pixels(
                &stroke.points[start..],
                stroke.style.width,
                (w, h),
                |x, y| {
                    let key = (x / TILE, y / TILE);
                    let tile = self.tiles.entry(key).or_insert_with(|| Tile {
                        // Preserve mosaic RGB under transparent coverage for filtering.
                        pixels: image::RgbaImage::from_fn(
                            TILE.min(w - key.0 * TILE),
                            TILE.min(h - key.1 * TILE),
                            |x, y| {
                                let c = colors[((key.1 * TILE + y) / 16) as usize * columns
                                    + ((key.0 * TILE + x) / 16) as usize];
                                // GPUI uploads raw image data as BGRA.
                                image::Rgba([c[2], c[1], c[0], 0])
                            },
                        ),
                        texture: None,
                    });
                    let pixel = tile.pixels.get_pixel_mut(x % TILE, y % TILE);
                    if pixel[3] == 255 {
                        return;
                    }
                    pixel[3] = 255;
                    dirty.insert(key);
                    // Only boundary coverage can change an adjacent texture's gutter.
                    if x % TILE == 0
                        || x % TILE == TILE - 1
                        || y % TILE == 0
                        || y % TILE == TILE - 1
                    {
                        for yy in y.saturating_sub(1)..=(y + 1).min(h - 1) {
                            for xx in x.saturating_sub(1)..=(x + 1).min(w - 1) {
                                gutter_dirty.insert((xx / TILE, yy / TILE));
                            }
                        }
                    }
                },
            );
        }
        dirty.extend(
            gutter_dirty
                .into_iter()
                .filter(|key| self.tiles.contains_key(key)),
        );
        for key in dirty {
            let pixels = self.texture_pixels(key, doc);
            let tile = self.tiles.get_mut(&key).unwrap();
            if let Some(texture) = tile.texture.take() {
                self.retired.push(texture);
            }
            tile.texture = Some(Arc::new(gpui_kit::RenderImage::new(vec![
                image::Frame::new(pixels),
            ])));
        }
        self.strokes = strokes.into_iter().cloned().collect();
    }
    fn texture_pixels(&self, key: (u32, u32), doc: &Document) -> image::RgbaImage {
        let tile = &self.tiles[&key];
        let colors = doc.mosaic_colors();
        let columns = doc.base.width().div_ceil(16) as usize;
        // A one-pixel gutter keeps filtering inside our image instead of atlas padding.
        let (w, h) = tile.pixels.dimensions();
        let mut pixels = image::RgbaImage::new(w + 2, h + 2);
        // Reuse the cached interior rather than recalculating its colors on every pointer event.
        let stride = (w + 2) as usize * 4;
        for (y, row) in tile
            .pixels
            .as_raw()
            .chunks_exact(w as usize * 4)
            .enumerate()
        {
            let start = (y + 1) * stride + 4;
            pixels.as_flat_samples_mut().samples[start..start + row.len()].copy_from_slice(row);
        }
        let gutter = |x: u32, y: u32| {
            let sx = (key.0 * TILE + x)
                .saturating_sub(1)
                .min(doc.base.width() - 1);
            let sy = (key.1 * TILE + y)
                .saturating_sub(1)
                .min(doc.base.height() - 1);
            let alpha = self
                .tiles
                .get(&(sx / TILE, sy / TILE))
                .map_or(0, |tile| tile.pixels.get_pixel(sx % TILE, sy % TILE)[3]);
            let c = colors[(sy / 16) as usize * columns + (sx / 16) as usize];
            image::Rgba([c[2], c[1], c[0], alpha])
        };
        for x in 0..w + 2 {
            pixels.put_pixel(x, 0, gutter(x, 0));
            pixels.put_pixel(x, h + 1, gutter(x, h + 1));
        }
        for y in 1..h + 1 {
            pixels.put_pixel(0, y, gutter(0, y));
            pixels.put_pixel(w + 1, y, gutter(w + 1, y));
        }
        pixels
    }
    fn retire_tiles(&mut self) {
        self.retired.extend(
            std::mem::take(&mut self.tiles)
                .into_values()
                .filter_map(|tile| tile.texture),
        );
    }
    pub fn retired_images(&mut self, all: bool) -> Vec<Arc<gpui_kit::RenderImage>> {
        if all {
            self.retire_tiles();
            self.strokes.clear();
        }
        std::mem::take(&mut self.retired)
    }
    pub fn paint(&self, origin: Pos, scale: f32, window: &mut gpui_kit::Window) {
        use gpui_kit::{Bounds, Corners, point, px, size};
        for ((x, y), tile) in &self.tiles {
            let Some(texture) = &tile.texture else {
                continue;
            };
            let bounds = Bounds::new(
                point(
                    px(origin.x + (x * TILE) as f32 * scale),
                    px(origin.y + (y * TILE) as f32 * scale),
                ),
                size(
                    px(tile.pixels.width() as f32 * scale),
                    px(tile.pixels.height() as f32 * scale),
                ),
            );
            let _ = window.paint_image(
                bounds,
                Bounds::new(
                    point(bounds.origin.x - px(scale), bounds.origin.y - px(scale)),
                    size(
                        bounds.size.width + px(2. * scale),
                        bounds.size.height + px(2. * scale),
                    ),
                ),
                Corners::default(),
                texture.clone(),
                0,
                false,
            );
        }
    }
}

/// Rasterize the union of round-ended segments at source pixel centers. Shared by preview/export.
fn brush_pixels(
    points: &[Pos],
    width: f32,
    image_size: (u32, u32),
    mut paint: impl FnMut(u32, u32),
) {
    let radius = width / 2.;
    for segment in points.windows(2) {
        let a = segment[0];
        let b = segment[1];
        // Bound work by the painted area, including when a pointer event jumps a long distance.
        let steps = (a.distance(b) / radius.max(1.)).ceil().max(1.) as usize;
        for step in 0..steps {
            let a = a.lerp(b, step as f32 / steps as f32);
            let b = segment[0].lerp(b, (step + 1) as f32 / steps as f32);
            let left = (a.x.min(b.x) - radius)
                .floor()
                .clamp(0., image_size.0 as f32) as u32;
            let top = (a.y.min(b.y) - radius)
                .floor()
                .clamp(0., image_size.1 as f32) as u32;
            let right = (a.x.max(b.x) + radius)
                .ceil()
                .clamp(0., image_size.0 as f32) as u32;
            let bottom = (a.y.max(b.y) + radius)
                .ceil()
                .clamp(0., image_size.1 as f32) as u32;
            let dx = b.x - a.x;
            let dy = b.y - a.y;
            let length = (dx * dx + dy * dy).max(0.001);
            for y in top..bottom {
                for x in left..right {
                    let p = Pos::new(x as f32 + 0.5, y as f32 + 0.5);
                    let t = (((p.x - a.x) * dx + (p.y - a.y) * dy) / length).clamp(0., 1.);
                    let on_line = a.lerp(b, t);
                    if (p.x - on_line.x).powi(2) + (p.y - on_line.y).powi(2) <= radius * radius {
                        paint(x, y);
                    }
                }
            }
        }
    }
}

pub(super) fn export(doc: &Document, pixmap: &mut Pixmap, bounds: Rect, scale: f32) {
    if !doc.marks.iter().any(|m| m.tool == Tool::Redact) {
        return;
    }
    let colors = doc.mosaic_colors();
    let columns = doc.base.width().div_ceil(16) as usize;
    let stride = pixmap.width() as usize;
    for mark in doc.marks.iter().filter(|m| m.tool == Tool::Redact) {
        brush_pixels(
            &mark.points,
            mark.style.width,
            doc.base.dimensions(),
            |x, y| {
                let color = colors[(y / 16) as usize * columns + (x / 16) as usize];
                let left = ((x as f32 - bounds.x) * scale).floor().max(0.) as u32;
                let top = ((y as f32 - bounds.y) * scale).floor().max(0.) as u32;
                let right =
                    (((x as f32 + 1. - bounds.x) * scale).ceil() as u32).min(pixmap.width());
                let bottom =
                    (((y as f32 + 1. - bounds.y) * scale).ceil() as u32).min(pixmap.height());
                for yy in top..bottom {
                    for xx in left..right {
                        let i = (yy as usize * stride + xx as usize) * 4;
                        pixmap.data_mut()[i..i + 4]
                            .copy_from_slice(&[color[0], color[1], color[2], 255]);
                    }
                }
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boundary_paint_refreshes_neighbor_gutters_without_changing_coverage() {
        let mut doc = Document {
            annotation_scale: OnceLock::new(),
            base: Arc::new(image::RgbaImage::from_pixel(
                530,
                270,
                image::Rgba([80, 100, 120, 255]),
            )),
            marks: vec![Mark::new(
                Tool::Redact,
                Pos::new(254., 100.),
                Style {
                    width: 2.,
                    ..Default::default()
                },
            )],
            undo: Vec::new(),
            redo: Vec::new(),
            mosaic: Arc::default(),
        };
        let mut preview = PixelationPreview::default();
        preview.update(&doc);
        let old = preview.tiles[&(0, 0)].texture.clone().unwrap();
        assert_eq!(
            preview.texture_pixels((0, 0), &doc).get_pixel(257, 101)[3],
            0
        );
        doc.marks.push(Mark::new(
            Tool::Redact,
            Pos::new(256.5, 100.5),
            Style {
                width: 1.,
                ..Default::default()
            },
        ));
        preview.update(&doc);
        assert!(
            !Arc::ptr_eq(&old, preview.tiles[&(0, 0)].texture.as_ref().unwrap()),
            "Painting the neighboring tile must refresh the first tile's gutter"
        );
        assert_eq!(
            *preview.texture_pixels((0, 0), &doc).get_pixel(257, 101),
            image::Rgba([120, 100, 80, 255])
        );
        // Logical coverage is unchanged; only the filtered texture's gutter is updated.
        assert_eq!(preview.tiles[&(0, 0)].pixels.get_pixel(255, 100)[3], 0);
        doc.marks.push(Mark::new(
            Tool::Redact,
            Pos::new(529.5, 269.5),
            Style {
                width: 1.,
                ..Default::default()
            },
        ));
        preview.update(&doc);
        let edge = preview.texture_pixels((2, 1), &doc);
        assert_eq!(edge.dimensions(), (20, 16));
        assert_eq!(*edge.get_pixel(19, 15), image::Rgba([120, 100, 80, 255]));
        assert_eq!(
            *edge.get_pixel(1, 1),
            image::Rgba([120, 100, 80, 0]),
            "Transparent texels retain mosaic RGB for linear filtering"
        );
    }

    #[test]
    fn brush_tiles_match_export_and_only_newly_painted_tiles_change() {
        let mut doc = Document {
            annotation_scale: OnceLock::new(),
            base: Arc::new(image::RgbaImage::from_fn(512, 512, |x, y| {
                image::Rgba([(x % 255) as u8, (y % 255) as u8, 80, 255])
            })),
            marks: Vec::new(),
            undo: Vec::new(),
            redo: Vec::new(),
            mosaic: Arc::default(),
        };
        let mut stroke = Mark::new(
            Tool::Redact,
            Pos::new(16., 16.),
            Style {
                width: 16.,
                ..Default::default()
            },
        );
        stroke.points[1] = Pos::new(60., 16.);
        doc.marks.push(stroke.clone());
        stroke.points = vec![Pos::new(450., 450.); 2];
        doc.marks.push(stroke);
        let mut preview = PixelationPreview::default();
        preview.update(&doc);
        let untouched = preview.tiles[&(1, 1)].texture.clone().unwrap();
        let changed = preview.tiles[&(0, 0)].texture.clone().unwrap();
        // Extending the active (last) stroke preserves all earlier tiles/textures.
        doc.marks[1].points.push(Pos::new(490., 460.));
        preview.update(&doc);
        assert!(Arc::ptr_eq(
            &changed,
            preview.tiles[&(0, 0)].texture.as_ref().unwrap()
        ));
        assert!(!Arc::ptr_eq(
            &untouched,
            preview.tiles[&(1, 1)].texture.as_ref().unwrap()
        ));
        assert!(
            preview
                .retired_images(false)
                .iter()
                .any(|image| Arc::ptr_eq(image, &untouched)),
            "Replaced textures must be released from the GPU atlas"
        );
        let (png, _) = doc.png(None).unwrap();
        let exported = image::load_from_memory(&png).unwrap().to_rgba8();
        for y in 0..512 {
            for x in 0..512 {
                let pixel = preview
                    .tiles
                    .get(&(x / TILE, y / TILE))
                    .map(|t| t.pixels.get_pixel(x % TILE, y % TILE));
                let expected = match pixel {
                    Some(p) if p[3] == 255 => image::Rgba([p[2], p[1], p[0], p[3]]),
                    _ => *doc.base.get_pixel(x, y),
                };
                assert_eq!(*exported.get_pixel(x, y), expected);
            }
        }
        doc.checkpoint();
        doc.marks.clear();
        preview.update(&doc);
        assert!(preview.tiles.is_empty());
        doc.undo();
        preview.update(&doc);
        assert!(!preview.tiles.is_empty());
        let after_undo = preview.tiles[&(0, 0)].pixels.clone();
        doc.revert();
        preview.update(&doc);
        assert!(preview.tiles.is_empty());
        assert!(after_undo.pixels().any(|p| p[3] == 255));
    }
    #[test]
    fn live_brush_updates_on_a_large_source_stay_interactive() {
        let mut doc = Document {
            annotation_scale: OnceLock::new(),
            base: Arc::new(image::RgbaImage::from_pixel(
                2400,
                1600,
                image::Rgba([50, 60, 70, 255]),
            )),
            marks: vec![Mark::new(
                Tool::Redact,
                Pos::new(100., 600.),
                Style {
                    width: 128.,
                    ..Default::default()
                },
            )],
            undo: Vec::new(),
            redo: Vec::new(),
            mosaic: Arc::default(),
        };
        let mut preview = PixelationPreview::default();
        preview.update(&doc);
        let start = std::time::Instant::now();
        for step in 1..=24 {
            doc.marks[0].points.push(Pos::new(
                100. + step as f32 * 12.,
                600. + (step as f32 * 0.2).sin() * 40.,
            ));
            preview.update(&doc);
            drop(preview.retired_images(false));
        }
        let elapsed = start.elapsed();
        eprintln!("24 live pixelation brush updates on 2400×1600: {elapsed:?}");
        assert!(
            elapsed < std::time::Duration::from_millis(500),
            "Brush updates block interaction: {elapsed:?}"
        );
    }
}

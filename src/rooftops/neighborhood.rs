//! Facade materials and window surrounds. All ornament stays below the
//! existing roof surface and outside the shared 66 × 62 apartment opening.

use super::{Roof, native_quad};
use crate::rooftops_score::PosterShape;
use gpui_kit::{
    Bounds, IntoElement, Pixels, Point, RenderImage, Styled as _, Window, point, px, size,
};
use std::sync::Arc;
use tiny_skia::{Paint, Pixmap, Rect, Transform};

const FLOOR_HEIGHT: f32 = 82.;
const WINDOW_TOP: f32 = 32.;
const WINDOW_WIDTH: f32 = 66.;

fn visible_floors(top: f32, height: f32) -> std::ops::Range<usize> {
    0..((height - top) / FLOOR_HEIGHT).ceil() as usize
}

pub(super) struct Apartments {
    pub columns: usize,
    spacing: f32,
}

impl Apartments {
    pub fn new(width: f32) -> Self {
        let columns = ((width - 20.) / 74.).floor().max(1.) as usize;
        Self {
            columns,
            spacing: (width - 20.) / columns as f32,
        }
    }

    pub fn window_x(&self, column: usize) -> f32 {
        10. + self.spacing * (column as f32 + 0.5) - WINDOW_WIDTH * 0.5
    }

    pub fn rows(roof_y: f32, height: f32) -> impl Iterator<Item = (usize, f32)> {
        let top = roof_y + WINDOW_TOP;
        visible_floors(top - 7., height).map(move |row| (row, top + row as f32 * FLOOR_HEIGHT))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Neighborhood {
    Brownstone,
    Loft,
    Limestone,
}

struct Palette {
    wall: u32,
    trim: u32,
    shadow: u32,
    shutter: u32,
}

impl Neighborhood {
    pub(super) fn title(self) -> &'static str {
        match self {
            Self::Brownstone => "Brownstones",
            Self::Loft => "SoHo",
            Self::Limestone => "Midtown",
        }
    }

    pub(super) fn checkpoint_palette(
        self,
        seed: usize,
    ) -> crate::rooftops_score::art::CheckpointPalette {
        let palette = self.palette(seed, true);
        crate::rooftops_score::art::CheckpointPalette {
            wall: palette.wall,
            coping: palette.trim,
            frame: palette.trim,
        }
    }
    pub(super) fn for_level(level: usize) -> Self {
        match level {
            0 => Self::Brownstone,
            1 => Self::Loft,
            _ => Self::Limestone,
        }
    }

    fn palette(self, seed: usize, lit: bool) -> Palette {
        let (walls, dark_walls, trim, dark_trim, shadow, shutter) = match self {
            Self::Brownstone => (
                [0x60463f, 0x694b43, 0x5d4544],
                [0x433338, 0x49363a, 0x403239],
                0xa38772,
                0x7a625a,
                0x32282f,
                0x3d4c46,
            ),
            Self::Loft => (
                [0x405456, 0x48595a, 0x3b5053],
                [0x2b3b42, 0x303e44, 0x293940],
                0x8b9e96,
                0x617b78,
                0x202e36,
                0x41545a,
            ),
            Self::Limestone => (
                [0x757063, 0x6d6c64, 0x7b7468],
                [0x4f4d4c, 0x494b4d, 0x524e4d],
                0xb6a68e,
                0x8b8276,
                0x3b3940,
                0x5f5b5b,
            ),
        };
        Palette {
            wall: if lit { walls } else { dark_walls }[seed % 3],
            trim: if lit { trim } else { dark_trim },
            shadow,
            shutter,
        }
    }

    pub(super) fn chimney_colors(self, seed: usize, lit: bool) -> (u32, u32) {
        let palette = self.palette(seed, lit);
        (palette.wall, palette.trim)
    }

    pub(super) fn surface(
        self,
        roof: Roof,
        seed: usize,
        lit: bool,
        height: f32,
        scale: f32,
    ) -> Vec<gpui_kit::PaintQuad> {
        let palette = self.palette(seed, lit);
        let mut quads = Vec::with_capacity(300);
        let mut add = |rect, ink, opacity| {
            quads.push(native_quad(rect, ink, opacity, 0., scale));
        };
        let Roof { x, y, width } = roof;
        let left = x + 5.;
        let right = x + width - 5.;
        add([left, y + 10., right - left, height - y], palette.wall, 1.);

        match self {
            Self::Brownstone => {
                // Projecting cornice, deep shadow and small paired brackets.
                add([x - 3., y, width + 6., 7.], palette.trim, 1.);
                add([x - 1., y + 7., width + 2., 5.], palette.shadow, 1.);
                add([x + 2., y + 12., width - 4., 5.], palette.trim, 0.8);
                for bracket in 0..(width / 24.) as usize {
                    let bx = x + 12. + bracket as f32 * 24.;
                    add([bx, y + 16., 4., 8.], palette.trim, 0.75);
                    add([bx - 1., y + 23., 6., 2.], palette.shadow, 0.8);
                }
                for row in visible_floors(y + 19., height).skip(1) {
                    let band = y + 19. + row as f32 * FLOOR_HEIGHT;
                    add([left, band, right - left, 3.], palette.trim, 0.38);
                }
            }
            Self::Loft => {
                // Repeated cast-iron bays, bolted beams and narrow columns.
                add([x - 3., y, width + 6., 8.], palette.trim, 1.);
                add([x - 1., y + 8., width + 2., 7.], palette.shadow, 1.);
                add([x + 2., y + 15., width - 4., 5.], palette.trim, 0.65);
                for edge in [left, right - 4.] {
                    add([edge, y + 20., 4., height - y - 20.], palette.trim, 0.6);
                    add(
                        [edge + 1., y + 20., 1., height - y - 20.],
                        palette.shadow,
                        0.65,
                    );
                }
                for row in visible_floors(y + 20., height) {
                    let beam = y + 20. + row as f32 * FLOOR_HEIGHT;
                    add([left, beam, right - left, 5.], palette.shadow, 0.85);
                    add([left, beam, right - left, 1.], palette.trim, 0.8);
                    for rivet in 0..((right - left) / 22.) as usize {
                        add(
                            [left + 10. + rivet as f32 * 22., beam + 2., 1.5, 1.5],
                            palette.trim,
                            0.7,
                        );
                    }
                }
            }
            Self::Limestone => {
                // A stepped stone cornice and slender fluted corner piers.
                add([x - 3., y, width + 6., 6.], palette.trim, 1.);
                add([x - 1., y + 6., width + 2., 6.], palette.shadow, 0.95);
                add([x + 2., y + 12., width - 4., 6.], palette.trim, 0.8);
                for edge in [left + 1., right - 6.] {
                    add([edge, y + 18., 5., height - y - 18.], palette.trim, 0.45);
                    for flute in [1., 3.] {
                        add(
                            [edge + flute, y + 20., 0.6, height - y - 20.],
                            palette.shadow,
                            0.55,
                        );
                    }
                }
                for row in visible_floors(y + 20., height).skip(1) {
                    let band = y + 20. + row as f32 * FLOOR_HEIGHT;
                    add([left, band, right - left, 2.], palette.trim, 0.5);
                    add([left, band + 2., right - left, 1.], palette.shadow, 0.5);
                }
            }
        }
        add([x, y - 2., width, 2.], palette.trim, 0.9);
        quads
    }

    fn surround(self, seed: usize, lit: bool) -> Vec<PosterShape> {
        let palette = self.palette(seed, lit);
        let mut quads = Vec::with_capacity(32);
        let mut add = |rect, ink, opacity| {
            quads.push(PosterShape {
                rect,
                ink,
                opacity,
                radius: 0.,
            })
        };
        add([-3., 62., 72., 3.], palette.trim, 1.);
        add([-1., 65., 68., 1.5], palette.shadow, 0.75);
        match self {
            Self::Brownstone => {
                add([-2., -5., 70., 4.], palette.trim, 0.9);
                add([29., -7., 8., 6.], palette.trim, 1.);
                if lit {
                    for sx in [-4., 67.] {
                        add([sx, 2., 3., 58.], palette.shutter, 1.);
                        for row in 0..10 {
                            add([sx, 5. + row as f32 * 5., 3., 0.7], palette.trim, 0.4);
                        }
                    }
                }
            }
            Self::Loft => {
                add([-3., -4., 72., 3.], palette.trim, 1.);
                for sx in [-4., 67.] {
                    add([sx, 0., 3., 61.], palette.trim, 0.9);
                    add([sx + 1., 3., 1., 54.], palette.shadow, 0.8);
                    for sy in [1., 56.] {
                        add([sx - 1., sy, 5., 3.], palette.trim, 1.);
                    }
                }
            }
            Self::Limestone => {
                add([-2., -5., 70., 4.], palette.trim, 1.);
                add([3., -7., 60., 2.], palette.trim, 0.65);
                for sx in [-3., 67.] {
                    add([sx, 0., 2., 60.], palette.shadow, 0.8);
                }
                // A small stepped spandrel medallion sits between floors.
                for (dx, dy, width) in [(23., 68., 20.), (27., 71., 12.), (31., 74., 4.)] {
                    add([dx, dy, width, 2.], palette.trim, 0.55);
                }
            }
        }
        quads
    }

    fn covering(self, seed: usize) -> Vec<PosterShape> {
        let palette = self.palette(seed, false);
        let mut quads = Vec::with_capacity(60);
        let mut add = |rect, ink, opacity| {
            quads.push(PosterShape {
                rect,
                ink,
                opacity,
                radius: 0.,
            })
        };
        add([2., 2., 62., 58.], palette.shutter, 1.);
        match self {
            Self::Brownstone => {
                // Paired painted louvres, with outer hinges and a center latch.
                for sx in [4., 35.] {
                    add([sx, 5., 27., 50.], palette.shadow, 0.7);
                    for row in 0..9 {
                        let sy = 7. + row as f32 * 5.;
                        add([sx + 2., sy, 23., 3.], palette.shutter, 1.);
                        add([sx + 2., sy, 23., 0.7], palette.trim, 0.3);
                    }
                }
                add([32., 2., 2., 58.], palette.shadow, 0.8);
                for sx in [3., 60.] {
                    for sy in [13., 45.] {
                        add([sx, sy, 3., 4.], palette.shadow, 1.);
                    }
                }
                add([30., 31., 6., 2.], palette.trim, 0.8);
            }
            Self::Loft => {
                // Ribbed metal shutters rather than wooden louvres.
                for column in 0..10 {
                    let sx = 4. + column as f32 * 6.;
                    add([sx, 4., 1., 52.], palette.trim, 0.35);
                    add([sx + 3., 4., 1., 52.], palette.shadow, 0.65);
                }
                for sy in [8., 49.] {
                    add([3., sy, 60., 3.], palette.shadow, 0.9);
                    for sx in [8., 27., 39., 58.] {
                        add([sx, sy + 1., 1.5, 1.5], palette.trim, 0.7);
                    }
                }
                add([32., 2., 2., 58.], palette.shadow, 1.);
                add([28., 30., 10., 3.], palette.trim, 0.7);
            }
            Self::Limestone => {
                // Broad horizontal blinds recessed inside a bronze surround.
                for row in 0..10 {
                    let sy = 3. + row as f32 * 5.4;
                    add([3., sy, 60., 1.], palette.trim, 0.35);
                    add([3., sy + 4., 60., 1.], palette.shadow, 0.7);
                }
                for sx in [15., 50.] {
                    add([sx, 3., 0.8, 53.], palette.trim, 0.35);
                }
                add([3., 56., 60., 3.], palette.shadow, 0.9);
            }
        }
        quads
    }
}

#[derive(Clone)]
pub(super) struct Tile {
    image: Arc<RenderImage>,
    size: [f32; 2],
    offset: [f32; 2],
}

pub(super) struct TilePainter<'a> {
    pixels: &'a mut Pixmap,
    transform: Transform,
}

impl TilePainter<'_> {
    fn paint(ink: u32, opacity: f32) -> Paint<'static> {
        let [_, r, g, b] = ink.to_be_bytes();
        let mut paint = Paint::default();
        paint.set_color_rgba8(r, g, b, (opacity * 255.).round() as u8);
        paint.anti_alias = true;
        paint
    }

    pub(super) fn rect(&mut self, rect: [f32; 4], ink: u32) {
        self.rounded(rect, 0., ink, 1.);
    }

    pub(super) fn rounded(&mut self, [x, y, w, h]: [f32; 4], radius: f32, ink: u32, opacity: f32) {
        let Some(rect) = Rect::from_xywh(x, y, w, h) else {
            return;
        };
        let paint = Self::paint(ink, opacity);
        let radius = radius.min(w * 0.5).min(h * 0.5);
        if radius == 0. {
            self.pixels.fill_rect(rect, &paint, self.transform, None);
            return;
        }
        let mut path = tiny_skia::PathBuilder::new();
        path.move_to(x + radius, y);
        path.line_to(x + w - radius, y);
        path.quad_to(x + w, y, x + w, y + radius);
        path.line_to(x + w, y + h - radius);
        path.quad_to(x + w, y + h, x + w - radius, y + h);
        path.line_to(x + radius, y + h);
        path.quad_to(x, y + h, x, y + h - radius);
        path.line_to(x, y + radius);
        path.quad_to(x, y, x + radius, y);
        path.close();
        self.fill(path, &paint);
    }

    pub(super) fn circle(&mut self, x: f32, y: f32, diameter: f32, ink: u32, opacity: f32) {
        let Some(rect) = Rect::from_xywh(x, y, diameter, diameter) else {
            return;
        };
        let mut path = tiny_skia::PathBuilder::new();
        path.push_oval(rect);
        self.fill(path, &Self::paint(ink, opacity));
    }

    pub(super) fn fill_ink(&mut self, path: tiny_skia::PathBuilder, ink: u32) {
        self.fill(path, &Self::paint(ink, 1.));
    }

    pub(super) fn stroke_ink(&mut self, path: tiny_skia::PathBuilder, ink: u32, width: f32) {
        if let Some(path) = path.finish() {
            let stroke = tiny_skia::Stroke {
                width,
                line_cap: tiny_skia::LineCap::Round,
                line_join: tiny_skia::LineJoin::Round,
                ..Default::default()
            };
            self.pixels
                .stroke_path(&path, &Self::paint(ink, 1.), &stroke, self.transform, None);
        }
    }

    pub(super) fn fill(&mut self, path: tiny_skia::PathBuilder, paint: &Paint<'_>) {
        if let Some(path) = path.finish() {
            self.pixels.fill_path(
                &path,
                paint,
                tiny_skia::FillRule::Winding,
                self.transform,
                None,
            );
        }
    }
}

impl Tile {
    fn new(bounds: [f32; 4], shapes: &[PosterShape]) -> Self {
        Self::draw(bounds, |painter| {
            for shape in shapes {
                painter.rounded(shape.rect, shape.radius, shape.ink, shape.opacity);
            }
        })
    }

    pub(super) fn draw(
        [x, y, width, height]: [f32; 4],
        draw: impl FnOnce(&mut TilePainter<'_>),
    ) -> Self {
        const DENSITY: f32 = 2.;
        let w = (width * DENSITY).ceil() as u32;
        let h = (height * DENSITY).ceil() as u32;
        let mut tile = Pixmap::new(w, h).expect("Rooftops tiles have fixed positive dimensions");
        let transform = Transform::from_row(DENSITY, 0., 0., DENSITY, -x * DENSITY, -y * DENSITY);
        draw(&mut TilePainter {
            pixels: &mut tile,
            transform,
        });
        // GPUI's sprite atlas takes straight-alpha BGRA.
        for pixel in tile.data_mut().as_chunks_mut::<4>().0 {
            let rgb =
                tiny_skia::PremultipliedColorU8::from_rgba(pixel[0], pixel[1], pixel[2], pixel[3])
                    .expect("Pixmap pixels are premultiplied")
                    .demultiply();
            *pixel = [rgb.blue(), rgb.green(), rgb.red(), rgb.alpha()];
        }
        let frame = image::RgbaImage::from_raw(w, h, tile.take()).expect("Fixed size tile");
        Self {
            image: Arc::new(RenderImage::new(vec![image::Frame::new(frame)])),
            size: [width, height],
            offset: [x, y],
        }
    }

    pub(super) fn paint(
        &self,
        origin: Point<Pixels>,
        scale: f32,
        clip: Bounds<Pixels>,
        window: &mut Window,
    ) {
        let bounds = Bounds::new(
            origin,
            size(px(self.size[0] * scale), px(self.size[1] * scale)),
        );
        let _ = window.paint_image(
            clip,
            bounds,
            gpui_kit::Corners::default(),
            self.image.clone(),
            0,
            false,
        );
    }

    pub(super) fn paint_at(
        &self,
        origin: Point<Pixels>,
        [x, y]: [f32; 2],
        scale: f32,
        window: &mut Window,
    ) {
        let origin = origin
            + point(
                px((x + self.offset[0]) * scale),
                px((y + self.offset[1]) * scale),
            );
        let clip = Bounds::new(
            origin,
            size(px(self.size[0] * scale), px(self.size[1] * scale)),
        );
        self.paint(origin, scale, clip, window);
    }
}

const ROOM_BOUNDS: [f32; 4] = [-6., -8., 78., 86.];

fn masonry(neighborhood: Neighborhood, seed: usize, lit: bool) -> Tile {
    let palette = neighborhood.palette(seed, lit);
    let (brick, course) = match neighborhood {
        Neighborhood::Brownstone if seed % 3 == 1 => (26., 9.),
        Neighborhood::Brownstone => (43., 18.),
        Neighborhood::Loft => (30., 11.),
        Neighborhood::Limestone => (48., 22.),
    };
    let width = brick * 4.;
    let height = course * 8.;
    let mut shapes = Vec::new();
    let mut add = |rect, ink, opacity| {
        shapes.push(PosterShape {
            rect,
            ink,
            opacity,
            radius: 0.,
        })
    };
    for row in 0..8 {
        let y = row as f32 * course;
        add([0., y, width, 0.8], palette.shadow, 0.42);
        add([0., y + 0.8, width, 0.6], palette.trim, 0.1);
        let stagger = if row % 2 == 0 { 0. } else { brick * 0.5 };
        for col in -1_i32..4 {
            let x = col as f32 * brick + stagger;
            add([x, y, 0.8, course], palette.shadow, 0.35);
            if (row + col.rem_euclid(4) as usize + seed).is_multiple_of(4) {
                add(
                    [x + 1., y + 2., brick - 2., course - 3.],
                    palette.trim,
                    0.045,
                );
            }
        }
    }
    Tile::new([0., 0., width, height], &shapes)
}

type SharedQuads = Arc<[gpui_kit::PaintQuad]>;
#[derive(PartialEq, Eq, Hash)]
struct SurfaceKey {
    neighborhood: Neighborhood,
    seed: usize,
    width: u32,
    y: u32,
    lit: bool,
}

/// Repeating room and masonry tiles stay in the sprite atlas while the camera
/// moves or the window resizes. Only the few facade trim quads depend on size.
#[derive(Default)]
pub(super) struct FacadeArt {
    size: (f32, f32),
    surfaces: std::collections::HashMap<SurfaceKey, SharedQuads>,
    materials: std::collections::HashMap<(Neighborhood, usize, bool), Tile>,
    surrounds: std::collections::HashMap<(Neighborhood, usize), Tile>,
    rooms: std::collections::HashMap<(Neighborhood, usize), Tile>,
}

impl FacadeArt {
    pub(super) fn resize(&mut self, height: f32, scale: f32) {
        let height = (height / FLOOR_HEIGHT).ceil() * FLOOR_HEIGHT;
        if self.size != (height, scale) {
            self.surfaces.clear();
        }
        self.size = (height, scale);
    }

    pub(super) fn wall(
        &mut self,
        neighborhood: Neighborhood,
        roof: Roof,
        seed: usize,
        lit: bool,
        x: f32,
        height: f32,
    ) -> impl IntoElement + use<> {
        let (cache_height, scale) = self.size;
        let trim = self
            .surfaces
            .entry(SurfaceKey {
                neighborhood,
                seed: seed % 3,
                width: roof.width.to_bits(),
                y: roof.y.to_bits(),
                lit,
            })
            .or_insert_with(|| {
                neighborhood
                    .surface(Roof { x: 0., ..roof }, seed, lit, cache_height, scale)
                    .into()
            })
            .clone();
        let material = self
            .materials
            .entry((neighborhood, seed % 3, lit))
            .or_insert_with(|| masonry(neighborhood, seed % 3, lit))
            .clone();

        gpui_kit::canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let origin = bounds.origin + point(px(x * scale), px(0.));
                let paint_quad = |quad: &gpui_kit::PaintQuad, window: &mut Window| {
                    let mut quad = quad.clone();
                    quad.bounds.origin += origin;
                    window.paint_quad(quad);
                };
                // The first quad is the base wall. Use its paint order around the texture.
                paint_quad(&trim[0], window);
                let top = roof.y + 21.;
                let width = roof.width - 10.;
                let [tile_width, tile_height] = material.size;
                let clip = Bounds::new(
                    origin + point(px(5. * scale), px(top * scale)),
                    size(px(width * scale), px((height - top).max(0.) * scale)),
                );
                for row in 0..((height - top) / tile_height).ceil() as usize {
                    for col in 0..(width / tile_width).ceil() as usize {
                        let position = clip.origin
                            + point(
                                px(col as f32 * tile_width * scale),
                                px(row as f32 * tile_height * scale),
                            );
                        material.paint(position, scale, clip, window);
                    }
                }
                for quad in &trim[1..] {
                    paint_quad(quad, window);
                }
            },
        )
        .absolute()
        .inset_0()
    }

    pub(super) fn open_frame(&mut self, neighborhood: Neighborhood, seed: usize) -> Tile {
        self.surrounds
            .entry((neighborhood, seed % 3))
            .or_insert_with(|| {
                let color = neighborhood.palette(seed, true).trim;
                let mut shapes = neighborhood.surround(seed, true);
                add_frame(&mut shapes, color);
                Tile::new(ROOM_BOUNDS, &shapes)
            })
            .clone()
    }

    pub(super) fn closed_room(&mut self, neighborhood: Neighborhood, seed: usize) -> Tile {
        self.rooms
            .entry((neighborhood, seed % 3))
            .or_insert_with(|| {
                let mut shapes = neighborhood.covering(seed);
                shapes.extend(neighborhood.surround(seed, false));
                add_frame(&mut shapes, neighborhood.palette(seed, false).trim);
                Tile::new(ROOM_BOUNDS, &shapes)
            })
            .clone()
    }
}

fn add_frame(shapes: &mut Vec<PosterShape>, ink: u32) {
    for rect in [
        [0., 0., 66., 2.],
        [0., 60., 66., 2.],
        [0., 2., 2., 58.],
        [64., 2., 2., 58.],
    ] {
        shapes.push(PosterShape {
            rect,
            ink,
            opacity: 1.,
            radius: 0.,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floor_layout_continues_through_the_bottom_of_a_tall_viewport() {
        let rows = |height| Apartments::rows(385., height).collect::<Vec<_>>();
        let normal = rows(565.);
        let tall = rows(1260.);
        assert_eq!(normal, tall[..normal.len()]);
        assert!(tall.len() > 8);
        assert!(1260. - tall.last().unwrap().1 < FLOOR_HEIGHT);
    }
}

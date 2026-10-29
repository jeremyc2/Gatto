//! Facade materials and window surrounds. All ornament stays below the
//! existing roof surface and outside the shared 66 × 62 apartment opening.

use super::{Roof, native_quad};

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

    pub(super) fn frame_color(self, lit: bool) -> u32 {
        self.palette(0, lit).trim
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

        // Staggered joints are anchored to each building, never to the camera.
        // Wide dressed blocks distinguish stone from the finer brick courses.
        let (block_width, course_height) = match self {
            Self::Brownstone if seed % 3 == 1 => (26., 9.),
            Self::Brownstone => (43., 18.),
            Self::Loft => (30., 11.),
            Self::Limestone => (48., 22.),
        };
        for row in 0..((height - y) / course_height).ceil().max(0.) as usize {
            let line_y = y + 21. + row as f32 * course_height;
            if line_y >= height {
                break;
            }
            add([left, line_y, right - left, 0.8], palette.shadow, 0.42);
            add([left, line_y + 0.8, right - left, 0.6], palette.trim, 0.1);
            let offset = if row % 2 == 0 { 0. } else { block_width * 0.5 };
            for col in 0..(width / block_width).ceil() as usize + 1 {
                let joint = left + col as f32 * block_width + offset;
                if joint > right - 1. {
                    break;
                }
                let joint_height = course_height.min(height - line_y);
                add([joint, line_y, 0.8, joint_height], palette.shadow, 0.35);
                if (row * 7 + col * 3 + seed).is_multiple_of(5) {
                    add(
                        [
                            joint + 1.,
                            line_y + 2.,
                            (right - joint - 2.).min(block_width - 2.).max(0.),
                            (joint_height - 3.).max(0.),
                        ],
                        palette.trim,
                        0.045,
                    );
                }
            }
        }

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
                for row in 1..4 {
                    let band = y + 19. + row as f32 * 82.;
                    if band < height {
                        add([left, band, right - left, 3.], palette.trim, 0.38);
                    }
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
                for row in 0..4 {
                    let beam = y + 20. + row as f32 * 82.;
                    if beam >= height {
                        break;
                    }
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
                for row in 1..4 {
                    let band = y + 20. + row as f32 * 82.;
                    if band < height {
                        add([left, band, right - left, 2.], palette.trim, 0.5);
                        add([left, band + 2., right - left, 1.], palette.shadow, 0.5);
                    }
                }
            }
        }
        add([x, y - 2., width, 2.], palette.trim, 0.9);
        quads
    }

    pub(super) fn surround(self, seed: usize, lit: bool, scale: f32) -> Vec<gpui_kit::PaintQuad> {
        let palette = self.palette(seed, lit);
        let mut quads = Vec::with_capacity(32);
        let mut add = |rect, ink, opacity| quads.push(native_quad(rect, ink, opacity, 0., scale));
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

    pub(super) fn covering(self, seed: usize, scale: f32) -> Vec<gpui_kit::PaintQuad> {
        let palette = self.palette(seed, false);
        let mut quads = Vec::with_capacity(60);
        let mut add = |rect, ink, opacity| quads.push(native_quad(rect, ink, opacity, 0., scale));
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

type SharedQuads = std::sync::Arc<[gpui_kit::PaintQuad]>;
#[derive(PartialEq, Eq, Hash)]
struct SurfaceKey {
    neighborhood: Neighborhood,
    seed: usize,
    width: u32,
    y: u32,
    lit: bool,
}

/// Static facade geometry belongs to this game window. Resizing discards the
/// previous size; camera motion only changes the element's placement.
#[derive(Default)]
pub(super) struct FacadeArt {
    size: (f32, f32),
    surfaces: std::collections::HashMap<SurfaceKey, SharedQuads>,
    surrounds: std::collections::HashMap<
        (Neighborhood, usize, bool),
        std::sync::Arc<[gpui_kit::PaintQuad]>,
    >,
    coverings:
        std::collections::HashMap<(Neighborhood, usize), std::sync::Arc<[gpui_kit::PaintQuad]>>,
}

impl FacadeArt {
    pub(super) fn resize(&mut self, height: f32, scale: f32) {
        if self.size != (height, scale) {
            *self = Self {
                size: (height, scale),
                ..Self::default()
            };
        }
    }
    pub(super) fn surface(
        &mut self,
        neighborhood: Neighborhood,
        roof: Roof,
        seed: usize,
        lit: bool,
    ) -> std::sync::Arc<[gpui_kit::PaintQuad]> {
        let (height, scale) = self.size;
        self.surfaces
            .entry(SurfaceKey {
                neighborhood,
                seed,
                width: roof.width.to_bits(),
                y: roof.y.to_bits(),
                lit,
            })
            .or_insert_with(|| {
                neighborhood
                    .surface(Roof { x: 0., ..roof }, seed, lit, height, scale)
                    .into()
            })
            .clone()
    }
    pub(super) fn surround(
        &mut self,
        neighborhood: Neighborhood,
        seed: usize,
        lit: bool,
    ) -> std::sync::Arc<[gpui_kit::PaintQuad]> {
        self.surrounds
            .entry((neighborhood, seed % 3, lit))
            .or_insert_with(|| neighborhood.surround(seed, lit, self.size.1).into())
            .clone()
    }
    pub(super) fn covering(
        &mut self,
        neighborhood: Neighborhood,
        seed: usize,
    ) -> std::sync::Arc<[gpui_kit::PaintQuad]> {
        self.coverings
            .entry((neighborhood, seed % 3))
            .or_insert_with(|| neighborhood.covering(seed, self.size.1).into())
            .clone()
    }
}

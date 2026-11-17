//! The painted Art Deco façades used in the menu and harbor fly-through.

use super::*;
use neighborhood::Tile;
use projection::Camera;

const SHAFT_TOP: f32 = 245.;
const FLOOR: f32 = 12.;
const SHAFT_HEIGHT: f32 = FLOOR * 6.;
const BASE_HEIGHT: f32 = 58.;
const GROUND: f32 = SHAFT_TOP + SHAFT_HEIGHT + BASE_HEIGHT;
pub(super) const DEPTH: f32 = 835.;
const SCALE: f32 = 0.68;
const STONE: u32 = 0x414358;
const STEEL: u32 = 0x42465f;
const GLASS: u32 = 0xe0bf92;

#[derive(Clone, Copy)]
enum Entrance {
    Limestone,
    Steel,
}

const EMPIRE_X: f32 = 0.128 * WIDTH;
const CHRYSLER_X: f32 = 0.877 * WIDTH;
const ARCHES: [(f32, f32, f32, u32); 5] = [
    (31., 213., 27., 0x87829c),
    (27., 200., 25., 0x9794ae),
    (22., 188., 24., 0xa6a4b7),
    (16., 176., 22., 0xacaabc),
    (10., 163., 19., 0xb8b2c0),
];
#[derive(Default)]
struct Facade(Vec<art::ArtPath>);

impl Facade {
    fn rect(&mut self, bounds: [f32; 4], ink: u32) {
        quad(&mut self.0, bounds, ink, 1.);
    }

    fn tint(&mut self, bounds: [f32; 4], ink: u32, opacity: f32) {
        quad(&mut self.0, bounds, ink, opacity);
    }

    fn polygon(&mut self, points: Vec<[f32; 2]>, ink: u32, opacity: f32) {
        shape(&mut self.0, points, ink, opacity);
    }
}

struct Landmark {
    tile: Tile,
    front: Facade,
}

fn landmark(width: f32, ink: u32, entrance: Entrance, crown: impl FnOnce(&mut Facade)) -> Landmark {
    let mut front = Facade::default();
    crown(&mut front);
    facade(
        &mut front,
        [-width * 0.5, SHAFT_TOP, width, SHAFT_HEIGHT],
        ink,
        0,
    );
    frontage(&mut front, width, entrance, ink);
    let base_width = width + 16.;
    let tile = Tile::draw(
        [-base_width * 0.5, 54., base_width, GROUND - 54.],
        |painter| {
            for shape in &front.0 {
                let mut path = tiny_skia::PathBuilder::new();
                path.move_to(shape.points[0][0], shape.points[0][1]);
                for &[x, y] in &shape.points[1..] {
                    path.line_to(x, y);
                }
                path.close();
                let [_, r, g, b] = shape.ink.to_be_bytes();
                let mut paint = tiny_skia::Paint::default();
                paint.set_color_rgba8(r, g, b, (shape.opacity * 255.).round() as u8);
                paint.anti_alias = true;
                painter.fill(path, &paint);
            }
        },
    );
    Landmark { tile, front }
}

static EMPIRE: LazyLock<Landmark> = LazyLock::new(empire_state);
static CHRYSLER: LazyLock<Landmark> = LazyLock::new(chrysler);

fn opening_placement(view: View, x: f32) -> ([f32; 2], f32) {
    let (foot, zoom) = view.ground_billboard(x, shore::STREET_DEPTH, shore::street_grade(view));
    (foot, zoom * 0.90)
}

fn placement(view: View, x: f32) -> ([f32; 2], f32) {
    let grade = shore::street_grade(view);
    let ([screen_x, cursor_y], zoom) = view.ground_billboard(x, shore::STREET_DEPTH, grade);
    let neutral = View {
        aim: Aim::default(),
        ..view
    };
    let ([_, reference_y], _) = neutral.ground_billboard(x, shore::STREET_DEPTH, grade);
    let foot = neutral.project(neutral.x_at_depth(x, DEPTH), DEPTH, grade)[1];
    // Keep the small authored response to the pointer at the deeper anchor.
    ([screen_x, foot + (cursor_y - reference_y)], zoom * SCALE)
}

pub(super) fn scene(view: View, scale: f32) -> impl IntoElement {
    gpui_kit::canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            for (building, x) in [(&*EMPIRE, EMPIRE_X), (&*CHRYSLER, CHRYSLER_X)] {
                let ([x, foot], zoom) = opening_placement(view, x);
                let origin = bounds.origin + gpui_kit::point(px(x * scale), px(foot * scale));
                building
                    .tile
                    .paint_at(origin, [0., -GROUND], scale * zoom, window);
            }
        },
    )
    .absolute()
    .inset_0()
}

pub(super) fn project(
    original: View,
    view: View,
    camera: Camera,
    inland: f32,
) -> Vec<art::ArtPath> {
    let mut paths = Vec::new();
    if camera.eye[2] >= depth(inland) {
        return paths;
    }
    for (building, position) in [(&*EMPIRE, EMPIRE_X), (&*CHRYSLER, CHRYSLER_X)] {
        let ([old_x, old_y], old_scale) = opening_placement(original, position);
        let ([new_x, new_y], new_scale) = placement(original, position);
        for shape in &building.front.0 {
            let points = shape.points.iter().map(|&[x, y]| {
                let opening = original.lift(
                    [old_x + x * old_scale, old_y + (y - GROUND) * old_scale],
                    shore::STREET_DEPTH,
                );
                let city = original.lift(
                    [new_x + x * new_scale, new_y + (y - GROUND) * new_scale],
                    DEPTH,
                );
                projection::mix(opening, city, inland)
            });
            view.face(&mut paths, points, shape.ink, shape.opacity);
        }
    }
    paths
}

pub(super) fn depth(inland: f32) -> f32 {
    shore::STREET_DEPTH + (DEPTH - shore::STREET_DEPTH) * inland
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_pointer_movement_keeps_landmarks_near_their_authored_positions() {
        for x in [EMPIRE_X, CHRYSLER_X] {
            for end in [-1., 1.] {
                let view = View::new(WIDTH, HEIGHT * HORIZON, Aim::new(end, 0.));
                let ([moved, _], _) = placement(view, x);
                let displacement = moved - x;
                assert!((16. ..25.).contains(&(displacement * end)));
            }
        }
    }
}

fn frontage(painter: &mut Facade, width: f32, entrance: Entrance, shaft: u32) {
    let (stone, trim, metal, portal) = match entrance {
        Entrance::Limestone => (0x5c5661, 0x877881, 0x95807d, 14.),
        Entrance::Steel => (0x474b60, 0x77778a, 0x95909c, 12.),
    };
    let half = width * 0.5;
    let rect = |painter: &mut Facade, [x, y, w, h]: [f32; 4], ink| {
        painter.rect([x, y + GROUND, w, h], ink);
    };
    let tint = |painter: &mut Facade, [x, y, w, h]: [f32; 4], ink, alpha| {
        painter.tint([x, y + GROUND, w, h], ink, alpha);
    };
    rect(painter, [-half, -BASE_HEIGHT, width, BASE_HEIGHT], stone);
    rect(painter, [-half, -BASE_HEIGHT, width, 7.], shaft);
    tint(painter, [-half, -BASE_HEIGHT + 4., width, 3.], stone, 0.5);
    tint(
        painter,
        [width * 0.23, -BASE_HEIGHT, width * 0.27, 15.],
        0x181c31,
        0.36,
    );
    tint(painter, [-half, -BASE_HEIGHT, 2., 15.], 0xaaa3ae, 0.27);
    for x in window_bays(width) {
        tint(
            painter,
            [-half + x - 1.2, -BASE_HEIGHT, 0.65, 7.],
            0x9a91a3,
            0.29,
        );
    }
    rect(painter, [-half - 5., -43., width + 10., 43.], stone);
    tint(
        painter,
        [half * 0.62, -43., half * 0.38 + 5., 43.],
        0x22263c,
        0.38,
    );
    for (y, extension, height) in [
        (-54., 0.5, 1.15),
        (-44., 5.5, 2.4),
        (-21., 5., 1.1),
        (-3.5, 6.5, 3.5),
    ] {
        tint(
            painter,
            [-half - extension, y, width + 2. * extension, height],
            trim,
            0.72,
        );
    }
    // Tall, recessed ground-floor glazing replaces the repeating office floors.
    for side in [-1., 1.] {
        for i in 0..3 {
            let x = side * (portal + 5. + i as f32 * (half - portal - 8.) / 3.);
            rect(painter, [x - 2.0, -39., 4., 11.], 0x323549);
            tint(painter, [x - 1.25, -38.5, 2.5, 7.8], GLASS, 0.35);
            rect(painter, [x - 2.65, -17., 5.3, 12.5], 0x262a3d);
            tint(painter, [x - 1.9, -16.5, 3.8, 10.7], GLASS, 0.27);
        }
    }
    let crown = match entrance {
        Entrance::Limestone => -31.,
        Entrance::Steel => -38.,
    };
    rect(
        painter,
        [-portal - 2., crown, (portal + 2.) * 2., -crown - 3.5],
        metal,
    );
    rect(
        painter,
        [-portal, crown + 1.5, portal * 2., -crown - 5.],
        0x252939,
    );
    for bay in [-1., 0., 1.] {
        let x = bay * (portal * 0.63);
        let pane = portal * 0.5;
        tint(
            painter,
            [x - pane * 0.5, crown + 4., pane, -crown - 11.],
            GLASS,
            0.42,
        );
        rect(painter, [x - pane * 0.5, -13., pane, 1.1], metal);
    }
    rect(
        painter,
        [-portal - 3.5, -4.1, (portal + 3.5) * 2., 1.6],
        trim,
    );
    rect(painter, [-half - 7., -1.5, width + 14., 1.5], 0x343345);
}

fn facade(painter: &mut Facade, [x, y, width, height]: [f32; 4], ink: u32, seed: usize) {
    painter.rect([x, y, width, height], ink);
    painter.tint([x + width * 0.73, y, width * 0.27, height], 0x181c31, 0.36);
    painter.tint([x, y, 2., height], 0xaaa3ae, 0.27);
    let floors = (height / FLOOR).floor() as usize;
    for (bay, offset) in window_bays(width).enumerate() {
        let wx = x + offset;
        painter.tint([wx - 1.2, y, 0.65, height], 0x9a91a3, 0.29);
        for floor in 0..floors {
            let wy = y + (floor as f32 + 0.5) * FLOOR - 2.7;
            let lit = (seed + bay * (5 + floor) + floor * 11) % 17 < 6;
            painter.tint([wx, wy, 3.2, 5.4], GLASS, if lit { 0.7 } else { 0.22 });
        }
    }
}

fn window_bays(width: f32) -> impl Iterator<Item = f32> {
    let bays = ((width - 5.) / 10.).floor().max(1.) as usize;
    let pitch = (width - 6.) / bays as f32;
    (0..bays).map(move |bay| 1.4 + (bay as f32 + 0.5) * pitch)
}

fn empire_state() -> Landmark {
    landmark(90., STONE, Entrance::Limestone, |painter| {
        // A stepped limestone shaft and the narrow, illuminated mooring mast.
        for (width, top, bottom, ink, seed) in [
            (90., 214., SHAFT_TOP, STONE, 1),
            (74., 180., 214., 0x47495f, 3),
            (57., 153., 180., 0x4c4c64, 4),
            (39., 134., 153., 0x59586e, 7),
        ] {
            facade(painter, [-width * 0.5, top, width, bottom - top], ink, seed);
            if width < 90. {
                painter.tint([-width * 0.5, top, width, 1.6], 0xc6b0a8, 0.51);
            }
        }
        for (width, y, height, color) in [
            (31., 127., 7., 0x777289),
            (25., 116., 11., 0x8c8293),
            (18., 105., 12., 0xa39a9c),
            (11., 96., 10., 0xa89e9e),
            (6., 84., 13., 0x95899a),
            (2.6, 64., 21., 0xb6a6a9),
        ] {
            painter.rect([-width * 0.5, y, width, height], color);
        }
        for x in [-7., -2., 3.] {
            painter.rect([x, 108., 2.1, 14.], 0xdac3a8);
        }
        painter.polygon(
            (0..16)
                .map(|i| {
                    let angle = i as f32 * std::f32::consts::TAU / 16.;
                    [1.5 * angle.cos(), 63.5 + 1.5 * angle.sin()]
                })
                .collect(),
            0xf4a47f,
            0.9,
        );
    })
}

fn chrysler() -> Landmark {
    landmark(72., STEEL, Entrance::Steel, |painter| {
        facade(painter, [-36., 211., 72., SHAFT_TOP - 211.], STEEL, 5);
        painter.rect([-36., 211., 72., 3.], 0x77738b);
        painter.rect([-25., 202., 50., 11.], 0x6e738d);
        // The stainless steel sunburst crown has nested arches and dart windows.
        for (rx, base, height, metal) in ARCHES {
            painter.polygon(
                (0..=20)
                    .map(|step| {
                        let angle = step as f32 * std::f32::consts::PI / 20.;
                        [rx * angle.cos(), base - height * angle.sin()]
                    })
                    .collect(),
                metal,
                1.,
            );
            for x in [-rx * 0.56, 0., rx * 0.56] {
                painter.polygon(
                    vec![[x - 2.5, base - 3.], [x, base - 10.], [x + 2.5, base - 3.]],
                    0x43465f,
                    1.,
                );
            }
        }
        painter.rect([-1.1, 115., 2.2, 31.], 0xb8afbc);
        painter.rect([-2.6, 137., 5.2, 12.], 0x9c95aa);
    })
}

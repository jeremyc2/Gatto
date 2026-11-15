//! The harbor's Art Deco landmarks, each painted once from crown to entrance.

use super::*;
use neighborhood::{Tile, TilePainter};

const SHAFT_TOP: f32 = 245.;
const FLOOR: f32 = 12.;
const SHAFT_HEIGHT: f32 = FLOOR * 6.;
const BASE_HEIGHT: f32 = 58.;
const GROUND: f32 = SHAFT_TOP + SHAFT_HEIGHT + BASE_HEIGHT;
const STONE: u32 = 0x414358;
const STEEL: u32 = 0x42465f;
const GLASS: u32 = 0xe0bf92;

#[derive(Clone, Copy)]
enum Entrance {
    Limestone,
    Steel,
}

fn landmark(
    width: f32,
    ink: u32,
    entrance: Entrance,
    crown: impl FnOnce(&mut TilePainter<'_>),
) -> Tile {
    let base_width = width + 16.;
    Tile::draw(
        [-base_width * 0.5, 54., base_width, GROUND - 54.],
        |painter| {
            crown(painter);
            facade(
                painter,
                [-width * 0.5, SHAFT_TOP, width, SHAFT_HEIGHT],
                ink,
                0,
            );
            frontage(painter, width, entrance, ink);
        },
    )
}

pub(super) fn scene(view: View, scale: f32) -> impl IntoElement {
    static EMPIRE: LazyLock<Tile> = LazyLock::new(empire_state);
    static CHRYSLER: LazyLock<Tile> = LazyLock::new(chrysler);
    gpui_kit::canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            for (building, position) in [(&*EMPIRE, 0.128), (&*CHRYSLER, 0.877)] {
                let ([x, foot], zoom) = view.ground_billboard(
                    view.width * position,
                    shore::STREET_DEPTH,
                    shore::street_grade(view),
                );
                let origin = bounds.origin + gpui_kit::point(px(x * scale), px(foot * scale));
                building.paint_at(origin, [0., -GROUND], scale * zoom * 0.90, window);
            }
        },
    )
    .absolute()
    .inset_0()
}

fn frontage(painter: &mut TilePainter<'_>, width: f32, entrance: Entrance, shaft: u32) {
    let (stone, trim, metal, portal) = match entrance {
        Entrance::Limestone => (0x5c5661, 0x877881, 0x95807d, 14.),
        Entrance::Steel => (0x474b60, 0x77778a, 0x95909c, 12.),
    };
    let half = width * 0.5;
    let rect = |painter: &mut TilePainter<'_>, [x, y, w, h]: [f32; 4], ink| {
        painter.rect([x, y + GROUND, w, h], ink);
    };
    let tint = |painter: &mut TilePainter<'_>, [x, y, w, h]: [f32; 4], ink, alpha| {
        painter.rounded([x, y + GROUND, w, h], 0., ink, alpha);
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

fn facade(painter: &mut TilePainter<'_>, [x, y, width, height]: [f32; 4], ink: u32, seed: usize) {
    painter.rect([x, y, width, height], ink);
    painter.rounded(
        [x + width * 0.73, y, width * 0.27, height],
        0.,
        0x181c31,
        0.36,
    );
    painter.rounded([x, y, 2., height], 0., 0xaaa3ae, 0.27);
    let floors = (height / FLOOR).floor() as usize;
    for (bay, offset) in window_bays(width).enumerate() {
        let wx = x + offset;
        painter.rounded([wx - 1.2, y, 0.65, height], 0., 0x9a91a3, 0.29);
        for floor in 0..floors {
            let wy = y + (floor as f32 + 0.5) * FLOOR - 2.7;
            let lit = (seed + bay * (5 + floor) + floor * 11) % 17 < 6;
            painter.rounded([wx, wy, 3.2, 5.4], 0., GLASS, if lit { 0.7 } else { 0.22 });
        }
    }
}

fn window_bays(width: f32) -> impl Iterator<Item = f32> {
    let bays = ((width - 5.) / 10.).floor().max(1.) as usize;
    let pitch = (width - 6.) / bays as f32;
    (0..bays).map(move |bay| 1.4 + (bay as f32 + 0.5) * pitch)
}

fn empire_state() -> Tile {
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
                painter.rounded([-width * 0.5, top, width, 1.6], 0., 0xc6b0a8, 0.51);
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
        painter.circle(-1.5, 62., 3., 0xf4a47f, 0.9);
    })
}

fn chrysler() -> Tile {
    landmark(72., STEEL, Entrance::Steel, |painter| {
        facade(painter, [-36., 211., 72., SHAFT_TOP - 211.], STEEL, 5);
        painter.rect([-36., 211., 72., 3.], 0x77738b);
        painter.rect([-25., 202., 50., 11.], 0x6e738d);
        // The stainless steel sunburst crown has nested arches and dart windows.
        for (rx, base, height, metal) in [
            (31., 213., 27., 0x87829c),
            (27., 200., 25., 0x9794ae),
            (22., 188., 24., 0xa6a4b7),
            (16., 176., 22., 0xacaabc),
            (10., 163., 19., 0xb8b2c0),
        ] {
            let mut path = tiny_skia::PathBuilder::new();
            path.move_to(rx, base);
            for step in 1..=20 {
                let angle = step as f32 * std::f32::consts::PI / 20.;
                path.line_to(rx * angle.cos(), base - height * angle.sin());
            }
            path.close();
            painter.fill_ink(path, metal);
            for x in [-rx * 0.56, 0., rx * 0.56] {
                let mut dart = tiny_skia::PathBuilder::new();
                dart.move_to(x - 2.5, base - 3.);
                dart.line_to(x, base - 10.);
                dart.line_to(x + 2.5, base - 3.);
                dart.close();
                painter.fill_ink(dart, 0x43465f);
            }
        }
        painter.rect([-1.1, 115., 2.2, 31.], 0xb8afbc);
        painter.rect([-2.6, 137., 5.2, 12.], 0x9c95aa);
    })
}

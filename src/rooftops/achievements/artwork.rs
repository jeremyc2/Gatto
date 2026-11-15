use super::{Achievement, LazyLock};
use crate::rooftops::neighborhood::{Tile, TilePainter};
use tiny_skia::{
    Color, GradientStop, LinearGradient, Paint, PathBuilder, Point, Rect, SpreadMode, Transform,
};

pub(super) const WIDTH: f32 = 96.;
pub(super) const HEIGHT: f32 = 80.;
const CENTER: [f32; 2] = [48., 37.];
const LINE: f32 = 2.2;

#[derive(Clone, Copy)]
struct Palette {
    rim: [u32; 3],
    face: [u32; 3],
    ink: u32,
    accent: u32,
}

const SILVER: Palette = Palette {
    rim: [0x8f9bb9, 0x4c5773, 0x76819e],
    face: [0x343d55, 0x272e44, 0x1d2437],
    ink: 0xadbad8,
    accent: 0x8898bd,
};

const GOLD: Palette = Palette {
    rim: [0xffedba, 0xad663d, 0xf4bf76],
    face: [0x5a4253, 0x392c43, 0x27213a],
    ink: 0xffeac2,
    accent: 0xf8bb76,
};

pub(super) fn medal(award: Achievement, unlocked: bool) -> Tile {
    static MEDALS: LazyLock<[[Tile; Achievement::ALL.len()]; 2]> = LazyLock::new(|| {
        let mut awards = Achievement::ALL;
        awards.sort_by_key(|award| *award as u8);
        [false, true].map(|unlocked| awards.map(|award| draw_medal(award, unlocked)))
    });
    MEDALS[usize::from(unlocked)][award as usize].clone()
}

fn draw_medal(award: Achievement, unlocked: bool) -> Tile {
    let palette = if unlocked { GOLD } else { SILVER };
    Tile::draw([0., 0., WIDTH, HEIGHT], |p| {
        if unlocked {
            p.circle(14., 3., 68., 0xffc779, 0.14);
            for x in [13., 83.] {
                star(p, [x, 33.], 4.9, palette.ink);
            }
            for x in [19., 77.] {
                star(p, [x, 13.], 2.5, palette.accent);
                dot(p, [x, 58.], 1.3, palette.accent);
            }
        }
        dot(p, [48., 39.], 31.5, 0x16182c);
        gradient_disc(p, CENTER, 31., palette.rim);
        dot(p, CENTER, 27.8, if unlocked { 0x815347 } else { 0x464f69 });
        gradient_disc(p, CENTER, 25.9, palette.face);

        match award {
            Achievement::HomeByMoonlight => home(p, palette),
            Achievement::CleanSweep => broom(p, palette),
            Achievement::Pacifist => dove(p, palette),
            Achievement::NightExpress => stopwatch(p, palette),
            Achievement::AllNineIntact => cat(p, palette),
            Achievement::EmptyPaws => empty_paws(p, palette),
            Achievement::DoubleFeature => double_feature(p, palette),
            Achievement::BorrowedLift => rooftop_jump(p, palette),
        }

        if unlocked {
            let seal = [48., 66.5];
            dot(p, seal, 9., 0x352435);
            gradient_disc(p, seal, 7.6, [0xffedb5, 0xf2bb72, 0xcd8a4f]);
            line(
                p,
                &[[44.7, 66.3], [47., 68.6], [51.4, 64.2]],
                0x51303c,
                1.85,
            );
        }
    })
}

fn gradient_disc(p: &mut TilePainter<'_>, [x, y]: [f32; 2], r: f32, colors: [u32; 3]) {
    let mut shape = PathBuilder::new();
    shape.push_oval(Rect::from_xywh(x - r, y - r, 2. * r, 2. * r).expect("positive medal radius"));
    let stops = [0., 0.58, 1.]
        .into_iter()
        .zip(colors)
        .map(|(offset, color)| {
            let [_, red, green, blue] = color.to_be_bytes();
            GradientStop::new(offset, Color::from_rgba8(red, green, blue, 255))
        })
        .collect();
    let paint = Paint {
        shader: LinearGradient::new(
            Point::from_xy(x - r * 0.45, y - r),
            Point::from_xy(x + r * 0.45, y + r),
            stops,
            SpreadMode::Pad,
            Transform::identity(),
        )
        .expect("medal gradient has distinct endpoints"),
        anti_alias: true,
        ..Paint::default()
    };
    p.fill(shape, &paint);
}

fn dot(p: &mut TilePainter<'_>, [x, y]: [f32; 2], r: f32, ink: u32) {
    p.circle(x - r, y - r, 2. * r, ink, 1.);
}

fn ring(p: &mut TilePainter<'_>, [x, y]: [f32; 2], r: f32, ink: u32) {
    let mut path = PathBuilder::new();
    path.push_oval(Rect::from_xywh(x - r, y - r, 2. * r, 2. * r).expect("positive symbol radius"));
    p.stroke_ink(path, ink, LINE);
}

fn open_path(points: &[[f32; 2]]) -> PathBuilder {
    let mut path = PathBuilder::new();
    path.move_to(points[0][0], points[0][1]);
    for &[x, y] in &points[1..] {
        path.line_to(x, y);
    }
    path
}

fn line(p: &mut TilePainter<'_>, points: &[[f32; 2]], ink: u32, width: f32) {
    p.stroke_ink(open_path(points), ink, width);
}

fn polygon(p: &mut TilePainter<'_>, points: &[[f32; 2]], ink: u32) {
    let mut path = open_path(points);
    path.close();
    p.fill_ink(path, ink);
}

fn star(p: &mut TilePainter<'_>, [x, y]: [f32; 2], r: f32, ink: u32) {
    let k = r * 0.25;
    polygon(
        p,
        &[
            [x, y - r],
            [x + k, y - k],
            [x + r, y],
            [x + k, y + k],
            [x, y + r],
            [x - k, y + k],
            [x - r, y],
            [x - k, y - k],
        ],
        ink,
    );
}

fn home(p: &mut TilePainter<'_>, palette: Palette) {
    let mut moon = PathBuilder::new();
    moon.move_to(48.5, 16.);
    moon.cubic_to(40.6, 16.4, 41.3, 27.3, 48.5, 27.3);
    moon.cubic_to(51.3, 27.3, 53.2, 25.7, 54., 23.5);
    moon.cubic_to(49.2, 24.2, 46.8, 20.2, 48.5, 16.);
    moon.close();
    p.fill_ink(moon, palette.accent);

    line(
        p,
        &[[29.5, 42.], [48., 30.], [66.5, 42.]],
        palette.ink,
        LINE,
    );
    line(
        p,
        &[[34., 39.5], [34., 53.], [62., 53.], [62., 39.5]],
        palette.ink,
        LINE,
    );
    p.rounded([44., 43., 8., 10.], 1.3, palette.accent, 1.);
    for x in [37.5, 54.5] {
        p.rounded([x, 44., 4., 4.], 0.7, palette.ink, 1.);
    }
}

fn broom(p: &mut TilePainter<'_>, palette: Palette) {
    for x in [32., 64.] {
        ring(p, [x, 34.], 5.5, palette.accent);
        star(p, [x, 34.], 2.35, palette.ink);
    }
    line(p, &[[48., 18.], [48., 36.]], palette.accent, 3.4);
    let mut bristles = PathBuilder::new();
    bristles.move_to(44., 34.);
    bristles.line_to(52., 34.);
    bristles.quad_to(53.5, 42., 59., 50.5);
    bristles.quad_to(48., 55.5, 37., 50.5);
    bristles.quad_to(42.5, 42., 44., 34.);
    bristles.close();
    p.stroke_ink(bristles, palette.ink, LINE);
    line(p, &[[42.6, 42.], [53.4, 42.]], palette.accent, 1.8);
    for (x, end) in [(45., 43.), (48., 48.), (51., 53.)] {
        line(p, &[[x, 46.], [end, 51.5]], palette.ink, 1.25);
    }
}

fn dove(p: &mut TilePainter<'_>, palette: Palette) {
    let mut wings = PathBuilder::new();
    wings.move_to(48., 31.5);
    wings.cubic_to(42.5, 29., 36.5, 26., 29.5, 22.5);
    wings.cubic_to(29.5, 35., 35.5, 41., 43.2, 44.);
    wings.quad_to(41.7, 49., 37.5, 51.);
    wings.quad_to(48., 57., 58.5, 51.);
    wings.quad_to(54.3, 49., 52.8, 44.);
    wings.cubic_to(60.5, 41., 66.5, 35., 66.5, 22.5);
    wings.cubic_to(59.5, 26., 53.5, 29., 48., 31.5);
    wings.close();
    p.stroke_ink(wings, palette.ink, LINE);
    for direction in [-1., 1.] {
        let mut feather = PathBuilder::new();
        feather.move_to(48. + direction * 15., 31.);
        feather.quad_to(48. + direction * 11., 36.5, 48. + direction * 5., 37.7);
        p.stroke_ink(feather, palette.accent, 1.6);
    }
    dot(p, [48., 31.5], 4.4, palette.ink);
    polygon(p, &[[45.6, 35.], [50.4, 35.], [48., 38.7]], palette.accent);
}

fn stopwatch(p: &mut TilePainter<'_>, palette: Palette) {
    line(p, &[[44., 19.5], [52., 19.5]], palette.accent, 3.);
    line(p, &[[48., 20.5], [48., 24.]], palette.ink, LINE);
    line(p, &[[57., 27.], [60., 24.]], palette.accent, LINE);
    ring(p, [48., 39.5], 15.1, palette.ink);
    for pt in [[37., 39.5], [59., 39.5], [48., 50.5]] {
        dot(p, pt, 1.15, palette.accent);
    }
    polygon(
        p,
        &[
            [50., 29.],
            [42.1, 40.7],
            [47.2, 40.7],
            [45.8, 48.5],
            [54.1, 36.5],
            [49.1, 36.5],
        ],
        palette.accent,
    );
}

fn cat(p: &mut TilePainter<'_>, palette: Palette) {
    for i in 0..9 {
        let angle = i as f32 * std::f32::consts::TAU / 9. - std::f32::consts::FRAC_PI_2;
        let center = [48. + 21.2 * angle.cos(), 37. + 21.2 * angle.sin()];
        if i == 0 {
            star(p, center, 2.3, palette.accent);
        } else {
            dot(p, center, 1.35, palette.accent);
        }
    }
    let mut face = PathBuilder::new();
    face.move_to(34.5, 40.);
    face.quad_to(34.2, 36., 36.1, 32.8);
    face.line_to(36.1, 25.);
    face.quad_to(36.1, 23.7, 37.5, 24.6);
    face.line_to(43.5, 29.3);
    face.quad_to(48., 28.2, 52.5, 29.3);
    face.line_to(58.5, 24.6);
    face.quad_to(59.9, 23.7, 59.9, 25.);
    face.line_to(59.9, 32.8);
    face.quad_to(61.8, 36., 61.5, 40.);
    face.cubic_to(61.5, 47.4, 55.7, 50.3, 48., 50.3);
    face.cubic_to(40.3, 50.3, 34.5, 47.4, 34.5, 40.);
    face.close();
    p.stroke_ink(face, palette.ink, LINE);
    for x in [42.2, 53.8] {
        dot(p, [x, 39.], 1.55, palette.ink);
    }
    polygon(
        p,
        &[[45.9, 43.2], [50.1, 43.2], [48., 45.8]],
        palette.accent,
    );
}

fn empty_paws(p: &mut TilePainter<'_>, palette: Palette) {
    ring(p, [48., 26.], 8.3, palette.accent);
    line(p, &[[42.2, 31.8], [53.8, 20.2]], palette.accent, LINE);
    for (x, y) in [(38., 44.), (44.2, 40.8), (51.8, 40.8), (58., 44.)] {
        dot(p, [x, y], 2.5, palette.ink);
    }
    let mut pad = PathBuilder::new();
    pad.move_to(48., 46.5);
    pad.cubic_to(51., 46.5, 51.7, 49., 54.6, 51.7);
    pad.cubic_to(57.8, 55.2, 53.8, 58.3, 50.6, 56.7);
    pad.quad_to(48., 55.4, 45.4, 56.7);
    pad.cubic_to(42.2, 58.3, 38.2, 55.2, 41.4, 51.7);
    pad.cubic_to(44.3, 49., 45., 46.5, 48., 46.5);
    pad.close();
    p.fill_ink(pad, palette.ink);
}

fn double_feature(p: &mut TilePainter<'_>, palette: Palette) {
    let mut flyer = PathBuilder::new();
    flyer.move_to(48., 22.5);
    flyer.quad_to(44., 22.5, 41.3, 19.);
    flyer.line_to(41.3, 23.6);
    flyer.quad_to(36., 24.3, 31., 22.);
    flyer.quad_to(31.7, 29.5, 38.5, 31.5);
    flyer.quad_to(45., 31.3, 48., 34.6);
    flyer.quad_to(51., 31.3, 57.5, 31.5);
    flyer.quad_to(64.3, 29.5, 65., 22.);
    flyer.quad_to(60., 24.3, 54.7, 23.6);
    flyer.line_to(54.7, 19.);
    flyer.quad_to(52., 22.5, 48., 22.5);
    flyer.close();
    p.stroke_ink(flyer, palette.ink, LINE);
    for x in [44.8, 51.2] {
        dot(p, [x, 27.6], 1.25, palette.ink);
    }
    line(p, &[[48., 36.8], [48., 41.5]], palette.accent, 1.75);
    line(
        p,
        &[[45.4, 39.5], [48., 42.1], [50.6, 39.5]],
        palette.accent,
        1.75,
    );

    for x in [41.7, 54.3] {
        dot(p, [x, 45.8], 2.9, palette.ink);
    }
    let mut rat = PathBuilder::new();
    rat.move_to(48., 45.);
    rat.cubic_to(56.6, 45., 57.3, 51.4, 48., 55.8);
    rat.cubic_to(38.7, 51.4, 39.4, 45., 48., 45.);
    rat.close();
    p.fill_ink(rat, palette.ink);
    for x in [44.8, 51.2] {
        dot(p, [x, 49.2], 1.1, palette.face[1]);
    }
    dot(p, [48., 53.7], 1.2, palette.face[1]);
}

fn rooftop_jump(p: &mut TilePainter<'_>, palette: Palette) {
    for [left, right, y] in [[30., 42., 48.5], [54., 66., 43.]] {
        let mut roof = open_path(&[[left, 55.], [left, y], [right, y], [right, 55.]]);
        roof.close();
        p.stroke_ink(roof, palette.ink, LINE);
        line(p, &[[left - 1.8, y], [right + 1.8, y]], palette.ink, LINE);
        for x in [left + 3.6, right - 3.6] {
            dot(p, [x, y + 3.25], 0.9, palette.accent);
        }
    }
    dot(p, [36., 45.], 1.55, palette.accent);
    let mut jump = PathBuilder::new();
    jump.move_to(36., 41.);
    jump.cubic_to(41., 24., 54., 23.5, 62.5, 35.7);
    p.stroke_ink(jump, palette.accent, LINE);
    line(
        p,
        &[[62.5, 30.5], [62.5, 35.7], [57.4, 35.]],
        palette.accent,
        LINE,
    );
}

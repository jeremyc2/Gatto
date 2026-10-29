//! Shared vector artwork for the game and the exported scorecard.

use super::PosterShape;

pub(crate) const PURPLE: u32 = 0x8b7bff; // The app's focus-ring accent.

pub(crate) struct ArtPath {
    pub points: Vec<[f32; 2]>,
    pub ink: u32,
    pub opacity: f32,
}

#[derive(Clone, Copy)]
pub(crate) struct CheckpointPalette {
    pub wall: u32,
    pub coping: u32,
    pub frame: u32,
}

fn polygon(points: Vec<[f32; 2]>, ink: u32, opacity: f32) -> ArtPath {
    ArtPath {
        points,
        ink,
        opacity,
    }
}

fn ellipse(x: f32, y: f32, rx: f32, ry: f32, ink: u32, opacity: f32) -> ArtPath {
    polygon(
        (0..128)
            .map(|i| {
                let angle = i as f32 * std::f32::consts::TAU / 128.;
                [x + rx * angle.cos(), y + ry * angle.sin()]
            })
            .collect(),
        ink,
        opacity,
    )
}

/// Checkpoint doorways and the final home's picture window share one facade
/// definition between gameplay and the poster. Door jambs meet the roof squarely.
pub(crate) fn shelter_shapes(x: f32, y: f32, home: bool, lit: bool, size: f32) -> Vec<PosterShape> {
    shelter_with_palette(
        x,
        y,
        home,
        lit,
        size,
        CheckpointPalette {
            wall: 0x4b3b4c,
            coping: 0x877181,
            frame: 0x937683,
        },
    )
}

pub(crate) fn checkpoint_shapes(
    x: f32,
    y: f32,
    lit: bool,
    size: f32,
    palette: CheckpointPalette,
) -> Vec<PosterShape> {
    shelter_with_palette(x, y, false, lit, size, palette)
}

fn shelter_with_palette(
    x: f32,
    y: f32,
    home: bool,
    lit: bool,
    size: f32,
    palette: CheckpointPalette,
) -> Vec<PosterShape> {
    let mut shapes = Vec::new();
    let mut add = |dx, dy, w, h, ink, radius| {
        shapes.push(PosterShape {
            rect: [x + dx * size, y + dy * size, w * size, h * size],
            ink,
            opacity: 1.,
            radius: radius * size,
        })
    };
    let light = if lit { 0xffdba0 } else { 0x242535 };
    if home {
        // A broad, asymmetrical cottage: a proper side door and a picture
        // window devoted to the cat's interior sleeping nook.
        add(-138., -110., 196., 110., 0x514052, 4.);
        add(-134., -106., 188., 5., 0x9b7d87, 1.);
        add(-132., -99., 4., 99., 0x705466, 0.);
        add(50., -99., 4., 99., 0x705466, 0.);
        add(-138., -8., 196., 8., 0x695163, 0.);
        add(-116., -150., 17., 41., 0x6c5361, 1.);
        add(-120., -154., 25., 5., 0xb69a9c, 2.);

        // Round only the arch; both outer jambs continue all the way down.
        add(-13., -82., 44., 82., 0xb39492, 22.);
        add(-13., -41., 44., 41., 0xb39492, 0.);
        add(-8., -77., 34., 77., 0x3b3448, 17.);
        add(-8., -39., 34., 39., 0x3b3448, 0.);
        add(-1., -67., 20., 31., light, 10.);
        add(-1., -50., 20., 14., light, 0.);
        add(-4., -28., 26., 23., 0x59475c, 2.);
        add(18., -34., 3., 3., 0xc7a69b, 1.5);
        add(-18., -2., 54., 2., 0xc7a69b, 0.);

        // Clear glass, with no mullions, curtains or reflections across the
        // view. Moonlight keeps the cat readable after the interior goes dark.
        add(-122., -91., 100., 74., 0xb39492, 14.);
        add(
            -117.,
            -86.,
            90.,
            64.,
            if lit { 0xf2c994 } else { 0x414056 },
            10.,
        );
        add(
            -115.,
            -37.,
            86.,
            15.,
            if lit { 0xc39c88 } else { 0x5d506b },
            4.,
        );
        add(
            -104.,
            -33.,
            64.,
            10.,
            if lit { 0xb08caa } else { 0x99839e },
            5.,
        );
        add(-125., -22., 108., 6., 0xc7a69b, 2.);
        add(-117., -16., 92., 4., 0x7c6076, 1.);
    } else {
        add(-47., -86., 112., 86., palette.wall, 6.);
        add(-52., -92., 122., 8., palette.coping, 3.);
        add(-17., -74., 52., 74., palette.frame, 25.);
        add(-17., -37., 52., 37., palette.frame, 0.);
        add(-13., -70., 44., 70., light, 22.);
        add(-13., -32., 44., 32., light, 0.);
        add(41., -42., 15., 20., palette.frame, 5.);
        add(44., -39., 9., 14., light, 3.);
        add(-20., -2., 58., 2., 0xb89a92, 1.);
    }
    for dx in if home { vec![-137., 39.] } else { vec![-38.] } {
        add(dx, -17., 16., 17., 0x84616a, 4.);
        add(dx - 4., -19., 24., 5., 0xb08787, 2.);
        add(dx - 5., -33., 26., 23., 0x687d70, 11.);
        add(dx + 2., -38., 14., 24., 0x81947a, 7.);
        if home {
            add(dx + 1., -37., 5., 5., 0xc4a3be, 2.5);
            add(dx + 11., -29., 5., 5., 0xc4a3be, 2.5);
        }
    }
    if home && !lit {
        let (cat_x, cat_y) = home_cat_position(x, y, size);
        shapes.extend(resting_cat_shapes(cat_x, cat_y, 0x05070d, size));
    }
    shapes
}

fn home_cat_position(x: f32, y: f32, size: f32) -> (f32, f32) {
    (x - 81. * size, y - 32. * size)
}

pub(crate) fn home_sleep_paths(x: f32, y: f32, time: f32, size: f32) -> Vec<ArtPath> {
    let (cat_x, cat_y) = home_cat_position(x, y, size);
    sleep_paths_with_rise(cat_x, cat_y, time, size, 24.)
}

pub(crate) fn home_roof_paths(x: f32, y: f32, size: f32, lit: bool) -> Vec<ArtPath> {
    let mut paths: Vec<_> = [
        (vec![[-150., -108.], [-40., -155.], [70., -108.]], 0xaf9297),
        (vec![[-137., -112.], [-40., -150.], [57., -112.]], 0x394353),
        (vec![[-124., -113.], [-40., -145.], [44., -113.]], 0x465567),
    ]
    .into_iter()
    .map(|(points, ink)| {
        polygon(
            points
                .into_iter()
                .map(|[dx, dy]| [x + dx * size, y + dy * size])
                .collect(),
            ink,
            1.,
        )
    })
    .collect();
    paths.push(ellipse(
        x - 40. * size,
        y - 121. * size,
        10. * size,
        10. * size,
        0xb39492,
        1.,
    ));
    paths.push(ellipse(
        x - 40. * size,
        y - 121. * size,
        7. * size,
        7. * size,
        if lit { 0xffdba0 } else { 0x242535 },
        1.,
    ));
    paths
}

pub(crate) fn resting_cat_shapes(x: f32, y: f32, ink: u32, size: f32) -> Vec<PosterShape> {
    [
        ([-11., -14., 37., 14.], 7.),
        ([13., -19., 16., 16.], 8.),
        ([20., -20., 4., 7.], 1.),
    ]
    .into_iter()
    .map(|([dx, dy, w, h], radius)| PosterShape {
        rect: [x + dx * size, y + dy * size, w * size, h * size],
        ink,
        opacity: 1.,
        radius: radius * size,
    })
    .collect()
}

pub(crate) fn sleep_paths(x: f32, y: f32, time: f32, size: f32) -> Vec<ArtPath> {
    sleep_paths_with_rise(x, y, time, size, 35.)
}

fn sleep_paths_with_rise(x: f32, y: f32, time: f32, size: f32, rise: f32) -> Vec<ArtPath> {
    (0..3)
        .map(|i| {
            let t = (time * 0.43 + i as f32 / 3.).rem_euclid(1.);
            let d = (5. + t * 3.) * size;
            let left = x + (23. + t * 13.) * size;
            let top = y - (29. + t * rise) * size;
            let points = [
                [0., 0.],
                [1., 0.],
                [1., 0.18],
                [0.28, 0.82],
                [1., 0.82],
                [1., 1.],
                [0., 1.],
                [0., 0.82],
                [0.72, 0.18],
                [0., 0.18],
            ];
            polygon(
                points
                    .into_iter()
                    .map(|[dx, dy]| [left + dx * d, top + dy * d])
                    .collect(),
                0xd4c4df,
                (t * 8.).min(1.) * (1. - t).min(0.3) / 0.3 * 0.8,
            )
        })
        .collect()
}

/// A sphere's terminator projects as an ellipse, not an offset circle or a wedge.
/// Keep the illuminated limb on the right throughout the waxing sequence.
pub(crate) fn moon_paths(x: f32, y: f32, diameter: f32, level: usize) -> Vec<ArtPath> {
    moon_with_terminator(x, y, diameter, [0.64, 0., -0.72][level.min(2)])
}

pub(crate) fn full_moon_paths(x: f32, y: f32, diameter: f32) -> Vec<ArtPath> {
    moon_with_terminator(x, y, diameter, -1.)
}

fn moon_with_terminator(x: f32, y: f32, diameter: f32, terminator: f32) -> Vec<ArtPath> {
    let r = diameter * 0.5;
    let cx = x + r;
    let cy = y + r;
    let illumination = (1. - terminator) * 0.5;
    let mut paths = vec![
        ellipse(cx, cy, r * 1.8, r * 1.8, 0xffdba0, 0.025 * illumination),
        ellipse(cx, cy, r * 1.35, r * 1.35, 0xffdba0, 0.04 * illumination),
        // Earthshine keeps the spherical silhouette barely visible at night.
        ellipse(cx, cy, r, r, 0x292c40, 1.),
    ];
    let mut points = Vec::with_capacity(258);
    for i in 0..=128 {
        let angle = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::PI / 128.;
        points.push([cx + r * angle.cos(), cy + r * angle.sin()]);
    }
    for i in 0..=128 {
        let angle = std::f32::consts::FRAC_PI_2 - i as f32 * std::f32::consts::PI / 128.;
        points.push([cx + terminator * r * angle.cos(), cy + r * angle.sin()]);
    }
    paths.push(polygon(points, 0xf3ddba, 1.));
    // Clip crater contours to the lit face so no crater leaks into the shadow.
    for (dx, dy, radius) in [
        (0.79, -0.2, 0.10),
        (0.64, 0.45, 0.13),
        (0.28, -0.57, 0.11),
        (-0.35, 0.22, 0.17),
    ] {
        let mut crater = ellipse(
            cx + dx * r,
            cy + dy * r,
            radius * r,
            radius * r,
            0xbda789,
            0.24,
        );
        for point in &mut crater.points {
            let yy = ((point[1] - cy) / r).clamp(-1., 1.);
            let edge = (1. - yy * yy).sqrt() * r;
            point[0] = point[0].clamp(cx + terminator * edge, cx + edge);
        }
        paths.push(crater);
    }
    paths
}

#[derive(Clone, Copy)]
pub(crate) enum StatIcon {
    Coin,
    Deaths,
    Kills,
    Time,
}

// One family of flat silhouettes, smooth curves and roomy cutouts at HUD size.
pub(crate) fn stat_paths(
    kind: StatIcon,
    x: f32,
    y: f32,
    size: f32,
    background: u32,
) -> Vec<ArtPath> {
    let mut paths = match kind {
        StatIcon::Coin => vec![
            ellipse(10., 10., 9., 9., PURPLE, 1.),
            ellipse(10., 10., 6.5, 6.5, background, 1.),
            capsule([10., 6.5], [10., 13.5], 2.2, PURPLE),
        ],
        StatIcon::Deaths => {
            // Author one smooth half, then mirror it exactly around the center.
            let mut right = vec![[10., 1.3]];
            for (control1, control2, end) in [
                ([15.3, 1.3], [18.2, 4.3], [18.2, 8.4]),
                ([18.2, 11.5], [17.2, 13.1], [15.5, 14.]),
                ([15., 14.3], [14.7, 14.7], [14.7, 15.4]),
                ([14.7, 18.3], [13.8, 18.8], [10., 18.8]),
            ] {
                append_curve(&mut right, control1, control2, end);
            }
            let left: Vec<_> = right
                .iter()
                .rev()
                .skip(1)
                .map(|p| [20. - p[0], p[1]])
                .collect();
            right.extend(left);
            vec![
                polygon(right, PURPLE, 1.),
                ellipse(6.7, 9.1, 2.05, 2.25, background, 1.),
                ellipse(13.3, 9.1, 2.05, 2.25, background, 1.),
                rounded_triangle(background),
                capsule([8.6, 16.4], [8.6, 17.4], 0.9, background),
                capsule([11.4, 16.4], [11.4, 17.4], 0.9, background),
            ]
        }
        StatIcon::Kills => {
            // Identical blades with rounded guards and pommels, mirrored as a pair.
            let mut swords = Vec::new();
            for angle in [-std::f32::consts::FRAC_PI_4, std::f32::consts::FRAC_PI_4] {
                let mut sword = vec![
                    outline(
                        &[
                            [10., 0.5],
                            [11.6, 3.4],
                            [11.25, 12.4],
                            [8.75, 12.4],
                            [8.4, 3.4],
                        ],
                        PURPLE,
                    ),
                    capsule([6.6, 12.9], [13.4, 12.9], 1.9, PURPLE),
                    capsule([10., 13.4], [10., 17.6], 2.1, PURPLE),
                    ellipse(10., 18., 1.5, 1.5, PURPLE, 1.),
                ];
                for path in &mut sword {
                    for point in &mut path.points {
                        let dx = point[0] - 10.;
                        let dy = point[1] - 10.;
                        *point = [
                            10. + dx * angle.cos() - dy * angle.sin(),
                            10. + dx * angle.sin() + dy * angle.cos(),
                        ];
                    }
                }
                swords.extend(sword);
            }
            swords
        }
        StatIcon::Time => vec![
            ellipse(10., 10., 9., 9., PURPLE, 1.),
            ellipse(10., 10., 6.5, 6.5, background, 1.),
            outline(
                &[
                    [9., 5.],
                    [11., 5.],
                    [11., 9.5],
                    [14., 11.5],
                    [13., 13.],
                    [9., 10.5],
                ],
                PURPLE,
            ),
        ],
    };
    let optical_scale = match kind {
        StatIcon::Coin => 0.95,
        StatIcon::Deaths => 0.93,
        StatIcon::Kills => 1.04,
        StatIcon::Time => 0.95,
    };
    for path in &mut paths {
        for point in &mut path.points {
            point[0] = x + (10. + (point[0] - 10.) * optical_scale) * size / 20.;
            point[1] = y + (10. + (point[1] - 10.) * optical_scale) * size / 20.;
        }
    }
    paths
}

fn outline(points: &[[f32; 2]], ink: u32) -> ArtPath {
    polygon(points.to_vec(), ink, 1.)
}

// Sample cubic curves densely enough to keep the native and export outlines smooth.
fn append_curve(points: &mut Vec<[f32; 2]>, c1: [f32; 2], c2: [f32; 2], end: [f32; 2]) {
    let start = *points.last().expect("curve needs a starting point");
    for i in 1..=24 {
        let t = i as f32 / 24.;
        let u = 1. - t;
        points.push([0, 1].map(|axis| {
            u.powi(3) * start[axis]
                + 3. * u * u * t * c1[axis]
                + 3. * u * t * t * c2[axis]
                + t.powi(3) * end[axis]
        }));
    }
}

fn capsule(a: [f32; 2], b: [f32; 2], width: f32, ink: u32) -> ArtPath {
    let angle = (b[1] - a[1]).atan2(b[0] - a[0]);
    let r = width * 0.5;
    let mut points = Vec::with_capacity(66);
    for (center, offset) in [
        (b, -std::f32::consts::FRAC_PI_2),
        (a, std::f32::consts::FRAC_PI_2),
    ] {
        for i in 0..=32 {
            let theta = angle + offset + i as f32 * std::f32::consts::PI / 32.;
            points.push([center[0] + r * theta.cos(), center[1] + r * theta.sin()]);
        }
    }
    polygon(points, ink, 1.)
}

fn rounded_triangle(ink: u32) -> ArtPath {
    let mut points = vec![[10., 11.7]];
    for (c1, c2, end) in [
        ([10.3, 11.7], [11.4, 13.6], [11.4, 13.9]),
        ([11.4, 14.3], [8.6, 14.3], [8.6, 13.9]),
        ([8.6, 13.6], [9.7, 11.7], [10., 11.7]),
    ] {
        append_curve(&mut points, c1, c2, end);
    }
    polygon(points, ink, 1.)
}

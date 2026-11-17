//! The far bank, drawn on the same water plane as the bridge and boats.

use super::*;

pub(super) const STREET_DEPTH: f32 = 270.;
pub(super) const COURT_REVEAL_DEPTH: f32 = -18.;
const WALL_TOP: f32 = -7.;
const DOCK_TOP: f32 = -5.;

pub(super) fn street_grade(view: View) -> f32 {
    view.waterline() - 24.
}

fn ease(x: f32) -> f32 {
    let t = x.clamp(0., 1.);
    t * t * (3. - 2. * t)
}

pub(super) fn recreation(
    view: View,
    camera_depth: f32,
    frontage: Option<[[f32; 2]; 2]>,
) -> [Vec<art::ArtPath>; 2] {
    let clarity = ease((camera_depth - COURT_REVEAL_DEPTH) / 185.);
    if clarity == 0. {
        return [Vec::new(), Vec::new()];
    }
    let mut park = Park {
        view,
        frontage,
        clarity,
        detail: ease((camera_depth - 98.) / 270.),
        surface: Vec::with_capacity(145),
        fixtures: Vec::with_capacity(160),
    };
    park.lawn();
    // Both lots meet the same paved apron and share its center walk.
    park.patch([-130., 661., 265., 162.], 0x343745);
    park.pitch([-120., 672., 101., 139.]);
    park.court([16., 677., 109., 129.]);
    park.patch([-19., 661., 35., 162.], 0x3a3c48);
    park.patch([-19., 806., 35., 1.2], 0x54515b);
    [park.surface, park.fixtures]
}

struct Park {
    view: View,
    frontage: Option<[[f32; 2]; 2]>,
    clarity: f32,
    detail: f32,
    surface: Vec<art::ArtPath>,
    fixtures: Vec<art::ArtPath>,
}

#[derive(Clone, Copy)]
enum Plant {
    Broadleaf,
    Evergreen,
    Shrub,
    Bush,
    Flowers,
}

impl Park {
    fn point(&self, [x, y, z]: [f32; 3]) -> [f32; 3] {
        [
            self.view.x_at_depth(self.view.width * 0.5, z) + x,
            street_grade(self.view) - y,
            z,
        ]
    }

    fn ground(&mut self, points: impl IntoIterator<Item = [f32; 2]>, ink: u32) {
        let points: Vec<_> = points
            .into_iter()
            .map(|[x, z]| self.point([x, 0., z]))
            .collect();
        self.view.face(&mut self.surface, points, ink, self.clarity);
    }

    fn patch(&mut self, [x, z, w, d]: [f32; 4], ink: u32) {
        self.ground([[x, z], [x + w, z], [x + w, z + d], [x, z + d]], ink);
    }

    fn outline(&mut self, [x, z, w, d]: [f32; 4], ink: u32) {
        const PAINT: f32 = 0.85;
        for rect in [
            [x, z, w, PAINT],
            [x, z + d - PAINT, w, PAINT],
            [x, z + PAINT, PAINT, d - 2. * PAINT],
            [x + w - PAINT, z + PAINT, PAINT, d - 2. * PAINT],
        ] {
            self.patch(rect, ink);
        }
    }

    fn arc(&mut self, [x, z]: [f32; 2], radius: f32, [from, to]: [f32; 2], ink: u32) {
        let points = |r, i| {
            let theta = from + (to - from) * i as f32 / 18.;
            [x + r * theta.cos(), z + r * theta.sin()]
        };
        self.ground(
            (0..=18)
                .map(|i| points(radius + 0.42, i))
                .chain((0..=18).rev().map(|i| points(radius - 0.42, i))),
            ink,
        );
    }

    fn fixture(&mut self, points: impl IntoIterator<Item = [f32; 3]>, ink: u32) {
        let points: Vec<_> = points.into_iter().map(|point| self.point(point)).collect();
        self.view
            .face(&mut self.fixtures, points, ink, self.clarity);
    }

    fn rail(&mut self, a: [f32; 3], b: [f32; 3], width: f32, ink: u32) {
        let [a, b] = [a, b].map(|point| self.point(point));
        self.view
            .line(&mut self.fixtures, a, b, width, ink, self.clarity);
    }

    fn place<const N: usize>(&mut self, feet: [[f32; 2]; N], draw: impl FnOnce(&mut Self)) {
        // The skyline can cover the far lawn as the pocket opens. Bring in a
        // fixture and its shadow together once its footing clears that edge.
        let visibility =
            if let (Some(camera), Some([[ax, ay], [bx, by]])) = (self.view.camera, self.frontage) {
                feet.into_iter()
                    .map(|[x, z]| {
                        camera.point(self.point([x, 0., z])).map_or(0., |[sx, sy]| {
                            let edge_y = ay + (by - ay) * (sx - ax) / (bx - ax);
                            ease((sy - edge_y - 1.5) / 7.)
                        })
                    })
                    .fold(1., f32::min)
            } else {
                1.
            };
        if visibility > 0. {
            let surface_start = self.surface.len();
            let fixture_start = self.fixtures.len();
            draw(self);
            for path in self.surface[surface_start..]
                .iter_mut()
                .chain(&mut self.fixtures[fixture_start..])
            {
                path.opacity *= visibility;
            }
        }
    }

    fn accent(&self, surface: u32, ink: u32) -> u32 {
        super::flight::blend_ink(surface, ink, 0.35 + 0.65 * self.detail)
    }

    fn lawn(&mut self) {
        use Plant::*;

        const GRASS: u32 = 0x273e36;
        self.ground(
            [
                [-345., 321.],
                [-292., 298.],
                [295., 298.],
                [363., 326.],
                [382., 1085.],
                [332., 1145.],
                [-326., 1145.],
                [-376., 1083.],
            ],
            GRASS,
        );
        for [x, z, w, d] in [
            [-319., 328., 120., 50.],
            [117., 315., 137., 58.],
            [-252., 409., 80., 62.],
            [-126., 383., 61., 53.],
            [85., 417., 91., 55.],
            [213., 442., 68., 82.],
            [-294., 534., 86., 45.],
            [-176., 574., 71., 57.],
            [34., 520., 105., 43.],
            [173., 580., 108., 67.],
            [-282., 686., 78., 62.],
            [170., 732., 119., 52.],
            [-195., 778., 44., 54.],
            [45., 816., 119., 24.],
            [-308., 868., 110., 67.],
            [-141., 850., 89., 110.],
            [45., 882., 100., 66.],
            [226., 886., 120., 82.],
            [-286., 1007., 96., 70.],
            [-112., 1045., 112., 68.],
            [94., 1010., 142., 94.],
        ] {
            self.ground(
                [
                    [x, z + d * 0.3],
                    [x + w * 0.42, z],
                    [x + w, z + d * 0.2],
                    [x + w * 0.86, z + d],
                    [x + w * 0.15, z + d],
                ],
                self.accent(GRASS, 0x2e453a),
            );
        }
        let blades = self.accent(GRASS, 0x3a5041);
        for [x, z, w, d] in [
            [-227., 340., 131., 250.],
            [45., 335., 193., 288.],
            [-253., 842., 440., 250.],
        ] {
            for i in 0..24 {
                let u = (i * 37 % 101) as f32 / 101.;
                let v = (i * 19 % 43) as f32 / 43.;
                let [x, z] = [x + w * u, z + d * v];
                let span = 2. + (i % 4) as f32;
                self.ground([[x, z], [x + span, z + 0.8], [x + 0.4, z + 1.5]], blades);
            }
        }
        self.patch([-10., 298., 21., 366.], 0x363944);
        // Keep the neighborhood courts open, with mixed planting along the
        // edges of the lawn and in the strip toward the city.
        for (kind, [x, z, height]) in [
            (Broadleaf, [77., 1076., 34.]),
            (Evergreen, [-107., 1063., 32.]),
            (Bush, [122., 1051., 9.]),
            (Flowers, [-44., 1035., 5.]),
            (Evergreen, [230., 1004., 31.]),
            (Broadleaf, [-247., 997., 34.]),
            (Shrub, [-194., 985., 11.]),
            (Flowers, [144., 958., 5.]),
            (Broadleaf, [-79., 941., 26.]),
            (Bush, [48., 925., 10.]),
            (Evergreen, [194., 913., 28.]),
            (Flowers, [-133., 889., 4.]),
            (Bush, [152., 869., 8.]),
            (Shrub, [-157., 834., 8.]),
            (Broadleaf, [-199., 791., 27.]),
            (Evergreen, [203., 779., 25.]),
            (Bush, [-150., 708., 9.]),
            (Flowers, [149., 709., 4.]),
            (Broadleaf, [166., 639., 24.]),
            (Bush, [39., 634., 7.]),
            (Shrub, [91., 624., 8.]),
            (Flowers, [-82., 625., 4.]),
            (Broadleaf, [-27., 597., 15.]),
            (Broadleaf, [-219., 583., 28.]),
            (Evergreen, [-69., 547., 21.]),
            (Broadleaf, [68., 533., 18.]),
            (Bush, [-147., 530., 9.]),
            (Flowers, [-83., 511., 3.6]),
            (Broadleaf, [118., 488., 22.]),
            (Flowers, [155., 472., 4.]),
            (Flowers, [43., 466., 3.8]),
            (Bush, [-33., 451., 5.5]),
            (Evergreen, [-177., 442., 23.]),
            (Shrub, [57., 415., 6.5]),
            (Shrub, [84., 397., 6.]),
            (Flowers, [-52., 375., 4.]),
            (Flowers, [31., 361., 3.6]),
            (Shrub, [-113., 350., 7.]),
        ] {
            self.place([[x, z]], |park| park.plant(kind, [x, z], height));
        }
    }

    fn plant(&mut self, kind: Plant, [x, z]: [f32; 2], size: f32) {
        let width = match kind {
            Plant::Broadleaf => size * 0.53,
            Plant::Evergreen => size * 0.37,
            Plant::Shrub => size * 1.05,
            Plant::Bush => size * 1.65,
            Plant::Flowers => size * 1.55,
        };
        self.ground(
            [
                [x - width * 0.7, z],
                [x + width * 0.65, z - width * 0.3],
                [x + width * 1.25, z + width * 0.55],
                [x - width * 0.3, z + width * 0.6],
            ],
            0x20332f,
        );
        let mut foliage = |silhouette: &[[f32; 2]], ink| {
            self.fixture(
                silhouette
                    .iter()
                    .map(|&[u, v]| [x + width * u, size * v, z]),
                ink,
            );
        };
        match kind {
            Plant::Broadleaf => {
                foliage(
                    &[[-0.09, 0.], [0.07, 0.], [0.04, 0.55], [-0.06, 0.55]],
                    0x484740,
                );
                foliage(
                    &[
                        [-0.96, 0.49],
                        [-0.86, 0.69],
                        [-0.59, 0.76],
                        [-0.37, 0.94],
                        [0.03, 1.],
                        [0.43, 0.88],
                        [0.65, 0.84],
                        [0.92, 0.71],
                        [1., 0.49],
                        [0.74, 0.35],
                        [0.18, 0.31],
                        [-0.49, 0.35],
                    ],
                    0x2c4839,
                );
                foliage(
                    &[
                        [-0.88, 0.55],
                        [-0.64, 0.71],
                        [-0.39, 0.73],
                        [-0.17, 0.91],
                        [0.25, 0.92],
                        [0.53, 0.75],
                        [0.35, 0.55],
                        [-0.3, 0.44],
                    ],
                    0x365443,
                );
            }
            Plant::Evergreen => {
                foliage(
                    &[[-0.08, 0.], [0.08, 0.], [0.04, 0.42], [-0.04, 0.42]],
                    0x454442,
                );
                foliage(
                    &[
                        [0., 1.],
                        [0.55, 0.68],
                        [0.37, 0.68],
                        [0.77, 0.48],
                        [0.57, 0.49],
                        [1., 0.24],
                        [0.5, 0.19],
                        [-0.42, 0.2],
                        [-1., 0.26],
                        [-0.54, 0.49],
                        [-0.77, 0.49],
                        [-0.39, 0.68],
                        [-0.57, 0.68],
                    ],
                    0x233c35,
                );
                foliage(
                    &[
                        [-0.82, 0.27],
                        [-0.42, 0.5],
                        [-0.63, 0.49],
                        [0., 0.94],
                        [0.16, 0.38],
                    ],
                    0x2e4a40,
                );
            }
            Plant::Shrub => {
                foliage(
                    &[
                        [-1., 0.15],
                        [-0.84, 0.58],
                        [-0.48, 0.67],
                        [-0.3, 0.95],
                        [0.23, 1.],
                        [0.48, 0.64],
                        [0.68, 0.73],
                        [0.95, 0.45],
                        [1., 0.16],
                        [0.46, 0.],
                        [-0.55, 0.],
                    ],
                    0x3b5140,
                );
                foliage(
                    &[
                        [-0.87, 0.42],
                        [-0.48, 0.58],
                        [-0.28, 0.87],
                        [0.06, 0.91],
                        [0.33, 0.6],
                        [-0.16, 0.35],
                    ],
                    0x475b46,
                );
            }
            Plant::Bush => {
                for (u, height, ink) in [
                    (-0.45, 0.65, 0x29433b),
                    (0.46, 0.79, 0x29463b),
                    (0.03, 1., 0x324f41),
                ] {
                    foliage(
                        &[
                            [u - 0.52, 0.],
                            [u - 0.49, height * 0.55],
                            [u - 0.22, height * 0.93],
                            [u + 0.15, height],
                            [u + 0.47, height * 0.68],
                            [u + 0.51, 0.],
                        ],
                        ink,
                    );
                }
            }
            Plant::Flowers => {
                foliage(
                    &[
                        [-0.94, 0.],
                        [-0.58, 0.55],
                        [-0.13, 0.39],
                        [0.4, 0.62],
                        [0.94, 0.15],
                        [0.38, 0.],
                    ],
                    0x425a44,
                );
                for [u, v] in [[-0.64, 0.55], [-0.20, 0.76], [0.16, 0.42], [0.54, 0.64]] {
                    foliage(
                        &[
                            [u - 0.13, v],
                            [u - 0.07, v + 0.2],
                            [u + 0.08, v + 0.24],
                            [u + 0.14, v + 0.04],
                            [u + 0.04, v - 0.1],
                        ],
                        if u < 0. { 0x7f6574 } else { 0x9c7c82 },
                    );
                }
            }
        }
    }

    fn pitch(&mut self, [x, z, w, d]: [f32; 4]) {
        use std::f32::consts::PI;
        let center = x + w * 0.5;
        let middle = z + d * 0.5;
        let grass = 0x34483c;
        let paint = self.accent(grass, 0x849386);
        self.patch([x - 5., z - 5., w + 10., d + 10.], 0x293c33);
        self.patch([x, z, w, d], grass);
        for stripe in [0., 2., 4.] {
            self.patch(
                [x, z + stripe * d / 6., w, d / 6.],
                self.accent(grass, 0x3b5144),
            );
        }
        self.outline([x, z, w, d], paint);
        self.patch([x, middle - 0.42, w, 0.85], paint);
        self.arc([center, middle], 13., [0., PI], paint);
        self.arc([center, middle], 13., [PI, 2. * PI], paint);
        self.patch([center - 0.8, middle - 0.8, 1.6, 1.6], paint);
        for (end, sign) in [(z + d, -1.), (z, 1.)] {
            let box_z = end.min(end + sign * 21.);
            self.outline([center - 25., box_z, 50., 21.], paint);
            let small = end.min(end + sign * 9.);
            self.outline([center - 13., small, 26., 9.], paint);
            let back = end - sign * 7.;
            self.place(
                [
                    [center - 8.5, end],
                    [center + 8.5, end],
                    [center - 8.5, back],
                    [center + 8.5, back],
                ],
                |park| park.goal(center, end, back),
            );
        }
    }

    fn goal(&mut self, center: f32, end: f32, back: f32) {
        const FRAME: u32 = 0xa8a4a1;
        let mesh = self.accent(0x53605b, 0x818982);
        for x in [center - 8.5, center + 8.5] {
            self.rail([x, 0., back], [x, 8., end], 0.44, mesh);
            self.rail([x, 0., end], [x, 0., back], 0.5, mesh);
            self.rail([x, 0., end], [x, 8., end], 0.8, FRAME);
        }
        for t in [0.28, 0.56, 0.83] {
            let z = end + (back - end) * t;
            self.rail(
                [center - 8.5, 8. * (1. - t), z],
                [center + 8.5, 8. * (1. - t), z],
                0.35,
                mesh,
            );
        }
        for x in [center - 4.2, center, center + 4.2] {
            self.rail([x, 8., end], [x, 0., back], 0.35, mesh);
        }
        self.rail(
            [center - 8.5, 8., end],
            [center + 8.5, 8., end],
            0.84,
            FRAME,
        );
    }

    fn court(&mut self, [x, z, w, d]: [f32; 4]) {
        use std::f32::consts::PI;
        const ASPHALT: u32 = 0x464557;
        let paint = self.accent(0x645f70, 0x958689);
        let center = x + w * 0.5;
        let baseline = z + d;
        let foul = baseline - 37.;
        self.patch([x - 5., z - 5., w + 10., d + 10.], 0x303442);
        self.patch([x, z, w, d], ASPHALT);
        self.patch(
            [center - 13.5, foul, 27., 37.],
            self.accent(0x504752, 0x62505b),
        );
        self.outline([x, z, w, d], paint);
        self.outline([center - 13.5, foul, 27., 37.], paint);
        self.arc([center, foul], 13.5, [PI, 2. * PI], paint);
        self.arc([center, baseline - 10.], 42., [PI, 2. * PI], paint);
        for side in [-1., 1.] {
            self.patch(
                [center + side * 42. - 0.42, baseline - 10., 0.85, 10.],
                paint,
            );
            for offset in [7., 14., 21.] {
                let mark = center + side * 13.5;
                self.patch(
                    [mark.min(mark + side * 2.6), foul + offset, 2.6, 0.85],
                    paint,
                );
            }
        }
        self.place([[center, baseline + 5.]], |park| {
            park.hoop(center, baseline)
        });
    }

    fn hoop(&mut self, x: f32, baseline: f32) {
        let pole = baseline + 5.;
        let board = baseline - 4.;
        let rim = board - 4.3;
        self.rail([x, 0., pole], [x, 25., pole], 1.4, 0x827a88);
        self.rail([x, 25., pole], [x, 25., board], 1., 0x797381);
        let rect = |w: f32, bottom, top, z| {
            [
                [x - w, bottom, z],
                [x + w, bottom, z],
                [x + w, top, z],
                [x - w, top, z],
            ]
        };
        self.fixture(rect(7.9, 21., 29.8, board + 0.8), 0x5b5769);
        self.fixture(
            rect(7.5, 21.2, 29.45, board),
            self.accent(0x8b828f, 0xa59ba1),
        );
        self.fixture(rect(3., 22.25, 26.25, board - 0.05), 0xc6b6b4);
        self.fixture(rect(2.5, 22.78, 25.71, board - 0.1), 0x8e8490);
        let ring = |i, radius, y| {
            let angle = i as f32 * std::f32::consts::TAU / 16.;
            [x + radius * angle.cos(), y, rim + radius * angle.sin()]
        };
        for i in 0..16 {
            self.rail(ring(i, 3.6, 22.), ring(i + 1, 3.6, 22.), 0.72, 0xad7e72);
            if i % 4 == 0 {
                self.rail(ring(i, 3.4, 22.), ring(i + 1, 2.4, 17.5), 0.38, 0x998e94);
            }
        }
    }
}

pub(super) struct Bank {
    view: View,
    edge: [[f32; 2]; 12],
}

impl Bank {
    pub(super) fn new(view: View) -> Self {
        let w = view.width;
        Self {
            view,
            edge: [
                [-420., 152.],
                [w * 0.12, 152.],
                [w * 0.12, 138.],
                [w * 0.25, 138.],
                [w * 0.25, 155.],
                [w * 0.43, 155.],
                [w * 0.43, 133.],
                [w * 0.65, 133.],
                [w * 0.65, 163.],
                [w * 0.77, 163.],
                [w * 0.77, 147.],
                [w + 315., 147.],
            ],
        }
    }

    pub(super) fn water(&self, height: f32) -> Vec<art::ArtPath> {
        let mut paths = Vec::with_capacity(1);
        let rim = self.edge.map(|[x, z]| self.view.on_water(x, z));
        let bottom = [[rim[11][0], height], [rim[0][0], height]];
        shape(
            &mut paths,
            rim.into_iter().chain(bottom).collect(),
            WATER,
            1.,
        );
        paths
    }

    pub(super) fn inland(&self) -> Vec<art::ArtPath> {
        let mut paths = Vec::with_capacity(4);
        let y = street_grade(self.view);
        let at = |x, depth| [x, y, depth];
        let pavement = STREET_DEPTH - 19.;
        let curb = STREET_DEPTH - 6.;
        let [left, _] = self.edge[0];
        let [right, _] = self.edge[self.edge.len() - 1];
        let edge = self
            .edge
            .map(|[x, z]| [x, self.view.waterline() + WALL_TOP, z + 52.]);
        self.view.face(
            &mut paths,
            edge.into_iter()
                .chain([at(right, pavement), at(left, pavement)]),
            0x222639,
            1.,
        );
        for ([near, far], color) in [
            ([pavement, curb], 0x1d2233),
            ([curb, STREET_DEPTH + 4.], 0x303043),
        ] {
            self.view.face(
                &mut paths,
                [
                    at(left, near),
                    at(right, near),
                    at(right, far),
                    at(left, far),
                ],
                color,
                1.,
            );
        }
        self.view.line(
            &mut paths,
            at(left, curb),
            at(right, curb),
            0.7,
            0x605b70,
            0.46,
        );
        paths
    }

    pub(super) fn shore(&self) -> Vec<art::ArtPath> {
        let mut paths = Vec::with_capacity(56);
        let y = self.view.waterline();
        let at = |[x, z]: [f32; 2], dy| [x, y + dy, z];
        let apron = self.edge.map(|p| at(p, WALL_TOP));
        let inland = self.edge.map(|[x, z]| at([x, z + 52.], WALL_TOP));
        self.view.face(
            &mut paths,
            apron.into_iter().chain(inland.into_iter().rev()),
            0x25293b,
            1.,
        );

        for face in self.edge.windows(2) {
            let a = face[0];
            let b = face[1];
            let along_bank = a[0] != b[0];
            let stone = if along_bank { 0x38384b } else { 0x2d3144 };
            self.view.face(
                &mut paths,
                [at(a, WALL_TOP), at(b, WALL_TOP), at(b, 0.), at(a, 0.)],
                stone,
                1.,
            );
            self.view.line(
                &mut paths,
                at(a, WALL_TOP),
                at(b, WALL_TOP),
                1.05,
                0x736b79,
                0.46,
            );
            self.view
                .line(&mut paths, at(a, -1.4), at(b, -1.4), 0.9, 0x151e31, 0.68);
            if along_bank {
                let [left, right] = [a[0].max(-140.), b[0].min(self.view.width + 75.)];
                for i in 0..((right - left) / 49.) as usize {
                    let x = left + 15. + i as f32 * 49.;
                    self.view.line(
                        &mut paths,
                        at([x - 1., a[1]], WALL_TOP - 1.4),
                        at([x - 1., a[1]], -4.),
                        1.65,
                        0x4c495c,
                        1.,
                    );
                }
            }
        }

        for (fraction, length) in [(0.31, 28.), (0.56, 25.), (0.86, 32.)] {
            let x = self.view.width * fraction;
            let bank = self.edge[self.edge.partition_point(|p| p[0] <= x) - 1][1];
            let tip = bank - length;
            let [a, b, c, d] = [[x, bank], [x + 18., bank], [x + 18., tip], [x, tip]];
            self.view.face(
                &mut paths,
                [
                    at(a, 0.),
                    at([x + 21., bank], 0.),
                    at([x + 21., tip - 3.], 0.),
                    at([x - 2., tip - 3.], 0.),
                ],
                0x070f20,
                0.35,
            );
            self.view.face(
                &mut paths,
                [at(d, DOCK_TOP), at(c, DOCK_TOP), at(c, -1.), at(d, -1.)],
                0x333447,
                1.,
            );
            self.view.face(
                &mut paths,
                [at(b, DOCK_TOP), at(c, DOCK_TOP), at(c, -1.), at(b, -1.)],
                0x272d40,
                1.,
            );
            self.view.face(
                &mut paths,
                [
                    at(a, DOCK_TOP),
                    at(b, DOCK_TOP),
                    at(c, DOCK_TOP),
                    at(d, DOCK_TOP),
                ],
                0x454354,
                1.,
            );
            for fraction in [0.35, 0.70] {
                let row = bank - length * fraction;
                self.view.line(
                    &mut paths,
                    at([x, row], DOCK_TOP),
                    at([x + 18., row], DOCK_TOP),
                    0.55,
                    0x2a2d40,
                    0.68,
                );
            }
            for point in [c, d, [x + 18., bank - length * 0.53]] {
                self.view.line(
                    &mut paths,
                    at(point, DOCK_TOP - 1.9),
                    at(point, 0.6),
                    1.3,
                    0x655a6a,
                    0.82,
                );
            }
        }
        paths
    }
}

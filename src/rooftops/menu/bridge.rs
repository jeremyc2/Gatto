//! Solid masonry and road spans projected into the harbor's 2D canvas.

use super::*;

const STONE: u32 = 0x504956;
const STONE_SIDE: u32 = 0x3f3c50;
const STONE_EDGE: u32 = 0x90818a;
const DECK: u32 = 0x413d50;
const STEEL: u32 = 0x746778;
const NEAR_PIER: [f32; 2] = [-43., -29.];
const CENTER_PIER: [f32; 2] = [-7., 7.];
const FAR_PIER: [f32; 2] = [29., 43.];
const OUTER_CABLE: f32 = 41.;
const TOWER_HALF_DEPTH: f32 = 10.;
const OVERHANG: f32 = 130.;
const FOOTING: [f32; 3] = [-23., 23., 48.];
const SEAT: [f32; 3] = [-TOWER_HALF_DEPTH, TOWER_HALF_DEPTH, FAR_PIER[1]];
const SHOULDER: [f32; 3] = [-14.5, 14.5, 44.8];
const SEAT_INK: u32 = 0x454050;
const PLINTH_END_DIVISIONS: usize = 4;
const COURSE: f32 = 7.8;

type Arch = [[f32; 2]; 7];

#[derive(Clone, Copy)]
enum Roadway {
    Far,
    Near,
}

impl Roadway {
    fn edges(self) -> [f32; 3] {
        match self {
            Self::Far => [FAR_PIER[1], FAR_PIER[0], CENTER_PIER[1]],
            Self::Near => [NEAR_PIER[0], NEAR_PIER[1], CENTER_PIER[0]],
        }
    }

    fn lane_depth(self) -> f32 {
        let [_, jamb, median] = self.edges();
        (jamb + median) * 0.5
    }
}

fn arch_cut(arch: Arch, y: f32) -> [f32; 2] {
    let center = arch[3][0];
    let side = arch[..=3]
        .windows(2)
        .find(|edge| edge[0][1] >= y && y >= edge[1][1])
        .map_or(center, |edge| {
            let [[z0, y0], [z1, y1]] = [edge[0], edge[1]];
            z0 + (z1 - z0) * (y0 - y) / (y0 - y1)
        });
    [side, 2. * center - side]
}

fn towers(width: f32) -> [f32; 2] {
    [
        width * 0.245 - TOWER_HALF_DEPTH,
        width * 0.761 - TOWER_HALF_DEPTH,
    ]
}

pub(super) fn tug_channel(view: View) -> [f32; 2] {
    let [west, east] = towers(view.width);
    // The oblique view puts more of the east footing across the tug's route.
    [west + FOOTING[1] + 68., east + FOOTING[0] - 65.]
}

type Mesh = Arc<[(gpui_kit::Path<gpui_kit::Pixels>, gpui_kit::Hsla)]>;

#[derive(Default)]
pub(in crate::rooftops) struct BridgeArt {
    layout: Option<(View, f32, f32)>,
    layers: [Mesh; 4],
}

impl BridgeArt {
    pub(super) fn render(&mut self, view: View, deck: f32, time: f32, scale: f32) -> gpui_kit::Div {
        let layout = Some((view, deck, scale));
        if self.layout != layout {
            self.layout = layout;
            self.layers = Bridge {
                view,
                ends: [-OVERHANG, view.width + OVERHANG],
                towers: towers(view.width),
                deck,
            }
            .layers()
            .map(|shapes| mesh(&shapes, scale));
        }
        div()
            .absolute()
            .inset_0()
            .child(paint(self.layers[0].clone()))
            .child(traffic::lane(
                view,
                deck,
                time,
                Roadway::Far.lane_depth(),
                scale,
                true,
            ))
            .child(paint(self.layers[1].clone()))
            .child(traffic::lane(
                view,
                deck,
                time,
                Roadway::Near.lane_depth(),
                scale,
                false,
            ))
            .child(paint(self.layers[2].clone()))
            .child(paint(self.layers[3].clone()))
    }
}

struct Bridge {
    view: View,
    ends: [f32; 2],
    towers: [f32; 2],
    deck: f32,
}

impl Bridge {
    fn top(&self) -> f32 {
        self.deck - 158.
    }

    fn spring(&self) -> f32 {
        self.top() + 63.
    }

    fn seat(&self) -> f32 {
        self.deck + 16.
    }

    fn shoulder(&self) -> f32 {
        self.deck + 24.
    }

    fn course_rows(&self, top: f32, bottom: f32) -> impl Iterator<Item = f32> + Clone {
        let start = self.top() + 9.;
        let first = ((top - start) / COURSE).ceil().max(0.) as u32;
        (first..)
            .map(move |row| start + row as f32 * COURSE)
            .take_while(move |&y| y < bottom)
    }

    fn spans(&self) -> [[f32; 2]; 3] {
        let [left, right] = self.ends;
        let [west, east] = self.towers;
        [
            [left, west - TOWER_HALF_DEPTH],
            [west + TOWER_HALF_DEPTH, east - TOWER_HALF_DEPTH],
            [east + TOWER_HALF_DEPTH, right],
        ]
    }

    fn layers(&self) -> [Vec<art::ArtPath>; 4] {
        let mut far = Vec::new();
        let mut middle = Vec::new();
        let mut near = Vec::new();
        let mut exposed = Vec::new();
        for tower in self.towers {
            self.foundation(&mut far, tower);
            self.deck_end(&mut far, tower + TOWER_HALF_DEPTH);
        }
        self.suspension(&mut far, &mut exposed, OUTER_CABLE, 0x85778b, 0.50);
        self.suspension(&mut far, &mut exposed, 4.5, 0x8c7c8d, 0.46);
        for tower in self.towers {
            self.masonry(&mut far, tower, FAR_PIER, 0x494451);
        }
        // Each lane's paving is nearer to the camera than its rear jamb. It
        // covers the base of that jamb before the traffic is painted.
        self.road(&mut far, Roadway::Far);

        self.promenade_bypasses(&mut middle, 1.);
        self.promenade(&mut middle);
        self.suspension(&mut middle, &mut exposed, -4.5, 0x9e8999, 0.52);
        for tower in self.towers {
            self.masonry(&mut middle, tower, CENTER_PIER, 0x4e4755);
        }
        self.road(&mut middle, Roadway::Near);

        self.truss(&mut near);
        self.suspension(&mut near, &mut exposed, -OUTER_CABLE, 0xb39ba7, 0.68);
        for tower in self.towers {
            self.near_bearing_face(&mut near, tower);
            self.masonry(&mut near, tower, NEAR_PIER, STONE);
            self.crown(&mut near, tower);
        }
        [far, middle, near, exposed]
    }

    fn deck_end(&self, paths: &mut Vec<art::ArtPath>, x: f32) {
        let front = NEAR_PIER[0];
        let back = FAR_PIER[1];
        let at = |z, y| self.view.project(x, z, y);
        shape(
            paths,
            vec![
                at(front, self.deck),
                at(back, self.deck),
                at(back, self.seat()),
                at(front, self.seat()),
            ],
            0x303346,
            1.,
        );
    }

    fn foundation(&self, paths: &mut Vec<art::ArtPath>, x: f32) {
        let waterline = self.view.waterline();
        let [left, right, half] = FOOTING;
        let contour = |spread: f32| {
            [
                [x + left - spread * 0.33, -half - spread],
                [x + right + spread * 0.58, -half - spread],
                [x + right + spread * 0.58, half + spread * 0.2],
            ]
            .map(|[x, depth]| self.view.on_water(x, depth))
        };
        // The contact shade shares the exact corners of the stone on both sides.
        let mut inner = contour(0.);
        for (spread, alpha) in [(2.3, 0.47), (5., 0.29), (8.7, 0.16), (14., 0.075)] {
            let outer = contour(spread);
            for side in 0..2 {
                shape(
                    paths,
                    vec![inner[side], inner[side + 1], outer[side + 1], outer[side]],
                    0x070f20,
                    alpha,
                );
            }
            inner = outer;
        }

        let top = self.seat();
        let base = self.shoulder();
        let [left, right, half] = SEAT;
        let at = |dx, z| self.view.project(x + dx, z, top);
        shape(
            paths,
            vec![
                at(left, -half),
                at(right, -half),
                at(right, half),
                at(left, half),
            ],
            0x645c6b,
            1.,
        );
        // The roadway bears on the upper masonry. The plinth steps outward below
        // it and broadens toward the river, with a long face and a transverse end.
        let stem = [-18.5, 18.5, 46.5];
        self.plinth(
            paths,
            x,
            [top, base],
            [SEAT, SHOULDER],
            [0x57515e, SEAT_INK],
        );
        self.plinth(
            paths,
            x,
            [base, base + 4.],
            [SHOULDER, stem],
            [0x615765, 0x4f4657],
        );
        let wet_height = 3.4;
        let wet_fraction = wet_height / (waterline - (base + 4.));
        let wet_edge = std::array::from_fn(|i| FOOTING[i] + (stem[i] - FOOTING[i]) * wet_fraction);
        self.plinth(
            paths,
            x,
            [base + 4., waterline - wet_height],
            [stem, wet_edge],
            [0x504b5a, 0x3b394b],
        );
        // A dark band on both faces marks where the masonry meets the river.
        self.plinth(
            paths,
            x,
            [waterline - wet_height, waterline],
            [wet_edge, FOOTING],
            [0x454453, 0x343749],
        );
        let [left, corner, right] = contour(0.);
        stroke(paths, left, corner, 0.85, 0x778198, 0.27);
        stroke(paths, corner, right, 0.7, 0x758196, 0.20);
    }

    fn plinth_edge(&self, x: f32, y: f32, [left, right, half]: [f32; 3]) -> [[f32; 2]; 3] {
        [[x + left, -half], [x + right, -half], [x + right, half]]
            .map(|[x, z]| self.view.project(x, z, y))
    }

    fn near_bearing_face(&self, paths: &mut Vec<art::ArtPath>, x: f32) {
        // Carry the back corner of the upright into the sloped footing. The
        // shoulder flares from the same vertices as the rest of the plinth.
        let edge = |t: f32| {
            let flare = (SHOULDER[2] - SEAT[2]) * t;
            let face = x + SEAT[1] + (SHOULDER[1] - SEAT[1]) * t;
            let y = self.seat() + (self.shoulder() - self.seat()) * t;
            [
                self.view.project(face, NEAR_PIER[0] - flare, y),
                self.view.project(face, NEAR_PIER[1] + flare, y),
            ]
        };
        let [front, rear] = edge(0.);
        let [foot_front, foot_rear] = edge(1.);
        let [mid_front, mid_rear] = edge(0.5);
        shape(
            paths,
            vec![front, rear, foot_rear, foot_front],
            SEAT_INK,
            1.,
        );
        stroke(paths, mid_front, mid_rear, 0.65, 0x242535, 0.67);
        stroke(paths, front, foot_front, 1.25, STONE_EDGE, 0.31);
    }

    fn plinth(
        &self,
        paths: &mut Vec<art::ArtPath>,
        x: f32,
        [top, bottom]: [f32; 2],
        [upper, lower]: [[f32; 3]; 2],
        [front, end]: [u32; 2],
    ) {
        let edge = |y: f32| {
            let t = (y - top) / (bottom - top);
            let profile = std::array::from_fn(|i| upper[i] + (lower[i] - upper[i]) * t);
            self.plinth_edge(x, y, profile)
        };
        let a = edge(top);
        let b = edge(bottom);
        shape(paths, vec![a[0], a[1], b[1], b[0]], front, 1.);
        shape(paths, vec![a[1], a[2], b[2], b[1]], end, 1.);
        let courses = ((bottom - top) / 7.4).floor() as usize;
        let pitch = (bottom - top) / (courses + 1) as f32;
        for course in 0..=courses {
            let y = top + pitch * course as f32;
            let row = edge(y);
            let below = edge(y + pitch);
            if course > 0 {
                stroke(paths, row[0], row[1], 0.65, 0x252636, 0.64);
                stroke(paths, row[1], row[2], 0.65, 0x242535, 0.67);
            }
            if courses > 0 {
                for (side, divisions) in [(0, 2), (1, PLINTH_END_DIVISIONS)] {
                    for joint in 1..=divisions {
                        let f =
                            (joint as f32 - (course % 2) as f32 * 0.48) / (divisions + 1) as f32;
                        let join = |line: [[f32; 2]; 3]| {
                            std::array::from_fn(|i| {
                                line[side][i] + (line[side + 1][i] - line[side][i]) * f
                            })
                        };
                        stroke(paths, join(row), join(below), 0.5, 0x242535, 0.34);
                    }
                }
            }
        }
        stroke(paths, a[1], b[1], 1.25, STONE_EDGE, 0.31);
    }

    fn masonry(&self, paths: &mut Vec<art::ArtPath>, x: f32, [a, b]: [f32; 2], ink: u32) {
        let top = self.spring();
        let bottom = self.seat();
        let front = x + TOWER_HALF_DEPTH;
        let at = |z, y| self.view.project(front, z, y);
        let outer = a == NEAR_PIER[0];
        let back = |z, y| self.view.project(x - TOWER_HALF_DEPTH, z, y);
        shape(
            paths,
            vec![back(a, top), at(a, top), at(a, bottom), back(a, bottom)],
            if outer { STONE_SIDE } else { 0x2d2e40 },
            1.,
        );
        shape(
            paths,
            vec![at(a, top), at(b, top), at(b, bottom), at(a, bottom)],
            ink,
            1.,
        );
        stroke(paths, at(a, top), at(a, bottom), 0.95, STONE_EDGE, 0.31);
        let rows = self.course_rows(top, bottom);
        for y in rows.clone() {
            stroke(paths, at(a, y), at(b, y), 0.65, STONE_SIDE, 0.72);
        }
        for y in rows {
            stroke(paths, at(a, y), back(a, y), 0.55, 0x211f32, 0.53);
        }
        for (row, y) in self.course_rows(top - 0.001, bottom - 2.).enumerate() {
            let joint = if row.is_multiple_of(2) {
                a + 4.
            } else {
                b - 4.
            };
            stroke(
                paths,
                at(joint, y),
                at(joint, (y + COURSE).min(bottom)),
                0.48,
                0x272838,
                0.31,
            );
        }
    }

    fn crown(&self, paths: &mut Vec<art::ArtPath>, x: f32) {
        let front = x + TOWER_HALF_DEPTH;
        let at = |z, y| self.view.project(front, z, y);
        let top = self.top();
        let spring = self.spring();
        let opening = |a: f32, b: f32| -> Arch {
            let width = b - a;
            [
                [a, spring],
                [a + width * 0.07, spring - 11.],
                [a + width * 0.22, spring - 23.],
                [(a + b) * 0.5, spring - 33.],
                [b - width * 0.22, spring - 23.],
                [b - width * 0.07, spring - 11.],
                [b, spring],
            ]
        };
        let near_arch = opening(NEAR_PIER[1], CENTER_PIER[0]);
        let far_arch = opening(CENTER_PIER[1], FAR_PIER[0]);
        let back = |z, y| self.view.project(x - TOWER_HALF_DEPTH, z, y);
        shape(
            paths,
            vec![
                at(NEAR_PIER[0], top),
                at(FAR_PIER[1], top),
                back(FAR_PIER[1], top),
                back(NEAR_PIER[0], top),
            ],
            0x726278,
            1.,
        );
        let rear = |[z, y]: [f32; 2]| back(z, y);
        let first_jamb_course = self
            .course_rows(spring, spring + 10.)
            .next()
            .unwrap_or(spring + 7.);
        for arch in [far_arch, near_arch] {
            for edge in arch.windows(2).skip(3) {
                shape(
                    paths,
                    vec![
                        at(edge[0][0], edge[0][1]),
                        at(edge[1][0], edge[1][1]),
                        rear(edge[1]),
                        rear(edge[0]),
                    ],
                    0x2d2e40,
                    1.,
                );
            }
        }
        // The closest wall covers the portions of the deeper arch throats that
        // project in front of it.
        shape(
            paths,
            vec![
                back(NEAR_PIER[0], top),
                at(NEAR_PIER[0], top),
                at(NEAR_PIER[0], spring),
                back(NEAR_PIER[0], spring),
            ],
            STONE_SIDE,
            1.,
        );
        let outline = [
            at(NEAR_PIER[0], top),
            at(FAR_PIER[1], top),
            at(FAR_PIER[1], spring),
        ]
        .into_iter()
        .chain(far_arch.into_iter().rev().map(|[z, y]| at(z, y)))
        .chain(near_arch.into_iter().rev().map(|[z, y]| at(z, y)))
        .chain([at(NEAR_PIER[0], spring)])
        .collect();
        shape(paths, outline, STONE, 1.);
        let west = NEAR_PIER[0];
        let east = FAR_PIER[1];
        let rows = self.course_rows(top, spring);
        for (row, y) in rows.clone().enumerate() {
            let end = (y + COURSE).min(spring);
            let near = arch_cut(near_arch, y);
            let far = arch_cut(far_arch, y);
            for [a, b] in [[west, near[0]], [near[1], far[0]], [far[1], east]] {
                stroke(paths, at(a, y), at(b, y), 0.65, STONE_SIDE, 0.67);
            }
            let [near, far] = [arch_cut(near_arch, end), arch_cut(far_arch, end)];
            let stagger = (row % 2) as f32 * 5.;
            for z in [-33., -22., -11., 0., 11., 22., 33.].map(|z| z + stagger) {
                if (z < near[0] || z > near[1]) && (z < far[0] || z > far[1]) {
                    stroke(paths, at(z, y), at(z, end), 0.5, 0x292838, 0.36);
                }
            }
        }
        for y in rows {
            stroke(paths, back(west, y), at(west, y), 0.55, 0x211f32, 0.53);
        }
        stroke(
            paths,
            at(west, top),
            at(west, spring),
            0.95,
            STONE_EDGE,
            0.31,
        );
        for (y, width, opacity) in [
            (top + 1., 3., 0.8),
            (top + 7., 1., 0.55),
            (top + 17., 1.6, 0.52),
        ] {
            stroke(
                paths,
                at(NEAR_PIER[0] - 0.7, y),
                at(FAR_PIER[1] + 0.7, y),
                width,
                STONE_EDGE,
                opacity,
            );
        }
        for arch in [near_arch, far_arch] {
            let center = arch[3][0];
            let half = center - arch[0][0];
            let inner = arch.map(|[z, y]| at(z, y));
            let rim = arch.map(|[z, y]| {
                let side = (z - center) / half;
                at(z + side * 3., y - (1. - side.abs()) * 4.)
            });
            for i in 0..6 {
                let stone = [0x675c69, 0x605563][i % 2];
                shape(
                    paths,
                    vec![rim[i], rim[i + 1], inner[i + 1], inner[i]],
                    stone,
                    1.,
                );
            }
            for i in 1..6 {
                stroke(paths, inner[i], rim[i], 0.55, 0x302b38, 0.8);
            }
            for (z, side, ink) in [(arch[0][0], -1., 0x675c69), (arch[6][0], 1., 0x605563)] {
                let block = |y, width: f32| [at(z, y), at(z + side * width, y)];
                let [inside, outside] = block(spring, 3.);
                let [next, edge] = block(first_jamb_course, 2.5);
                let [last, foot] = block(first_jamb_course + COURSE, 1.2);
                shape(paths, vec![inside, outside, edge, next], ink, 0.86);
                shape(paths, vec![next, edge, foot, last], ink, 0.44);
                stroke(paths, next, edge, 0.47, 0x302b38, 0.65);
            }
        }
        // Each steel saddle sits on the cornice and carries the cable over it.
        for z in [-OUTER_CABLE, -4.5, 4.5, OUTER_CABLE] {
            let [x, y] = self.view.project(x, z, top);
            quad(paths, [x - 6.7, y - 2.2, 13.4, 2.2], 0x846e7c, 1.);
            quad(paths, [x - 4.4, y - 5.3, 8.8, 3.2], 0x9c8391, 1.);
        }
    }

    fn road(&self, paths: &mut Vec<art::ArtPath>, road: Roadway) {
        let [outer, wall, inner] = road.edges();
        let (ink, shoulder) = match road {
            Roadway::Far => (0x504b5b, 0x3f3a4e),
            Roadway::Near => (0x51495b, DECK),
        };
        let y = self.deck;
        let at = |x, z| self.view.project(x, z, y);
        let [left, right] = self.ends;
        for [start, end] in self.spans() {
            shape(
                paths,
                vec![
                    at(start, outer),
                    at(end, outer),
                    at(end, wall),
                    at(start, wall),
                ],
                shoulder,
                1.,
            );
            stroke(paths, at(start, outer), at(end, outer), 0.85, STEEL, 0.8);
        }
        shape(
            paths,
            vec![
                at(left, wall),
                at(right, wall),
                at(right, inner),
                at(left, inner),
            ],
            ink,
            1.,
        );
        if matches!(road, Roadway::Far) {
            // This exposed deck edge supports the far roadway inside the view of
            // the center arch. The near roadway and its truss cover the rest.
            let lip = |x| self.view.project(x, inner, y + 3.);
            shape(
                paths,
                vec![at(left, inner), at(right, inner), lip(right), lip(left)],
                0x353143,
                1.,
            );
            stroke(paths, at(left, inner), at(right, inner), 0.68, STEEL, 0.84);
        }
    }

    fn truss(&self, paths: &mut Vec<art::ArtPath>) {
        let at = |x, dy| self.view.project(x, NEAR_PIER[0], self.deck + dy);
        let soffit = self.seat() - self.deck;
        let upper = 3.;
        let lower = soffit - 0.7;
        for [left, right] in self.spans() {
            shape(
                paths,
                vec![
                    at(left, 0.),
                    at(right, 0.),
                    at(right, soffit),
                    at(left, soffit),
                ],
                0x272b3d,
                1.,
            );
            for chord in [0.65, upper, lower] {
                stroke(paths, at(left, chord), at(right, chord), 1.5, STEEL, 0.9);
            }
            let bays = ((right - left) / 18.).ceil() as usize;
            let bay_at = |index| left + (right - left) * index / bays as f32;
            for bay in 0..bays {
                let a = at(bay_at(bay as f32), upper);
                let tip = at(bay_at(bay as f32 + 0.5), lower);
                let b = at(bay_at(bay as f32 + 1.), upper);
                stroke(paths, a, tip, 1.08, STEEL, 0.85);
                stroke(paths, tip, b, 1.08, STEEL, 0.85);
            }
            for end in [left, right] {
                stroke(paths, at(end, upper), at(end, lower), 1.4, STEEL, 0.84);
            }
        }
    }

    fn promenade(&self, paths: &mut Vec<art::ArtPath>) {
        let y = self.deck - 12.;
        let [left, right] = self.ends;
        let [west, east] = self.towers;
        let margin = TOWER_HALF_DEPTH + 24.;
        for [start, end] in [
            [left, west - margin],
            [west + margin, east - margin],
            [east + margin, right],
        ] {
            self.walkway(
                paths,
                y,
                &[[start, -8.], [end, -8.]],
                &[[start, 8.], [end, 8.]],
            );
        }
        self.promenade_bypasses(paths, -1.);
    }

    fn promenade_bypasses(&self, paths: &mut Vec<art::ArtPath>, side: f32) {
        let y = self.deck - 12.;
        for x in self.towers {
            let margin = TOWER_HALF_DEPTH + 24.;
            let corner = TOWER_HALF_DEPTH + 5.;
            let bent = |initial: f32, shifted: f32| {
                [
                    [x - margin, initial * side],
                    [x - corner, shifted * side],
                    [x + corner, shifted * side],
                    [x + margin, initial * side],
                ]
            };
            self.walkway(paths, y, &bent(8., 17.), &bent(0., 9.));
        }
    }

    fn walkway(&self, paths: &mut Vec<art::ArtPath>, y: f32, a: &[[f32; 2]], b: &[[f32; 2]]) {
        let at = |p: &[f32; 2]| self.view.project(p[0], p[1], y);
        let surface = a.iter().chain(b.iter().rev()).map(at).collect();
        shape(paths, surface, 0x786575, 0.96);
        for edge in [a, b] {
            for span in edge.windows(2) {
                let [ax, ay] = at(&span[0]);
                let [bx, by] = at(&span[1]);
                stroke(paths, [ax, ay], [bx, by], 1.35, 0x473b4e, 1.);
                stroke(paths, [ax, ay - 3.5], [bx, by - 3.5], 0.82, 0x9c818c, 0.65);
                let count = ((span[1][0] - span[0][0]) / 15.).ceil() as usize;
                for index in 0..count {
                    let t = index as f32 / count as f32;
                    let x = ax + (bx - ax) * t;
                    let py = ay + (by - ay) * t;
                    quad(paths, [x, py - 3.5, 0.65, 3.5], 0x8b7484, 0.73);
                    if t == 0. && count > 5 {
                        quad(paths, [x, py - 10., 0.95, 10.], STEEL, 1.);
                        quad(paths, [x - 0.9, py - 10.6, 2.7, 2.2], LIGHT, 0.88);
                    }
                }
            }
        }
    }

    fn suspension(
        &self,
        paths: &mut Vec<art::ArtPath>,
        exposed: &mut Vec<art::ArtPath>,
        z: f32,
        ink: u32,
        opacity: f32,
    ) {
        let [west, east] = self.towers;
        let [left, right] = self.ends;
        let outer = z.abs() > 20.;
        let rail = if outer {
            self.deck + 3.
        } else {
            self.deck - 10.
        };
        let top = self.top() - 6.;
        let rise = self.deck - 17. - top;
        let at = |x, y| self.view.project(x, z, y);
        let spans = self.spans();
        let mut sockets = Vec::new();
        let intervals = [
            [left, west - 4.9],
            [west - 4.9, west + 4.9],
            [west + 4.9, east - 4.9],
            [east - 4.9, east + 4.9],
            [east + 4.9, right],
        ];
        for (index, [start, finish]) in intervals.into_iter().enumerate() {
            let cable = |x: f32| {
                let t = ((x - start) / (finish - start)).clamp(0., 1.);
                let sag = match index {
                    0 => 0.91 * (1. - t).powf(1.12),
                    2 => 4. * t * (1. - t),
                    4 => 0.91 * t.powf(1.12),
                    _ => 0.,
                };
                at(x, top + rise * sag)
            };
            let count = ((finish - start) / 8.).ceil() as usize;
            let points: Vec<_> = (0..=count)
                .map(|i| cable(start + (finish - start) * i as f32 / count as f32))
                .collect();
            let weight = if outer { 1.65 } else { 1.15 };
            let cable_ribbon = |paths: &mut Vec<art::ArtPath>, points: &[[f32; 2]]| {
                let ribbon = points
                    .iter()
                    .copied()
                    .chain(points.iter().rev().map(|&[x, y]| [x, y + weight]))
                    .collect();
                shape(paths, ribbon, ink, opacity);
            };
            // The camera sees the right face of each tower. A cable leaving
            // that face passes in front of the masonry until it clears it.
            match index {
                0 => cable_ribbon(paths, &points),
                2 => {
                    let mid = points.len() / 2;
                    cable_ribbon(exposed, &points[..=mid]);
                    cable_ribbon(paths, &points[mid..]);
                }
                _ => cable_ribbon(exposed, &points),
            }
            if index.is_multiple_of(2) {
                let [a, b] = spans[index / 2];
                let pitch = if outer { 19. } else { 29. };
                let bays = ((b - a) / pitch).round().max(1.) as usize;
                stroke(
                    paths,
                    at(a, rail),
                    at(b, rail),
                    1.15,
                    STEEL,
                    if outer { 0.68 } else { 0.36 },
                );
                for i in 1..bays {
                    let x = a + (b - a) * i as f32 / bays as f32;
                    let socket = at(x, rail);
                    stroke(paths, cable(x), socket, 0.65, ink, opacity * 0.55);
                    if outer {
                        sockets.push(socket);
                    }
                }
            }
        }
        if outer {
            for (base, ends) in [(west, [left, east]), (east, [west, right])] {
                for end in ends {
                    let face = base + (end - base).signum() * TOWER_HALF_DEPTH;
                    let layer = if end > base {
                        &mut *exposed
                    } else {
                        &mut *paths
                    };
                    for fraction in [0.15, 0.24, 0.33] {
                        let socket = at(base + (end - base) * fraction, rail);
                        stroke(
                            layer,
                            at(face, top + 5.5),
                            socket,
                            0.68,
                            ink,
                            opacity * 0.53,
                        );
                        sockets.push(socket);
                    }
                }
            }
            for [x, y] in sockets {
                quad(paths, [x - 0.9, y - 1., 1.8, 2.2], ink, opacity * 0.84);
            }
        }
    }
}

fn mesh(shapes: &[art::ArtPath], scale: f32) -> Mesh {
    shapes
        .chunk_by(|a, b| a.ink == b.ink && a.opacity == b.opacity)
        .filter_map(|group| {
            let mut builder = gpui_kit::PathBuilder::fill();
            let at = |p: &[f32; 2]| gpui_kit::point(px(p[0] * scale), px(p[1] * scale));
            for shape in group {
                builder.move_to(at(&shape.points[0]));
                for point in &shape.points[1..] {
                    builder.line_to(at(point));
                }
                builder.close();
            }
            let color = gpui_kit::Hsla::from(rgb(group[0].ink)).opacity(group[0].opacity);
            builder.build().ok().map(|path| (path, color))
        })
        .collect()
}

fn paint(layers: Mesh) -> impl IntoElement {
    gpui_kit::canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            let origin = bounds.origin;
            for (path, color) in layers.iter() {
                let mut path = path.clone();
                path.bounds.origin += origin;
                for vertex in &mut path.vertices {
                    vertex.xy_position += origin;
                }
                window.paint_path(path, *color);
            }
        },
    )
    .absolute()
    .inset_0()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn top_opaque_ink<'a>(
        paths: impl DoubleEndedIterator<Item = &'a art::ArtPath>,
        sample: [f32; 2],
    ) -> Option<u32> {
        paths.rev().find_map(|path| {
            let inside = path
                .points
                .iter()
                .enumerate()
                .fold(false, |inside, (i, &[x, y])| {
                    let [other_x, other_y] = path.points[(i + 1) % path.points.len()];
                    inside
                        ^ ((y > sample[1]) != (other_y > sample[1])
                            && sample[0] < x + (other_x - x) * (sample[1] - y) / (other_y - y))
                });
            (path.opacity == 1. && inside).then_some(path.ink)
        })
    }

    #[test]
    fn near_lane_pavement_remains_visible_below_a_truck_at_the_arch() {
        let view = View::new(WIDTH, HEIGHT * HORIZON, Aim::default());
        let bridge = Bridge {
            view,
            ends: [-OVERHANG, WIDTH + OVERHANG],
            towers: towers(WIDTH),
            deck: view.horizon - 53.,
        };
        let [x, y] = view.project(bridge.towers[1] + 13., -21.3, bridge.deck);
        let sample = [x, y + 0.8];
        let layers = bridge.layers();
        assert_eq!(
            top_opaque_ink(layers[1].iter(), sample),
            Some(0x51495b),
            "The roadway under a tire should be visible where it passes in front of the center pier"
        );
    }

    #[test]
    fn stone_directly_below_the_near_leg_is_visible_in_front_of_the_truss() {
        for yaw in [-1., 0., 1.] {
            for tilt in [-1., 0., 1.] {
                let view = View::new(WIDTH, HEIGHT * HORIZON, Aim::new(yaw, tilt));
                let bridge = Bridge {
                    view,
                    ends: [-OVERHANG, WIDTH + OVERHANG],
                    towers: towers(WIDTH),
                    deck: view.horizon - 53.,
                };
                let layers = bridge.layers();
                for tower in bridge.towers {
                    let [x, y] =
                        view.project(tower + TOWER_HALF_DEPTH, NEAR_PIER[1], bridge.seat());
                    let mut stone = Vec::new();
                    bridge.near_bearing_face(&mut stone, tower);
                    assert_eq!(stone[0].points[1], [x, y]);
                    assert_eq!(
                        top_opaque_ink(layers.iter().flatten(), [x, y + 0.9]),
                        Some(SEAT_INK),
                        "The sloping stone seat should be visible below the back of the leg in {view:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn footing_cap_tower_legs_and_truss_share_bearing_corners() {
        for yaw in [-1., 0., 1.] {
            for tilt in [-1., 0., 1.] {
                let view = View::new(WIDTH, HEIGHT * HORIZON, Aim::new(yaw, tilt));
                let bridge = Bridge {
                    view,
                    ends: [-OVERHANG, WIDTH + OVERHANG],
                    towers: towers(WIDTH),
                    deck: view.horizon - 53.,
                };
                let mut truss = Vec::new();
                bridge.truss(&mut truss);
                let spans: Vec<_> = truss.iter().filter(|path| path.ink == 0x272b3d).collect();
                let diagonals = truss
                    .iter()
                    .filter(|path| path.ink == STEEL && path.opacity == 0.85);
                let layers = bridge.layers();
                for (i, tower) in bridge.towers.into_iter().enumerate() {
                    let mut foundation = Vec::new();
                    bridge.foundation(&mut foundation, tower);
                    let cap = foundation.iter().find(|path| path.opacity == 1.).unwrap();

                    let mut near = Vec::new();
                    bridge.masonry(&mut near, tower, NEAR_PIER, STONE);
                    let mut far = Vec::new();
                    bridge.masonry(&mut far, tower, FAR_PIER, STONE);
                    let seated = [near[0].points[3], near[0].points[2], far[1].points[2]];
                    for (cap, leg) in cap.points.iter().zip(seated) {
                        assert!(
                            (cap[0] - leg[0]).abs() < 0.02 && (cap[1] - leg[1]).abs() < 0.02,
                            "Bearing corner {cap:?} misses the leg at {leg:?} in {view:?}"
                        );
                    }
                    assert_eq!(spans[i].points[2], cap.points[0]);
                    assert_eq!(spans[i + 1].points[3], cap.points[1]);
                    assert!(
                        layers[0].iter().any(|path| {
                            path.points.contains(&cap.points[1])
                                && path.points.contains(&cap.points[2])
                                && path.points.contains(&view.project(
                                    tower + TOWER_HALF_DEPTH,
                                    FAR_PIER[1],
                                    bridge.deck,
                                ))
                        }),
                        "The right end of the deck should join the front and rear bearing corners"
                    );
                    for x in [tower - TOWER_HALF_DEPTH, tower + TOWER_HALF_DEPTH] {
                        let joint = view.project(x, NEAR_PIER[0], bridge.deck + 3.);
                        assert!(
                            diagonals.clone().any(|brace| {
                                [(0, 3), (1, 2)].into_iter().any(|(a, b)| {
                                    (0..2).all(|axis| {
                                        ((brace.points[a][axis] + brace.points[b][axis]) * 0.5
                                            - joint[axis])
                                            .abs()
                                            < 0.01
                                    })
                                })
                            }),
                            "A diagonal should reach the end of the upper chord at {joint:?} in {view:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn boats_keep_visible_clearance_from_both_pier_footings() {
        let bounds = |points: &[[f32; 2]]| {
            let mut min = [f32::INFINITY; 2];
            let mut max = [f32::NEG_INFINITY; 2];
            for point in points {
                for axis in 0..2 {
                    min[axis] = min[axis].min(point[axis]);
                    max[axis] = max[axis].max(point[axis]);
                }
            }
            [min, max]
        };
        for width in [900., 995.55, 1236., 1800.] {
            for yaw in [-1., 0., 1.] {
                for tilt in [-1., 0., 1.] {
                    let view = View::new(width, 560. * HORIZON, Aim::new(yaw, tilt));
                    let [left, right, half] = FOOTING;
                    let piers = towers(width).map(|x| {
                        bounds(
                            &[[left, -half], [right, -half], [right, half], [left, half]]
                                .map(|[dx, z]| view.on_water(x + dx, z)),
                        )
                    });
                    for sample in 0..100 {
                        let time = sample as f32 * 2.6;
                        let extent = |shapes: &[art::ArtPath]| {
                            let points: Vec<_> = shapes
                                .iter()
                                .flat_map(|shape| shape.points.iter().copied())
                                .collect();
                            bounds(&points)
                        };
                        let [tug_left, tug_right] = extent(&tugboat(view, time));
                        assert!(
                            tug_left[0] > piers[0][1][0] + 6. && tug_right[0] < piers[1][0][0] - 6.,
                            "Tug at {time}s in {view:?}"
                        );

                        let foreground = [ferry(view, time), sailboat(view, time)]
                            .map(|paths| extent(&paths[9..]));
                        let [ferry, sailboat] = foreground;
                        assert!(ferry[1][0] < sailboat[0][0] || sailboat[1][0] < ferry[0][0]);
                        for [[left, top], [right, _]] in foreground {
                            for [[pier_left, _], [pier_right, pier_bottom]] in piers {
                                if right > pier_left && left < pier_right {
                                    assert!(top > pier_bottom + 7., "Boat at {time}s in {view:?}");
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

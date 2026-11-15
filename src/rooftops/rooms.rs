//! Procedural apartment interiors shared by every facade. Room art is painted
//! into small atlas entries when a pose is first needed, never per building.

use super::neighborhood::{Tile, TilePainter};

const PHASES: usize = 16;
const ROOM_VARIANTS: usize = 12;
const TAU: f32 = std::f32::consts::TAU;
const INK: u32 = 0x39303d;
type Joint = (f32, f32);

#[derive(Default)]
pub(super) struct RoomArt(std::collections::HashMap<(usize, usize), Tile>);

impl RoomArt {
    pub(super) fn interior(&mut self, seed: usize, time: f32) -> Tile {
        let phase = ((time * 1.5 + seed as f32) / TAU * PHASES as f32)
            .round()
            .rem_euclid(PHASES as f32) as usize;
        self.0
            .entry((seed % ROOM_VARIANTS, phase))
            .or_insert_with(|| apartment(seed % ROOM_VARIANTS, phase))
            .clone()
    }

    pub(super) fn still(&mut self, seed: usize) -> Tile {
        self.interior(seed, 0.)
    }
}

fn apartment(seed: usize, phase: usize) -> Tile {
    let angle = phase as f32 / PHASES as f32 * TAU;
    let wave = angle.sin();
    let light = [0xd4b58b, 0xc9b69a, 0xdac39b, 0xd1aa8c][seed % 4];
    Tile::draw([0., 0., 66., 62.], |room| {
        room.rect([0., 0., 66., 62.], light);
        room.rect([0., 56., 66., 6.], 0x8e746e);
        room.rounded([0., 0., 66., 14.], 0., 0xffe0ac, 0.13);
        match seed % 6 {
            0 => {
                // Vacuuming, with a planted stance and long handle.
                let base = 46. + wave * 2.;
                let grip = (base - 14., 31.);
                for (a, b, widths) in [
                    ((8., 56.), (base - 12., 56.), (0.8, 0.8)),
                    ((base - 12., 56.), (base - 8., 51.), (0.8, 0.8)),
                    ((base - 3., 52.), (base - 10., 39.), (5.5, 7.)),
                    ((base - 3., 52.), (base - 16., 27.), (1.7, 1.7)),
                    ((base - 16., 27.), (base - 12., 27.), (2.3, 2.3)),
                ] {
                    limb(room, a, b, widths);
                }
                resident(
                    room,
                    Resident {
                        shoulder: (24. + wave * 0.3, 24.),
                        hip: (22., 36.),
                        hands: [grip, (20., 37.)],
                        feet: [(18., 56.), (28., 56.)],
                        facing: 1.,
                        seated: false,
                    },
                );
                room.rounded([base - 7., 52., 17., 4.], 1.5, INK, 1.);
                room.circle(base - 5., 54., 3., INK, 1.);
                room.circle(base + 6., 54., 3., INK, 1.);
            }
            1 => {
                room.rounded([37., 29., 25., 13.], 3., INK, 1.);
                for rect in [
                    [34., 41., 28., 3.],
                    [39., 44., 2., 13.],
                    [59., 44., 2., 13.],
                    [14., 44., 17., 3.],
                    [17., 47., 2., 10.],
                ] {
                    room.rect(rect, INK);
                }
                room.rect([35., 41., 26., 1.5], light);
                resident(
                    room,
                    Resident {
                        shoulder: (23., 29.),
                        hip: (22., 43.),
                        hands: [(33., 39. + wave * 0.3), (35., 40. - wave * 0.3)],
                        feet: [(32., 56.), (28., 56.)],
                        facing: 1.,
                        seated: true,
                    },
                );
                for key in 0..7 {
                    room.rect([36. + key as f32 * 3.5, 41., 1., 1.], INK);
                }
            }
            2 => {
                let hand = (31. + wave * 0.35, 33. + wave * 0.25);
                resident(
                    room,
                    Resident {
                        shoulder: (43., 23.),
                        hip: (44., 36.),
                        hands: [hand, (34., 35.)],
                        feet: [(40., 56.), (46., 56.)],
                        facing: -1.,
                        seated: false,
                    },
                );
                room.rounded([hand.0 - 7., hand.1, 8., 6.], 2., INK, 1.);
                limb(room, (hand.0 - 6., hand.1 + 3.), (20., 37.), (2., 1.2));
                room.rounded([8., 48., 14., 9.], 3., INK, 1.);
                limb(room, (15., 50.), (15., 31.), (1.4, 1.1));
                for leaf in 0..4 {
                    let y = 32. + leaf as f32 * 4.;
                    limb(
                        room,
                        (15., y + 3.),
                        (if leaf % 2 == 0 { 9. } else { 21. }, y),
                        (3.8, 1.),
                    );
                }
                for drop in 0..2 {
                    room.circle(
                        20.,
                        39. + (angle * 7. / TAU + drop as f32 * 4.).rem_euclid(7.),
                        1.,
                        INK,
                        1.,
                    );
                }
            }
            3 => {
                let hand = (30. + wave * 0.8, 33. + angle.cos() * 0.4);
                room.rect([7., 43., 26., 14.], INK);
                room.rounded([11., 37., 15., 7.], 3., INK, 1.);
                room.rect([24., 38., 7., 1.5], INK);
                resident(
                    room,
                    Resident {
                        shoulder: (42., 23.),
                        hip: (45., 36.),
                        hands: [hand, (35., 34.)],
                        feet: [(41., 56.), (48., 56.)],
                        facing: -1.,
                        seated: false,
                    },
                );
                limb(room, hand, (23. + wave * 0.8, 41.), (1., 1.));
                room.rounded([10., 47., 20., 7.], 0., light, 0.2);
                room.circle(11., 44., 1.5, light, 1.);
                room.circle(16., 44., 1.5, light, 1.);
                for puff in 0..3 {
                    let rise = (angle * 28. / TAU + puff as f32 * 5.).rem_euclid(14.);
                    room.circle(
                        14. + rise * 0.2,
                        30. - rise,
                        3.,
                        INK,
                        (1. - rise / 14.) * 0.25,
                    );
                }
            }
            4 => {
                resident(
                    room,
                    Resident {
                        shoulder: (33. - wave * 0.7, 22.),
                        hip: (33. + wave * 0.8, 36.),
                        hands: [(21., 20. + wave), (45., 17. - wave)],
                        feet: [(27., 55.), (40., 56.)],
                        facing: 1.,
                        seated: false,
                    },
                );
                for rect in [[3., 44., 15., 4.], [5., 48., 2., 9.], [14., 48., 2., 9.]] {
                    room.rect(rect, INK);
                }
                limb(room, (7., 41.), (15., 41.), (2., 2.));
                limb(room, (15., 39.), (12., 42.), (1., 1.));
            }
            _ => {
                resident(
                    room,
                    Resident {
                        shoulder: (29., 30.),
                        hip: (22., 37.),
                        hands: [(39., 33. + wave * 0.2), (36., 37.)],
                        feet: [(17., 56.), (28., 56.)],
                        facing: 1.,
                        seated: false,
                    },
                );
                for (a, b, widths) in [
                    ((33., 23.), (55., 17.), (5., 7.)),
                    ((44., 22.), (44., 39.), (2., 2.)),
                    ((44., 38.), (36., 56.), (1.5, 1.)),
                    ((44., 38.), (53., 56.), (1.5, 1.)),
                    ((44., 38.), (44., 57.), (1.5, 1.)),
                ] {
                    limb(room, a, b, widths);
                }
            }
        }
        for x in [0., 62.] {
            room.rounded([x, 0., 4., 56.], 0., INK, 0.14);
        }
    })
}

struct Resident {
    shoulder: Joint,
    hip: Joint,
    hands: [Joint; 2],
    feet: [Joint; 2],
    facing: f32,
    seated: bool,
}

fn resident(room: &mut TilePainter<'_>, pose: Resident) {
    let Resident {
        shoulder,
        hip,
        hands,
        feet,
        facing,
        seated,
    } = pose;
    let head = (shoulder.0 + facing * 0.6, shoulder.1 - 7.);
    let mut path = tiny_skia::PathBuilder::new();
    for (i, foot) in feet.into_iter().enumerate() {
        let pelvis = (hip.0 + (i as f32 - 0.5) * 2.5, hip.1);
        let knee = elbow(pelvis, foot, 10., if seated { -facing } else { facing });
        append_limb(&mut path, pelvis, knee, (4.1, 3.));
        append_limb(&mut path, knee, foot, (3., 1.9));
        append_limb(
            &mut path,
            foot,
            (foot.0 + facing * 3., foot.1 + 0.3),
            (2.2, 2.5),
        );
    }
    append_limb(&mut path, shoulder, hip, (7.5, 5.8));
    append_limb(&mut path, (head.0, head.1 + 2.), shoulder, (2.6, 3.));
    append_limb(
        &mut path,
        (head.0, head.1 - 1.),
        (head.0, head.1 + 1.3),
        (5.4, 4.6),
    );
    append_limb(
        &mut path,
        head,
        (head.0 + facing * 2., head.1 + 0.7),
        (2.2, 1.8),
    );
    for (i, hand) in hands.into_iter().enumerate() {
        let socket = (shoulder.0 + (i as f32 - 0.5) * 3., shoulder.1 + 0.6);
        let joint = elbow(socket, hand, 7.5, if i == 0 { -1. } else { 1. });
        append_limb(&mut path, socket, joint, (3.2, 2.3));
        append_limb(&mut path, joint, hand, (2.3, 2.2));
    }
    room.fill_ink(path, INK);
}

fn elbow(a: Joint, b: Joint, length: f32, bend: f32) -> Joint {
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    let distance = dx.hypot(dy).max(0.01);
    let offset = (length * length - distance * distance * 0.25)
        .max(0.)
        .sqrt()
        * bend;
    (
        (a.0 + b.0) * 0.5 - dy / distance * offset,
        (a.1 + b.1) * 0.5 + dx / distance * offset,
    )
}

fn limb(room: &mut TilePainter<'_>, a: Joint, b: Joint, widths: (f32, f32)) {
    let mut path = tiny_skia::PathBuilder::new();
    append_limb(&mut path, a, b, widths);
    room.fill_ink(path, INK);
}

fn append_limb(path: &mut tiny_skia::PathBuilder, a: Joint, b: Joint, (start, end): (f32, f32)) {
    use std::f32::consts::FRAC_1_SQRT_2 as D;
    const ARC: [(f32, f32); 5] = [(0., 1.), (D, D), (1., 0.), (D, -D), (0., -1.)];
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    let length = dx.hypot(dy);
    let (dx, dy) = if length > 0. {
        (dx / length, dy / length)
    } else {
        (1., 0.)
    };
    let cap = |(x, y): Joint, radius: f32| {
        ARC.map(|(forward, side)| {
            (
                x + radius * (dx * forward - dy * side),
                y + radius * (dy * forward + dx * side),
            )
        })
    };
    let front = cap(b, end * 0.5);
    path.move_to(front[0].0, front[0].1);
    for (x, y) in front.into_iter().skip(1).chain(cap(a, -start * 0.5)) {
        path.line_to(x, y);
    }
    path.close();
}

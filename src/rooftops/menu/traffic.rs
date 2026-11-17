//! Traffic projected with the same road plane as the bridge around it.

use super::*;

#[derive(Clone, Copy)]
enum Vehicle {
    Car,
    Truck,
    Semi,
    Bus,
}

impl Vehicle {
    fn width(self) -> f32 {
        match self {
            Self::Car => 15.,
            Self::Truck => 20.,
            Self::Semi => 33.,
            Self::Bus => 27.,
        }
    }

    fn half_track(self) -> f32 {
        match self {
            Self::Car => 2.7,
            Self::Truck => 3.3,
            Self::Semi | Self::Bus => 3.9,
        }
    }

    fn paint(self, painter: &mut VehiclePainter<'_>) {
        let width = self.width();
        let axles: &[_] = match self {
            Self::Semi => &[5., 10., 24., 29.],
            _ => &[3., width - 5.2],
        };
        for &axle in axles {
            painter.wheel(axle, -painter.near_side, false);
        }
        use End::*;
        let glass = 0x8196b0;
        match self {
            Self::Car => {
                let paint = [0x986a77, 0xba8990, 0x795361];
                painter.body([0., width], [-4.7, -1.25], paint, [Tailgate, Grille]);
                painter.body([3.2, 11.3], [-7.1, -4.7], paint, [RearGlass, Windshield]);
                painter.side([4.3, -6.65, 5.7, 1.65], glass);
                painter.side([7.3, -6.65, 0.5, 4.55], 0x785966);
                painter.side([8.05, -4.15, 1., 0.42], 0xc99a9c);
            }
            Self::Truck => {
                let paint = [0x667b90, 0x8197a9, 0x4c5c73];
                painter.body([0., width], [-5.5, -1.2], paint, [Tailgate, Grille]);
                painter.roof([0.9, 9.7], -5.5, 0x45576c);
                painter.body([10.1, 17.], [-8.7, -5.5], paint, [RearGlass, Windshield]);
                painter.side([11.7, -8.05, 3.9, 2.25], glass);
                painter.side([11.35, -5.5, 0.43, 3.4], 0x4c5e73);
                painter.side([12.1, -4.8, 1.35, 0.5], 0x9aaab7);
            }
            Self::Semi => {
                painter.body(
                    [4., 30.],
                    [-3., -1.1],
                    [0x644b55, 0x766069, 0x4c384a],
                    [Plain; 2],
                );
                let trailer = |painter: &mut VehiclePainter<'_>| {
                    painter.body(
                        [0., 22.1],
                        [-10.2, -2.1],
                        [0xaca198, 0xc5b4a6, 0x837b7f],
                        [CargoDoors, Plain],
                    );
                    painter.side([0.5, -9.7, 21.1, 0.6], 0xc4b6a8);
                    painter.side([0.55, -3.55, 21., 0.8], 0x82777b);
                    for x in [5.5, 11., 16.5] {
                        painter.side([x, -8.75, 0.26, 4.6], 0x9d918b);
                    }
                };
                let cab = |painter: &mut VehiclePainter<'_>| {
                    let paint = [0x88636f, 0xb18187, 0x6a4e63];
                    painter.body([23., width], [-5.5, -1.2], paint, [Plain, Grille]);
                    painter.body([23., 30.4], [-8.5, -5.5], paint, [RearGlass, Windshield]);
                    painter.side([25.8, -8., 3.4, 2.15], glass);
                    painter.side([25.5, -5.5, 0.39, 3.1], 0x694b5b);
                };
                if painter.eye_x > 22.1 {
                    trailer(painter);
                    cab(painter);
                } else {
                    cab(painter);
                    trailer(painter);
                }
            }
            Self::Bus => {
                painter.body(
                    [0., width],
                    [-9.3, -1.3],
                    [0xa28065, 0xc0a283, 0x795e57],
                    [BusRear, BusFront],
                );
                painter.side([0., -3.4, width, 1.], 0xc59d7b);
                for i in 0..5 {
                    let x = 1.4 + i as f32 * 3.8;
                    painter.side([x, -7.85, 2.95, 2.65], glass);
                    painter.side([x + 0.25, -7.6, 2.45, 0.42], 0xa5b2c2);
                }
                painter.side([20.5, -8.05, 3.7, 6.55], 0x79635b);
                painter.side([20.85, -7.75, 1.3, 3.2], glass);
                painter.side([22.53, -7.75, 1.3, 3.2], glass);
                painter.side([24.8, -7.85, 1.35, 2.65], glass);
            }
        }
        for &axle in axles {
            painter.wheel(axle, painter.near_side, true);
        }
        painter.side([0.05, -3.8, 1.2, 1.1], 0xd88280);
        painter.side([width - 1.3, -3.8, 1.25, 1.1], 0xffdb9b);
    }
}

#[derive(Clone, Copy)]
enum End {
    Plain,
    Grille,
    Tailgate,
    Windshield,
    RearGlass,
    CargoDoors,
    BusFront,
    BusRear,
}

impl End {
    fn paint(self, mut panel: impl FnMut([f32; 4], u32)) {
        let glass = 0x7a8fa9;
        let dark = 0x403844;
        match self {
            Self::Plain => {}
            Self::Windshield | Self::RearGlass => {
                panel([0.13, 0.17, 0.74, 0.69], 0x424858);
                panel([0.18, 0.23, 0.64, 0.5], glass);
                panel([0.2, 0.25, 0.59, 0.12], 0x99a9bd);
            }
            Self::Grille | Self::BusFront => {
                let grille_y = if matches!(self, Self::BusFront) {
                    panel([0.13, 0.08, 0.74, 0.11], dark);
                    panel([0.29, 0.105, 0.42, 0.048], 0xd2a672);
                    panel([0.12, 0.24, 0.76, 0.35], 0x4b4e60);
                    panel([0.16, 0.27, 0.31, 0.28], glass);
                    panel([0.53, 0.27, 0.31, 0.28], glass);
                    0.66
                } else {
                    0.23
                };
                panel([0.28, grille_y, 0.44, 0.78 - grille_y], dark);
                panel([0.32, grille_y + 0.06, 0.36, 0.045], 0x80727b);
                panel([0.075, 0.63, 0.18, 0.16], 0xffdd9f);
                panel([0.745, 0.63, 0.18, 0.16], 0xffdd9f);
                panel([0.03, 0.85, 0.94, 0.115], 0x9e9294);
            }
            Self::Tailgate | Self::CargoDoors | Self::BusRear => {
                match self {
                    Self::CargoDoors => {
                        panel([0.11, 0.1, 0.78, 0.65], 0x655c64);
                        for u in [0.135, 0.51] {
                            panel([u, 0.135, 0.355, 0.58], 0x9a8c87);
                            panel([u + 0.08, 0.16, 0.03, 0.51], 0x655c64);
                            panel([u + 0.08, 0.53, 0.12, 0.055], 0xb6a79c);
                        }
                    }
                    Self::BusRear => {
                        panel([0.17, 0.15, 0.66, 0.31], dark);
                        panel([0.22, 0.18, 0.56, 0.24], glass);
                        panel([0.29, 0.59, 0.42, 0.05], 0x9d7d69);
                    }
                    _ => {
                        panel([0.21, 0.23, 0.58, 0.05], dark);
                        panel([0.42, 0.34, 0.16, 0.09], 0xa29699);
                    }
                }
                panel([0.076, 0.68, 0.15, 0.14], 0xd88280);
                panel([0.774, 0.68, 0.15, 0.14], 0xd88280);
                panel([0.035, 0.87, 0.93, 0.095], dark);
                panel([0.42, 0.8, 0.16, 0.105], 0xb3a5a2);
            }
        }
    }
}

struct VehiclePainter<'a> {
    window: &'a mut Window,
    origin: gpui_kit::Point<gpui_kit::Pixels>,
    view: View,
    scale: f32,
    start: f32,
    direction: f32,
    depth: f32,
    half_track: f32,
    near_side: f32,
    eye_x: f32,
    eye_y: f32,
    road_y: f32,
}

impl VehiclePainter<'_> {
    fn face<const N: usize>(&mut self, points: [[f32; 3]; N], ink: u32) {
        let face = self.view.polygon(points.map(|[x, z, y]| {
            [
                self.start + x * self.direction,
                self.road_y + y,
                self.depth + z,
            ]
        }));
        let at = |[x, y]: [f32; 2]| {
            self.origin + gpui_kit::point(px(x * self.scale), px(y * self.scale))
        };
        if face.len() < 3 {
            return;
        }
        let mut path = gpui_kit::Path::new(at(face[0]));
        for &point in &face[1..] {
            path.line_to(at(point));
        }
        self.window.paint_path(path, rgb(ink));
    }

    fn side(&mut self, [x, y, w, h]: [f32; 4], ink: u32) {
        let z = self.near_side;
        self.face(
            [[x, z, y], [x + w, z, y], [x + w, z, y + h], [x, z, y + h]],
            ink,
        );
    }

    fn body(
        &mut self,
        [left, right]: [f32; 2],
        [top, bottom]: [f32; 2],
        [side, roof, end]: [u32; 3],
        [rear, front]: [End; 2],
    ) {
        let z = self.half_track;
        let near = [[left, -z, top], [right, -z, top]];
        let far = [[left, z, top], [right, z, top]];
        if self.eye_y < self.road_y + top {
            self.face([near[0], near[1], far[1], far[0]], roof);
        }
        self.side([left, top, right - left, bottom - top], side);
        let (x, detail) = if self.eye_x < left {
            (left, rear)
        } else if self.eye_x > right {
            (right, front)
        } else {
            return;
        };
        self.face(
            [[x, -z, top], [x, z, top], [x, z, bottom], [x, -z, bottom]],
            end,
        );
        detail.paint(|[u, v, w, h], ink| {
            let a = z * (2. * u - 1.);
            let b = a + 2. * z * w;
            let y = top + v * (bottom - top);
            let foot = y + h * (bottom - top);
            self.face([[x, a, y], [x, b, y], [x, b, foot], [x, a, foot]], ink);
        });
    }

    fn roof(&mut self, [left, right]: [f32; 2], y: f32, ink: u32) {
        if self.eye_y < self.road_y + y {
            let z = self.half_track * 0.73;
            self.face(
                [[left, -z, y], [right, -z, y], [right, z, y], [left, z, y]],
                ink,
            );
        }
    }

    fn wheel(&mut self, axle: f32, z: f32, show_hub: bool) {
        let radius = 1.42;
        let center = axle + radius;
        let circle = |r| {
            std::array::from_fn::<_, 16, _>(|i| {
                let angle = i as f32 * std::f32::consts::TAU / 16.;
                let (sin, cos) = angle.sin_cos();
                // The lowest point sits on the pavement.
                [center + r * sin, z, -radius + r * cos]
            })
        };
        self.face(circle(radius), 0x1d1b2b);
        if show_hub {
            self.face(circle(0.61), 0x777486);
        }
    }
}

const LANES: [[(Vehicle, f32); 5]; 2] = [
    [
        (Vehicle::Car, 0.08),
        (Vehicle::Semi, 0.26),
        (Vehicle::Car, 0.43),
        (Vehicle::Truck, 0.68),
        (Vehicle::Bus, 0.88),
    ],
    [
        (Vehicle::Bus, 0.16),
        (Vehicle::Car, 0.31),
        (Vehicle::Truck, 0.57),
        (Vehicle::Car, 0.74),
        (Vehicle::Semi, 0.94),
    ],
];

pub(super) fn lane(
    view: View,
    road_y: f32,
    time: f32,
    depth: f32,
    scale: f32,
    westbound: bool,
) -> impl IntoElement {
    gpui_kit::canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            const PAD: f32 = 105.;
            let length = view.width + PAD * 2.;
            let speed: f32 = if westbound { -31. } else { 38. };
            let direction = speed.signum();
            let eye = view
                .camera
                .unwrap_or_else(|| projection::Camera::at_menu(view))
                .eye;
            for &(vehicle, departure) in &LANES[usize::from(!westbound)] {
                let x = (departure * length + time * speed).rem_euclid(length) - PAD;
                let half_width = vehicle.width() * 0.5;
                let start = x - half_width * direction;
                let half_track = vehicle.half_track();
                vehicle.paint(&mut VehiclePainter {
                    window,
                    origin: bounds.origin,
                    view,
                    scale,
                    start,
                    direction,
                    depth,
                    half_track,
                    near_side: if eye[2] < depth {
                        -half_track
                    } else {
                        half_track
                    },
                    eye_x: (eye[0] - start) * direction,
                    eye_y: eye[1],
                    road_y,
                });
            }
        },
    )
    .absolute()
    .inset_0()
}

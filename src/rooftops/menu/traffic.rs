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
            painter.wheel(axle, painter.half_track);
        }
        let glass = 0x8694ae;
        match self {
            Self::Car => {
                let paint = [0x986a77, 0xba8990, 0x795361];
                painter.body([0., width], [-4.7, -1.25], paint);
                painter.body([3.2, 11.3], [-7.1, -4.7], paint);
                painter.side([4.3, -6.65, 5.7, 1.65], glass);
            }
            Self::Truck => {
                let paint = [0x667b90, 0x8197a9, 0x4c5c73];
                painter.body([0., width], [-5.5, -1.2], paint);
                painter.body([10.1, 17.], [-8.7, -5.5], paint);
                painter.side([11.7, -8.05, 3.9, 2.25], glass);
            }
            Self::Semi => {
                painter.body([4., 30.], [-3., -1.1], [0x644b55, 0x766069, 0x4c384a]);
                painter.body([0., 22.1], [-10.2, -2.1], [0xaca198, 0xc5b4a6, 0x837b7f]);
                let paint = [0x88636f, 0xb18187, 0x6a4e63];
                painter.body([23., width], [-5.5, -1.2], paint);
                painter.body([23., 30.4], [-8.5, -5.5], paint);
                painter.side([25.8, -8., 3.4, 2.15], glass);
            }
            Self::Bus => {
                painter.body([0., width], [-9.3, -1.3], [0xa28065, 0xc0a283, 0x795e57]);
                painter.side([0., -3.4, width, 1.], 0xc59d7b);
                for i in 0..6 {
                    painter.side([1.6 + i as f32 * 4.1, -7.8, 2.7, 2.65], glass);
                }
            }
        }
        for &axle in axles {
            painter.wheel(axle, -painter.half_track);
        }
        painter.side([0.05, -3.8, 1.2, 1.1], 0xd88280);
        painter.side([width - 1.3, -3.8, 1.25, 1.1], 0xffdb9b);
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
    road_y: f32,
}

impl VehiclePainter<'_> {
    fn project(&self, [x, z, y]: [f32; 3]) -> gpui_kit::Point<gpui_kit::Pixels> {
        let [x, y] = self.view.project(
            self.start + x * self.direction,
            self.depth + z,
            self.road_y + y,
        );
        self.origin + gpui_kit::point(px(x * self.scale), px(y * self.scale))
    }

    fn face<const N: usize>(&mut self, points: [[f32; 3]; N], ink: u32) {
        let face = points.map(|point| self.project(point));
        let mut path = gpui_kit::Path::new(face[0]);
        for point in &face[1..] {
            path.line_to(*point);
        }
        self.window.paint_path(path, rgb(ink));
    }

    fn side(&mut self, [x, y, w, h]: [f32; 4], ink: u32) {
        let z = -self.half_track;
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
    ) {
        let z = self.half_track;
        let near = [[left, -z, top], [right, -z, top]];
        let far = [[left, z, top], [right, z, top]];
        self.face([near[0], near[1], far[1], far[0]], roof);
        self.side([left, top, right - left, bottom - top], side);
        // The harbor camera sees the same longitudinal end in both lanes.
        let x = if self.direction > 0. { right } else { left };
        self.face(
            [[x, -z, top], [x, z, top], [x, z, bottom], [x, -z, bottom]],
            end,
        );
    }

    fn wheel(&mut self, x: f32, z: f32) {
        // These two bottom vertices lie on the road plane at every camera angle.
        self.face(
            [
                [x, z, -2.8],
                [x + 2.7, z, -2.8],
                [x + 2.7, z, 0.],
                [x, z, 0.],
            ],
            0x1d1b2b,
        );
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
            for &(vehicle, departure) in &LANES[usize::from(!westbound)] {
                let x = (departure * length + time * speed).rem_euclid(length) - PAD;
                let half_width = vehicle.width() * 0.5;
                vehicle.paint(&mut VehiclePainter {
                    window,
                    origin: bounds.origin,
                    view,
                    scale,
                    start: x - half_width * direction,
                    direction,
                    depth,
                    half_track: vehicle.half_track(),
                    road_y,
                });
            }
        },
    )
    .absolute()
    .inset_0()
}

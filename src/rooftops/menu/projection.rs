//! World coordinates use x across the harbor, y down, and z inland.

use super::*;

pub(super) type Point = [f32; 3];

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Camera {
    pub(super) eye: Point,
    // Horizontal and vertical numerators, followed by forward depth.
    axes: [Point; 3],
    center: [f32; 2],
    clip: [f32; 4],
}

fn dot(a: Point, b: Point) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub(super) fn mix<const N: usize>(a: [f32; N], b: [f32; N], t: f32) -> [f32; N] {
    std::array::from_fn(|i| a[i] + (b[i] - a[i]) * t)
}

impl Camera {
    pub(super) fn at_menu(view: View) -> Self {
        let Aim { x, y } = view.aim;
        let yaw = HARBOR_YAW + x * 0.07;
        let recession = VIEW_RECESSION + x * 0.04;
        let tilt = 0.25 + y * 0.035;
        let lens = 0.70 / view.width;
        // The vanishing point of the idle projection is the first camera position.
        let z = (-1. / lens - recession * x * 2.8) / (VIEW_DEPTH + recession * yaw);
        let dx = -yaw * z - x * 2.8;
        let forward = [-recession, 0., VIEW_DEPTH];
        // Re-center the calibrated lens without changing the image at rest.
        let vertical = [0.012 / lens, 1. / lens, -tilt / lens];
        let offset = view.horizon - HEIGHT * 0.5;
        Self {
            eye: [
                view.width * 0.5 + dx,
                view.horizon + z * tilt - dx * 0.012 - y * 2.,
                z,
            ],
            axes: [
                [1. / lens, 0., yaw / lens],
                std::array::from_fn(|i| vertical[i] + offset * forward[i]),
                forward,
            ],
            center: [view.width * 0.5, HEIGHT * 0.5],
            clip: [0., 0., WIDTH, HEIGHT],
        }
    }

    pub(super) fn dolly(mut self, eye: Point, pitch: f32) -> Self {
        self.eye = eye;
        let (sine, cosine) = pitch.sin_cos();
        // Rotate only the vertical and forward axes of the calibrated lens.
        // Its horizontal heading and focal length stay fixed.
        let focal = 1550.;
        let [_, y, z] = self.axes;
        self.axes[1] = std::array::from_fn(|i| y[i] * cosine - z[i] * focal * sine);
        self.axes[2] = std::array::from_fn(|i| z[i] * cosine + y[i] * sine / focal);
        self
    }

    pub(super) fn with_clip(mut self, clip: [f32; 4]) -> Self {
        self.clip = clip;
        self
    }

    fn transform(self, point: Point) -> Point {
        let delta = std::array::from_fn(|i| point[i] - self.eye[i]);
        self.axes.map(|axis| dot(axis, delta))
    }

    fn screen(self, [x, y, depth]: Point) -> [f32; 2] {
        [self.center[0] + x / depth, self.center[1] + y / depth]
    }

    fn planes(self) -> [Point; 4] {
        // A small guard band keeps clipped outlines out of view.
        let [x, y] = self.center;
        let [left, top, right, bottom] = self.clip;
        [
            [1., 0., x - left + 16.],
            [-1., 0., right - x + 16.],
            [0., 1., y - top + 16.],
            [0., -1., bottom - y + 16.],
        ]
    }

    fn face(self, points: impl IntoIterator<Item = Point>) -> Vec<[f32; 2]> {
        let mut points: Vec<_> = points.into_iter().map(|p| self.transform(p)).collect();
        let distance = |plane, p: Point| match plane {
            Some(normal) => dot(normal, p),
            None => p[2] - 4.,
        };
        let [left, right, top, bottom] = self.planes();
        let planes = [None, Some(left), Some(right), Some(top), Some(bottom)];
        const ALL_PLANES: u8 = 0b11111;
        let (outside_all, outside_any) = points.iter().fold((ALL_PLANES, 0), |(all, any), &p| {
            let mask = planes.iter().enumerate().fold(0, |mask, (i, &plane)| {
                mask | (u8::from(distance(plane, p) < 0.) << i)
            });
            (all & mask, any | mask)
        });
        if outside_all != 0 {
            return Vec::new();
        }
        let mut clipped = Vec::new();
        for (i, plane) in planes.into_iter().enumerate() {
            if outside_any & (1 << i) == 0 {
                continue;
            }
            let Some(&last) = points.last() else {
                break;
            };
            let mut a = last;
            let mut da = distance(plane, a);
            for &b in &points {
                let db = distance(plane, b);
                if (da >= 0.) != (db >= 0.) {
                    clipped.push(mix(a, b, da / (da - db)));
                }
                if db >= 0. {
                    clipped.push(b);
                }
                a = b;
                da = db;
            }
            std::mem::swap(&mut clipped, &mut points);
            clipped.clear();
        }
        points.into_iter().map(|p| self.screen(p)).collect()
    }

    pub(super) fn point(self, point: Point) -> Option<[f32; 2]> {
        let at = self.transform(point);
        (at[2] >= 4.).then(|| self.screen(at))
    }
}

impl View {
    pub(super) fn polygon(self, points: impl IntoIterator<Item = Point>) -> Vec<[f32; 2]> {
        if let Some(camera) = self.camera {
            camera.face(points)
        } else {
            points
                .into_iter()
                .map(|[x, y, z]| self.project(x, z, y))
                .collect()
        }
    }

    pub(super) fn face(
        self,
        paths: &mut Vec<art::ArtPath>,
        points: impl IntoIterator<Item = Point>,
        ink: u32,
        opacity: f32,
    ) {
        let polygon = self.polygon(points);
        if polygon.len() >= 3 {
            shape(paths, polygon, ink, opacity);
        }
    }

    pub(super) fn line(
        self,
        paths: &mut Vec<art::ArtPath>,
        a: Point,
        b: Point,
        width: f32,
        ink: u32,
        opacity: f32,
    ) {
        if let Some(camera) = self.camera {
            let [mut a, mut b] = [camera.transform(a), camera.transform(b)];
            for (normal, offset) in std::iter::once(([0., 0., 1.], 4.))
                .chain(camera.planes().map(|normal| (normal, 0.)))
            {
                let da = dot(normal, a) - offset;
                let db = dot(normal, b) - offset;
                if da < 0. && db < 0. {
                    return;
                }
                if da < 0. {
                    a = mix(a, b, da / (da - db));
                } else if db < 0. {
                    b = mix(a, b, da / (da - db));
                }
            }
            let zoom = (690. / a[2].min(b[2])).clamp(0.85, 6.);
            stroke(
                paths,
                camera.screen(a),
                camera.screen(b),
                width * zoom,
                ink,
                opacity,
            );
        } else {
            let project = |[x, y, z]: Point| self.project(x, z, y);
            stroke(paths, project(a), project(b), width, ink, opacity);
        }
    }

    pub(super) fn offset(self, [x, y, z]: Point, dx: f32, dy: f32) -> Point {
        if self.camera.is_some() {
            [x + dx, y + dy, z]
        } else {
            let [sx, sy] = self.project(x, z, y);
            self.lift([sx + dx, sy + dy], z)
        }
    }

    pub(super) fn detail(
        self,
        paths: &mut Vec<art::ArtPath>,
        at: Point,
        [x, y, w, h]: [f32; 4],
        ink: u32,
        opacity: f32,
    ) {
        self.face(
            paths,
            [[x, y], [x + w, y], [x + w, y + h], [x, y + h]]
                .map(|[dx, dy]| self.offset(at, dx, dy)),
            ink,
            opacity,
        );
    }

    // Recover a point on a given depth plane from the authored menu composition.
    pub(super) fn lift(self, [sx, sy]: [f32; 2], z: f32) -> Point {
        let x = self.x_at_depth(sx, z);
        let y =
            self.horizon + (sy - self.project(x, z, self.horizon)[1]) / self.magnification(x, z);
        [x, y, z]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flight_starts_with_the_current_menu_projection() {
        for aim in [Aim::default(), Aim::new(0.8, -0.9), Aim::new(-0.7, 1.)] {
            let view = View::new(WIDTH, HEIGHT * HORIZON, aim);
            let camera = Camera::at_menu(view);
            for [x, y, z] in [[200., 240., 0.], [600., 400., -250.], [720., 110., 270.]] {
                let expected = view.project(x, z, y);
                let actual = camera.point([x, y, z]).unwrap();
                let lifted = view.lift(expected, z);
                assert!((expected[0] - actual[0]).abs() < 0.001);
                assert!((expected[1] - actual[1]).abs() < 0.001);
                assert!((x - lifted[0]).abs() < 0.001 && (y - lifted[1]).abs() < 0.001);
            }
        }
    }

    #[test]
    fn clips_a_face_that_straddles_the_camera_without_projecting_its_back() {
        let camera = Camera {
            eye: [0.; 3],
            axes: [[400., 0., 0.], [0., 400., 0.], [0., 0., 1.]],
            center: [WIDTH * 0.5, HEIGHT * 0.5],
            clip: [0., 0., WIDTH, HEIGHT],
        };
        let rim = [
            [-5., 5., -10.],
            [5., 5., -10.],
            [5., 5., 40.],
            [-5., 5., 40.],
        ];
        let visible = camera.face(rim);
        assert_eq!(visible.len(), 4);
        assert!(
            visible
                .iter()
                .all(|&[x, y]| (0. ..WIDTH).contains(&x) && (280. ..=HEIGHT + 16.1).contains(&y))
        );
        assert!(camera.face(rim.map(|[x, y, z]| [x, y, z - 50.])).is_empty());
    }
}

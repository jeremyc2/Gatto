//! The far bank, drawn on the same water plane as the bridge and boats.

use super::*;

pub(super) const STREET_DEPTH: f32 = 270.;

pub(super) fn street_grade(view: View) -> f32 {
    view.waterline() - 24.
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
        let at = |x, depth| self.view.project(x, depth, y);
        let pavement = STREET_DEPTH - 19.;
        let curb = STREET_DEPTH - 6.;
        let [left, _] = self.edge[0];
        let [right, _] = self.edge[self.edge.len() - 1];
        let edge = self
            .edge
            .map(|[x, z]| self.view.project(x, z + 52., self.view.waterline() - 7.));
        shape(
            &mut paths,
            edge.into_iter()
                .chain([at(right, pavement), at(left, pavement)])
                .collect(),
            0x222639,
            1.,
        );
        for ([near, far], color) in [
            ([pavement, curb], 0x1d2233),
            ([curb, STREET_DEPTH + 4.], 0x303043),
        ] {
            shape(
                &mut paths,
                vec![
                    at(left, near),
                    at(right, near),
                    at(right, far),
                    at(left, far),
                ],
                color,
                1.,
            );
        }
        stroke(
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
        let at = |[x, z]: [f32; 2], dy| self.view.project(x, z, y + dy);
        let apron = self.edge.map(|p| at(p, -7.));
        let inland = self.edge.map(|[x, z]| at([x, z + 52.], -7.));
        shape(
            &mut paths,
            apron.into_iter().chain(inland.into_iter().rev()).collect(),
            0x25293b,
            1.,
        );

        for face in self.edge.windows(2) {
            let a = face[0];
            let b = face[1];
            let along_bank = a[0] != b[0];
            let stone = if along_bank { 0x38384b } else { 0x2d3144 };
            shape(
                &mut paths,
                vec![at(a, -7.), at(b, -7.), at(b, 0.), at(a, 0.)],
                stone,
                1.,
            );
            stroke(&mut paths, at(a, -7.), at(b, -7.), 1.05, 0x736b79, 0.46);
            stroke(&mut paths, at(a, -1.4), at(b, -1.4), 0.9, 0x151e31, 0.68);
            if along_bank {
                let [left, right] = [a[0].max(-140.), b[0].min(self.view.width + 75.)];
                for i in 0..((right - left) / 49.) as usize {
                    let x = left + 15. + i as f32 * 49.;
                    stroke(
                        &mut paths,
                        at([x - 1., a[1]], -8.8),
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
            shape(
                &mut paths,
                vec![
                    at(a, 0.),
                    at([x + 21., bank], 0.),
                    at([x + 21., tip - 3.], 0.),
                    at([x - 2., tip - 3.], 0.),
                ],
                0x070f20,
                0.35,
            );
            shape(
                &mut paths,
                vec![at(d, -5.), at(c, -5.), at(c, -1.), at(d, -1.)],
                0x333447,
                1.,
            );
            shape(
                &mut paths,
                vec![at(b, -5.), at(c, -5.), at(c, -1.), at(b, -1.)],
                0x272d40,
                1.,
            );
            shape(
                &mut paths,
                vec![at(a, -5.), at(b, -5.), at(c, -5.), at(d, -5.)],
                0x454354,
                1.,
            );
            for fraction in [0.35, 0.70] {
                let row = bank - length * fraction;
                stroke(
                    &mut paths,
                    at([x, row], -5.),
                    at([x + 18., row], -5.),
                    0.55,
                    0x2a2d40,
                    0.68,
                );
            }
            for point in [c, d, [x + 18., bank - length * 0.53]] {
                stroke(
                    &mut paths,
                    at(point, -7.),
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

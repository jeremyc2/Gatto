//! The Play camera moves from the menu's oblique lens into the harbor and city.

use super::*;
use projection::{Camera, Point};

const HANDOFF: f32 = 9.65;
const LAUNCH: f32 = 0.14;
const SPEED: f32 = 2.;
const SHOT_END: f32 = LAUNCH + HANDOFF / SPEED;
const FADE_OUT: f32 = 0.12;
const CUT: f32 = SHOT_END + FADE_OUT;
const ROUTE_END: f32 = HANDOFF + FADE_OUT * SPEED;
const BLACK_BEAT: f32 = 0.02;
const FADE_IN: f32 = 0.10;
const CITY_REVEAL_DEPTH: f32 = -550.;
// The truss is still centered in the frame at the start of the waterfront reveal.
const POCKET_REVEAL_DEPTH: f32 = -660.;
const POCKET_OPENS_BY: f32 = 330.;
const HAZE_INK: u32 = 0x121525;
const GROUND_FLOOR: f32 = 25.;
const LAND_BAND: f32 = 230.;
const FLIGHT_MOON_DIAMETER: f32 = COURSE_MOON_DIAMETER * 1.3;
const MENU_ROW_DEPTHS: [f32; 3] = [725., 565., 400.];
#[derive(Clone, Copy)]
struct CityRow {
    depth: f32,
    spacing: f32,
    offset: f32,
    heights: [f32; 2],
    ink: u32,
}

impl CityRow {
    const fn from_skyline(depth: f32, heights: [f32; 2], row: SkylineRow) -> Self {
        Self {
            depth,
            spacing: row.spacing,
            offset: -145. * row.parallax,
            heights,
            ink: row.ink,
        }
    }

    fn height(self, index: i64, layer: usize, horizon_row: bool) -> f32 {
        let variation = if horizon_row {
            [8., 34., 16., 39., 21., 0., 27., 12., 31., 3., 24.][index.rem_euclid(11) as usize]
                / 39.
        } else {
            SkylineRow::height_variation(index, layer) / 129.
        };
        self.heights[0] + (self.heights[1] - self.heights[0]) * variation
    }
}

const SHORE_ROWS: [CityRow; 3] = [
    CityRow::from_skyline(1240., [86., 193.], SKYLINE_ROWS[0]),
    CityRow::from_skyline(940., [73., 156.], SKYLINE_ROWS[1]),
    CityRow::from_skyline(580., [58., 128.], SKYLINE_ROWS[2]),
];

const DISTANT_ROWS: [CityRow; 6] = [
    CityRow {
        depth: 3500.,
        spacing: 61.,
        offset: 39.6,
        heights: [164., 242.],
        ink: 0x191e31,
    },
    CityRow {
        depth: 3100.,
        spacing: 106.,
        offset: 58.6,
        heights: [159., 247.],
        ink: 0x191d30,
    },
    CityRow {
        depth: 2740.,
        spacing: 134.,
        offset: 71.8,
        heights: [137., 243.],
        ink: 0x1d2035,
    },
    CityRow {
        depth: 2340.,
        spacing: 113.,
        offset: 85.,
        heights: [119., 211.],
        ink: 0x1c2033,
    },
    CityRow {
        depth: 1970.,
        spacing: 98.,
        offset: 105.,
        heights: [82., 159.],
        ink: 0x202237,
    },
    CityRow {
        depth: 1620.,
        spacing: 117.,
        offset: 121.,
        heights: [67., 146.],
        ink: 0x212338,
    },
];

#[derive(Clone, Copy, Default)]
pub(in crate::rooftops) struct Flight {
    time: f32,
}

// Time, forward position, altitude (y points down), and pitch from the menu lens.
// The only turn is in pitch. The camera follows the menu's center line over water.
const ROUTE: [[f32; 4]; 19] = [
    [0., 0., 0., 0.],
    [0.70, -1170., 115., 0.021],
    [1.42, -650., 286., 0.040],
    [1.80, -430., 362., 0.046],
    [2.02, -295., 393., 0.030],
    [2.44, -125., 415., -0.022],
    [2.79, -32., 418., -0.075],
    [3.24, 91., 410., -0.129],
    [3.88, 226., 392., -0.164],
    [4.64, 370., 362., -0.162],
    [5.35, 537., 310., -0.146],
    [6.13, 820., 256., -0.139],
    [6.97, 1140., 206., -0.142],
    [7.63, 1430., 166., -0.147],
    [8.23, 1710., 142., -0.150],
    [8.85, 2000., 122., -0.154],
    [9.28, 2180., 114., -0.161],
    [HANDOFF, 2310., 110., -0.167],
    [ROUTE_END, 2420., 105., -0.173],
];

fn smooth(t: f32) -> f32 {
    let t = t.clamp(0., 1.);
    t * t * (3. - 2. * t)
}

impl Flight {
    pub(in crate::rooftops) const DURATION: f32 = CUT + BLACK_BEAT + FADE_IN;

    fn shot_time(self) -> f32 {
        ((self.time - LAUNCH) * SPEED).clamp(0., ROUTE_END)
    }

    #[cfg(feature = "walkthrough-screenshots")]
    pub(in crate::rooftops) fn for_preview(time: f32) -> Self {
        Self {
            time: time % Self::DURATION,
        }
    }

    pub(in crate::rooftops) fn advance(&mut self, elapsed: f32) -> bool {
        self.time += elapsed;
        self.time >= Self::DURATION
    }

    pub(in crate::rooftops) fn reveals_course(self) -> bool {
        self.time >= CUT
    }

    fn camera(self, view: View) -> Camera {
        let rest = Camera::at_menu(view);
        let mut route = ROUTE;
        route[0][1] = rest.eye[2];
        route[0][2] = rest.eye[1];
        let shot_time = self.shot_time();
        let i = route
            .partition_point(|p| p[0] <= shot_time)
            .clamp(1, route.len() - 1);
        let a = route[i - 1];
        let b = route[i];
        let p = route[i.saturating_sub(2)];
        let q = route[(i + 1).min(route.len() - 1)];
        let span = b[0] - a[0];
        let t = ((shot_time - a[0]) / span).clamp(0., 1.);
        let h = smooth(t);
        let track = |j: usize| {
            if j == 3 && a[j] == b[j] {
                return a[j];
            }
            let v0 = if i == 1 || (j == 3 && p[j] == a[j]) {
                0.
            } else {
                (b[j] - p[j]) * span / (b[0] - p[0])
            };
            let v1 = if j == 3 && b[j] == q[j] {
                0.
            } else {
                (q[j] - a[j]) * span / (q[0] - a[0])
            };
            a[j] * (1. - h) + b[j] * h + (v0 * (1. - t) - v1 * t) * t * (1. - t)
        };
        let z = track(1);
        rest.dolly([view.x_at_depth(WIDTH * 0.5, z), track(2), z], track(3))
    }

    pub(in crate::rooftops) fn render(
        self,
        viewport: gpui_kit::Size<gpui_kit::Pixels>,
        menu_time: f32,
        aim: Aim,
        art: &mut HarborArt,
    ) -> gpui_kit::Div {
        let veil = |opacity| {
            div()
                .absolute()
                .inset_0()
                .bg(rgb(0x000000).opacity(opacity))
        };
        if self.reveals_course() {
            return veil(1. - smooth((self.time - CUT - BLACK_BEAT) / FADE_IN));
        }
        let (frame, scale) = frame(viewport);
        let original = View::new(WIDTH, HEIGHT * HORIZON, aim);
        let [pad_x, pad_y] = [
            f32::from(frame.origin.x) / scale,
            f32::from(frame.origin.y) / scale,
        ];
        let clip = [-pad_x, -pad_y, WIDTH + pad_x, HEIGHT + pad_y];
        let camera = self.camera(original).with_clip(clip);
        let time = menu_time + self.time;
        let scenery = if self.time <= LAUNCH {
            menu_scenery(original, shore::Bank::new(original), time, scale, art)
        } else {
            harbor(original, camera, time, scale, &mut art.bridge, clip)
        };
        let mut scene = div()
            .absolute()
            .left(frame.origin.x)
            .top(frame.origin.y)
            .w(frame.size.width)
            .h(frame.size.height)
            .child(scenery);
        let title_fade = LAUNCH + 0.04;
        if self.time < title_fade {
            scene = scene.child(
                v_flex()
                    .absolute()
                    .w_full()
                    .top(px(HEIGHT * scale * 0.115))
                    .items_center()
                    .opacity(1. - smooth(self.time / title_fade))
                    .child(title(scale)),
            );
        }
        let settle = smooth((self.time - SHOT_END) / FADE_OUT);
        div().absolute().inset_0().child(scene).child(veil(settle))
    }
}

fn skyline_camera(original: View, mut camera: Camera) -> Camera {
    // The bridge and streets move with the dolly. Limit the skyline's forward
    // parallax under the span so the street distance resolves on the other side.
    const COURT: f32 = 535.;
    const CITY: f32 = 1660.;
    const LAG: f32 = 650.;
    let travel = camera.eye[2];
    let opening = smooth((travel - POCKET_REVEAL_DEPTH) / (POCKET_OPENS_BY - POCKET_REVEAL_DEPTH));
    let lag = if travel < POCKET_OPENS_BY {
        LAG * opening
    } else {
        LAG * (1. - smooth((travel - COURT) / (CITY - COURT)))
    };
    // Bring the street façades down to meet the quay while the truss fills the
    // frame. The offset eases away as the nearer ground opens into the park.
    let compact = smooth((travel + 1440.) / 480.) * (1. - opening);
    let z = travel - lag;
    camera.eye = [
        original.x_at_depth(WIDTH * 0.5, z),
        camera.eye[1] - 30. * compact,
        z,
    ];
    camera
}

fn harbor(
    original: View,
    camera: Camera,
    time: f32,
    scale: f32,
    bridge: &mut BridgeArt,
    clip: [f32; 4],
) -> gpui_kit::Div {
    let view = View {
        camera: Some(camera),
        ..original
    };
    let bank = shore::Bank::new(view);
    let inland = pocket_depth(camera.eye[2]);
    let city_camera = skyline_camera(original, camera);
    let city_view = View {
        camera: Some(city_camera),
        ..original
    };
    let [farther, nearer] = city(
        original,
        city_view,
        city_camera,
        city_relief(camera.eye[2]) * inland,
        clip,
        inland,
    );
    let [surfaces, fixtures] = recreation(view, camera, city_camera, inland);
    let [x, y, right, bottom] = clip;
    let [w, h] = [right - x, bottom - y];
    div()
        .absolute()
        .inset_0()
        .child(
            div()
                .absolute()
                .left(px(x * scale))
                .top(px(y * scale))
                .w(px(w * scale))
                .h(px(h * scale))
                .child(sky(w, h, time, scale, 145.)),
        )
        .children((inland > 0.).then(|| vector_art(streets(view, camera, inland), scale)))
        .child(harbor_moon(original, camera, scale))
        .child(paint_mesh(art_mesh(&surfaces, scale)))
        .child(paint_mesh(art_mesh(&farther, scale)))
        .child(vector_art(
            landmarks::project(original, city_view, city_camera, inland),
            scale,
        ))
        .child(paint_mesh(art_mesh(&nearer, scale)))
        .child(paint_mesh(art_mesh(&fixtures, scale)))
        .child(vector_art(water(original, view, time), scale))
        .child(vector_art(bank.inland(), scale))
        .child(vector_art(bank.shore(), scale))
        .child(vector_art(boats(original, view, time, TUG_DEPTH), scale))
        .child(bridge.render(view, view.deck(), time, scale))
        .child(vector_art(boats(original, view, time, FERRY_DEPTH), scale))
        .child(vector_art(
            boats(original, view, time, SAILBOAT_DEPTH),
            scale,
        ))
}

fn recreation(
    view: View,
    camera: Camera,
    city_camera: Camera,
    inland: f32,
) -> [Vec<art::ArtPath>; 2] {
    let layer = SHORE_ROWS.len() - 1;
    let open = MENU_ROW_DEPTHS[layer];
    let depth = SHORE_ROWS[layer].depth;
    let frontage = if city_camera.eye[2] < open + (depth - open) * inland {
        let [left, right] = [0., WIDTH].map(|x| {
            let painted = view.lift([x, view.horizon], open);
            let street = [view.x_at_depth(x, depth), shore::street_grade(view), depth];
            city_camera.point(projection::mix(painted, street, inland))
        });
        left.zip(right).map(|(left, right)| [left, right])
    } else {
        None
    };
    shore::recreation(view, camera.eye[2], frontage)
}

fn streets(view: View, camera: Camera, clarity: f32) -> Vec<art::ArtPath> {
    let mut paths = Vec::new();
    let grade = shore::street_grade(view);
    // Distant pavement takes on the sky color. Its far edge is a painted
    // ground cue, so the moon is composited over it and behind the buildings.
    for band in (0..21).rev() {
        let near = shore::STREET_DEPTH + band as f32 * LAND_BAND;
        let distance = (near - camera.eye[2].max(shore::STREET_DEPTH)).max(0.);
        let amount = smooth(1. - distance / 2600.);
        let haze = blend_ink(SKY, HAZE_INK, smooth(amount * 4.));
        let ink = blend_ink(haze, 0x222636, amount);
        view.face(
            &mut paths,
            [
                [-100000., grade, near],
                [100000., grade, near],
                [100000., grade, near + LAND_BAND],
                [-100000., grade, near + LAND_BAND],
            ],
            ink,
            clarity,
        );
    }
    paths
}

fn moon_projection(original: View, camera: Camera) -> Option<([f32; 2], f32)> {
    let rest = Camera::at_menu(original);
    let start = rest.eye[2];
    let finish = ROUTE[ROUTE.len() - 2][1];
    let growth = FLIGHT_MOON_DIAMETER / MOON_DIAMETER;
    // The wide sky follows the tilt with less travel than the nearby scene.
    // In the rooftop climb the moon clears the skyline and fills more of the frame.
    let depth = finish + (finish - start) / (growth - 1.);
    let scale = ((depth - start) / (depth - camera.eye[2])).min(growth);
    let anchor = original.lift(MOON_CENTER, 0.);
    let [x, y] = camera.point(std::array::from_fn(|i| {
        camera.eye[i] + (anchor[i] - rest.eye[i])
    }))?;
    let climb = smooth((camera.eye[2] - 100.) / (finish - 100.));
    let x = x - 18. * climb;
    let y = MOON_CENTER[1] + (y - MOON_CENTER[1]) * 0.34 - 44. * climb;
    let radius = MOON_DIAMETER * scale * 0.5;
    let skyline = skyline_camera(original, camera);
    let y = skyline_street_y(original, skyline, x).map_or(y, |street_y| {
        // Keep the full disc above street level without clipping against the horizon.
        let limit = street_y - radius - 8.;
        let blend = (22. - (limit - y).abs()).max(0.) / 22.;
        limit.min(y) - blend * blend * 5.5
    });
    let center = [x, y];
    Some((center, scale))
}

fn skyline_street_y(original: View, camera: Camera, screen_x: f32) -> Option<f32> {
    let depth = SHORE_ROWS[1].depth;
    if camera.eye[2] >= depth {
        return None;
    }
    let grade = shore::street_grade(original);
    let [a, b] = [0., WIDTH].map(|x| camera.point([original.x_at_depth(x, depth), grade, depth]));
    let ([ax, ay], [bx, by]) = a.zip(b)?;
    Some(ay + (by - ay) * (screen_x - ax) / (bx - ax))
}

fn harbor_moon(original: View, camera: Camera, scale: f32) -> impl IntoElement {
    let paths = moon_projection(original, camera).map_or_else(Vec::new, |(center, zoom)| {
        moon()
            .iter()
            .map(|path| art::ArtPath {
                points: path
                    .points
                    .iter()
                    .map(|point| {
                        std::array::from_fn(|axis| {
                            center[axis] + (point[axis] - MOON_CENTER[axis]) * zoom
                        })
                    })
                    .collect(),
                ink: path.ink,
                opacity: path.opacity,
            })
            .collect()
    });
    vector_art(paths, scale)
}

fn boats(original: View, view: View, time: f32, depth: f32) -> Vec<art::ArtPath> {
    let art = if depth == TUG_DEPTH {
        tugboat(original, time)
    } else if depth == FERRY_DEPTH {
        ferry(original, time)
    } else {
        sailboat(original, time)
    };
    let mut paths = Vec::with_capacity(art.len());
    let eye = Camera::at_menu(original).eye;
    for (index, shape) in art.into_iter().enumerate() {
        let surface = |point| {
            let at = original.lift(point, depth);
            if index < WAKE_STREAKS {
                // Lift the wake onto the same horizontal plane as the water.
                let t = (original.waterline() - eye[1]) / (at[1] - eye[1]);
                projection::mix(eye, at, t)
            } else {
                at
            }
        };
        view.face(
            &mut paths,
            shape.points.into_iter().map(surface),
            shape.ink,
            shape.opacity,
        );
    }
    paths
}

fn water(original: View, view: View, time: f32) -> Vec<art::ArtPath> {
    let mut paths = Vec::new();
    let y = view.waterline();
    view.face(
        &mut paths,
        [
            [-100000., y, -100000.],
            [100000., y, -100000.],
            [100000., y, 152.],
            [-100000., y, 152.],
        ],
        WATER,
        1.,
    );
    let eye = Camera::at_menu(original).eye;
    let on_water = |screen| {
        let at = original.lift(screen, 0.);
        let t = (y - eye[1]) / (at[1] - eye[1]);
        projection::mix(eye, at, t)
    };
    water_detail(original, HEIGHT, time, |[x, y, w, h], ink, opacity| {
        let rect = [[x, y], [x + w, y], [x + w, y + h], [x, y + h]];
        view.face(&mut paths, rect.map(on_water), ink, opacity);
    });
    paths
}

fn city_relief(depth: f32) -> f32 {
    // The skyline keeps its painted profile as the bridge cables fill the
    // frame. Reveal roof and side surfaces during the last part of the dive.
    smooth((depth - CITY_REVEAL_DEPTH) / 175.)
}

fn horizon_visibility(camera_depth: f32, row_depth: f32) -> f32 {
    let clear_to = 1130. + (camera_depth - 125.).max(0.) * 1.36;
    smooth((clear_to - row_depth) / 510.)
}

fn pocket_depth(depth: f32) -> f32 {
    smooth((depth - POCKET_REVEAL_DEPTH) / (135. - POCKET_REVEAL_DEPTH))
}

pub(super) fn opening_skyline(original: View, scale: f32) -> Mesh {
    let clip = [0., 0., WIDTH, HEIGHT];
    let camera = Camera::at_menu(original);
    let view = View {
        camera: Some(camera),
        ..original
    };
    let [paths, _] = city(original, view, camera, 0., clip, 0.);
    art_mesh(&paths, scale)
}

fn window_run(
    span: f32,
    pane: f32,
    spacing: f32,
    margin: f32,
) -> impl Iterator<Item = f32> + Clone {
    let room = span - 2. * margin - pane;
    let count = if room >= 0. {
        1 + (room / spacing) as usize
    } else {
        0
    };
    let start = (span - pane - count.saturating_sub(1) as f32 * spacing) * 0.5;
    (0..count).map(move |i| start + i as f32 * spacing)
}

fn city(
    original: View,
    view: View,
    camera: Camera,
    relief: f32,
    [left_edge, _, right_edge, _]: [f32; 4],
    inland: f32,
) -> [Vec<art::ArtPath>; 2] {
    let mut buildings = [Vec::new(), Vec::new()];
    let grade = shore::street_grade(original);
    let rows = DISTANT_ROWS
        .into_iter()
        .enumerate()
        .map(|(layer, row)| (layer + SHORE_ROWS.len(), row))
        .chain(SHORE_ROWS.into_iter().enumerate());
    for (layer, row) in rows {
        let distant = layer >= SHORE_ROWS.len();
        let painted_depth = MENU_ROW_DEPTHS.get(layer).copied();
        let open = painted_depth.unwrap_or(row.depth);
        let distance = open + (row.depth - open) * inland;
        let paths = &mut buildings[usize::from(distance < landmarks::depth(inland))];
        let horizon_row = layer == SHORE_ROWS.len();
        let visibility = if distant {
            horizon_visibility(camera.eye[2], distance)
        } else {
            1.
        };
        if visibility == 0. {
            continue;
        }
        // Fog changes the ink on solid faces. Each row occludes the rows behind it.
        let ink = row.ink;
        let fogged = |ink| blend_ink(HAZE_INK, ink, visibility);
        let wall = fogged(ink);
        let surface = |light: f32| fogged(shade(ink, 1. + relief * (light - 1.)));
        let heading = original.x_at_depth(WIDTH * 0.5, distance);
        // Keep drawing past the crop, including on wide windows.
        let first = ((left_edge - row.offset) / row.spacing).floor() as i64 - 3;
        let last = ((right_edge - row.offset) / row.spacing).ceil() as i64 + 3;
        let width = row.spacing - if horizon_row { 3. } else { 9. };
        // On each side of the street, the parcels closest to the center
        // conceal the receding walls of the parcels farther from us.
        let center = ((WIDTH * 0.5 - row.offset) / row.spacing).floor() as i64;
        let left = first..=last.min(center - 1);
        let right = first.max(center + 1)..=last;
        let middle = (first..=last).contains(&center).then_some(center);
        for index in left.chain(right.rev()).chain(middle) {
            let left = index as f32 * row.spacing + row.offset;
            let roof_style = index.rem_euclid(5);
            // Roofs and foundations stay at fixed elevations on the same street.
            let height = row.height(index, layer, horizon_row);
            let top = grade - height;
            const FLOOR_SCALE: f32 = 0.86;
            let rise = |offset| top + offset * FLOOR_SCALE;
            let across = |offset| left + offset;
            let painted = SKYLINE_ROWS.get(layer).map(|skyline| {
                let lean = [290., 219., 160.][layer];
                let ([x, foot], zoom) = original.billboard(left, lean, original.horizon);
                let top = foot + (skyline.top(index, layer) - original.horizon) * zoom;
                (x, top.min(original.horizon), zoom)
            });
            let point = |[x, y]: [f32; 2]| {
                let world = [original.x_at_depth(x, row.depth), y, row.depth];
                let Some((old_x, old_top, zoom)) = painted else {
                    return world;
                };
                let old_y = if y < top {
                    old_top + (y - top) * zoom / FLOOR_SCALE
                } else {
                    old_top + (original.horizon - old_top) * (y - top) / height
                };
                let art = original.lift([old_x + (x - left) * zoom, old_y], open);
                projection::mix(art, world, inland)
            };
            let [a, b, c, d] = [
                [left, top],
                [across(width), top],
                [across(width), grade],
                [left, grade],
            ]
            .map(point);
            let bounds = [a, b, c, d];
            let extent = 87. + (index * 13 + layer as i64 * 7).rem_euclid(31) as f32;
            let block_depth = extent * relief;
            let back = |p: Point| original.rear(p, block_depth);
            let front = view.polygon(bounds);
            if front.is_empty()
                && (relief == 0.
                    || (view.polygon(bounds.map(back)).is_empty()
                        && view.polygon([a, back(a), back(d), d]).is_empty()
                        && view.polygon([b, back(b), back(c), c]).is_empty()
                        && view.polygon([a, b, back(b), back(a)]).is_empty()))
            {
                continue;
            }
            let [pane_width, pane_height, column_step, floor_step] = if horizon_row {
                match roof_style {
                    0 | 2 => [4.5, 9.5, 12.5, 20.],
                    1 | 3 => [6.5, 7., 15., 19.],
                    _ => [4., 8.5, 14., 22.],
                }
            } else {
                [6., 8.6, 20., 23.]
            };
            let rows = window_run(height - GROUND_FLOOR, pane_height, floor_step, 9.);
            let phase = index.rem_euclid(13) as usize;
            let glow = if distant || layer == 0 { 0.13 } else { 0.23 };
            let windows = |paths: &mut Vec<art::ArtPath>,
                           span,
                           wall_ink,
                           light,
                           at: &dyn Fn([f32; 2]) -> Point| {
                let columns = window_run(span, pane_width, column_step, 9.);
                let palette = [0.34, 0.58, 0.89]
                    .map(|strength| fogged(blend_ink(wall_ink, 0xd9b49b, strength * glow * light)));
                let mut panes: [_; 3] = std::array::from_fn(|_| Vec::new());
                for (floor, y) in rows.clone().enumerate() {
                    for (col, x) in columns.clone().enumerate() {
                        let color = (phase * 11 + floor * 7 + col * (5 + floor) + col * col)
                            % 13
                            % palette.len();
                        let [r, b] = [x + pane_width, y + pane_height];
                        view.face(
                            &mut panes[color],
                            [[x, y], [r, y], [r, b], [x, b]].map(at),
                            palette[color],
                            1.,
                        );
                    }
                }
                paths.extend(panes.into_iter().flatten());
            };
            let front_at = |[x, y]: [f32; 2]| point([across(x), top + y]);
            if relief > 0. {
                if camera.eye[2] > distance + block_depth {
                    view.face(paths, [a, b, c, d].map(back), wall, 1.);
                    windows(paths, width, ink, 0.72, &|p| back(front_at(p)));
                }
                let side = if heading < a[0] {
                    Some((a, d, 0.86))
                } else if heading > b[0] {
                    Some((b, c, 0.78))
                } else {
                    None
                };
                if let Some((top_edge, foot, light)) = side {
                    let points = [top_edge, back(top_edge), back(foot), foot];
                    let polygon = view.polygon(points);
                    if polygon.len() >= 3 {
                        let side_ink = shade(ink, 1. + relief * (light - 1.));
                        shape(paths, polygon, fogged(side_ink), 1.);
                        windows(paths, extent, side_ink, relief * 0.62, &|[x, y]| {
                            original.rear(projection::mix(top_edge, foot, y / height), x * relief)
                        });
                    }
                }
                if camera.eye[1] < (a[1] + b[1]) * 0.5 {
                    view.face(paths, [a, b, back(b), back(a)], surface(1.32), 1.);
                }
            }
            let face = camera.eye[2] <= distance;
            if face && front.len() >= 3 {
                shape(paths, front, wall, 1.);
            }
            // Setbacks share the same facade dimensions as the silhouettes in
            // the menu and stand on the same solid block in perspective.
            let mut setback = |rect: [f32; 4], length| {
                let [left, right, top, bottom] = rect;
                let front = [
                    [across(left), rise(top)],
                    [across(right), rise(top)],
                    [across(right), rise(bottom)],
                    [across(left), rise(bottom)],
                ]
                .map(point);
                let rear = front.map(|p| original.rear(p, block_depth * length));
                let [a, b, c, d] = front;
                if relief > 0. {
                    if heading < a[0] {
                        view.face(paths, [a, rear[0], rear[3], d], surface(0.84), 1.);
                    } else if heading > b[0] {
                        view.face(paths, [b, rear[1], rear[2], c], surface(0.83), 1.);
                    }
                    if camera.eye[1] < (a[1] + b[1]) * 0.5 {
                        view.face(paths, [a, b, rear[1], rear[0]], surface(1.44), 1.);
                    }
                    if camera.eye[2] >= rear[0][2] {
                        view.face(paths, rear, wall, 1.);
                    }
                }
                if face {
                    view.face(paths, front, wall, 1.);
                }
            };
            if horizon_row {
                match roof_style {
                    0 => {
                        setback([8., 24., -7., 1.], 0.56);
                        setback([11., 16., -25., -6.], 0.28);
                        setback([9.5, 17.5, -27., -24.], 0.32);
                    }
                    1 => {
                        setback([7., 51., -8., 1.], 0.7);
                        setback([25., 46., -19., -7.], 0.52);
                    }
                    2 => {
                        setback([13., 45., -7., 1.], 0.66);
                        setback([20., 34., -24., -6.], 0.44);
                        setback([28., 30., -40., -23.], 0.2);
                    }
                    3 => {
                        setback([6., 28., -11., 1.], 0.66);
                        setback([38., 52., -5., 1.], 0.4);
                    }
                    _ => {}
                }
            } else {
                setback([13., row.spacing - 22., -9., 1.], 0.79);
                if index % 3 == 0 {
                    setback([35., 56., -30., -6.], 0.49);
                    setback([41., 43., -46., -29.], 0.21);
                }
            }
            if !face {
                continue;
            }
            if !distant && inland > 0. {
                let detail = smooth(1. - (distance - camera.eye[2] - 320.) / 700.);
                ground_floor(index, width, ink, detail, |[x, y, w, h], color| {
                    let [x, y] = [left + x, grade + y];
                    view.face(
                        paths,
                        [[x, y], [x + w, y], [x + w, y + h], [x, y + h]].map(point),
                        fogged(blend_ink(ink, color, inland)),
                        1.,
                    );
                });
            }
            windows(paths, width, ink, 1., &front_at);
        }
    }
    buildings
}

fn ground_floor(
    index: i64,
    width: f32,
    wall: u32,
    detail: f32,
    mut panel: impl FnMut([f32; 4], u32),
) {
    let center = width * 0.5;
    let base = shade(wall, 0.9);
    let recess = shade(wall, 0.68);
    let glass = blend_ink(recess, 0x655865, 0.37 * detail);
    let trim = blend_ink(base, 0x665867, 0.39 * detail);
    panel([0., -GROUND_FLOOR, width, GROUND_FLOOR], base);
    panel([0., -GROUND_FLOOR, width, 1.6], trim);
    match index.rem_euclid(3) {
        0 => {
            panel([center - 11., -18., 22., 18.], recess);
            panel([center - 14., -19.3, 28., 2.3], trim);
            for x in [center - 8.4, center + 0.8] {
                panel([x, -14., 7.6, 10.], glass);
            }
        }
        1 => {
            panel([13., -17., width - 26., 17.], recess);
            panel([12., -18.5, width - 24., 2.5], trim);
            let pane = (width - 42.) * 0.5;
            for x in [16., 26. + pane] {
                panel([x, -12., pane, 8.], glass);
            }
        }
        _ => {
            panel([center - 6., -16., 12., 16.], recess);
            panel([center - 4.2, -12.4, 8.4, 4.], glass);
            panel([center - 8., -1.4, 16., 1.4], trim);
        }
    }
}

pub(super) fn blend_ink(from: u32, to: u32, amount: f32) -> u32 {
    let from = from.to_be_bytes().map(f32::from);
    let to = to.to_be_bytes().map(f32::from);
    u32::from_be_bytes(projection::mix(from, to, amount).map(|channel| channel.round() as u8))
}

fn shade(ink: u32, factor: f32) -> u32 {
    let [_, r, g, b] = ink.to_be_bytes();
    let shade = |value| (f32::from(value) * factor).min(255.) as u8;
    u32::from_be_bytes([0, shade(r), shade(g), shade(b)])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_fit_above_the_full_entrance_story() {
        let pane = 8.6;
        let margin = 9.;
        for (height, count) in [(58., 1), (86., 2), (127., 4), (242., 9)] {
            let facade = height - GROUND_FLOOR;
            let rows: Vec<_> = window_run(facade, pane, 23., margin).collect();
            assert_eq!(rows.len(), count);
            assert!(rows[0] >= margin);
            let below_last_window = facade - rows[count - 1] - pane;
            assert!((rows[0] - below_last_window).abs() < 0.001);
        }
    }

    #[test]
    fn the_city_reveal_waits_for_the_bridge_approach() {
        let view = View::new(WIDTH, HEIGHT * HORIZON, Aim::default());
        let camera_at = |time| Flight { time }.camera(view);
        assert_eq!(city_relief(camera_at(0.90).eye[2]), 0.);
        let at_bridge = city_relief(camera_at(0.98).eye[2]);
        assert!(at_bridge > 0. && at_bridge < 1.);
    }

    #[test]
    fn the_inland_geometry_unfolds_continuously_after_the_bridge_cue() {
        let original = View::new(WIDTH, HEIGHT * HORIZON, Aim::default());
        let travel = |frame| {
            Flight {
                time: frame as f32 / 60.,
            }
            .camera(original)
            .eye[2]
        };
        assert_eq!(pocket_depth(travel(49)), 0.);
        assert!(pocket_depth(travel(55)) > 0.);
        assert!((0.1..0.9).contains(&pocket_depth(travel(75))));
        assert_eq!(pocket_depth(travel(114)), 1.);
        let mut last = 0.;
        for frame in 50..=114 {
            let next = pocket_depth(travel(frame));
            assert!(next >= last && next - last < 0.075);
            last = next;
        }
    }

    #[test]
    fn the_camera_looks_into_the_dive_then_lifts_through_the_fade() {
        let view = View::new(WIDTH, HEIGHT * HORIZON, Aim::default());
        let camera_at = |time| Flight { time }.camera(view);
        let forward = |camera: Camera| {
            let [x, y, z] = camera.eye;
            camera.point([x - HARBOR_YAW * 300., y, z + 300.]).unwrap()[1]
        };
        assert!(forward(camera_at(0.49)) < forward(camera_at(LAUNCH)) - 20.);
        assert!(forward(camera_at(1.04)) < forward(camera_at(0.49)) - 24.);
        assert!(forward(camera_at(2.08)) > forward(camera_at(1.04)) + 250.);
        for frame in 25..129 {
            let a = forward(camera_at(frame as f32 / 60.));
            let b = forward(camera_at((frame + 1) as f32 / 60.));
            assert!((b - a).abs() < 11.);
        }
        assert!(forward(camera_at(SHOT_END)) > forward(camera_at(2.85)) + 20.);
        assert!(forward(camera_at(CUT)) > forward(camera_at(SHOT_END)) + 5.);
        assert!(camera_at(CUT).eye[1] < camera_at(SHOT_END).eye[1]);
        assert!(camera_at(CUT).eye[2] > camera_at(SHOT_END).eye[2]);
    }

    #[test]
    fn the_extra_rows_emerge_in_order_after_the_bridge() {
        let view = View::new(WIDTH, HEIGHT * HORIZON, Aim::default());
        let rest = Camera::at_menu(view);
        let [farther, nearer] = city(view, view, rest, 0., [0., 0., WIDTH, HEIGHT], 0.);
        let opening = || farther.iter().chain(&nearer);
        assert!(
            SHORE_ROWS[1..]
                .iter()
                .all(|row| opening().any(|path| path.ink == row.ink))
        );
        assert!(opening().any(|path| path.ink == SHORE_ROWS[0].ink));
        let rows_at = |time| {
            let depth = skyline_camera(view, Flight { time }.camera(view)).eye[2];
            DISTANT_ROWS.map(|row| horizon_visibility(depth, row.depth))
        };
        assert_eq!(rows_at(LAUNCH), [0.; 6]);
        assert_eq!(rows_at(2.66), [0.; 6]);
        let [rest @ .., near] = rows_at(3.42);
        assert!(near > 0. && near < 1.);
        assert_eq!(rest, [0.; 5]);
        let [far, farther, middle, near, second, first] = rows_at(4.12);
        assert!(far == 0. && farther < 0.001);
        assert!(middle > 0. && middle < near && near <= second && second <= first);
        assert_eq!(rows_at(SHOT_END), [1.; 6]);
    }

    #[test]
    fn skyline_advances_slowly_and_rejoins_the_flight_without_reversing() {
        let view = View::new(WIDTH, HEIGHT * HORIZON, Aim::default());
        let dolly = |z| {
            let mut camera = Camera::at_menu(view);
            camera.eye = [view.x_at_depth(WIDTH * 0.5, z), 320., z];
            skyline_camera(view, camera).eye[2]
        };
        assert_eq!(dolly(-900.), -900.);
        assert_eq!(dolly(1660.), 1660.);
        let before = Flight { time: 49. / 60. }.camera(view);
        let during = Flight { time: 54. / 60. }.camera(view);
        let compact = skyline_camera(view, before);
        assert_eq!(compact.eye[2], before.eye[2]);
        assert!(compact.eye[1] < before.eye[1] - 25.);
        assert!(skyline_camera(view, during).eye[2] < during.eye[2]);
        assert!(dolly(70.) - dolly(-450.) < 160.);
        assert!(dolly(420.) < 0.);
        let mut last = dolly(-900.);
        for i in -89..=170 {
            let next = dolly(i as f32 * 10.);
            assert!(last < next);
            last = next;
        }
    }

    #[test]
    fn the_moon_rises_over_the_city_before_the_fade() {
        let original = View::new(WIDTH, HEIGHT * HORIZON, Aim::default());
        let under = Flight { time: 2.10 }.camera(original);
        let end = Flight { time: SHOT_END }.camera(original);
        let ([_, low], _) = moon_projection(original, under).unwrap();
        let ([_, high], zoom) = moon_projection(original, end).unwrap();
        assert!(high < low - 40.);
        assert!((zoom * MOON_DIAMETER - FLIGHT_MOON_DIAMETER).abs() < 0.001);
    }

    #[test]
    fn the_moon_stays_above_street_level_during_the_bridge_pass() {
        for aim in [Aim::default(), Aim::new(-1., -1.), Aim::new(1., 1.)] {
            let original = View::new(WIDTH, HEIGHT * HORIZON, aim);
            for frame in 35..=150 {
                let camera = Flight {
                    time: frame as f32 / 60.,
                }
                .camera(original);
                let ([x, y], zoom) = moon_projection(original, camera).unwrap();
                let street =
                    skyline_street_y(original, skyline_camera(original, camera), x).unwrap();
                let bottom = y + MOON_DIAMETER * zoom * 0.5;
                assert!(bottom <= 0. || bottom < street);
            }
        }
    }

    #[test]
    fn the_court_stays_hidden_until_the_camera_clears_the_near_side_of_the_span() {
        let original = View::new(WIDTH, HEIGHT * HORIZON, Aim::default());
        for frame in [49, 51, 54, 73, 91] {
            let camera = Flight {
                time: frame as f32 / 60.,
            }
            .camera(original);
            let view = View {
                camera: Some(camera),
                ..original
            };
            assert!(
                recreation(
                    view,
                    camera,
                    skyline_camera(original, camera),
                    pocket_depth(camera.eye[2])
                )
                .iter()
                .all(Vec::is_empty)
            );
        }
        let camera = Flight { time: 114. / 60. }.camera(original);
        let view = View {
            camera: Some(camera),
            ..original
        };
        let [ground, fixtures] = recreation(
            view,
            camera,
            skyline_camera(original, camera),
            pocket_depth(camera.eye[2]),
        );
        assert!(!ground.is_empty() && !fixtures.is_empty());
    }

    #[test]
    fn the_hoop_waits_for_the_ground_at_its_base_to_clear_the_skyline() {
        let original = View::new(WIDTH, HEIGHT * HORIZON, Aim::default());
        let pole_at = |frame| {
            let camera = Flight {
                time: frame as f32 / 60.,
            }
            .camera(original);
            let view = View {
                camera: Some(camera),
                ..original
            };
            let [ground, fixtures] = recreation(
                view,
                camera,
                skyline_camera(original, camera),
                pocket_depth(camera.eye[2]),
            );
            assert!(!ground.is_empty());
            fixtures
                .iter()
                .find(|path| path.ink == 0x827a88)
                .map(|path| path.opacity)
        };
        assert_eq!(
            pole_at(99),
            None,
            "the skyline still covers the hoop's base"
        );
        assert!(pole_at(114).is_some_and(|opacity| opacity > 0.9));
        assert_eq!(pole_at(130), Some(1.));
    }

    #[test]
    fn the_farthest_row_reaches_both_edges_of_the_shot() {
        let original = View::new(WIDTH, HEIGHT * HORIZON, Aim::default());
        for pad in [0., 210.] {
            let clip = [-pad, 0., WIDTH + pad, HEIGHT];
            let camera = Flight { time: SHOT_END }.camera(original).with_clip(clip);
            let view = View {
                camera: Some(camera),
                ..original
            };
            let [paths, _] = city(original, view, camera, 1., clip, 1.);
            let far_wall = DISTANT_ROWS[0].ink;
            for edge in [clip[0], clip[2]] {
                assert!(
                    paths
                        .iter()
                        .filter(|path| path.ink == far_wall)
                        .any(|path| {
                            path.points.iter().any(|point| point[0] <= edge + 8.)
                                && path.points.iter().any(|point| point[0] >= edge - 8.)
                        })
                );
            }
        }
    }

    #[test]
    fn the_shore_buildings_anchor_their_full_height_to_the_street() {
        let view = View::new(WIDTH, HEIGHT * HORIZON, Aim::default());
        let grade = shore::street_grade(view);
        for row in SHORE_ROWS {
            let mut camera = Camera::at_menu(view);
            camera.eye[2] = row.depth - 5.;
            let paths = city(view, view, camera, 0., [0., 0., WIDTH, HEIGHT], 1.);
            let wall = paths
                .iter()
                .flatten()
                .find(|path| path.ink == row.ink)
                .unwrap();
            let [a, b, c, d] = <[[f32; 2]; 4]>::try_from(wall.points.as_slice()).unwrap();
            for foot in [c, d] {
                assert!((view.lift(foot, row.depth)[1] - grade).abs() < 0.001);
            }
            for roof in [a, b] {
                assert!(view.lift(roof, row.depth)[1] <= grade - row.heights[0] + 0.001);
            }
        }
    }

    #[test]
    fn the_flight_projects_the_same_boat_art_as_the_menu() {
        let original = View::new(WIDTH, HEIGHT * HORIZON, Aim::default());
        let camera =
            Camera::at_menu(original).with_clip([-200., -100., WIDTH + 200., HEIGHT + 100.]);
        let view = View {
            camera: Some(camera),
            ..original
        };
        let time = 0.65;
        for (depth, art) in [
            (TUG_DEPTH, tugboat(original, time)),
            (FERRY_DEPTH, ferry(original, time)),
            (SAILBOAT_DEPTH, sailboat(original, time)),
        ] {
            let flight = boats(original, view, time, depth);
            assert_eq!(flight.len(), art.len());
            for (projected, authored) in flight.iter().zip(art) {
                assert_eq!(projected.ink, authored.ink);
                assert_eq!(projected.opacity, authored.opacity);
                assert_eq!(projected.points.len(), authored.points.len());
                for (projected, authored) in projected.points.iter().zip(authored.points) {
                    assert!((projected[0] - authored[0]).abs() < 0.001);
                    assert!((projected[1] - authored[1]).abs() < 0.001);
                }
            }
        }
    }

    #[test]
    fn the_crescent_grows_to_the_flight_diameter_and_returns_to_frame() {
        for aim in [
            Aim::default(),
            Aim::new(1., -1.),
            Aim::new(-1., 1.),
            Aim::new(1., 1.),
            Aim::new(-1., -1.),
        ] {
            let view = View::new(WIDTH, HEIGHT * HORIZON, aim);
            let moon_at = |time| moon_projection(view, Flight { time }.camera(view)).unwrap();
            let (center, mut previous) = moon_at(0.);
            assert!((center[0] - MOON_CENTER[0]).abs() < 0.001);
            assert!((center[1] - MOON_CENTER[1]).abs() < 0.001);
            assert_eq!(previous, 1.);
            for step in 0..=220 {
                let ([x, y], scale) = moon_at(step as f32 / 220. * CUT);
                let radius = scale * MOON_DIAMETER * 0.5;
                assert!(scale >= previous);
                assert!((radius..WIDTH - radius).contains(&x));
                assert!(y < HEIGHT - radius);
                if step == 220 {
                    assert!(y > radius);
                }
                previous = scale;
            }
            assert!((previous * MOON_DIAMETER - FLIGHT_MOON_DIAMETER).abs() < 0.001);
        }
    }

    #[test]
    fn forward_swoop_holds_heading_and_clears_both_bridge_faces() {
        for aim in [Aim::default(), Aim::new(1., -1.), Aim::new(-1., 1.)] {
            let view = View::new(WIDTH, HEIGHT * HORIZON, aim);
            let [west, east] = bridge::tug_channel(view);
            let start = Camera::at_menu(view);
            assert_eq!(
                Flight::default().camera(view),
                Flight { time: LAUNCH }.camera(view)
            );
            let end = Flight { time: CUT }.camera(view);
            assert!(end.eye[2] > DISTANT_ROWS[4].depth + 120.);
            assert_eq!(
                end,
                Flight {
                    time: Flight::DURATION
                }
                .camera(view)
            );
            let mut previous_z = start.eye[2] - 0.01;
            for frame in 0..=(Flight::DURATION * 60.).ceil() as usize {
                let camera = Flight {
                    time: frame as f32 / 60.,
                }
                .camera(view);
                let [x, y, z] = camera.eye;
                assert!(z >= previous_z);
                assert!((view.x_at_depth(WIDTH * 0.5, z) - x).abs() < 0.001);
                assert!(y < view.waterline() - 15.);
                if (-53. ..=53.).contains(&z) {
                    assert!((west..east).contains(&x));
                    assert!(y > view.deck() + 21.);
                }
                let city_depth = skyline_camera(view, camera).eye[2];
                for row in SHORE_ROWS.into_iter().chain(DISTANT_ROWS) {
                    if (row.depth..row.depth + 116.).contains(&city_depth) {
                        assert!(y < shore::street_grade(view) - row.heights[1] - 54.);
                    }
                }
                previous_z = z;
            }
        }
    }
}

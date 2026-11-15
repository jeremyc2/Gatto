//! A harbor diorama drawn from native shapes in the Rooftops window.

mod bridge;
mod landmarks;
mod shore;
mod traffic;

pub(super) use bridge::BridgeArt;

use super::*;

const WATER: u32 = 0x111e32;
const TRIM: u32 = 0x938095;
const LIGHT: u32 = 0xf5be87;
pub(super) const HORIZON: f32 = 0.755;
const WIDTH: f32 = 900.;
const HEIGHT: f32 = 560.;
const FERRY_DEPTH: f32 = -216.;
const SAILBOAT_DEPTH: f32 = -238.;
const TUG_DEPTH: f32 = 8.;
const FOREGROUND_BOAT_SPEED: f32 = 14.;
const TUG_TURN_DISTANCE: f32 = 12.;
const TUG_PHASE: f32 = 1.32;
const HARBOR_YAW: f32 = 0.64;
const VIEW_DEPTH: f32 = 0.72;
const VIEW_RECESSION: f32 = 0.13;
const MOON_CENTER: f32 = 706.;

pub(super) fn moon() -> Arc<[art::ArtPath]> {
    static MOON: LazyLock<Arc<[art::ArtPath]>> = LazyLock::new(|| {
        let center_y = 69.;
        let mut paths = Vec::with_capacity(12);
        for (radius, opacity) in [
            (58., 0.005),
            (52., 0.007),
            (47., 0.009),
            (42., 0.012),
            (38., 0.017),
            (34., 0.018),
        ] {
            let points = (0..64)
                .map(|step| {
                    let angle = step as f32 * std::f32::consts::TAU / 64.;
                    [
                        MOON_CENTER + radius * angle.cos(),
                        center_y + radius * angle.sin(),
                    ]
                })
                .collect();
            shape(&mut paths, points, 0xffdba0, opacity);
        }
        paths.extend(
            art::full_moon_paths(MOON_CENTER - 32., center_y - 32., 64.)
                .into_iter()
                .skip(2),
        );
        paths.into()
    });
    MOON.clone()
}

fn frame(viewport: gpui_kit::Size<gpui_kit::Pixels>) -> (gpui_kit::Bounds<gpui_kit::Pixels>, f32) {
    let scale = (f32::from(viewport.width) / WIDTH)
        .min(f32::from(viewport.height) / HEIGHT)
        .max(0.01);
    let size = gpui_kit::size(px(WIDTH * scale), px(HEIGHT * scale));
    let origin = gpui_kit::point(
        (viewport.width - size.width) * 0.5,
        (viewport.height - size.height) * 0.5,
    );
    (gpui_kit::Bounds::new(origin, size), scale)
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct Aim {
    x: f32,
    y: f32,
}

impl Aim {
    pub(super) fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    fn from_pointer(
        position: gpui_kit::Point<gpui_kit::Pixels>,
        size: gpui_kit::Size<gpui_kit::Pixels>,
    ) -> Self {
        let axis = |position: gpui_kit::Pixels, extent: gpui_kit::Pixels| {
            let extent = f32::from(extent).max(1.);
            -(2. * f32::from(position) / extent - 1.).clamp(-1., 1.)
        };
        Self {
            x: axis(position.x, size.width),
            y: axis(position.y, size.height),
        }
    }

    pub(super) fn follow(&mut self, target: Self, elapsed: f32) {
        let amount = 1. - (-12. * elapsed).exp();
        let step = |current: f32, target: f32| {
            if (current - target).abs() < 0.005 {
                target
            } else {
                current + (target - current) * amount
            }
        };
        *self = Self::new(step(self.x, target.x), step(self.y, target.y));
    }
}

/// Weak perspective keeps the broadside composition readable while the
/// transverse surfaces turn with the camera. Positive depth is away from us.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct View {
    pub(super) width: f32,
    pub(super) horizon: f32,
    aim: Aim,
}

impl View {
    fn new(width: f32, horizon: f32, aim: Aim) -> Self {
        Self {
            width,
            horizon,
            aim,
        }
    }

    fn magnification(self, x: f32, z: f32) -> f32 {
        let dx = x - self.width * 0.5;
        let distance = z * VIEW_DEPTH - dx * (VIEW_RECESSION + 0.04 * self.aim.x);
        1. / (1. + distance * (0.70 / self.width))
    }

    fn project(self, x: f32, z: f32, y: f32) -> [f32; 2] {
        let dx = x - self.width * 0.5;
        let zoom = self.magnification(x, z);
        let yaw = HARBOR_YAW + self.aim.x * 0.07;
        let tilt = 0.25 + self.aim.y * 0.035;
        [
            self.width * 0.5 + (dx + yaw * z + self.aim.x * 2.8) * zoom,
            self.horizon + (y - self.horizon - z * tilt + dx * 0.012 + self.aim.y * 2.) * zoom,
        ]
    }

    fn x_at_depth(self, screen_x: f32, depth: f32) -> f32 {
        let center = self.width * 0.5;
        let dx = screen_x - center;
        let lens = 0.70 / self.width;
        let yaw = HARBOR_YAW + self.aim.x * 0.07;
        let recession = VIEW_RECESSION + self.aim.x * 0.04;
        center
            + (dx * (1. + depth * VIEW_DEPTH * lens) - yaw * depth - self.aim.x * 2.8)
                / (1. + dx * recession * lens)
    }

    fn waterline(self) -> f32 {
        self.horizon + 29.
    }

    fn on_water(self, x: f32, depth: f32) -> [f32; 2] {
        self.project(x, depth, self.waterline())
    }

    // Pick the point on the street that projects to the landmark's original
    // horizontal position when the camera is centered.
    fn ground_billboard(self, screen_x: f32, depth: f32, grade: f32) -> ([f32; 2], f32) {
        let neutral = Self {
            aim: Aim::default(),
            ..self
        };
        let x = neutral.x_at_depth(screen_x, depth);
        let zoom = self.magnification(x, depth) / neutral.magnification(x, depth);
        (self.project(x, depth, grade), zoom)
    }

    // Distant silhouettes and the small boats retain their painted rest pose.
    // Their anchors and apparent size follow the same camera as the bridge.
    pub(super) fn billboard(self, x: f32, z: f32, y: f32) -> ([f32; 2], f32) {
        let neutral = Self {
            aim: Aim::default(),
            ..self
        };
        let [at_x, at_y] = self.project(x, z, y);
        let [rest_x, rest_y] = neutral.project(x, z, y);
        (
            [x + (at_x - rest_x), y + (at_y - rest_y)],
            self.magnification(x, z) / neutral.magnification(x, z),
        )
    }
}

impl Rooftops {
    pub(super) fn aim_menu_camera(
        &mut self,
        event: &gpui_kit::MouseMoveEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.menu_open
            && !self.options_open
            && !self.achievements_open
            && !self.reduced_motion(cx)
        {
            let (frame, _) = frame(window.viewport_size());
            let pointer = Aim::from_pointer(event.position - frame.origin, frame.size);
            if pointer != self.menu_aim {
                self.menu_aim = pointer;
                cx.notify();
            }
        }
    }

    pub(super) fn main_menu(
        &mut self,
        viewport: gpui_kit::Size<gpui_kit::Pixels>,
        cx: &mut Context<Self>,
    ) -> gpui_kit::Div {
        let (time, aim) = if self.reduced_motion(cx) {
            (0., Aim::default())
        } else {
            (self.menu_time, self.menu_camera)
        };
        let (frame, scale) = frame(viewport);
        let view = View::new(WIDTH, HEIGHT * HORIZON, aim);
        let bank = shore::Bank::new(view);
        let sized = |button: Button| {
            button
                .h(px(36. * scale))
                .w_full()
                .px(px(16. * scale))
                .py(px(8. * scale))
                .rounded(px(5. * scale))
                .text_size(px(14. * scale))
                .line_height(px(20. * scale))
        };
        let mut actions = v_flex()
            .w(px(194. * scale))
            .gap(px(8. * scale))
            .justify_center()
            .items_center()
            .child(
                sized(score_button("rooftops-play", "Play", cx))
                    .debug_selector(|| "rooftops-play".into())
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.reset_run();
                        window.focus(&this.focus, cx);
                        cx.notify();
                    })),
            )
            .child(
                sized(menu_button("rooftops-options", "Settings", cx))
                    .on_click(cx.listener(Self::toggle_options)),
            )
            .child(
                sized(menu_button("rooftops-achievements", "Achievements", cx))
                    .on_click(cx.listener(Self::open_achievements)),
            );
        if let Some(scorecard) = self.best_scorecard {
            actions = actions.child(
                sized(menu_button("rooftops-menu-scorecard", "Scorecard", cx)).on_click(
                    cx.listener(move |this, _, window, cx| {
                        this.show_scorecard(scorecard, window, cx);
                    }),
                ),
            );
        }
        div()
            .absolute()
            .left(frame.origin.x)
            .top(frame.origin.y)
            .w(frame.size.width)
            .h(frame.size.height)
            .overflow_hidden()
            .child(backdrop(
                WIDTH,
                HEIGHT,
                time,
                scale,
                BackdropView::Harbor(view),
            ))
            .child(vector_art(bank.inland(), scale))
            .child(landmarks::scene(view, scale))
            .child(harbor(
                view,
                bank,
                HEIGHT,
                time,
                scale,
                &mut self.menu_bridge,
            ))
            .child(
                v_flex()
                    .absolute()
                    .w_full()
                    .top(px(HEIGHT * scale * 0.115))
                    .items_center()
                    .gap(px(14. * scale))
                    .child(
                        div()
                            .font_family("Georgia")
                            .font_bold()
                            .text_size(px(119. * scale))
                            .line_height(px(132. * scale))
                            .text_color(rgb(0xffe1b1))
                            .child("Rooftops"),
                    )
                    .child(actions),
            )
    }
}

pub(super) fn menu_button(id: &'static str, label: &'static str, cx: &App) -> Button {
    Button::new(id)
        .debug_selector(move || id.into())
        .label(label)
        .custom(
            ButtonCustomVariant::new(cx)
                .color(rgb(0x776477).into())
                .foreground(rgb(0xffe1b1).into())
                .hover(rgb(0x3c354b).opacity(0.5).into())
                .active(rgb(0x4c3e55).opacity(0.66).into()),
        )
        .bg(rgb(0x222437).opacity(0.2))
        .h(px(36.))
        .px(px(16.))
        .rounded(px(5.))
        .border_1()
}

fn harbor(
    view: View,
    bank: shore::Bank,
    h: f32,
    time: f32,
    scale: f32,
    bridge: &mut BridgeArt,
) -> gpui_kit::Div {
    let View {
        width: w, horizon, ..
    } = view;
    let deck = horizon - 53.;
    let mut water = Vec::with_capacity(134);
    let mut rect = |bounds, ink, alpha| water.push(native_quad(bounds, ink, alpha, 0., scale));
    let depth = h - horizon;
    for i in 0..8 {
        rect(
            [0., horizon + depth * i as f32 / 8., w, depth / 8. + 0.2],
            0x080d20,
            i as f32 * 0.058,
        );
    }
    for i in 0..68 {
        let d = ((i * 47 + 2) % 101) as f32 / 101.;
        let y = horizon + 3. + (depth - 5.) * d;
        let base = ((i * 197 + 57) % 997) as f32 / 997.;
        let x = (base * w + time * (0.35 + d * 1.28)).rem_euclid(w + 56.) - 28.;
        let ([x, y], zoom) = view.billboard(x, -120. * d, y);
        let length = (7. + (i % 11) as f32 * 4.) * (0.4 + d) * zoom;
        let glint = (time * (0.41 + (i % 5) as f32 * 0.048) + i as f32 * 1.61).sin();
        rect(
            [x, y, length, 0.65 + d * 0.8],
            0x66749c,
            0.1 + (glint + 1.) * 0.095,
        );

        let reflection = MOON_CENTER + (i as f32 * 2.39 + time * 0.39).sin() * (3. + d * 34.);
        if i % 2 == 0 {
            rect(
                [reflection - length * 0.24, y, length * 0.48, 0.8],
                0xc1a2a1,
                0.06 + (glint + 1.) * 0.095,
            );
        }
    }
    let tug = tugboat(view, time);
    let mut boats = ferry(view, time);
    boats.extend(sailboat(view, time));
    div()
        .absolute()
        .inset_0()
        .child(vector_art(bank.water(h), scale))
        .child(div().absolute().child(NativeArt { quads: water }))
        .child(vector_art(bank.shore(), scale))
        .child(vector_art(tug, scale))
        .child(bridge.render(view, deck, time, scale))
        .child(vector_art(boats, scale))
}

fn tug_route([start, end]: [f32; 2], time: f32) -> (f32, f32) {
    let distance = end - start;
    let leg = distance + 2. * TUG_TURN_DISTANCE;
    let trip = (time * (FOREGROUND_BOAT_SPEED / 3.) + leg * TUG_PHASE).rem_euclid(2. * leg);
    let (along, facing) = if trip < leg {
        (trip, 1.)
    } else {
        (2. * leg - trip, -1.)
    };
    let easing = |d: f32| d * d / (4. * TUG_TURN_DISTANCE);
    let position = if along < 2. * TUG_TURN_DISTANCE {
        easing(along)
    } else if leg - along < 2. * TUG_TURN_DISTANCE {
        distance - easing(leg - along)
    } else {
        along - TUG_TURN_DISTANCE
    };
    (start + position, facing)
}

#[cfg(any(test, all(target_os = "macos", feature = "walkthrough-screenshots")))]
#[allow(dead_code)]
pub(crate) fn tug_turnarounds() -> [f32; 2] {
    let view = View::new(WIDTH, HEIGHT * HORIZON, Aim::default());
    let [start, end] = bridge::tug_channel(view);
    let duration = (end - start + 2. * TUG_TURN_DISTANCE) / (FOREGROUND_BOAT_SPEED / 3.);
    [(2. - TUG_PHASE) * duration, (3. - TUG_PHASE) * duration]
}

fn foreground_passage(view: View, time: f32, phase: f32) -> f32 {
    // The ferry and sailboat keep their spacing and reenter from offscreen.
    let length = view.width + 470.;
    -185. + length * (time * FOREGROUND_BOAT_SPEED / length + phase).rem_euclid(1.)
}

fn position_boat(
    mut paths: Vec<art::ArtPath>,
    view: View,
    [x, depth]: [f32; 2],
    size: f32,
) -> Vec<art::ArtPath> {
    let [left, waterline] = view.on_water(x, depth);
    let zoom = view.magnification(x, depth) * size;
    for [px, py] in paths.iter_mut().flat_map(|path| &mut path.points) {
        *px = left + (*px - x) * zoom;
        *py = waterline + *py * zoom;
    }
    paths
}

fn ferry(view: View, time: f32) -> Vec<art::ArtPath> {
    let x = foreground_passage(view, time, 0.32);
    let facing = 1.;
    let y = (time * 1.45).sin() * 0.9;
    let hull =
        [[-26., -7.], [31., -7.], [23., 0.], [-21., 0.]].map(|p| [x + p[0] * facing, y + p[1]]);
    let mut paths = Vec::with_capacity(30);
    wake(&mut paths, hull[3], facing, time, 1.);
    shape(&mut paths, hull.to_vec(), 0x080e1b, 1.);
    quad(&mut paths, [x - 23., y - 7.5, 46., 1.5], 0x9a6b70, 0.82);
    quad(&mut paths, [x - 16., y - 16., 33., 8.], 0x3e4154, 1.);
    quad(&mut paths, [x - 19., y - 17., 39., 1.7], TRIM, 1.);
    quad(&mut paths, [x - 4., y - 23., 8., 6.], 0x64596b, 1.);
    quad(&mut paths, [x - 0.5, y - 29., 1., 7.], TRIM, 1.);
    quad(&mut paths, [x - 1., y - 29., 2., 2.], LIGHT, 0.82);
    for i in 0..5 {
        quad(
            &mut paths,
            [x - 13. + i as f32 * 6., y - 14., 3., 3.],
            LIGHT,
            0.84,
        );
    }
    position_boat(paths, view, [x, FERRY_DEPTH], 0.90)
}

fn tugboat(view: View, time: f32) -> Vec<art::ArtPath> {
    let [start, end] = bridge::tug_channel(view);
    let (x, facing) = tug_route([start, end], time);
    let y = (time * 1.2 + 0.8).sin() * 0.65;
    let at = |points: &[[f32; 2]]| {
        points
            .iter()
            .map(|p| [x + p[0] * facing, y + p[1]])
            .collect()
    };
    let mut paths = Vec::with_capacity(26);
    let behind = if facing > 0. { x - start } else { end - x };
    let wake_size = (behind / 104.).clamp(0.13, 0.56);
    wake(&mut paths, [x - facing * 13., y], facing, time, wake_size);
    shape(
        &mut paths,
        at(&[[-17., -5.], [17., -5.], [13., 0.], [-13., 0.]]),
        0x101422,
        1.,
    );
    quad(&mut paths, [x - 15., y - 5.5, 30., 1.5], 0x965d61, 0.86);
    shape(
        &mut paths,
        at(&[[-9., -5.], [-9., -12.], [1., -12.], [7., -5.]]),
        0x7e6874,
        1.,
    );
    quad(&mut paths, [x - 4., y - 16., 11., 5.], 0x9c8584, 1.);
    quad(&mut paths, [x - 5., y - 17.5, 13., 1.8], 0x332c43, 1.);
    quad(&mut paths, [x - 2.5, y - 15., 3., 2.5], LIGHT, 0.74);
    quad(&mut paths, [x + 2., y - 15., 3., 2.5], LIGHT, 0.74);
    let stack = x - facing * 8.;
    quad(&mut paths, [stack - 2., y - 21., 4., 10.], 0x615263, 1.);
    quad(&mut paths, [stack - 2.8, y - 21., 5.6, 2.], 0x9d7576, 1.);
    for puff in 0..3 {
        let age = (time * 0.38 + puff as f32 / 3.).fract();
        let cx = stack - facing * age * 15.;
        let cy = y - 21. - age * 13.;
        let r = 1.5 + age * 3.;
        shape(
            &mut paths,
            vec![
                [cx - r, cy],
                [cx, cy - r * 0.6],
                [cx + r, cy],
                [cx, cy + r * 0.6],
            ],
            0xa39bb7,
            (1. - age) * 0.19,
        );
    }
    position_boat(paths, view, [x, TUG_DEPTH], 1.)
}

fn sailboat(view: View, time: f32) -> Vec<art::ArtPath> {
    let x = foreground_passage(view, time, 0.70);
    let facing = 1.;
    let y = (time * 1.3 + 1.).sin() * 1.15;
    let lean = (time * 0.7).sin() * 1.2;
    let at = |points: &[[f32; 2]]| {
        points
            .iter()
            .map(|p| [x + p[0] * facing, y + p[1]])
            .collect()
    };
    let mut paths = Vec::with_capacity(20);
    wake(&mut paths, [x - facing * 15., y], facing, time, 0.44);
    shape(
        &mut paths,
        at(&[[-18., -4.7], [21., -4.7], [15., 0.], [-15., 0.]]),
        0x2c2437,
        1.,
    );
    quad(&mut paths, [x - 15., y - 5.3, 30., 1.3], 0xbb8c82, 0.77);
    shape(
        &mut paths,
        at(&[[lean - 1.5, -34.], [-2., -8.], [-17., -8.], [-11., -17.]]),
        0xc7b0a4,
        0.93,
    );
    shape(
        &mut paths,
        at(&[[lean + 1.5, -28.], [3., -8.5], [16., -8.5], [11., -16.]]),
        0x817788,
        1.,
    );
    stroke(
        &mut paths,
        [x, y - 5.],
        [x + lean * facing, y - 35.],
        1.15,
        0xb5a4aa,
        1.,
    );
    quad(
        &mut paths,
        [x - facing * 11. - 1., y - 7., 2., 2.],
        LIGHT,
        0.88,
    );
    position_boat(paths, view, [x, SAILBOAT_DEPTH], 0.82)
}

fn wake(
    paths: &mut Vec<art::ArtPath>,
    [stern_x, stern_y]: [f32; 2],
    facing: f32,
    time: f32,
    size: f32,
) {
    // The hull covers the leading pixel and the wake fades into fainter glints.
    for i in 0..9 {
        let age = i as f32;
        let x = stern_x + facing * (1. - age * 10.) * size;
        let offset = (time * 1.35 + age * 0.66).sin() * (age * 0.35) * size;
        let y = stern_y + age * 0.9 * size;
        stroke(
            paths,
            [x, y + offset],
            [x - facing * (9. + age) * size, y + offset * 0.75],
            size,
            0x8994aa,
            0.36 * (1. - age / 9.),
        );
    }
}

fn shape(paths: &mut Vec<art::ArtPath>, points: Vec<[f32; 2]>, ink: u32, opacity: f32) {
    paths.push(art::ArtPath {
        points,
        ink,
        opacity,
    });
}

fn quad(paths: &mut Vec<art::ArtPath>, [x, y, w, h]: [f32; 4], ink: u32, opacity: f32) {
    shape(
        paths,
        vec![[x, y], [x + w, y], [x + w, y + h], [x, y + h]],
        ink,
        opacity,
    );
}

fn stroke(
    paths: &mut Vec<art::ArtPath>,
    a: [f32; 2],
    b: [f32; 2],
    width: f32,
    ink: u32,
    opacity: f32,
) {
    let [dx, dy] = [b[0] - a[0], b[1] - a[1]];
    let scale = width * 0.5 / dx.hypot(dy).max(0.001);
    let [ox, oy] = [-dy * scale, dx * scale];
    shape(
        paths,
        vec![
            [a[0] + ox, a[1] + oy],
            [b[0] + ox, b[1] + oy],
            [b[0] - ox, b[1] - oy],
            [a[0] - ox, a[1] - oy],
        ],
        ink,
        opacity,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn harbor_camera_changes_the_visible_width_and_pitch_of_the_road() {
        let viewport = gpui_kit::size(px(960.), px(600.));
        let center = Aim::from_pointer(gpui_kit::point(px(480.), px(300.)), viewport);
        let target = Aim::from_pointer(gpui_kit::point(px(0.), px(0.)), viewport);
        assert_eq!(center, Aim::default());
        let footprint = |aim| {
            let view = View::new(960., 453., aim);
            let [nx, ny] = view.project(219., -44., 416.);
            let [fx, fy] = view.project(219., 44., 416.);
            [fx - nx, ny - fy]
        };
        let mut camera = center;
        camera.follow(target, 1. / 60.);
        for axis in 0..2 {
            assert!(footprint(camera)[axis] > footprint(center)[axis]);
            assert!(footprint(camera)[axis] < footprint(target)[axis]);
        }
        for _ in 0..60 {
            camera.follow(target, 1. / 60.);
        }
        assert_eq!(camera, target);
    }

    #[test]
    fn fading_wake_starts_under_the_stern_in_both_directions() {
        let view = View::new(900., 453., Aim::default());
        for (paths, expected_facing) in [
            (ferry(view, 0.), 1.),
            (tugboat(view, 0.), -1.),
            (tugboat(view, tug_turnarounds()[0] + 17.), 1.),
        ] {
            let hull = &paths[9].points;
            let [stern_x, stern_y] = hull[3];
            let facing = (hull[2][0] - stern_x).signum();
            assert_eq!(facing, expected_facing);
            let first = &paths[0].points;
            let start_x = (first[0][0] + first[3][0]) * 0.5;
            let end_x = (first[1][0] + first[2][0]) * 0.5;
            assert!((0.4..1.2).contains(&((start_x - stern_x) * facing)));
            assert!((end_x - stern_x) * facing < 0.);
            assert!(((first[0][1] + first[3][1]) * 0.5 - stern_y).abs() < 0.001);
            assert!(
                paths[..9]
                    .windows(2)
                    .all(|pair| pair[0].opacity > pair[1].opacity)
            );
        }
    }
}

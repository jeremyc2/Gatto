//! A harbor diorama drawn from native shapes in the Rooftops window.

mod bridge;
mod flight;
mod landmarks;
mod projection;
mod shore;
mod traffic;

pub(super) use flight::Flight;

use super::*;
use bridge::BridgeArt;

const WATER: u32 = 0x111e32;
const TRIM: u32 = 0x938095;
const LIGHT: u32 = 0xf5be87;
pub(super) const HORIZON: f32 = 0.755;
const WIDTH: f32 = 900.;
const HEIGHT: f32 = 560.;
const FERRY_DEPTH: f32 = -216.;
const SAILBOAT_DEPTH: f32 = -238.;
const TUG_DEPTH: f32 = 8.;
const WAKE_STREAKS: usize = 9;
const FOREGROUND_BOAT_SPEED: f32 = 14.;
const TUG_TURN_DISTANCE: f32 = 12.;
const TUG_PHASE: f32 = 1.32;
const HARBOR_YAW: f32 = 0.64;
const VIEW_DEPTH: f32 = 0.72;
const VIEW_RECESSION: f32 = 0.13;
const MOON_CENTER: [f32; 2] = [832., 64.];
const MOON_DIAMETER: f32 = 64.;

#[derive(Default)]
pub(super) struct HarborArt {
    bridge: BridgeArt,
    skyline: Option<(View, f32, Mesh)>,
}

impl HarborArt {
    fn skyline(&mut self, view: View, scale: f32) -> Mesh {
        if let Some((previous, previous_scale, mesh)) = &self.skyline
            && (*previous, *previous_scale) == (view, scale)
        {
            return mesh.clone();
        }
        let mesh = flight::opening_skyline(view, scale);
        self.skyline = Some((view, scale, mesh.clone()));
        mesh
    }
}

pub(super) fn moon() -> Arc<[art::ArtPath]> {
    static MOON: LazyLock<Arc<[art::ArtPath]>> = LazyLock::new(|| {
        let [x, y] = MOON_CENTER.map(|center| center - MOON_DIAMETER * 0.5);
        art::moon_paths(x, y, MOON_DIAMETER, 0).into()
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
    camera: Option<projection::Camera>,
}

impl View {
    fn new(width: f32, horizon: f32, aim: Aim) -> Self {
        Self {
            width,
            horizon,
            aim,
            camera: None,
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

    fn rear(self, [x, y, z]: projection::Point, depth: f32) -> projection::Point {
        // The city streets run in the direction of the camera's fixed heading.
        let yaw = HARBOR_YAW + self.aim.x * 0.07;
        [x - yaw * depth, y, z + depth]
    }

    fn deck(self) -> f32 {
        self.horizon - 53.
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
                        this.play_from_menu(window, cx);
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
            .child(menu_scenery(view, bank, time, scale, &mut self.harbor_art))
            .child(
                v_flex()
                    .absolute()
                    .w_full()
                    .top(px(HEIGHT * scale * 0.115))
                    .items_center()
                    .gap(px(14. * scale))
                    .child(title(scale))
                    .child(actions),
            )
    }
}

fn title(scale: f32) -> gpui_kit::Div {
    div()
        .font_family("Georgia")
        .font_bold()
        .text_size(px(119. * scale))
        .line_height(px(132. * scale))
        .text_color(rgb(0xffe1b1))
        .child("Rooftops")
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

fn menu_scenery(
    view: View,
    bank: shore::Bank,
    time: f32,
    scale: f32,
    harbor: &mut HarborArt,
) -> gpui_kit::Div {
    sky(WIDTH, HEIGHT, time, scale, 145.)
        .child(vector_art(moon(), scale))
        .child(paint_mesh(harbor.skyline(view, scale)))
        .child(vector_art(bank.inland(), scale))
        .child(landmarks::scene(view, scale))
        .child(idle_water_and_bridge(
            view,
            bank,
            HEIGHT,
            time,
            scale,
            &mut harbor.bridge,
        ))
}

fn idle_water_and_bridge(
    view: View,
    bank: shore::Bank,
    h: f32,
    time: f32,
    scale: f32,
    bridge: &mut BridgeArt,
) -> gpui_kit::Div {
    let mut water = Vec::with_capacity(110);
    water_detail(view, h, time, |bounds, ink, alpha| {
        water.push(native_quad(bounds, ink, alpha, 0., scale));
    });
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
        .child(bridge.render(view, view.deck(), time, scale))
        .child(vector_art(boats, scale))
}

fn water_detail(view: View, h: f32, time: f32, mut rect: impl FnMut([f32; 4], u32, f32)) {
    let View {
        width: w, horizon, ..
    } = view;
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

        let reflection = MOON_CENTER[0] + (i as f32 * 2.39 + time * 0.39).sin() * (3. + d * 34.);
        if i % 2 == 0 {
            rect(
                [reflection - length * 0.24, y, length * 0.48, 0.8],
                0xc1a2a1,
                0.06 + (glint + 1.) * 0.095,
            );
        }
    }
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
    let mut paths = Vec::with_capacity(44);
    wake(&mut paths, hull[3], facing, time, 1.);
    shape(&mut paths, hull.to_vec(), 0x080e1b, 1.);
    quad(&mut paths, [x - 23., y - 7.5, 46., 1.5], 0x9a6b70, 0.82);
    stroke(
        &mut paths,
        [x - 21., y - 3.8],
        [x + 23., y - 3.8],
        0.65,
        0x616578,
        0.77,
    );
    deck_rail(&mut paths, [x - 24., y - 7.5], 8.);
    deck_rail(&mut paths, [x + 17., y - 7.5], 11.);
    quad(&mut paths, [x - 16., y - 16., 33., 8.], 0x3e4154, 1.);
    quad(&mut paths, [x - 15., y - 10., 31., 1.1], 0x64596b, 1.);
    quad(&mut paths, [x - 19., y - 17., 39., 1.7], TRIM, 1.);
    quad(&mut paths, [x - 4., y - 23., 8., 6.], 0x64596b, 1.);
    quad(&mut paths, [x - 2.9, y - 21.7, 5.8, 2.25], 0x9aabc2, 0.88);
    quad(&mut paths, [x - 4.7, y - 23.4, 9.4, 0.9], TRIM, 1.);
    quad(&mut paths, [x - 0.5, y - 29., 1., 7.], TRIM, 1.);
    quad(&mut paths, [x - 1., y - 29., 2., 2.], LIGHT, 0.82);
    quad(&mut paths, [x + 24., y - 9., 1.65, 1.3], 0x9dbead, 0.86);
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
    let mut paths = Vec::with_capacity(44);
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
    for along in [-12., 11.] {
        let fender = x + along * facing;
        oval(&mut paths, [fender, y - 3.4], [1.5, 1.7], 0x1d1b2a, 1.);
        oval(&mut paths, [fender, y - 3.4], [0.64, 0.77], 0x5f5263, 0.9);
    }
    deck_rail(&mut paths, [x + facing * 9., y - 5.4], facing * 5.4);
    shape(
        &mut paths,
        at(&[[-9., -5.], [-9., -12.], [1., -12.], [7., -5.]]),
        0x7e6874,
        1.,
    );
    quad(&mut paths, [x - 5., y - 8.8, 10., 1.05], 0x9a7c81, 1.);
    quad(&mut paths, [x - 4., y - 16., 11., 5.], 0x9c8584, 1.);
    quad(&mut paths, [x - 5., y - 17.5, 13., 1.8], 0x332c43, 1.);
    quad(&mut paths, [x - 2.5, y - 15., 3., 2.5], LIGHT, 0.74);
    quad(&mut paths, [x + 2., y - 15., 3., 2.5], LIGHT, 0.74);
    let stack = x - facing * 8.;
    quad(&mut paths, [stack - 2., y - 21., 4., 10.], 0x615263, 1.);
    quad(&mut paths, [stack - 1.55, y - 19., 0.9, 7.2], 0x827081, 1.);
    quad(&mut paths, [stack - 2.8, y - 21., 5.6, 2.], 0x9d7576, 1.);
    oval(&mut paths, [stack, y - 21.], [2.6, 0.8], 0x393043, 1.);
    for puff in 0..5 {
        let phase = puff as f32;
        let age = (time * 0.47 + phase / 5.).fract();
        let drift = (time * 1.7 + phase * 2.4).sin() * age * 1.2;
        let cx = stack - facing * age * 13. + drift;
        let cy = y - 21.4 - age * 15.;
        let r = 1.3 + age * 3.;
        let opacity = (age * 14.).min(1.) * (1. - age).powi(2) * 0.14;
        oval(&mut paths, [cx, cy], [r, r * 0.73], 0xb4adc2, opacity);
        oval(
            &mut paths,
            [cx + drift * 0.65, cy - r * 0.38],
            [r * 0.74, r * 0.55],
            0xa39bb7,
            opacity * 0.6,
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
    let mut paths = Vec::with_capacity(28);
    wake(&mut paths, [x - facing * 15., y], facing, time, 0.44);
    shape(
        &mut paths,
        at(&[[-18., -4.7], [21., -4.7], [15., 0.], [-15., 0.]]),
        0x2c2437,
        1.,
    );
    quad(&mut paths, [x - 15., y - 5.3, 30., 1.3], 0xbb8c82, 0.77);
    stroke(
        &mut paths,
        [x - 14., y - 2.5],
        [x + 16., y - 2.5],
        0.65,
        0x8b6b7b,
        0.9,
    );
    quad(&mut paths, [x - 5., y - 7., 10., 2.], 0x68505e, 1.);
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
        [x - 13.1, y - 14.5],
        [x - 1.5, y - 14.5],
        0.62,
        0x937d83,
        0.65,
    );
    stroke(
        &mut paths,
        [x + 5., y - 13.3],
        [x + 12.4, y - 13.3],
        0.55,
        0xb2a0a6,
        0.64,
    );
    stroke(
        &mut paths,
        [x - 16., y - 5.6],
        [x + lean - 0.6, y - 34.3],
        0.46,
        0x918799,
        0.72,
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

fn deck_rail(paths: &mut Vec<art::ArtPath>, [x, y]: [f32; 2], length: f32) {
    stroke(
        paths,
        [x, y - 2.9],
        [x + length, y - 2.9],
        0.65,
        0x96899c,
        0.86,
    );
    for amount in [0.04, 0.5, 0.96] {
        let x = x + length * amount;
        stroke(paths, [x, y - 2.9], [x, y], 0.6, 0x96899c, 0.8);
    }
}

fn oval(
    paths: &mut Vec<art::ArtPath>,
    [x, y]: [f32; 2],
    [rx, ry]: [f32; 2],
    ink: u32,
    opacity: f32,
) {
    let points = (0..16)
        .map(|i| {
            let (sin, cos) = (i as f32 * std::f32::consts::TAU / 16.).sin_cos();
            [x + rx * cos, y + ry * sin]
        })
        .collect();
    shape(paths, points, ink, opacity);
}

fn wake(
    paths: &mut Vec<art::ArtPath>,
    [stern_x, stern_y]: [f32; 2],
    facing: f32,
    time: f32,
    size: f32,
) {
    // The hull covers the leading pixel and the wake fades into fainter glints.
    for i in 0..WAKE_STREAKS {
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
            0.36 * (1. - age / WAKE_STREAKS as f32),
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

type Mesh = Arc<[(gpui_kit::Path<gpui_kit::Pixels>, gpui_kit::Hsla)]>;

fn art_mesh(shapes: &[art::ArtPath], scale: f32) -> Mesh {
    shapes
        .chunk_by(|a, b| a.ink == b.ink && a.opacity == b.opacity)
        .filter_map(|group| {
            use gpui_kit::{FillOptions, FillRule, PathBuilder, PathStyle};

            // Overlapping solid shapes cover each other, including rooftop
            // setbacks that extend into the walls below them.
            let solid = group[0].opacity == 1.;
            let mut builder = PathBuilder::fill();
            if solid {
                builder.style =
                    PathStyle::Fill(FillOptions::default().with_fill_rule(FillRule::NonZero));
            }
            let at = |p: &[f32; 2]| gpui_kit::point(px(p[0] * scale), px(p[1] * scale));
            for shape in group {
                let points = &shape.points;
                let reversed = solid
                    && points
                        .iter()
                        .zip(points.iter().cycle().skip(1))
                        .map(|(a, b)| (a[0] - b[0]) * (a[1] + b[1]))
                        .sum::<f32>()
                        < 0.;
                let mut points = (0..points.len()).map(|i| {
                    if reversed {
                        &points[points.len() - 1 - i]
                    } else {
                        &points[i]
                    }
                });
                if let Some(point) = points.next() {
                    builder.move_to(at(point));
                    for point in points {
                        builder.line_to(at(point));
                    }
                    builder.close();
                }
            }
            let color = gpui_kit::Hsla::from(rgb(group[0].ink)).opacity(group[0].opacity);
            builder.build().ok().map(|path| (path, color))
        })
        .collect()
}

fn paint_mesh(layers: Mesh) -> impl IntoElement {
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

    #[test]
    fn overlapping_roof_and_wall_contours_fill_the_join() {
        let mut paths = Vec::new();
        quad(&mut paths, [0., 0., 10., 10.], 0x2c2b43, 1.);
        quad(&mut paths, [0., 8., 10., 10.], 0x2c2b43, 1.);
        paths[1].points.reverse();
        let mesh = art_mesh(&paths, 1.);
        assert_eq!(mesh.len(), 1);
        let area: f32 = mesh[0]
            .0
            .vertices
            .as_chunks::<3>()
            .0
            .iter()
            .map(|triangle| {
                let [a, b, c] = triangle.each_ref().map(|vertex| vertex.xy_position);
                let ab = b - a;
                let ac = c - a;
                (f32::from(ab.x) * f32::from(ac.y) - f32::from(ab.y) * f32::from(ac.x)).abs() * 0.5
            })
            .sum();
        assert!((area - 180.).abs() < 0.01);
    }

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
            let hull = &paths[WAKE_STREAKS].points;
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
                paths[..WAKE_STREAKS]
                    .windows(2)
                    .all(|pair| pair[0].opacity > pair[1].opacity)
            );
        }
    }
}

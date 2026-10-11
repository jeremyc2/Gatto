use std::{sync::Arc, time::Instant};

use crate::rooftops_score::Score;
use gpui_kit::component::{
    StyledExt as _,
    button::{Button, ButtonCustomVariant, ButtonVariants as _},
    h_flex, v_flex,
};

use gpui_kit::{
    App, AppContext as _, Context, FocusHandle, Focusable, Image, ImageFormat,
    InteractiveElement as _, IntoElement, KeyDownEvent, KeyUpEvent, ParentElement as _, Render,
    Styled as _, Subscription, Window, div, img, px, rgb,
};

const STEP: f32 = 1. / 120.;
const SPEED: f32 = 310.;
const GRAVITY: f32 = 1450.;
const JUMP: f32 = 535.;
const SKY: u32 = 0x101322;
const GOLD: u32 = 0xffdba0;
// The Button component intentionally fades custom colors at rest. Set this
// surface directly below so the primary action remains visibly available.
const SCORE_BUTTON_NORMAL: u32 = 0xffdfac;
const SCORE_BUTTON_HOVER: u32 = 0xffedc9;
const SCORE_BUTTON_ACTIVE: u32 = 0xe3b66e;

#[derive(Clone, Copy)]
struct Roof {
    x: f32,
    y: f32,
    width: f32,
}

const ROOFS: [Roof; 17] = [
    Roof {
        x: -100.,
        y: 430.,
        width: 430.,
    },
    Roof {
        x: 390.,
        y: 390.,
        width: 170.,
    },
    Roof {
        x: 635.,
        y: 340.,
        width: 145.,
    },
    Roof {
        x: 875.,
        y: 385.,
        width: 180.,
    },
    Roof {
        x: 1140.,
        y: 320.,
        width: 200.,
    },
    Roof {
        x: 1420.,
        y: 360.,
        width: 140.,
    },
    Roof {
        x: 1640.,
        y: 295.,
        width: 200.,
    },
    Roof {
        x: 1930.,
        y: 350.,
        width: 180.,
    },
    Roof {
        x: 2190.,
        y: 300.,
        width: 240.,
    },
    Roof {
        x: 2530.,
        y: 365.,
        width: 170.,
    },
    Roof {
        x: 2790.,
        y: 310.,
        width: 170.,
    },
    Roof {
        x: 3050.,
        y: 350.,
        width: 240.,
    },
    Roof {
        x: 3370.,
        y: 285.,
        width: 210.,
    },
    Roof {
        x: 3670.,
        y: 320.,
        width: 200.,
    },
    Roof {
        x: 3950.,
        y: 260.,
        width: 200.,
    },
    Roof {
        x: 4230.,
        y: 315.,
        width: 200.,
    },
    Roof {
        x: 4510.,
        y: 360.,
        width: 400.,
    },
];
pub(crate) const LEVELS: usize = 3;

fn level_roofs(level: usize) -> Vec<Roof> {
    if level == 0 {
        return ROOFS.to_vec();
    }
    let heights = [
        430., 380., 425., 350., 400., 340., 410., 335., 390., 320., 375., 305., 370., 320., 380.,
        310., 360., 300., 360., 310., 360.,
    ];
    let count = if level == 1 { 19 } else { 21 };
    let mut roofs = vec![Roof {
        x: -100.,
        y: 430.,
        width: 360.,
    }];
    for i in 1..count {
        let previous = roofs[i - 1];
        let y = if i == count - 1 { 360. } else { heights[i] };
        let gap = if previous.y - y > 60. {
            105.
        } else {
            [115., 135., 125.][i % 3] + level as f32 * 3.
        };
        let width = if i == count - 1 {
            400.
        } else if level == 1 {
            [150., 135., 170., 125.][i % 4]
        } else {
            [115., 145., 125.][i % 3]
        };
        roofs.push(Roof {
            x: previous.x + previous.width + gap,
            y,
            width,
        });
    }
    roofs
}

struct Coin {
    x: f32,
    y: f32,
    collected: bool,
}

#[derive(Clone, Copy)]
enum EnemyKind {
    Rat,
    Owl,
}

struct Enemy {
    roof: usize,
    kind: EnemyKind,
    defeated: bool,
    phase: f32,
    speed: f32,
}

impl Enemy {
    fn position(&self, roofs: &[Roof], time: f32) -> (f32, f32) {
        let roof = roofs[self.roof];
        let swing = (time * self.speed + self.phase).sin();
        match self.kind {
            EnemyKind::Rat => (
                roof.x + roof.width * 0.5 + swing * (roof.width * 0.5 - 28.),
                roof.y - 11.,
            ),
            EnemyKind::Owl => (
                roof.x + roof.width * 0.55 + swing * 70.,
                roof.y - 78. + (time * 2.1 + self.phase).sin() * 20.,
            ),
        }
    }
}

#[derive(Default)]
struct Input {
    left: bool,
    right: bool,
    jump: bool,
}

struct Run {
    level: usize,
    banked_coins: usize,
    banked_returns: usize,
    banked_kills: usize,
    elapsed: f64,
    roofs: Vec<Roof>,
    coins: Vec<Coin>,
    enemies: Vec<Enemy>,
    x: f32,
    y: f32, // Feet, rather than the sprite's top edge.
    vx: f32,
    vy: f32,
    facing: f32,
    ground: Option<usize>,
    checkpoint: usize,
    coyote: f32,
    buffer: f32,
    squash: f32,
    time: f32,
    jumps: usize,
    returns: usize,
    lit: Vec<bool>,
    invulnerable: f32,
    input: Input,
    home: bool,
}

impl Run {
    fn new() -> Self {
        Self::for_level(0)
    }

    fn for_level(level: usize) -> Self {
        let roofs = level_roofs(level);
        let mut lit = vec![false; roofs.len()];
        lit[0] = true;
        let coins = roofs
            .iter()
            .enumerate()
            .map(|(i, roof)| {
                let (x, y) = if i > 0 && i % 2 == 1 {
                    let previous = roofs[i - 1];
                    (
                        (previous.x + previous.width + roof.x) * 0.5,
                        (previous.y + roof.y) * 0.5 - 78.,
                    )
                } else {
                    (roof.x + roof.width * 0.72, roof.y - 104.)
                };
                Coin {
                    x,
                    y,
                    collected: false,
                }
            })
            .collect();
        let mut enemies = Vec::new();
        for i in 2..roofs.len() - 1 {
            if i % (if level == 2 { 2 } else { 3 }) == 0 {
                enemies.push(Enemy {
                    roof: i,
                    kind: EnemyKind::Rat,
                    defeated: false,
                    phase: i as f32 * 1.7,
                    speed: 1. + level as f32 * 0.35,
                });
            }
            if level > 0 && i % 5 == 2 {
                enemies.push(Enemy {
                    roof: i,
                    kind: EnemyKind::Owl,
                    defeated: false,
                    phase: i as f32,
                    speed: 0.8 + level as f32 * 0.25,
                });
            }
        }
        Self {
            level,
            banked_coins: 0,
            banked_returns: 0,
            banked_kills: 0,
            elapsed: 0.,
            roofs,
            coins,
            enemies,
            x: 105.,
            y: ROOFS[0].y,
            vx: 0.,
            vy: 0.,
            facing: 1.,
            ground: Some(0),
            checkpoint: 0,
            coyote: 0.12,
            buffer: 0.,
            squash: 0.,
            time: 0.,
            jumps: 0,
            returns: 0,
            lit,
            invulnerable: 0.,
            input: Input::default(),
            home: false,
        }
    }

    fn jump_down(&mut self) {
        if self.home {
            let next = (self.level + 1) % LEVELS;
            let score = self.score();
            *self = Self::for_level(next);
            if next != 0 {
                self.banked_coins = score.coins;
                self.banked_returns = score.deaths;
                self.banked_kills = score.kills;
                self.elapsed = score.seconds;
            }
        } else if !self.input.jump {
            self.input.jump = true;
            self.buffer = 0.12;
        }
    }

    fn jump_up(&mut self) {
        self.input.jump = false;
        // Releasing early gives small hops; holding gives the full arc.
        if self.vy < -200. {
            self.vy *= 0.55;
        }
    }

    fn step(&mut self) {
        self.time += STEP;
        self.invulnerable = (self.invulnerable - STEP).max(0.);
        self.squash = (self.squash - STEP * 5.).max(0.);
        if self.home {
            return;
        }

        self.buffer = (self.buffer - STEP).max(0.);
        self.coyote = if self.ground.is_some() {
            0.12
        } else {
            (self.coyote - STEP).max(0.)
        };
        let axis = self.input.right as u8 as f32 - self.input.left as u8 as f32;
        let target = axis * SPEED;
        let acceleration = if axis == 0. { 2200. } else { 1850. };
        self.vx += (target - self.vx).clamp(-acceleration * STEP, acceleration * STEP);
        if axis != 0. {
            self.facing = axis;
        }
        if self.buffer > 0. && self.coyote > 0. {
            self.vy = -JUMP;
            self.ground = None;
            self.buffer = 0.;
            self.coyote = 0.;
            self.jumps += 1;
        }

        let previous_y = self.y;
        self.move_horizontal();
        self.vy += GRAVITY * STEP;
        self.y += self.vy * STEP;
        let previous_ground = self.ground.take();
        if self.invulnerable == 0. {
            for i in 0..self.enemies.len() {
                let enemy = &mut self.enemies[i];
                if enemy.defeated {
                    continue;
                }
                let (x, y) = enemy.position(&self.roofs, self.time);
                let top = y - 11.;
                if (self.x - x).abs() < 25. && self.y > top && self.y - 30. < y + 11. {
                    if self.vy > 0. && previous_y <= top + 5. {
                        enemy.defeated = true;
                        self.y = top;
                        self.vy = if self.input.jump { -JUMP } else { -410. };
                        self.coyote = 0.;
                        break;
                    } else {
                        self.respawn();
                        return;
                    }
                }
            }
        }
        if self.vy >= 0. {
            for i in 0..self.roofs.len() {
                let roof = self.roofs[i];
                if self.x + 11. > roof.x
                    && self.x - 11. < roof.x + roof.width
                    && previous_y <= roof.y + 0.1
                    && self.y >= roof.y
                {
                    self.y = roof.y;
                    self.vy = 0.;
                    self.ground = Some(i);
                    if previous_ground != Some(i) {
                        self.squash = 1.;
                        self.lit[i] = true;
                        if i.is_multiple_of(self.checkpoint_spacing()) {
                            self.checkpoint = i;
                        }
                    }
                    break;
                }
            }
        }
        for i in 0..self.coins.len() {
            let coin = &mut self.coins[i];
            if !coin.collected
                && (self.x - coin.x).abs() < 19.
                && (self.y - 20. - coin.y).abs() < 22.
            {
                coin.collected = true;
            }
        }
        if self.y > 720. {
            self.respawn();
        }
        if self.x >= self.destination() && self.ground == Some(self.roofs.len() - 1) {
            self.home = true;
            self.vx = 0.;
            self.input = Input::default();
        }
    }

    fn score(&self) -> Score {
        Score {
            coins: self.coin_count(),
            deaths: self.banked_returns + self.returns,
            kills: self.banked_kills + self.enemies.iter().filter(|enemy| enemy.defeated).count(),
            seconds: self.elapsed,
        }
    }

    fn record_active_time(&mut self, elapsed: f64) {
        if !self.home {
            self.elapsed += elapsed;
        }
    }

    fn checkpoint_spacing(&self) -> usize {
        if self.level == 0 { 4 } else { 5 }
    }

    fn move_horizontal(&mut self) {
        let previous_x = self.x;
        self.x = (self.x + self.vx * STEP).clamp(15., self.destination() + 35.);
        let mut blocked = false;
        for roof in &self.roofs {
            // Resolve horizontal motion at the old foot height before falling.
            // A cat above the roof can still cross the edge and land on top.
            if self.y <= roof.y + 0.1 {
                continue;
            }
            let right = roof.x + roof.width;
            if previous_x + 11. <= roof.x + 0.1 && self.x + 11. > roof.x {
                self.x = self.x.min(roof.x - 11.);
                blocked = true;
            } else if previous_x - 11. >= right - 0.1 && self.x - 11. < right {
                self.x = self.x.max(right + 11.);
                blocked = true;
            }
        }
        if blocked {
            self.vx = 0.;
            self.coyote = 0.;
        }
    }

    fn coin_count(&self) -> usize {
        self.banked_coins + self.coins.iter().filter(|coin| coin.collected).count()
    }

    fn destination(&self) -> f32 {
        let last = self.roofs.last().unwrap();
        last.x + last.width - 140.
    }

    fn respawn(&mut self) {
        let roof = self.roofs[self.checkpoint];
        self.x = if self.checkpoint == 0 {
            105.
        } else {
            roof.x + 24.
        };
        self.y = roof.y;
        self.vx = 0.;
        self.vy = 0.;
        self.ground = Some(self.checkpoint);
        self.coyote = 0.12;
        self.buffer = 0.;
        self.invulnerable = 1.2;
        self.returns += 1;
    }
}

pub struct Rooftops {
    run: Run,
    focus: FocusHandle,
    last_frame: Option<Instant>,
    remainder: f32,
    camera: f32,
    coin_art: Arc<Image>,
    save_message: Option<String>,
    saving: bool,
    #[cfg(feature = "walkthrough-screenshots")]
    preview: bool,
    _activation: Subscription,
    _blur: Subscription,
}

impl Rooftops {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus = cx.focus_handle();
        let activation = cx.observe_window_activation(window, |this, _, cx| {
            this.run.input = Input::default();
            this.run.buffer = 0.;
            this.last_frame = None;
            this.remainder = 0.;
            cx.notify();
        });
        let blur = cx.on_blur(&focus, window, |this, _, cx| {
            this.run.input = Input::default();
            this.run.buffer = 0.;
            this.last_frame = None;
            this.remainder = 0.;
            cx.notify();
        });
        // Match the other utility windows: avoid native window teardown in
        // bundled builds. Reset the run before hiding the retained window.
        let game = cx.entity().downgrade();
        window.on_window_should_close(cx, move |window, cx| {
            let _ = game.update(cx, |game, cx| {
                game.run = Run::new();
                game.last_frame = None;
                game.remainder = 0.;
                game.camera = 0.;
                game.save_message = None;
                game.saving = false;
                #[cfg(feature = "walkthrough-screenshots")]
                {
                    game.preview = false;
                }
                cx.notify();
            });
            crate::window_limits::hide_instead_of_close(window)
        });
        Self {
            run: Run::new(),
            focus,
            last_frame: None,
            remainder: 0.,
            camera: 0.,
            coin_art: kitten_coin_art(),
            save_message: None,
            saving: false,
            #[cfg(feature = "walkthrough-screenshots")]
            preview: false,
            _activation: activation,
            _blur: blur,
        }
    }

    #[cfg(feature = "walkthrough-screenshots")]
    #[allow(dead_code)]
    pub(crate) fn for_preview(window: &mut Window, cx: &mut Context<Self>, level: usize) -> Self {
        let mut game = Self::new(window, cx);
        game.preview = true;
        game.run = Run::for_level(level);
        let run = &mut game.run;
        match level {
            0 => {
                // An early leap: first coins and lit apartments.
                run.x = run.roofs[2].x + run.roofs[2].width * 0.4;
                run.y = run.roofs[2].y - 76.;
                run.vy = -130.;
                run.ground = None;
                run.vx = SPEED;
                run.time = 3.2;
                run.jumps = 2;
                run.lit[..=2].fill(true);
                run.coins[..2]
                    .iter_mut()
                    .for_each(|coin| coin.collected = true);
                game.camera = run.x - 300.;
            }
            1 => {
                // Mid-level, descending toward an owl above a narrow roof.
                run.time = 18.4;
                let owl = run
                    .enemies
                    .iter()
                    .find(|enemy| enemy.roof == 7 && matches!(enemy.kind, EnemyKind::Owl))
                    .unwrap();
                let (x, y) = owl.position(&run.roofs, run.time);
                run.x = x;
                run.y = y - 22.;
                run.vy = 190.;
                run.vx = 90.;
                run.ground = None;
                run.jumps = 12;
                run.checkpoint = 5;
                run.banked_coins = 11;
                run.lit[..7].fill(true);
                run.coins[..6]
                    .iter_mut()
                    .for_each(|coin| coin.collected = true);
                for enemy in &mut run.enemies {
                    enemy.defeated = enemy.roof < 6;
                }
                game.camera = run.x - 410.;
            }
            _ => {
                // The final results poster after a complete run.
                let last = run.roofs.len() - 1;
                run.x = run.destination();
                run.y = run.roofs[last].y;
                run.ground = Some(last);
                run.time = 54.6;
                run.jumps = 32;
                run.checkpoint = last;
                run.home = true;
                run.banked_coins = 25;
                run.banked_returns = 6;
                run.banked_kills = 9;
                run.elapsed = 143.6;
                run.lit.fill(true);
                for (i, coin) in run.coins.iter_mut().enumerate() {
                    coin.collected = i % 4 != 1;
                }
                for enemy in &mut run.enemies {
                    enemy.defeated = enemy.roof < last - 2;
                }
                game.camera = run.roofs[last].x + run.roofs[last].width - 900.;
            }
        }
        game
    }

    fn save_score(&mut self, _: &gpui_kit::ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.saving || !self.run.home || self.run.level + 1 != LEVELS {
            return;
        }
        self.saving = true;
        self.save_message = None;
        let score = self.run.score();
        let task = cx.background_spawn(async move { score.save() });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                this.saving = false;
                // A restarted game must not receive the previous run's notice.
                if this.run.home && this.run.elapsed == score.seconds {
                    this.save_message = Some(match result {
                        Ok(path) => format!(
                            "Saved {} to Downloads",
                            path.file_name().unwrap().to_string_lossy()
                        ),
                        Err(error) => format!("Could not save score: {error}"),
                    });
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn results(&self, cx: &mut Context<Self>, width: f32, height: f32) -> gpui_kit::Div {
        let scale = ((width - 48.) / 900.).min((height - 100.) / 540.).max(0.1);
        let quads = crate::rooftops_score::poster_shapes()
            .into_iter()
            .map(|shape| native_quad(shape.rect, shape.ink, shape.opacity, shape.radius, scale))
            .collect();
        let mut poster = div()
            .relative()
            .w(px(900. * scale))
            .h(px(540. * scale))
            .flex_shrink_0()
            .overflow_hidden()
            .rounded(px(8. * scale))
            .child(div().absolute().child(NativeArt { quads }));
        for item in crate::rooftops_score::poster_text(self.run.score()) {
            let label = div()
                .absolute()
                .left(px(item.x * scale))
                .top(px(item.y * scale))
                .w(px((864. - item.x) * scale))
                .whitespace_nowrap()
                // GPUI's system-font raster bounds clip large glyphs on macOS.
                .font_family(crate::rooftops_score::POSTER_FONT)
                .text_size(px(item.size * scale))
                .line_height(px(item.size * 1.2 * scale))
                .text_color(rgb(item.ink))
                .child(item.label);
            poster = poster.child(if item.bold { label.font_bold() } else { label });
        }
        let mut card = v_flex().gap_3().items_center().child(poster).child(
            h_flex()
                .w(px(860. * scale))
                .justify_between()
                .items_center()
                .child(control_prompt("Space to play again"))
                .child(
                    Button::new("save-rooftops-score")
                        .loading(self.saving)
                        .custom(
                            ButtonCustomVariant::new(cx)
                                .color(rgb(SCORE_BUTTON_NORMAL).into())
                                .foreground(rgb(SKY).into())
                                .hover(rgb(SCORE_BUTTON_HOVER).into())
                                .active(rgb(SCORE_BUTTON_ACTIVE).into()),
                        )
                        .bg(rgb(SCORE_BUTTON_NORMAL))
                        .label("Save score")
                        .on_click(cx.listener(Self::save_score)),
                ),
        );
        if let Some(message) = &self.save_message {
            card = card.child(
                div()
                    .text_sm()
                    .text_color(rgb(0xb8aec4))
                    .child(message.clone()),
            );
        }
        card
    }

    fn key_down(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        if event.keystroke.modifiers.platform
            || event.keystroke.modifiers.control
            || event.keystroke.modifiers.alt
        {
            return;
        }
        match event.keystroke.key.as_str() {
            "left" => self.run.input.left = true,
            "right" => self.run.input.right = true,
            "space" if !event.is_held => {
                if self.run.home {
                    self.last_frame = None;
                    self.remainder = 0.;
                    self.save_message = None;
                }
                self.run.jump_down();
            }
            _ => return,
        }
        cx.stop_propagation();
        cx.notify();
    }

    fn key_up(&mut self, event: &KeyUpEvent, _: &mut Window, cx: &mut Context<Self>) {
        match event.keystroke.key.as_str() {
            "left" => self.run.input.left = false,
            "right" => self.run.input.right = false,
            "space" => self.run.jump_up(),
            _ => return,
        }
        cx.stop_propagation();
        cx.notify();
    }

    fn active_frame(&mut self, now: Instant, active: bool) -> Option<f32> {
        if !active {
            self.last_frame = None;
            return None;
        }
        let elapsed = self
            .last_frame
            .replace(now)
            .map_or(0., |last| now.duration_since(last).as_secs_f64());
        self.run.record_active_time(elapsed);
        // Count actual active time, while limiting physics catch-up after a stall.
        Some((elapsed as f32).min(0.05))
    }

    fn advance(&mut self, window: &mut Window, cx: &Context<Self>, view_width: f32) {
        #[cfg(feature = "walkthrough-screenshots")]
        if self.preview {
            return;
        }
        let active = window.is_window_active() && self.focus.is_focused(window);
        let Some(elapsed) = self.active_frame(Instant::now(), active) else {
            return;
        };
        self.remainder += elapsed;
        let returns = self.run.returns;
        while self.remainder >= STEP {
            self.run.step();
            self.remainder -= STEP;
        }
        let end = self.run.roofs.last().unwrap().x + self.run.roofs.last().unwrap().width;
        let target = (self.run.x - view_width * 0.34).clamp(0., end - view_width.min(end));
        if self.run.returns != returns || self.run.jumps == 0 {
            self.camera = target;
        } else {
            self.camera += (target - self.camera) * (1. - (-elapsed * 7.).exp());
        }
        // Movement is essential gameplay. Decorative drift is separately
        // disabled below when the user asks the system to reduce motion.
        if !self.run.home || (self.run.level + 1 < LEVELS && !cx.reduce_motion()) {
            window.request_animation_frame();
        }
    }
}

impl Focusable for Rooftops {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

fn control_prompt(label: &'static str) -> gpui_kit::Div {
    div()
        .px_4()
        .py_2()
        .rounded_full()
        .bg(rgb(SKY))
        .border_1()
        .border_color(rgb(0x4c4256))
        .text_sm()
        .text_color(rgb(0xf3ddba))
        .child(label)
}

// Native GPUI elements render the scene; physics stays independent of drawing.
fn block(x: f32, y: f32, w: f32, h: f32, color: u32, scale: f32) -> gpui_kit::Div {
    div()
        .absolute()
        .left(px(x * scale))
        .top(px(y * scale))
        .w(px(w * scale))
        .h(px(h.max(0.) * scale))
        .bg(rgb(color))
}

fn circle(x: f32, y: f32, diameter: f32, color: u32, scale: f32) -> gpui_kit::Div {
    block(x, y, diameter, diameter, color, scale).rounded_full()
}

fn kitten_coin_art() -> Arc<Image> {
    // Prefilter the fine engraving before it becomes a small GPU texture,
    // retaining the transparent edge and avoiding speckled highlights.
    let artwork = image::load_from_memory(include_bytes!("../packaging/kitten-coin.png"))
        .expect("embedded kitten coin must be a valid PNG")
        .resize_exact(112, 112, image::imageops::FilterType::Lanczos3);
    let mut bytes = std::io::Cursor::new(Vec::new());
    artwork
        .write_to(&mut bytes, image::ImageFormat::Png)
        .expect("embedded kitten coin must encode as PNG");
    Arc::new(Image::from_bytes(ImageFormat::Png, bytes.into_inner()))
}

fn cat_coin(x: f32, y: f32, scale: f32, artwork: &Arc<Image>) -> impl IntoElement {
    img(artwork.clone())
        .absolute()
        .left(px((x - 14.) * scale))
        .top(px((y - 14.) * scale))
        .w(px(28. * scale))
        .h(px(28. * scale))
}

type Joint = (f32, f32);

// One native GPUI element paints the connected discs, avoiding a separate
// layout node for every point along every limb. No canvas or image cache needed.
struct NativeArt {
    quads: Vec<gpui_kit::PaintQuad>,
}
impl IntoElement for NativeArt {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl gpui_kit::Element for NativeArt {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<gpui_kit::ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&gpui_kit::GlobalElementId>,
        _: Option<&gpui_kit::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (gpui_kit::LayoutId, ()) {
        (
            window.request_layout(gpui_kit::Style::default(), [], cx),
            (),
        )
    }
    fn prepaint(
        &mut self,
        _: Option<&gpui_kit::GlobalElementId>,
        _: Option<&gpui_kit::InspectorElementId>,
        _: gpui_kit::Bounds<gpui_kit::Pixels>,
        _: &mut (),
        _: &mut Window,
        _: &mut App,
    ) {
    }
    fn paint(
        &mut self,
        _: Option<&gpui_kit::GlobalElementId>,
        _: Option<&gpui_kit::InspectorElementId>,
        bounds: gpui_kit::Bounds<gpui_kit::Pixels>,
        _: &mut (),
        _: &mut (),
        window: &mut Window,
        _: &mut App,
    ) {
        for quad in &self.quads {
            let mut quad = quad.clone();
            quad.bounds.origin += bounds.origin;
            window.paint_quad(quad);
        }
    }
}

fn limb(a: Joint, b: Joint, widths: (f32, f32), ink: u32, scale: f32) -> gpui_kit::Div {
    let mut quads = Vec::with_capacity(24);
    append_limb(&mut quads, a, b, widths, ink, scale);
    div().absolute().child(NativeArt { quads })
}

fn append_limb(
    quads: &mut Vec<gpui_kit::PaintQuad>,
    a: Joint,
    b: Joint,
    widths: (f32, f32),
    ink: u32,
    scale: f32,
) {
    let distance = (b.0 - a.0).hypot(b.1 - a.1);
    let steps = (distance / 1.1).ceil().max(1.) as usize;
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let diameter = widths.0 + (widths.1 - widths.0) * t;
        quads.push(native_quad(
            [
                a.0 + (b.0 - a.0) * t - diameter * 0.5,
                a.1 + (b.1 - a.1) * t - diameter * 0.5,
                diameter,
                diameter,
            ],
            ink,
            1.,
            diameter * 0.5,
            scale,
        ));
    }
}

fn native_quad(
    rect: [f32; 4],
    color: u32,
    opacity: f32,
    radius: f32,
    scale: f32,
) -> gpui_kit::PaintQuad {
    let [x, y, w, h] = rect;
    let mut quad = gpui_kit::fill(
        gpui_kit::Bounds::new(
            gpui_kit::point(px(x * scale), px(y * scale)),
            gpui_kit::size(px(w * scale), px(h.max(0.) * scale)),
        ),
        gpui_kit::Hsla::from(rgb(color)).opacity(opacity),
    );
    quad.corner_radii = gpui_kit::Corners::all(px(radius * scale));
    quad
}

fn backdrop(w: f32, h: f32, camera: f32, time: f32, scale: f32) -> gpui_kit::Div {
    let mut quads = Vec::with_capacity(600);
    for band in 0..10 {
        quads.push(native_quad(
            [0., h * 0.48 + band as f32 * h * 0.052, w, h * 0.055],
            0x41324b,
            0.025 + band as f32 * 0.013,
            0.,
            scale,
        ));
    }
    for i in 0..75 {
        let x = ((i * 173 + 31) as f32 - camera * 0.06).rem_euclid(w);
        let y = ((i * 79 + 19) % 270) as f32;
        let d = if i % 9 == 0 { 2.4 } else { 1.2 };
        quads.push(native_quad(
            [x, y, d, d],
            0xc2c3e1,
            0.22 + 0.35 * ((time * 0.7 + i as f32).sin() * 0.5 + 0.5),
            d * 0.5,
            scale,
        ));
    }
    let moon = w - 155.;
    for (rect, color, opacity) in [
        ([moon - 34., 24., 152., 152.], GOLD, 0.025),
        ([moon - 15., 43., 114., 114.], GOLD, 0.04),
        ([moon, 58., 84., 84.], 0xf3ddba, 1.),
        ([moon + 13., 65., 18., 18.], 0xe5cda9, 0.35),
        ([moon + 44., 95., 26., 26.], 0xe5cda9, 0.24),
    ] {
        quads.push(native_quad(rect, color, opacity, rect[2] * 0.5, scale));
    }
    for layer in 0..3 {
        let parallax = [0.12, 0.26, 0.43][layer];
        let spacing = [112., 98., 125.][layer];
        let base = [365., 440., 510.][layer];
        let color = [0x1b2034, 0x24263d, 0x2c2b43][layer];
        let first = (camera * parallax / spacing).floor() as usize;
        for i in first..first + (w / spacing) as usize + 3 {
            let x = i as f32 * spacing - camera * parallax;
            if x > w {
                break;
            }
            let tall = 55. + ((i * 73 + layer * 41) % 130) as f32;
            let mut building = native_quad(
                [x, base - tall, spacing - 9., tall + h - base],
                color,
                1.,
                3.,
                scale,
            );
            building.corner_radii.bottom_left = px(0.);
            building.corner_radii.bottom_right = px(0.);
            quads.push(building);
            quads.push(native_quad(
                [x + 13., base - tall - 9., spacing - 35., 10.],
                color,
                1.,
                0.,
                scale,
            ));
            if i % 3 == 0 {
                let mut tower =
                    native_quad([x + 35., base - tall - 30., 21., 24.], color, 1., 7., scale);
                tower.corner_radii.bottom_left = px(0.);
                tower.corner_radii.bottom_right = px(0.);
                quads.push(tower);
                quads.push(native_quad(
                    [x + 41., base - tall - 46., 2., 17.],
                    color,
                    1.,
                    0.,
                    scale,
                ));
            }
            for row in 0..(tall / 27.) as usize {
                for col in 0..3 {
                    if (i + row * 3 + col).is_multiple_of(4) {
                        continue;
                    }
                    quads.push(native_quad(
                        [
                            x + 15. + col as f32 * 24.,
                            base - tall + 17. + row as f32 * 27.,
                            6.,
                            10.,
                        ],
                        0xd9b49b,
                        if layer == 0 { 0.13 } else { 0.27 },
                        0.,
                        scale,
                    ));
                }
            }
        }
    }
    div().absolute().child(NativeArt { quads })
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

struct Resident {
    shoulder: Joint,
    hip: Joint,
    hands: [Joint; 2],
    feet: [Joint; 2],
    facing: f32,
    seated: bool,
}

fn resident(pose: Resident, ink: u32, scale: f32) -> gpui_kit::Div {
    let Resident {
        shoulder,
        hip,
        hands,
        feet,
        facing,
        seated,
    } = pose;
    let head = (shoulder.0 + facing * 0.6, shoulder.1 - 7.);
    // A whole resident shares one layout node instead of one node per limb.
    let mut quads = Vec::with_capacity(192);
    for (i, foot) in feet.into_iter().enumerate() {
        let pelvis = (hip.0 + (i as f32 - 0.5) * 2.5, hip.1);
        let knee = elbow(pelvis, foot, 10., if seated { -facing } else { facing });
        append_limb(&mut quads, pelvis, knee, (4.1, 3.), ink, scale);
        append_limb(&mut quads, knee, foot, (3., 1.9), ink, scale);
        append_limb(
            &mut quads,
            foot,
            (foot.0 + facing * 3., foot.1 + 0.3),
            (2.2, 2.5),
            ink,
            scale,
        );
    }
    append_limb(&mut quads, shoulder, hip, (7.5, 5.8), ink, scale);
    append_limb(
        &mut quads,
        (head.0, head.1 + 2.),
        shoulder,
        (2.6, 3.),
        ink,
        scale,
    );
    quads.push(native_quad(
        [head.0 - 2.5, head.1 - 3.3, 5., 6.8],
        ink,
        1.,
        2.3,
        scale,
    ));
    quads.push(native_quad(
        [head.0 + facing * 2. - 1., head.1 - 0.3, 2., 2.],
        ink,
        1.,
        1.,
        scale,
    ));
    let mut hair = native_quad([head.0 - 2.8, head.1 - 3.6, 5.4, 2.8], ink, 1., 2., scale);
    hair.corner_radii.bottom_left = px(0.);
    hair.corner_radii.bottom_right = px(0.);
    quads.push(hair);
    for (i, hand) in hands.into_iter().enumerate() {
        let socket = (shoulder.0 + (i as f32 - 0.5) * 3., shoulder.1 + 0.6);
        let joint = elbow(socket, hand, 7.5, if i == 0 { -1. } else { 1. });
        append_limb(&mut quads, socket, joint, (3.2, 2.3), ink, scale);
        append_limb(&mut quads, joint, hand, (2.3, 1.6), ink, scale);
        quads.push(native_quad(
            [hand.0 - 1.1, hand.1 - 1.1, 2.2, 2.2],
            ink,
            1.,
            1.1,
            scale,
        ));
    }
    div().absolute().child(NativeArt { quads })
}

fn window_frame(seed: usize, lit: bool, scale: f32) -> gpui_kit::Div {
    let frame = if lit { 0x695665 } else { 0x484557 };
    let mut window = div()
        .absolute()
        .w(px(66. * scale))
        .h(px(62. * scale))
        .rounded_t(px(3. * scale))
        .overflow_hidden()
        .border_2()
        .border_color(rgb(frame));
    if lit {
        // Open coverings leave the entire room visible, without glass overlays.
        return window;
    }
    let mut quads = Vec::with_capacity(36);
    let color = [0x434153, 0x494355, 0x3e4652][seed % 3];
    quads.push(native_quad([2., 2., 62., 58.], color, 1., 0., scale));
    if seed.is_multiple_of(2) {
        // Closed Venetian blinds, with alternating slat edges and lift cords.
        for slat in 0..12 {
            let y = 3. + slat as f32 * 4.5;
            quads.push(native_quad([3., y, 60., 1.], 0x9a8791, 0.24, 0., scale));
            quads.push(native_quad(
                [3., y + 3., 60., 1.],
                0x171c2b,
                0.36,
                0.,
                scale,
            ));
        }
        for x in [15., 50.] {
            quads.push(native_quad([x, 3., 0.8, 53.], 0xb0a0a3, 0.22, 0., scale));
        }
        quads.push(native_quad([3., 56., 60., 3.], 0x655a6d, 1., 0., scale));
    } else {
        // A pair of painted wood shutters, with inset louvres and hinges.
        for x in [4., 35.] {
            quads.push(native_quad([x, 5., 27., 50.], 0x242b3a, 0.45, 1., scale));
            for slat in 0..9 {
                let y = 7. + slat as f32 * 5.;
                quads.push(native_quad([x + 2., y, 23., 3.], color, 1., 0., scale));
                quads.push(native_quad(
                    [x + 2., y, 23., 0.7],
                    0xa18e98,
                    0.22,
                    0.,
                    scale,
                ));
            }
        }
        quads.push(native_quad([32., 2., 2., 58.], 0x242b3a, 0.7, 0., scale));
        for x in [3., 60.] {
            for y in [13., 45.] {
                quads.push(native_quad([x, y, 3., 4.], 0x222838, 1., 0.5, scale));
            }
        }
        quads.push(native_quad([30., 31., 6., 2.], 0x897582, 1., 0.5, scale));
    }
    window = window.child(NativeArt { quads });
    window
}

fn apartment(x: f32, y: f32, seed: usize, lit: bool, time: f32, scale: f32) -> impl IntoElement {
    let ink = 0x39303d;
    let light = [0xd4b58b, 0xc9b69a, 0xdac39b, 0xd1aa8c][seed % 4];
    let mut room = div()
        .absolute()
        .left(px(x * scale))
        .top(px(y * scale))
        .w(px(66. * scale))
        .h(px(64. * scale));
    if !lit {
        return room
            .child(block(0., 0., 66., 62., 0x141b2c, scale).rounded_t(px(3. * scale)))
            .child(window_frame(seed, false, scale))
            .child(block(-3., 62., 72., 3., 0x736174, scale));
    }
    let wave = (time * 1.5 + seed as f32).sin();
    let mut interior = div()
        .relative()
        .w(px(66. * scale))
        .h(px(62. * scale))
        .overflow_hidden()
        .rounded_t(px(3. * scale))
        .bg(rgb(light))
        .child(block(0., 56., 66., 6., 0x8e746e, scale))
        .child(block(0., 0., 66., 14., 0xffe0ac, scale).opacity(0.13));
    match seed % 6 {
        0 => {
            // A planted stance and a long handle make the vacuuming pose readable.
            let base = 46. + wave * 2.;
            let grip = (base - 14., 31.);
            interior = interior
                .child(limb((8., 56.), (base - 12., 56.), (0.8, 0.8), ink, scale))
                .child(limb(
                    (base - 12., 56.),
                    (base - 8., 51.),
                    (0.8, 0.8),
                    ink,
                    scale,
                ))
                .child(resident(
                    Resident {
                        shoulder: (24. + wave * 0.3, 24.),
                        hip: (22., 36.),
                        hands: [grip, (20., 37.)],
                        feet: [(18., 56.), (28., 56.)],
                        facing: 1.,
                        seated: false,
                    },
                    ink,
                    scale,
                ))
                .child(limb(
                    (base - 3., 52.),
                    (base - 10., 39.),
                    (5.5, 7.),
                    ink,
                    scale,
                ))
                .child(limb(
                    (base - 3., 52.),
                    (base - 16., 27.),
                    (1.7, 1.7),
                    ink,
                    scale,
                ))
                .child(limb(
                    (base - 16., 27.),
                    (base - 12., 27.),
                    (2.3, 2.3),
                    ink,
                    scale,
                ))
                .child(block(base - 7., 52., 17., 4., ink, scale).rounded(px(1.5 * scale)))
                .child(circle(base - 5., 54., 3., ink, scale))
                .child(circle(base + 6., 54., 3., ink, scale));
        }
        1 => {
            // Wrists move over the keyboard while the seated legs stay planted.
            interior = interior
                .child(block(37., 29., 25., 13., ink, scale).rounded_t(px(3. * scale)))
                .child(block(34., 41., 28., 3., ink, scale))
                .child(block(35., 41., 26., 1.5, light, scale))
                .child(block(39., 44., 2., 13., ink, scale))
                .child(block(59., 44., 2., 13., ink, scale))
                .child(block(14., 44., 17., 3., ink, scale))
                .child(block(17., 47., 2., 10., ink, scale))
                .child(resident(
                    Resident {
                        shoulder: (23., 29.),
                        hip: (22., 43.),
                        hands: [(33., 39. + wave * 0.3), (35., 40. - wave * 0.3)],
                        feet: [(32., 56.), (28., 56.)],
                        facing: 1.,
                        seated: true,
                    },
                    ink,
                    scale,
                ));
            for key in 0..7 {
                interior = interior.child(block(36. + key as f32 * 3.5, 41., 1., 1., ink, scale));
            }
        }
        2 => {
            let hand = (31. + wave * 0.35, 33. + wave * 0.25);
            interior = interior
                .child(resident(
                    Resident {
                        shoulder: (43., 23.),
                        hip: (44., 36.),
                        hands: [hand, (34., 35.)],
                        feet: [(40., 56.), (46., 56.)],
                        facing: -1.,
                        seated: false,
                    },
                    ink,
                    scale,
                ))
                .child(block(hand.0 - 7., hand.1, 8., 6., ink, scale).rounded(px(2. * scale)))
                .child(limb(
                    (hand.0 - 6., hand.1 + 3.),
                    (20., 37.),
                    (2., 1.2),
                    ink,
                    scale,
                ))
                .child(block(8., 48., 14., 9., ink, scale).rounded_b(px(3. * scale)))
                .child(limb((15., 50.), (15., 31.), (1.4, 1.1), ink, scale));
            for leaf in 0..4 {
                let y = 32. + leaf as f32 * 4.;
                interior = interior.child(limb(
                    (15., y + 3.),
                    (if leaf % 2 == 0 { 9. } else { 21. }, y),
                    (3.8, 1.),
                    ink,
                    scale,
                ));
            }
            for drop in 0..2 {
                interior = interior.child(circle(
                    20.,
                    39. + (time * 7. + drop as f32 * 4.).rem_euclid(7.),
                    1.,
                    ink,
                    scale,
                ));
            }
        }
        3 => {
            // The spoon traces a small ellipse; the elbow follows the wrist.
            let hand = (
                30. + wave * 0.8,
                33. + (time * 1.5 + seed as f32).cos() * 0.4,
            );
            interior = interior
                .child(block(7., 43., 26., 14., ink, scale))
                .child(block(11., 37., 15., 7., ink, scale).rounded_b(px(3. * scale)))
                .child(block(24., 38., 7., 1.5, ink, scale))
                .child(resident(
                    Resident {
                        shoulder: (42., 23.),
                        hip: (45., 36.),
                        hands: [hand, (35., 34.)],
                        feet: [(41., 56.), (48., 56.)],
                        facing: -1.,
                        seated: false,
                    },
                    ink,
                    scale,
                ))
                .child(limb(hand, (23. + wave * 0.8, 41.), (1., 1.), ink, scale))
                .child(block(10., 47., 20., 7., light, scale).opacity(0.2))
                .child(circle(11., 44., 1.5, light, scale))
                .child(circle(16., 44., 1.5, light, scale));
            for puff in 0..3 {
                let rise = (time * 5. + puff as f32 * 5.).rem_euclid(14.);
                interior = interior.child(
                    circle(14. + rise * 0.2, 30. - rise, 3., ink, scale)
                        .opacity((1. - rise / 14.) * 0.25),
                );
            }
        }
        4 => {
            // Weight shifts over planted feet; shoulders counter the hip sway.
            interior = interior
                .child(resident(
                    Resident {
                        shoulder: (33. - wave * 0.7, 22.),
                        hip: (33. + wave * 0.8, 36.),
                        hands: [(21., 20. + wave), (45., 17. - wave)],
                        feet: [(27., 55.), (40., 56.)],
                        facing: 1.,
                        seated: false,
                    },
                    ink,
                    scale,
                ))
                .child(block(3., 44., 15., 4., ink, scale))
                .child(block(5., 48., 2., 9., ink, scale))
                .child(block(14., 48., 2., 9., ink, scale))
                .child(limb((7., 41.), (15., 41.), (2., 2.), ink, scale))
                .child(limb((15., 39.), (12., 42.), (1., 1.), ink, scale));
        }
        _ => {
            // A forward lean brings the eye to the eyepiece; feet remain grounded.
            interior = interior
                .child(resident(
                    Resident {
                        shoulder: (29., 30.),
                        hip: (22., 37.),
                        hands: [(39., 33. + wave * 0.2), (36., 37.)],
                        feet: [(17., 56.), (28., 56.)],
                        facing: 1.,
                        seated: false,
                    },
                    ink,
                    scale,
                ))
                .child(limb((33., 23.), (55., 17.), (5., 7.), ink, scale))
                .child(limb((44., 22.), (44., 39.), (2., 2.), ink, scale))
                .child(limb((44., 38.), (36., 56.), (1.5, 1.), ink, scale))
                .child(limb((44., 38.), (53., 56.), (1.5, 1.), ink, scale))
                .child(limb((44., 38.), (44., 57.), (1.5, 1.), ink, scale));
        }
    }
    room = room.child(
        interior
            .child(block(0., 0., 4., 56., ink, scale).opacity(0.14))
            .child(block(62., 0., 4., 56., ink, scale).opacity(0.14))
            .child(window_frame(seed, true, scale)),
    );
    room.child(block(-3., 62., 72., 3., 0x736174, scale))
}

fn enemy_sprite(enemy: &Enemy, x: f32, y: f32, time: f32, scale: f32) -> impl IntoElement {
    let mut sprite = div()
        .absolute()
        .left(px((x - 17.) * scale))
        .top(px((y - 11.) * scale))
        .w(px(34. * scale))
        .h(px(24. * scale));
    let right = (time * enemy.speed + enemy.phase).cos() > 0.;
    match enemy.kind {
        EnemyKind::Rat => {
            let nose = if right { 26. } else { 1. };
            sprite = sprite
                .child(block(4., 7., 26., 16., 0x72516b, scale).rounded_full())
                .child(circle(
                    if right { 20. } else { 5. },
                    1.,
                    10.,
                    0x966882,
                    scale,
                ))
                .child(circle(nose, 10., 7., 0x9c708a, scale))
                .child(block(
                    if right { 23. } else { 7. },
                    10.,
                    3.,
                    3.,
                    0xffa27d,
                    scale,
                ))
                .child(
                    block(if right { -7. } else { 29. }, 15., 13., 3., 0xa0758d, scale)
                        .rounded_full(),
                );
        }
        EnemyKind::Owl => {
            let wing = (time * 9. + enemy.phase).sin() * 7.;
            sprite = sprite
                .child(block(-8., 8. + wing, 20., 8., 0x746080, scale).rounded_full())
                .child(block(23., 8. + wing, 20., 8., 0x746080, scale).rounded_full())
                .child(block(8., 2., 20., 23., 0x9b7e9e, scale).rounded(px(9. * scale)))
                .child(circle(10., 6., 7., 0xf4d4a8, scale))
                .child(circle(20., 6., 7., 0xf4d4a8, scale))
                .child(block(13., 8., 2., 3., 0x292031, scale))
                .child(block(22., 8., 2., 3., 0x292031, scale))
                .child(block(17., 14., 3., 4., 0xe6a47c, scale));
        }
    }
    sprite
}

fn silhouette(x: f32, y: f32, facing: f32, gait: f32, squash: f32, scale: f32) -> impl IntoElement {
    let head = if facing > 0. { 21. } else { 0. };
    let body = if facing > 0. { 3. } else { 12. };
    // In profile, the far ear peeks out beside the near ear at the back
    // of the head. Mirror both ears with the face when turning around.
    let near_ear = head + if facing > 0. { 2. } else { 13. };
    let far_ear = head + if facing > 0. { 5. } else { 11. };
    // Start inside the rounded haunch, and overlap every segment. The root
    // stays attached even at maximum landing compression and running gait.
    let tail = if facing > 0. { body + 2. } else { body + 21. };
    let compression = squash * 5.;
    let mut cat = div()
        .absolute()
        .left(px(x * scale))
        .top(px((y + compression) * scale))
        .w(px(43. * scale))
        .h(px(36. * scale))
        .child(
            block(
                body,
                15.,
                27. + compression,
                18. - compression,
                0x05070d,
                scale,
            )
            .rounded(px(9. * scale)),
        )
        .child(block(head, 7., 19., 20., 0x05070d, scale).rounded(px(7. * scale)))
        .child(block(far_ear, 4., 3., 8., 0x05070d, scale).rounded_t(px(1.5 * scale)))
        .child(block(near_ear, 1., 4., 11., 0x05070d, scale).rounded_t(px(2. * scale)))
        .child(block(near_ear + 1., 6., 2., 3., 0x71506e, scale))
        .child(
            block(
                head + if facing > 0. { 10. } else { 3. },
                15.,
                5.,
                3.,
                0xa8f0ce,
                scale,
            )
            .rounded_full(),
        )
        .child(
            block(body + 4., 29. - compression, 6., 6. + gait, 0x05070d, scale)
                .rounded_b(px(3. * scale)),
        )
        .child(
            block(
                body + 19.,
                29. - compression,
                6.,
                6. - gait,
                0x05070d,
                scale,
            )
            .rounded_b(px(3. * scale)),
        );
    for i in 0..5 {
        cat = cat.child(circle(
            tail - facing * i as f32 * 3.,
            22. - i as f32 * 2.8 + if i == 0 { 0. } else { gait * 0.25 },
            7.,
            0x05070d,
            scale,
        ));
    }
    cat
}

impl Render for Rooftops {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let viewport = window.viewport_size();
        let width = f32::from(viewport.width);
        let height = f32::from(viewport.height);
        let scale = (width / 900.).min(height / 560.).max(0.1);
        let w = width / scale;
        let h = height / scale;
        self.advance(window, cx, w);
        let run = &self.run;
        let drift = if cx.reduce_motion() { 0. } else { run.time };
        let camera = self.camera;
        let mut scene = div()
            .id("rooftops")
            .relative()
            .size_full()
            .overflow_hidden()
            .bg(rgb(SKY))
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::key_down))
            .on_key_up(cx.listener(Self::key_up))
            .on_mouse_down(
                gpui_kit::MouseButton::Left,
                cx.listener(|this, _, window, cx| window.focus(&this.focus, cx)),
            );

        if run.home && run.level + 1 == LEVELS {
            return scene
                .flex()
                .items_center()
                .justify_center()
                .child(self.results(cx, width, height));
        }

        scene = scene.child(backdrop(w, h, camera, drift, scale));

        for (i, roof) in run.roofs.iter().enumerate() {
            let x = roof.x - camera;
            if x + roof.width < -80. || x > w + 80. {
                continue;
            }
            let facade = if run.lit[i] { 0x353249 } else { 0x25283c };
            scene = scene
                .child(block(
                    x + 5.,
                    roof.y + 10.,
                    roof.width - 10.,
                    h - roof.y,
                    facade,
                    scale,
                ))
                .child(
                    block(x - 3., roof.y, roof.width + 6., 11., 0x606079, scale)
                        .rounded_t(px(3. * scale)),
                )
                .child(block(
                    x - 3.,
                    roof.y + 11.,
                    roof.width + 6.,
                    5.,
                    0x161b2b,
                    scale,
                ))
                .child(block(
                    x,
                    roof.y - 2.,
                    roof.width,
                    2.,
                    if run.lit[i] { 0xc1ac9d } else { 0x81819b },
                    scale,
                ));
            let columns = ((roof.width - 20.) / 74.).floor().max(1.) as usize;
            let spacing = (roof.width - 20.) / columns as f32;
            for row in 0..4 {
                let wy = roof.y + 32. + row as f32 * 82.;
                if wy > h {
                    break;
                }
                for col in 0..columns {
                    let wx = x + 10. + spacing * (col as f32 + 0.5) - 33.;
                    if wx + 72. < 0. || wx - 3. > w {
                        continue;
                    }
                    scene = scene.child(apartment(
                        wx,
                        wy,
                        i * 7 + row * 3 + col,
                        run.lit[i],
                        drift,
                        scale,
                    ));
                }
            }
            if i != run.roofs.len() - 1 {
                // Chimneys stay behind the cat and do not change the collision surface.
                scene = scene
                    .child(block(x + 17., roof.y - 32., 23., 32., 0x454052, scale))
                    .child(block(x + 14., roof.y - 34., 29., 5., 0x797080, scale));
                if i % 2 == 0 {
                    for puff in 0..3 {
                        let rise = (drift * 11. + puff as f32 * 18.).rem_euclid(58.);
                        scene = scene.child(
                            circle(
                                x + 18. + rise * 0.15,
                                roof.y - 45. - rise,
                                14. + rise * 0.23,
                                0xb5a8c1,
                                scale,
                            )
                            .opacity((1. - rise / 58.) * 0.075),
                        );
                    }
                }
            }
            if i.is_multiple_of(run.checkpoint_spacing()) {
                // Automatically lit lanterns mark safe return points.
                let lamp = x + roof.width - 24.;
                scene = scene
                    .child(block(lamp, roof.y - 43., 3., 43., 0x7b697b, scale))
                    .child(
                        block(
                            lamp - 6.,
                            roof.y - 46.,
                            15.,
                            16.,
                            if run.lit[i] { GOLD } else { 0x756477 },
                            scale,
                        )
                        .rounded(px(3. * scale)),
                    );
                if run.lit[i] {
                    scene = scene
                        .child(circle(lamp - 20., roof.y - 60., 43., GOLD, scale).opacity(0.055));
                }
            }
        }

        for coin in run.coins.iter().filter(|coin| !coin.collected) {
            let x = coin.x - camera;
            if !(-25. ..w + 25.).contains(&x) {
                continue;
            }
            scene = scene.child(cat_coin(x, coin.y, scale, &self.coin_art));
        }
        for enemy in run.enemies.iter().filter(|enemy| !enemy.defeated) {
            let (x, y) = enemy.position(&run.roofs, run.time);
            if !(-60. ..w + 60.).contains(&(x - camera)) {
                continue;
            }
            scene = scene.child(enemy_sprite(
                enemy,
                x - camera,
                y,
                if cx.reduce_motion() { 0. } else { run.time },
                scale,
            ));
        }

        // A warm doorway is the destination; it needs no explanatory label.
        let house = run.destination() - camera - 15.;
        if house + 88. >= 0. && house - 27. <= w {
            scene = scene
                .child(
                    block(house - 22., 274., 105., 86., 0x4b3b4c, scale).rounded_t(px(6. * scale)),
                )
                .child(block(house - 27., 268., 115., 8., 0x877181, scale).rounded(px(3. * scale)))
                .child(block(house + 16., 290., 29., 70., GOLD, scale).rounded_t(px(14. * scale)))
                .child(circle(house + 44., 317., 3., 0x80575f, scale))
                .child(
                    block(house + 59., 318., 14., 19., 0xd4ba98, scale).rounded_t(px(5. * scale)),
                )
                .child(block(house - 7., 342., 15., 18., 0x84616a, scale).rounded_b(px(4. * scale)))
                .child(circle(house - 13., 330., 26., 0x687d70, scale));
        }

        let airborne = run.ground.is_none();
        let gait = if airborne {
            1.5
        } else {
            (run.time * 26.).sin() * (run.vx.abs() / SPEED) * 2.5
        };
        if run.home {
            scene = scene
                .child(
                    block(run.x - camera - 11., run.y - 14., 37., 14., 0x05070d, scale)
                        .rounded_full(),
                )
                .child(circle(
                    run.x - camera + 13.,
                    run.y - 19.,
                    16.,
                    0x05070d,
                    scale,
                ))
                .child(block(
                    run.x - camera + 20.,
                    run.y - 20.,
                    4.,
                    7.,
                    0x05070d,
                    scale,
                ));
        } else {
            scene = scene
                .child(
                    circle(run.x - camera - 17., run.y - 3., 34., 0x070a12, scale)
                        .h(px(5. * scale))
                        .opacity(if airborne { 0.18 } else { 0.45 }),
                )
                .child(
                    div()
                        .absolute()
                        .inset_0()
                        .opacity(if run.invulnerable == 0. {
                            1.
                        } else if cx.reduce_motion() {
                            0.65
                        } else if ((run.time * 12.) as usize).is_multiple_of(2) {
                            0.5
                        } else {
                            1.
                        })
                        .child(silhouette(
                            run.x - camera - 20.,
                            run.y - 36.,
                            run.facing,
                            gait,
                            run.squash,
                            scale,
                        )),
                );
        }
        scene = scene.child(
            h_flex()
                .absolute()
                .top(px(18.))
                .left(px(18.))
                .gap_2()
                .px_3()
                .py_2()
                .rounded_full()
                .bg(rgb(SKY))
                .border_1()
                .border_color(rgb(0x343247))
                .child(div().relative().w(px(28.)).h(px(28.)).child(cat_coin(
                    14.,
                    14.,
                    1.,
                    &self.coin_art,
                )))
                .child(
                    div()
                        .text_sm()
                        .font_semibold()
                        .text_color(rgb(GOLD))
                        .child(run.coin_count().to_string()),
                ),
        );
        let mut progress = h_flex()
            .absolute()
            .top(px(18.))
            .left(px(width * 0.5 - 38.))
            .gap_3()
            .px_4()
            .h(px(38.))
            .items_center()
            .rounded_full()
            .bg(rgb(SKY))
            .border_1()
            .border_color(rgb(0x343247));
        for level in 0..LEVELS {
            progress = progress.child(
                div()
                    .w(px(6.))
                    .h(px(6.))
                    .rounded_full()
                    .bg(rgb(if level <= run.level { GOLD } else { 0x7b718e })),
            );
        }
        scene = scene.child(progress);
        if run.home {
            scene = scene.child(
                div()
                    .absolute()
                    .bottom(px(20.))
                    .w_full()
                    .flex()
                    .justify_center()
                    .child(control_prompt("Space for next level")),
            );
        }
        scene
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[gpui_kit::test]
    fn closing_rooftops_retains_the_window_and_resets_the_game(cx: &mut gpui_kit::TestAppContext) {
        let (view, window) = cx.add_window_view(Rooftops::new);
        window.update(|_, cx| {
            view.update(cx, |game, _| {
                game.run = Run::for_level(2);
                game.run.elapsed = 42.;
                game.run.banked_returns = 3;
                game.run.banked_kills = 5;
                game.run.input.right = true;
                game.run.buffer = 0.1;
                game.camera = 800.;
                game.save_message = Some("Saved previous score".into());
                game.saving = true;
                game.last_frame = Some(Instant::now());
                game.remainder = STEP / 2.;
                game.run.coins[0].collected = true;
            });
        });
        assert!(
            !window.simulate_close(),
            "Rooftops must hide instead of destroying its native window"
        );
        window.update(|_, cx| {
            view.update(cx, |game, _| {
                assert!(!game.run.input.right);
                assert_eq!(game.run.buffer, 0.);
                assert!(game.last_frame.is_none());
                assert_eq!(game.remainder, 0.);
                assert_eq!(game.run.level, 0);
                assert!(game.run.coins.iter().all(|coin| !coin.collected));
                assert_eq!(game.run.score().coins, 0);
                assert_eq!(game.run.score().deaths, 0);
                assert_eq!(game.run.score().kills, 0);
                assert_eq!(game.run.elapsed, 0.);
                assert_eq!(game.run.x, Run::new().x);
                assert_eq!(game.run.y, Run::new().y);
                assert_eq!(game.camera, 0.);
                assert!(game.save_message.is_none());
                assert!(!game.saving);
            });
        });
    }

    #[gpui_kit::test]
    fn play_time_excludes_pauses_and_completion(cx: &mut gpui_kit::TestAppContext) {
        let (view, cx) = cx.add_window_view(Rooftops::new);
        cx.update(|_, cx| {
            view.update(cx, |game, _| {
                let start = Instant::now();
                let after = |seconds| start + std::time::Duration::from_secs(seconds);
                game.active_frame(start, true);
                game.active_frame(after(2), true);
                game.active_frame(after(10), false);
                game.active_frame(after(20), true);
                assert_eq!(game.run.score().seconds, 2.);
                game.active_frame(after(23), true);
                assert_eq!(game.run.score().seconds, 5.);
                game.run.home = true;
                game.active_frame(after(40), true);
                assert_eq!(game.run.score().seconds, 5.);
            })
        });
    }

    #[test]
    fn results_accumulate_across_levels_and_restart_clears_them() {
        let mut run = Run::new();
        for level in 0..LEVELS {
            assert_eq!(run.level, level);
            run.coins[0].collected = true;
            run.enemies[0].defeated = true;
            run.respawn();
            run.record_active_time(12.5);
            run.home = true;
            run.record_active_time(30.);
            assert_eq!(run.score().coins, level + 1);
            assert_eq!(run.score().deaths, level + 1);
            assert_eq!(run.score().kills, level + 1);
            assert_eq!(run.score().seconds, (level + 1) as f64 * 12.5);
            run.jump_down();
        }
        assert_eq!(run.level, 0);
        assert_eq!(run.score().coins, 0);
        assert_eq!(run.score().deaths, 0);
        assert_eq!(run.score().kills, 0);
        assert_eq!(run.score().seconds, 0.);
    }

    #[test]
    fn building_sides_block_both_directions_without_stopping_the_fall() {
        for from_left in [true, false] {
            let mut run = Run::new();
            run.enemies.clear();
            let roof = run.roofs[1];
            let boundary = if from_left {
                roof.x - 11.
            } else {
                roof.x + roof.width + 11.
            };
            run.x = boundary + if from_left { -1. } else { 1. };
            run.y = roof.y + 40.;
            run.vx = if from_left { SPEED } else { -SPEED };
            run.vy = 30.;
            run.ground = None;
            run.input.right = from_left;
            run.input.left = !from_left;
            let y = run.y;
            run.step();
            assert_eq!(run.x, boundary);
            assert_eq!(run.vx, 0.);
            assert!(run.y > y && run.vy > 30.);
            assert!(run.ground.is_none());
            assert_eq!(run.coyote, 0.);
            run.jump_down();
            run.step();
            assert!(run.vy > 0., "a wall must not allow another jump");
            run.input.right = !from_left;
            run.input.left = from_left;
            run.step();
            assert!(
                if from_left {
                    run.x < boundary
                } else {
                    run.x > boundary
                },
                "the cat can steer away from the wall"
            );
        }
    }

    #[test]
    fn walls_cannot_be_tunneled_through_and_roof_corners_still_allow_landings() {
        let mut run = Run::new();
        run.enemies.clear();
        let roof = run.roofs[1];
        for from_left in [true, false] {
            run.x = if from_left {
                roof.x - 40.
            } else {
                roof.x + roof.width + 40.
            };
            run.y = roof.y + 40.;
            run.ground = None;
            run.vx = if from_left { 90000. } else { -90000. };
            run.step();
            assert_eq!(
                run.x,
                if from_left {
                    roof.x - 11.
                } else {
                    roof.x + roof.width + 11.
                }
            );
        }
        run.x = roof.x - 12.;
        run.y = roof.y - 1.;
        run.vx = SPEED;
        run.vy = 200.;
        run.input.right = true;
        run.step();
        assert_eq!(run.ground, Some(1));
        assert_eq!(run.y, roof.y);
        assert!(run.x > roof.x - 11.);
    }

    #[test]
    fn every_level_geometry_is_reachable_with_right_and_space() {
        for level in 0..LEVELS {
            let mut run = Run::for_level(level);
            // Validate jump geometry independently of enemy timing. Enemy damage
            // and stomping have separate tests below.
            run.enemies.clear();
            run.input.right = true;
            for _ in 0..120 * 40 {
                if let Some(i) = run.ground
                    && i + 1 < run.roofs.len()
                    && run.x
                        > run.roofs[i].x + run.roofs[i].width
                            - if level == 0 {
                                32.
                            } else if run.roofs[i + 1].y > run.roofs[i].y + 30. {
                                48.
                            } else {
                                16.
                            }
                {
                    run.input.jump = false;
                    run.jump_down();
                }
                run.step();
                if run.home {
                    break;
                }
            }
            assert!(
                run.home,
                "level {} cat stopped at {}, {}",
                level + 1,
                run.x,
                run.y
            );
            assert_eq!(run.returns, 0);
            assert!(run.lit.iter().all(|lit| *lit));
            let coins = run.coin_count();
            run.jump_down();
            assert!(!run.home);
            assert_eq!(run.x, 105.);
            assert_eq!(run.level, (level + 1) % LEVELS);
            assert_eq!(run.coin_count(), if run.level == 0 { 0 } else { coins });
            assert!(run.coins.iter().all(|coin| !coin.collected));
        }
    }

    #[test]
    fn short_hops_coyote_time_and_buffered_jumps_work() {
        let mut run = Run::new();
        run.jump_down();
        run.step();
        let full = run.vy;
        run.jump_up();
        assert!(run.vy > full && run.vy < 0.);
        run.ground = None;
        run.coyote = 0.06;
        run.vy = 20.;
        run.jump_down();
        run.step();
        assert!(run.vy < -500.);
        run.jump_up();
        run.x = 105.;
        run.y = 429.;
        run.vy = 40.;
        run.ground = None;
        run.coyote = 0.;
        run.jump_down();
        for _ in 0..6 {
            run.step();
        }
        assert!(
            run.vy < 0.,
            "a jump pressed just before landing should be buffered"
        );
    }

    #[test]
    fn falls_return_to_the_last_lantern_and_keep_coins() {
        let mut run = Run::new();
        run.checkpoint = 4;
        run.coins[1].collected = true;
        run.y = 800.;
        run.ground = None;
        run.step();
        assert_eq!(run.ground, Some(4));
        assert_eq!(run.y, ROOFS[4].y);
        assert_eq!(run.returns, 1);
        assert!(run.coins[1].collected);
    }

    #[test]
    fn coins_require_a_jump_and_actual_contact() {
        let mut run = Run::new();
        run.enemies.clear();
        run.x = run.coins[0].x;
        run.step();
        assert!(
            !run.coins[0].collected,
            "walking underneath must not collect a coin"
        );
        run.ground = None;
        run.y = run.coins[0].y + 20.;
        run.step();
        assert!(run.coins[0].collected);
    }

    #[test]
    fn enemies_hurt_from_the_side_and_can_be_stomped() {
        for kind in [EnemyKind::Rat, EnemyKind::Owl] {
            let mut run = Run::new();
            run.enemies = vec![Enemy {
                roof: 3,
                kind,
                defeated: false,
                phase: 0.,
                speed: 0.,
            }];
            let (x, y) = run.enemies[0].position(&run.roofs, STEP);
            run.x = x;
            run.y = y + 11.;
            run.ground = None;
            run.coyote = 0.;
            run.step();
            assert_eq!(run.returns, 1);
            assert!(run.invulnerable > 0.);
            run.invulnerable = 0.;
            let (x, y) = run.enemies[0].position(&run.roofs, run.time + STEP);
            run.x = x;
            run.y = y - 12.;
            run.vy = 200.;
            run.ground = None;
            run.coyote = 0.;
            run.step();
            assert!(run.enemies[0].defeated);
            assert!(run.vy < 0.);
            assert_eq!(run.returns, 1);
        }
    }

    #[test]
    fn returning_to_a_lantern_grants_a_safe_recovery_window() {
        let mut run = Run::new();
        run.enemies = vec![Enemy {
            roof: 0,
            kind: EnemyKind::Rat,
            defeated: false,
            phase: 0.,
            speed: 0.,
        }];
        run.respawn();
        for _ in 0..60 {
            run.step();
        }
        assert_eq!(run.returns, 1);
        assert!(run.invulnerable > 0.);
        assert!(!run.enemies[0].defeated);
    }

    #[test]
    fn score_button_label_has_readable_contrast_in_every_state() {
        fn luminance(color: u32) -> f32 {
            fn channel(value: u32) -> f32 {
                let value = value as f32 / 255.;
                if value <= 0.04045 {
                    value / 12.92
                } else {
                    ((value + 0.055) / 1.055).powf(2.4)
                }
            }
            0.2126 * channel(color >> 16)
                + 0.7152 * channel((color >> 8) & 0xff)
                + 0.0722 * channel(color & 0xff)
        }

        for background in [SCORE_BUTTON_NORMAL, SCORE_BUTTON_HOVER, SCORE_BUTTON_ACTIVE] {
            let contrast = (luminance(background) + 0.05) / (luminance(SKY) + 0.05);
            assert!(contrast >= 4.5, "contrast ratio was {contrast:.2}");
        }
    }

    #[gpui_kit::test]
    fn arrow_press_and_release_and_space_reach_the_view(cx: &mut gpui_kit::TestAppContext) {
        cx.update(gpui_kit::init);
        let (view, cx) = cx.add_window_view(|window, cx| {
            let game = Rooftops::new(window, cx);
            window.focus(&game.focus, cx);
            game
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.simulate_keystrokes("right");
        cx.update(|_, cx| assert!(view.read(cx).run.input.right));
        cx.simulate_event(KeyUpEvent {
            keystroke: gpui_kit::Keystroke::parse("right").unwrap(),
        });
        cx.update(|_, cx| {
            assert!(
                !view.read(cx).run.input.right,
                "key release must clear movement"
            )
        });
        cx.simulate_keystrokes("space");
        cx.update(|_, cx| {
            view.update(cx, |game, _| {
                game.run.step();
                assert!(game.run.vy < 0.);
            })
        });
    }
}

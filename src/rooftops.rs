use std::{
    sync::{Arc, LazyLock},
    time::Instant,
};

mod arrival;
pub(crate) mod audio;
mod neighborhood;
mod options;

use gpui_kit::component::slider::{SliderEvent, SliderState};
use neighborhood::Neighborhood;

use crate::rooftops_score::Score;
use crate::rooftops_score::art::{self, StatIcon};
use crate::rooftops_score::replay::{Frame, GameData, Pose, Preferences, Recording, Replay};
use gpui_kit::component::{
    StyledExt as _,
    button::{Button, ButtonCustomVariant, ButtonVariants as _},
    h_flex, v_flex,
};

use gpui_kit::{
    App, AppContext as _, Context, EventEmitter, FocusHandle, Focusable, Image, ImageFormat,
    InteractiveElement as _, IntoElement, KeyDownEvent, KeyUpEvent, ParentElement as _, Render,
    Styled as _, Subscription, Window, div, img, px, rgb,
};

const STEP: f32 = 1. / 120.;
const SPEED: f32 = 310.;
const GRAVITY: f32 = 1450.;
const JUMP: f32 = 535.;
const SKY: u32 = 0x101322;
const GOLD: u32 = 0xffdba0;
const START_X: f32 = 105.;
const DESTINATION_INSET: f32 = 140.;
const REST_ROOF_WIDTH: f32 = 400.;
// A 27-unit cat body is about 0.42 m; most rooftop gaps are about 2 m.
const WORLD_UNITS_PER_METER: f32 = 64.;
const PLAYER_AHEAD_GREEN: u32 = 0x3ddc97;
const PLAYER_BEHIND_RED: u32 = 0xff6b81;
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
        width: REST_ROOF_WIDTH,
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
    let count = ROOFS.len() + level * 2;
    let mut roofs = vec![Roof {
        // The previous destination, translated so its doorway is at START_X.
        x: START_X - (REST_ROOF_WIDTH - DESTINATION_INSET),
        y: 360.,
        width: REST_ROOF_WIDTH,
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
            REST_ROOF_WIDTH
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

fn course_progress(level: usize, x: f32) -> f32 {
    static OFFSETS: LazyLock<[f32; LEVELS + 1]> = LazyLock::new(|| {
        let mut offsets = [0.; LEVELS + 1];
        for level in 0..LEVELS {
            let roofs = level_roofs(level);
            let last = roofs.last().expect("every level has a destination");
            offsets[level + 1] = offsets[level] + last.x + last.width - DESTINATION_INSET - START_X;
        }
        offsets
    });
    OFFSETS[level] + (x - START_X).clamp(0., OFFSETS[level + 1] - OFFSETS[level])
}

fn ghost_in_view(ghost: Frame, level: usize, camera: f32, width: f32) -> bool {
    if ghost.level != level {
        return false;
    }
    let (left, right) = if ghost.pose.home {
        (-11., 29.)
    } else if ghost.pose.facing > 0. {
        (-27., 20.)
    } else {
        (-20., 32.)
    };
    ghost.pose.x + right >= camera && ghost.pose.x + left <= camera + width
}

struct GhostGap {
    ahead: bool,
    meters: f32,
}

fn ghost_gap(player: &Run, ghost: Frame, camera: f32, width: f32) -> Option<GhostGap> {
    if ghost_in_view(ghost, player.level, camera, width) {
        return None;
    }
    let delta =
        course_progress(player.level, player.x) - course_progress(ghost.level, ghost.pose.x);
    Some(GhostGap {
        ahead: if ghost.level != player.level {
            player.level > ghost.level
        } else {
            delta > 0.
        },
        meters: delta.abs() / WORLD_UNITS_PER_METER,
    })
}

fn ghost_gap_indicator(gap: GhostGap, height: f32) -> gpui_kit::Div {
    let color = if gap.ahead {
        PLAYER_AHEAD_GREEN
    } else {
        PLAYER_BEHIND_RED
    };
    let mut marker = v_flex()
        .absolute()
        .top(px(height * 0.43))
        .gap(px(2.))
        .font_family(crate::rooftops_score::POSTER_FONT)
        .text_color(rgb(color))
        .child(
            div()
                .text_size(px(18.))
                .line_height(px(22.))
                .font_semibold()
                .child(if gap.ahead {
                    format!("← {:.1} m", gap.meters)
                } else {
                    format!("{:.1} m →", gap.meters)
                }),
        )
        .child(
            div()
                .text_size(px(10.))
                .line_height(px(14.))
                .font_semibold()
                .child(if gap.ahead { "AHEAD" } else { "BEHIND" }),
        );
    marker = if gap.ahead {
        marker.left(px(20.)).items_start()
    } else {
        marker.right(px(20.)).items_end()
    };
    marker
}

#[derive(Clone)]
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

#[derive(Clone)]
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

#[derive(Clone, Default)]
struct Input {
    left: bool,
    right: bool,
    jump: bool,
}

#[derive(Clone)]
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

    fn for_arrival() -> Self {
        let mut run = Self::new();
        run.lit[0] = false;
        run
    }

    fn for_level(level: usize) -> Self {
        let roofs = level_roofs(level);
        let y = roofs[0].y;
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
            x: START_X,
            y,
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
            let time = self.time;
            let doorway_coin_collected = self.coins.last().is_some_and(|coin| coin.collected);
            *self = Self::for_level(next);
            if next != 0 {
                self.banked_coins = score.coins;
                self.banked_returns = score.deaths;
                self.banked_kills = score.kills;
                self.elapsed = score.seconds;
                self.time = time;
                if doorway_coin_collected {
                    self.coins.remove(0);
                }
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
            self.x = self.destination();
            self.home = true;
            self.vx = 0.;
            self.vy = 0.;
            self.squash = 0.;
            self.facing = 1.;
            self.buffer = 0.;
            self.input = Input::default();
        }
    }

    fn score(&self) -> Score {
        Score {
            coins: self.coin_count(),
            deaths: self.banked_returns + self.returns,
            kills: self.banked_kills + self.enemies.iter().filter(|enemy| enemy.defeated).count(),
            seconds: self.elapsed,
            personal_best: None,
        }
    }

    fn record_active_time(&mut self, elapsed: f64) {
        if !self.home {
            self.elapsed += elapsed;
        }
    }

    fn ghost_frame(&self) -> Frame {
        Frame {
            seconds: self.elapsed,
            level: self.level,
            returns: self.banked_returns + self.returns,
            pose: Pose {
                x: self.x,
                y: self.y,
                facing: self.facing,
                squash: self.squash,
                home: self.home,
                gait: if self.ground.is_none() {
                    1.5
                } else {
                    (self.time * 26.).sin() * (self.vx.abs() / SPEED) * 2.5
                },
            },
        }
    }

    fn checkpoint_spacing(&self) -> usize {
        if self.level == 0 { 4 } else { 5 }
    }

    fn roof_identity(&self, index: usize) -> (usize, usize) {
        if index == 0 && self.level > 0 {
            (self.level - 1, ROOFS.len() + (self.level - 1) * 2 - 1)
        } else {
            (self.level, index)
        }
    }

    fn has_lantern(&self, index: usize) -> bool {
        index > 0 && index + 1 < self.roofs.len() && index.is_multiple_of(self.checkpoint_spacing())
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
        last.x + last.width - DESTINATION_INSET
    }

    fn respawn(&mut self) {
        let roof = self.roofs[self.checkpoint];
        self.x = if self.checkpoint == 0 {
            START_X
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

/// Both courses share one physical roof during the pan. Only scenery time runs.
struct LevelSlide {
    previous: Run,
    time: f32,
    camera_start: f32,
}

impl LevelSlide {
    const PAN: f32 = 1.15;
    const DURATION: f32 = Self::PAN + 0.3;

    fn offset(&self) -> f32 {
        START_X - self.previous.destination()
    }

    fn camera(&self) -> f32 {
        let t = (self.time / Self::PAN).clamp(0., 1.);
        self.camera_start * (1. - t * t * (3. - 2. * t))
    }

    fn sleeping(&self) -> bool {
        self.time < Self::PAN
    }
}

pub struct Rooftops {
    run: Run,
    arrival: Option<f32>,
    slide: Option<LevelSlide>,
    started: bool,
    options_open: bool,
    music_preview: bool,
    effects_adjustment: Option<options::EffectsAdjustment>,
    preferences: Preferences,
    preferences_message: Option<String>,
    preferences_dirty: bool,
    music_slider: gpui_kit::Entity<SliderState>,
    effects_slider: gpui_kit::Entity<SliderState>,
    volume_focus: [FocusHandle; 2],
    _volume_subscriptions: [Subscription; 2],
    audio: audio::Audio,
    facade_art: neighborhood::FacadeArt,
    poster_art: Option<(f32, Arc<[gpui_kit::PaintQuad]>)>,
    focus: FocusHandle,
    options_focus: FocusHandle,
    option_controls_focus: [FocusHandle; 2],
    last_frame: Option<Instant>,
    remainder: f32,
    camera: f32,
    coin_art: Arc<Image>,
    personal_best: Option<f64>,
    run_personal_best: Option<f64>,
    best_replay: Option<Arc<Replay>>,
    ghost_replay: Option<Arc<Replay>>,
    recording: Recording,
    best_path: Option<std::path::PathBuf>,
    completion_recorded: bool,
    save_message: Option<String>,
    saving: bool,
    #[cfg(feature = "walkthrough-screenshots")]
    preview: bool,
    _activation: Subscription,
    _blur: Subscription,
}

pub(crate) struct Completed;
impl EventEmitter<Completed> for Rooftops {}

impl Rooftops {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self::new_with_records(window, cx, !cfg!(test))
    }

    fn new_with_records(window: &mut Window, cx: &mut Context<Self>, remember: bool) -> Self {
        let focus = cx.focus_handle();
        let activation = cx.observe_window_activation(window, |this, _, cx| {
            this.run.input = Input::default();
            this.run.buffer = 0.;
            this.last_frame = None;
            this.remainder = 0.;
            this.music_preview = false;
            this.effects_adjustment = None;
            this.audio.suspend();
            cx.notify();
        });
        let blur = cx.on_blur(&focus, window, |this, _, cx| {
            this.run.input = Input::default();
            this.run.buffer = 0.;
            this.last_frame = None;
            this.remainder = 0.;
            this.music_preview = false;
            this.effects_adjustment = None;
            this.audio.suspend();
            cx.notify();
        });
        let game = cx.entity().downgrade();
        window.on_window_should_close(cx, move |window, cx| {
            let _ = game.update(cx, |game, cx| {
                game.run = Run::for_arrival();
                game.arrival = Some(0.);
                game.slide = None;
                game.started = false;
                game.set_options_open(false);
                window.focus(&game.focus, cx);
                game.audio.stop();
                game.run_personal_best = game.personal_best;
                game.ghost_replay = game.best_replay.clone();
                game.recording = Recording::new(game.run.ghost_frame());
                game.last_frame = None;
                game.remainder = 0.;
                game.camera = 0.;
                game.save_message = None;
                game.completion_recorded = false;
                #[cfg(feature = "walkthrough-screenshots")]
                {
                    game.preview = false;
                }
                cx.notify();
            });
            crate::window_limits::close(window, cx)
        });
        let best_path = remember.then(|| GameData::path().ok()).flatten();
        let stored = best_path
            .as_deref()
            .map(|path| {
                GameData::read(path).unwrap_or_else(|error| {
                    eprintln!("Could not load Rooftops data: {error}");
                    GameData::default()
                })
            })
            .unwrap_or_default();
        let personal_best = stored.personal_best;
        let preferences = stored.preferences;
        let music_slider =
            cx.new(|_| SliderState::new().default_value(f32::from(preferences.music_volume)));
        let effects_slider =
            cx.new(|_| SliderState::new().default_value(f32::from(preferences.effects_volume)));
        let music_subscription = cx.subscribe_in(
            &music_slider,
            window,
            |this, _, event: &SliderEvent, window, cx| {
                this.volume_changed(options::Volume::Music, event, window, cx)
            },
        );
        let effects_subscription = cx.subscribe_in(
            &effects_slider,
            window,
            |this, _, event: &SliderEvent, window, cx| {
                this.volume_changed(options::Volume::Effects, event, window, cx)
            },
        );
        let best_replay = stored.replay.map(Arc::new);
        let run = Run::for_arrival();
        let recording = Recording::new(run.ghost_frame());
        let mut audio = audio::Audio::new(remember);
        if let Some(events) = audio.take_events() {
            cx.spawn(async move |this, cx| {
                while events.recv().await.is_ok() {
                    if this.update(cx, |_, cx| cx.notify()).is_err() {
                        break;
                    }
                }
            })
            .detach();
        }
        Self {
            run,
            arrival: Some(0.),
            slide: None,
            started: false,
            options_open: false,
            music_preview: false,
            effects_adjustment: None,
            preferences,
            preferences_message: None,
            preferences_dirty: false,
            music_slider,
            effects_slider,
            volume_focus: std::array::from_fn(|_| cx.focus_handle().tab_stop(true)),
            _volume_subscriptions: [music_subscription, effects_subscription],
            audio,
            facade_art: neighborhood::FacadeArt::default(),
            poster_art: None,
            focus,
            options_focus: cx.focus_handle(),
            option_controls_focus: std::array::from_fn(|_| cx.focus_handle()),
            last_frame: None,
            remainder: 0.,
            camera: 0.,
            coin_art: kitten_coin_art(),
            personal_best,
            run_personal_best: personal_best,
            ghost_replay: best_replay.clone(),
            best_replay,
            recording,
            best_path,
            completion_recorded: false,
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
    pub(crate) fn for_preview(
        window: &mut Window,
        cx: &mut Context<Self>,
        level: usize,
        show_results: bool,
    ) -> Self {
        let mut game = Self::new_with_records(window, cx, false);
        game.preview = true;
        game.arrival = None;
        game.started = true;
        game.run = Run::for_level(level);
        let run = &mut game.run;
        match level {
            0 | 2 if !show_results => {
                // An early leap: first coins and lit apartments.
                run.x = run.roofs[2].x + run.roofs[2].width * 0.4;
                run.y = run.roofs[2].y - 76.;
                run.vy = -130.;
                run.ground = None;
                run.vx = SPEED;
                run.time = 3.2;
                run.elapsed = 3.2;
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
                run.elapsed = 58.4;
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
                game.personal_best = Some(138.2);
                game.run_personal_best = game.personal_best;
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
        if level == 0 {
            game.set_preview_ghost(game.run.x + 98.);
        }
        game
    }

    #[cfg(feature = "walkthrough-screenshots")]
    #[allow(dead_code)]
    pub(crate) fn for_arrival_preview(
        window: &mut Window,
        cx: &mut Context<Self>,
        time: f32,
    ) -> Self {
        let mut game = Self::new_with_records(window, cx, false);
        game.preview = true;
        game.arrival = (time < arrival::DURATION).then_some(time);
        game.run.lit[0] = time >= arrival::LANDING;
        game
    }

    #[cfg(feature = "walkthrough-screenshots")]
    #[allow(dead_code)]
    pub(crate) fn for_checkpoint_preview(
        window: &mut Window,
        cx: &mut Context<Self>,
        level: usize,
        slide_time: Option<f32>,
    ) -> Self {
        let mut game = Self::new_with_records(window, cx, false);
        game.preview = true;
        game.arrival = None;
        game.started = true;
        game.run = Run::for_level(level);
        let last = game.run.roofs.len() - 1;
        game.run.x = game.run.destination();
        game.run.y = game.run.roofs[last].y;
        game.run.ground = Some(last);
        game.run.lit.fill(true);
        game.run.time = 8.;
        game.run.elapsed = 42. + level as f64 * 49.;
        game.run.home = true;
        game.camera = game.run.roofs[last].x + game.run.roofs[last].width - 900.;
        if level + 1 == LEVELS {
            // Show the grand home just before arrival; results have their own capture.
            game.run.home = false;
            game.run.x -= 42.;
        } else if let Some(time) = slide_time {
            game.continue_from_home();
            let slide = game.slide.as_mut().unwrap();
            slide.time = time;
            game.camera = slide.camera();
            if time >= LevelSlide::DURATION {
                game.finish_slide();
            }
        }
        game
    }

    #[cfg(feature = "walkthrough-screenshots")]
    #[allow(dead_code)]
    pub(crate) fn for_options_preview(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut game = Self::for_preview(window, cx, 0, false);
        game.set_options_open(true);
        game
    }

    #[cfg(feature = "walkthrough-screenshots")]
    #[allow(dead_code)]
    pub(crate) fn for_options_endpoint_preview(
        window: &mut Window,
        cx: &mut Context<Self>,
        value: u8,
    ) -> Self {
        let mut game = Self::for_options_preview(window, cx);
        game.preferences.music_volume = value;
        game.preferences.effects_volume = value;
        game.music_slider.update(cx, |state, cx| {
            state.set_value(f32::from(value), window, cx)
        });
        game.effects_slider.update(cx, |state, cx| {
            state.set_value(f32::from(value), window, cx)
        });
        window.focus(&game.volume_focus[0], cx);
        game
    }

    #[cfg(feature = "walkthrough-screenshots")]
    #[allow(dead_code)]
    pub(crate) fn for_gap_preview(
        window: &mut Window,
        cx: &mut Context<Self>,
        ahead: bool,
    ) -> Self {
        let mut game = Self::for_preview(window, cx, 0, false);
        if ahead {
            game.run.x = 2000.;
            game.camera = 1700.;
        }
        game.set_preview_ghost(if ahead {
            game.camera - 84.
        } else {
            game.camera + 1024.
        });
        game
    }

    #[cfg(feature = "walkthrough-screenshots")]
    fn set_preview_ghost(&mut self, x: f32) {
        // Fixed examples only: never load or write the player's replay for captures.
        let mut demo = Recording::new(Run::new().ghost_frame());
        let mut frame = self.run.ghost_frame();
        frame.pose.x = x;
        frame.pose.y -= 16.;
        demo.capture(frame);
        for level in 0..LEVELS {
            let mut example = Run::for_level(level);
            if level > 0 {
                example.elapsed = level as f64 * 40.;
                demo.capture(example.ghost_frame());
            }
            example.elapsed = (level + 1) as f64 * 40.;
            example.x = example.destination();
            example.home = true;
            demo.capture(example.ghost_frame());
            frame = example.ghost_frame();
        }
        self.ghost_replay = demo.finish(frame).map(Arc::new);
    }

    #[cfg(feature = "walkthrough-screenshots")]
    #[allow(dead_code)]
    pub(crate) fn audit_frame(&mut self, frame: usize, cx: &mut Context<Self>) {
        self.run.time = frame as f32 / 60.;
        cx.notify();
    }

    fn save_score(&mut self, _: &gpui_kit::ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.saving || !self.run.home || self.run.level + 1 != LEVELS {
            return;
        }
        self.saving = true;
        self.save_message = None;
        let score = self.score();
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

    fn results(&mut self, cx: &mut Context<Self>, width: f32, height: f32) -> gpui_kit::Div {
        let scale = ((width - 48.) / 900.).min((height - 100.) / 540.).max(0.1);
        if self
            .poster_art
            .as_ref()
            .is_none_or(|(old_scale, _)| *old_scale != scale)
        {
            self.poster_art = Some((
                scale,
                crate::rooftops_score::poster_shapes()
                    .into_iter()
                    .skip(1)
                    .map(|shape| {
                        native_quad(shape.rect, shape.ink, shape.opacity, shape.radius, scale)
                    })
                    .collect::<Vec<_>>()
                    .into(),
            ));
        }
        let quads = self.poster_art.as_ref().unwrap().1.clone();
        static MOON: LazyLock<Arc<[art::ArtPath]>> =
            LazyLock::new(|| art::full_moon_paths(654., 130., 122.).into());
        static DETAILS: LazyLock<Arc<[art::ArtPath]>> =
            LazyLock::new(|| crate::rooftops_score::poster_static_paths().into());
        let mut poster = div()
            .relative()
            .w(px(900. * scale))
            .h(px(540. * scale))
            .flex_shrink_0()
            .overflow_hidden()
            .rounded(px(8. * scale))
            .bg(rgb(0x171a2d))
            .child(vector_art(MOON.clone(), scale))
            .child(div().absolute().child(NativeArt { quads }))
            .child(vector_art(DETAILS.clone(), scale))
            .child(vector_art(
                art::home_sleep_paths(
                    740.,
                    337.,
                    if self.reduced_motion(cx) {
                        1.2
                    } else {
                        self.run.time
                    },
                    1.1,
                ),
                scale,
            ));
        for item in crate::rooftops_score::poster_text(self.score()) {
            let label = div()
                .absolute()
                .left(px(item.x * scale))
                .top(px(item.y * scale))
                .w(px(item.centered_width.unwrap_or(864. - item.x) * scale))
                .whitespace_nowrap()
                // GPUI's system-font raster bounds clip large glyphs on macOS.
                .font_family(crate::rooftops_score::POSTER_FONT)
                .text_size(px(item.size * scale))
                .line_height(px(item.size * 1.2 * scale))
                .text_color(rgb(item.ink))
                .child(item.label);
            let label = if item.centered_width.is_some() {
                label.text_center()
            } else {
                label
            };
            poster = poster.child(if item.bold { label.font_bold() } else { label });
        }
        let mut card = v_flex().gap_3().items_center().child(poster).child(
            h_flex()
                .w(px(860. * scale))
                .justify_between()
                .items_center()
                .child(
                    h_flex()
                        .gap_3()
                        .child(
                            score_button("play-rooftops-again", "Play again", cx)
                                .debug_selector(|| "play-rooftops-again".into())
                                .on_click(cx.listener(Self::play_again)),
                        )
                        .child(
                            score_button("exit-rooftops", "Exit", cx)
                                .debug_selector(|| "exit-rooftops".into())
                                .on_click(cx.listener(Self::exit_game)),
                        ),
                )
                .child(
                    score_button("save-rooftops-score", "Download Scorecard", cx)
                        .debug_selector(|| "download-rooftops-scorecard".into())
                        .loading(self.saving)
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

    fn continue_from_home(&mut self) {
        self.finish_arrival();
        let restart = self.run.level + 1 == LEVELS;
        let previous = (!restart).then(|| self.run.clone());
        let previous_camera = self.camera;
        if restart {
            self.started = false;
            self.audio.stop();
            self.run_personal_best = self.personal_best;
            self.ghost_replay = self.best_replay.clone();
        }
        self.last_frame = None;
        self.remainder = 0.;
        self.save_message = None;
        self.completion_recorded = false;
        self.camera = 0.;
        self.run.jump_down();
        if let Some(previous) = previous {
            let offset = START_X - previous.destination();
            self.slide = Some(LevelSlide {
                previous,
                time: 0.,
                camera_start: previous_camera + offset,
            });
            self.camera = self.slide.as_ref().unwrap().camera();
        } else {
            self.slide = None;
        }
        if restart {
            self.recording = Recording::new(self.run.ghost_frame());
        } else {
            self.recording.capture(self.run.ghost_frame());
        }
    }

    pub(crate) fn reset_progress(&mut self, cx: &mut Context<Self>) {
        self.run = Run::for_arrival();
        self.arrival = Some(0.);
        self.slide = None;
        self.started = false;
        self.options_open = false;
        self.preferences = Preferences::default();
        self.preferences_dirty = false;
        self.preferences_message = None;
        self.audio.stop();
        self.personal_best = None;
        self.run_personal_best = None;
        self.best_replay = None;
        self.ghost_replay = None;
        self.recording = Recording::new(self.run.ghost_frame());
        self.last_frame = None;
        self.remainder = 0.;
        self.camera = 0.;
        self.completion_recorded = false;
        self.save_message = None;
        #[cfg(feature = "walkthrough-screenshots")]
        {
            self.preview = false;
        }
        cx.notify();
    }

    pub(crate) fn has_completed(&self) -> bool {
        self.personal_best.is_some()
    }

    fn play_again(
        &mut self,
        _: &gpui_kit::ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.run.home && self.run.level + 1 == LEVELS {
            self.continue_from_home();
            window.focus(&self.focus, cx);
            cx.notify();
        }
    }

    fn exit_game(&mut self, _: &gpui_kit::ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        // Use the normal close request so the run resets through the window's
        // cleanup callback and retains the native macOS teardown workaround.
        window.defer(cx, |window, cx| {
            crate::window_limits::request_close(window, cx)
        });
    }

    fn key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.options_open || !self.focus.is_focused(window) {
            return;
        }
        if event.keystroke.modifiers.platform
            || event.keystroke.modifiers.control
            || event.keystroke.modifiers.alt
        {
            return;
        }
        match event.keystroke.key.as_str() {
            "left" => {
                self.begin_play(Instant::now());
                self.run.input.left = true;
            }
            "right" => {
                self.begin_play(Instant::now());
                self.run.input.right = true;
            }
            "space" if !event.is_held => {
                if self.run.home {
                    self.continue_from_home();
                } else {
                    self.begin_play(Instant::now());
                    self.run.jump_down();
                }
            }
            _ => return,
        }
        cx.stop_propagation();
        cx.notify();
    }

    fn key_up(&mut self, event: &KeyUpEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.options_open || !self.focus.is_focused(window) {
            return;
        }
        match event.keystroke.key.as_str() {
            "left" => self.run.input.left = false,
            "right" => self.run.input.right = false,
            "space" => self.run.jump_up(),
            _ => return,
        }
        cx.stop_propagation();
        cx.notify();
    }

    fn finish_arrival(&mut self) {
        if self.arrival.take().is_some() {
            self.run.lit[0] = true;
            self.last_frame = None;
            self.remainder = 0.;
        }
    }

    fn begin_play(&mut self, now: Instant) {
        if self.run.home {
            return;
        }
        self.finish_arrival();
        self.finish_slide();
        if !self.started {
            self.started = true;
            self.last_frame = Some(now);
            self.remainder = 0.;
        }
    }

    fn active_frame(&mut self, now: Instant, active: bool) -> Option<f32> {
        if !active || self.options_open {
            self.last_frame = None;
            return None;
        }
        let elapsed = self
            .last_frame
            .replace(now)
            .map_or(0., |last| now.duration_since(last).as_secs_f64());
        if let Some(slide) = &mut self.slide {
            slide.time += (elapsed as f32).min(0.05);
            self.camera = slide.camera();
            if slide.time >= LevelSlide::DURATION {
                self.finish_slide();
            }
            return None;
        }
        if let Some(arrival) = &mut self.arrival {
            let was_lit = self.run.lit[0];
            *arrival += (elapsed as f32).min(0.05);
            self.run.lit[0] = *arrival >= arrival::LANDING;
            if !was_lit && self.run.lit[0] {
                self.audio.cue(audio::Cue::Shutters);
            }
            if *arrival >= arrival::DURATION {
                self.finish_arrival();
            }
            // The arrival is scenery, outside the race and replay clock.
            return None;
        }
        if !self.started {
            self.last_frame = None;
            return None;
        }
        self.run.record_active_time(elapsed);
        // Count actual active time, while limiting physics catch-up after a stall.
        Some((elapsed as f32).min(0.05))
    }

    fn finish_slide(&mut self) {
        if let Some(slide) = self.slide.take() {
            self.run.time += slide.time;
            self.camera = 0.;
            self.last_frame = None;
            self.remainder = 0.;
        }
    }

    fn score(&self) -> Score {
        Score {
            personal_best: self.run_personal_best,
            ..self.run.score()
        }
    }

    fn record_completion(&mut self, cx: &mut Context<Self>) {
        if !self.run.home || self.run.level + 1 != LEVELS || self.completion_recorded {
            return;
        }
        self.completion_recorded = true;
        let seconds = self.run.elapsed;
        if !seconds.is_finite() || seconds <= 0. {
            return;
        }
        let replay = self.recording.finish(self.run.ghost_frame()).map(Arc::new);
        let faster_time = self.personal_best.is_none_or(|best| seconds < best);
        let faster_replay = replay.as_ref().is_some_and(|replay| {
            self.best_replay
                .as_ref()
                .is_none_or(|best| replay.seconds < best.seconds)
        });
        if faster_time {
            self.personal_best = Some(seconds);
        }
        if faster_replay {
            self.best_replay = replay.clone();
        }
        cx.emit(Completed);
        if !faster_time && !faster_replay {
            return;
        }
        if let Some(path) = self.best_path.clone() {
            let generation = match GameData::save_generation(&path) {
                Ok(generation) => generation,
                Err(error) => {
                    eprintln!("Could not save Rooftops data: {error}");
                    return;
                }
            };
            cx.background_spawn(async move {
                if let Err(error) =
                    GameData::save_if_current(&path, seconds, replay.as_deref(), generation)
                {
                    eprintln!("Could not save Rooftops data: {error}");
                }
            })
            .detach();
        }
    }

    fn advance(&mut self, window: &mut Window, cx: &mut Context<Self>, view_width: f32) {
        #[cfg(feature = "walkthrough-screenshots")]
        if self.preview {
            return;
        }
        let active =
            window.is_window_active() && self.focus.is_focused(window) && !self.options_open;
        if self.options_open && self.music_preview {
            self.sync_music_preview(window.is_window_active());
        } else {
            self.sync_audio(active);
        }
        if self.slide.is_some() && active {
            if self.reduced_motion(cx) {
                self.finish_slide();
            }
            window.request_animation_frame();
        }
        if self.arrival.is_some() && active {
            if self.reduced_motion(cx) {
                self.finish_arrival();
            }
            window.request_animation_frame();
        }
        let Some(elapsed) = self.active_frame(Instant::now(), active) else {
            return;
        };
        self.remainder += elapsed;
        let returns = self.run.returns;
        let coins = self.run.coin_count();
        let lights = self.run.lit.iter().filter(|&&lit| lit).count();
        let home = self.run.home;
        while self.remainder >= STEP {
            self.run.step();
            self.remainder -= STEP;
        }
        if self.run.coin_count() > coins {
            self.audio.cue(audio::Cue::Coin);
        }
        if self.run.lit.iter().filter(|&&lit| lit).count() > lights {
            self.audio.cue(audio::Cue::Shutters);
        }
        if self.run.returns > returns {
            self.audio.cue(audio::Cue::Return);
        }
        if self.run.home != home {
            self.sync_audio(active);
        }
        self.recording.capture(self.run.ghost_frame());
        self.record_completion(cx);
        let end = self.run.roofs.last().unwrap().x + self.run.roofs.last().unwrap().width;
        let target = (self.run.x - view_width * 0.34).clamp(0., end - view_width.min(end));
        if self.run.returns != returns || self.run.jumps == 0 || self.reduced_motion(cx) {
            self.camera = target;
        } else {
            self.camera += (target - self.camera) * (1. - (-elapsed * 7.).exp());
        }
        // Movement is essential gameplay. Decorative drift is separately
        // disabled below when the user asks the system to reduce motion.
        if !self.run.home || !self.reduced_motion(cx) {
            window.request_animation_frame();
        }
    }

    fn reduced_motion(&self, cx: &App) -> bool {
        !self.preferences.animations || cx.reduce_motion()
    }

    fn sync_audio(&self, active: bool) {
        self.audio.sync(
            audio::Scene {
                active,
                level: if self.run.home && self.run.level + 1 == LEVELS {
                    3
                } else {
                    self.run.level
                },
                resting: self.run.home || self.slide.is_some(),
                thruster: self.arrival.is_some_and(arrival::flames_on),
            },
            self.preferences,
        );
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

fn score_button(id: &'static str, label: &'static str, cx: &App) -> Button {
    Button::new(id)
        .custom(
            ButtonCustomVariant::new(cx)
                .color(rgb(SCORE_BUTTON_NORMAL).into())
                .foreground(rgb(SKY).into())
                .hover(rgb(SCORE_BUTTON_HOVER).into())
                .active(rgb(SCORE_BUTTON_ACTIVE).into()),
        )
        .bg(rgb(SCORE_BUTTON_NORMAL))
        .h(px(36.))
        .px(px(18.))
        .py(px(8.))
        .rounded(px(6.))
        .text_size(px(14.))
        .line_height(px(20.))
        .label(label)
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
struct NativeArt<Q = Vec<gpui_kit::PaintQuad>> {
    quads: Q,
}
impl<Q: AsRef<[gpui_kit::PaintQuad]> + 'static> IntoElement for NativeArt<Q> {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl<Q: AsRef<[gpui_kit::PaintQuad]> + 'static> gpui_kit::Element for NativeArt<Q> {
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
        for quad in self.quads.as_ref() {
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

fn vector_art(paths: impl AsRef<[art::ArtPath]> + 'static, scale: f32) -> impl IntoElement {
    gpui_kit::canvas(
        move |_, _, _| (),
        move |bounds, _, window, _| {
            for shape in paths.as_ref() {
                let Some(first) = shape.points.first() else {
                    continue;
                };
                let at = |p: &[f32; 2]| {
                    bounds.origin + gpui_kit::point(px(p[0] * scale), px(p[1] * scale))
                };
                let mut path = gpui_kit::PathBuilder::fill();
                path.move_to(at(first));
                for p in &shape.points[1..] {
                    path.line_to(at(p));
                }
                path.close();
                if let Ok(path) = path.build() {
                    window.paint_path(
                        path,
                        gpui_kit::Hsla::from(rgb(shape.ink)).opacity(shape.opacity),
                    );
                }
            }
        },
    )
    .absolute()
    .inset_0()
}

fn stat_counter(icon: impl IntoElement, count: usize) -> gpui_kit::Div {
    h_flex()
        .items_center()
        .gap(px(6.))
        .child(div().relative().w(px(20.)).h(px(20.)).child(icon))
        .child(
            div()
                .text_size(px(12.))
                .font_semibold()
                .text_color(rgb(art::PURPLE))
                .child(count.to_string()),
        )
}

fn backdrop(w: f32, h: f32, camera: f32, time: f32, scale: f32, level: usize) -> gpui_kit::Div {
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
    div()
        .absolute()
        .size_full()
        .child(NativeArt { quads })
        .child(vector_art(art::moon_paths(moon, 58., 84., level), scale))
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

fn window_frame(
    lit: bool,
    neighborhood: Neighborhood,
    covering: Arc<[gpui_kit::PaintQuad]>,
    scale: f32,
) -> gpui_kit::Div {
    let mut window = div()
        .absolute()
        .w(px(66. * scale))
        .h(px(62. * scale))
        .rounded_t(px(3. * scale))
        .overflow_hidden()
        .border_2()
        .border_color(rgb(neighborhood.frame_color(lit)));
    if !lit {
        window = window.child(NativeArt { quads: covering });
    }
    window
}

fn apartment(
    x: f32,
    y: f32,
    seed: usize,
    lit: bool,
    time: f32,
    facade: (Neighborhood, &mut neighborhood::FacadeArt),
    scale: f32,
) -> impl IntoElement {
    let (neighborhood, cache) = facade;
    let covering = cache.covering(neighborhood, seed);
    let surround_quads = cache.surround(neighborhood, seed, lit);
    let ink = 0x39303d;
    let light = [0xd4b58b, 0xc9b69a, 0xdac39b, 0xd1aa8c][seed % 4];
    let mut room = div()
        .absolute()
        .left(px(x * scale))
        .top(px(y * scale))
        .w(px(66. * scale))
        .h(px(64. * scale));
    let surround = || {
        div().absolute().inset_0().child(NativeArt {
            quads: surround_quads.clone(),
        })
    };
    if !lit {
        return room
            .child(block(0., 0., 66., 62., 0x141b2c, scale).rounded_t(px(3. * scale)))
            .child(window_frame(false, neighborhood, covering.clone(), scale))
            .child(surround());
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
            .child(window_frame(true, neighborhood, covering, scale)),
    );
    room.child(surround())
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

fn silhouette(x: f32, y: f32, facing: f32, gait: f32, squash: f32, scale: f32) -> gpui_kit::Div {
    let ink = 0x05070d;
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
            block(body, 15., 27. + compression, 18. - compression, ink, scale)
                .rounded(px(9. * scale)),
        )
        .child(block(head, 7., 19., 20., ink, scale).rounded(px(7. * scale)))
        .child(block(far_ear, 4., 3., 8., ink, scale).rounded_t(px(1.5 * scale)))
        .child(block(near_ear, 1., 4., 11., ink, scale).rounded_t(px(2. * scale)))
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
            block(body + 4., 29. - compression, 6., 6. + gait, ink, scale)
                .rounded_b(px(3. * scale)),
        )
        .child(
            block(body + 19., 29. - compression, 6., 6. - gait, ink, scale)
                .rounded_b(px(3. * scale)),
        );
    for i in 0..5 {
        cat = cat.child(circle(
            tail - facing * i as f32 * 3.,
            22. - i as f32 * 2.8 + if i == 0 { 0. } else { gait * 0.25 },
            7.,
            ink,
            scale,
        ));
    }
    cat
}

fn player_sprite(
    pose: Pose,
    camera: f32,
    gait: f32,
    squash: f32,
    opacity: f32,
    scale: f32,
) -> impl IntoElement {
    let pose = Pose {
        gait,
        squash,
        ..pose
    };
    let head = if pose.facing > 0. { 21. } else { 0. };
    let near = head + if pose.facing > 0. { 2. } else { 13. };
    let x = pose.x - camera - 20.;
    let y = pose.y - 36. + squash * 5.;
    // The body is one union fill: transparent overlaps never darken the cat.
    div()
        .absolute()
        .inset_0()
        .child(cat_shape(pose, camera, scale, 0x05070d, opacity))
        .child(block(x + near + 1., y + 6., 2., 3., 0x71506e, scale).opacity(opacity))
        .child(
            block(
                x + head + if pose.facing > 0. { 10. } else { 3. },
                y + 15.,
                5.,
                3.,
                0xa8f0ce,
                scale,
            )
            .rounded_full()
            .opacity(opacity),
        )
}

#[cfg(feature = "walkthrough-screenshots")]
#[allow(dead_code)]
pub(crate) fn respawn_probe() -> impl IntoElement {
    let pose = Pose {
        x: 105.,
        y: 130.,
        facing: 1.,
        gait: 0.,
        squash: 0.,
        home: false,
    };
    div()
        .size_full()
        .bg(rgb(SKY))
        .child(player_sprite(pose, 0., 0., 0., 0.5, 3.))
}

fn scene_shapes(shapes: Vec<crate::rooftops_score::PosterShape>, scale: f32) -> gpui_kit::Div {
    div().absolute().inset_0().child(NativeArt {
        quads: shapes
            .into_iter()
            .map(|shape| native_quad(shape.rect, shape.ink, shape.opacity, shape.radius, scale))
            .collect::<Vec<_>>(),
    })
}

fn resting_cat(x: f32, y: f32, ink: u32, scale: f32) -> gpui_kit::Div {
    scene_shapes(art::resting_cat_shapes(x, y, ink, 1.), scale)
}

fn shelter(
    x: f32,
    y: f32,
    home: bool,
    lit: bool,
    palette: art::CheckpointPalette,
    scale: f32,
) -> gpui_kit::Div {
    let mut scene = div().absolute().inset_0();
    if home {
        scene = scene.child(vector_art(art::home_roof_paths(x, y, 1., lit), scale));
    }
    scene.child(scene_shapes(
        if home {
            art::shelter_shapes(x, y, true, lit, 1.)
        } else {
            art::checkpoint_shapes(x, y, lit, 1., palette)
        },
        scale,
    ))
}

fn ghost_sprite(pose: Pose, camera: f32, scale: f32) -> impl IntoElement {
    cat_shape(pose, camera, scale, 0xb8abff, 0.28)
}

fn cat_shape(pose: Pose, camera: f32, scale: f32, ink: u32, opacity: f32) -> impl IntoElement {
    gpui_kit::canvas(
        move |_, _, _| (),
        move |bounds, _, window, _| {
            let mut parts = Vec::with_capacity(12);
            let x = pose.x - camera;
            let y = pose.y;
            if pose.home {
                parts.extend([
                    [x - 11., y - 14., 37., 14., 7.],
                    [x + 13., y - 19., 16., 16., 8.],
                    [x + 20., y - 20., 4., 7., 0.],
                ]);
            } else {
                let x = x - 20.;
                let compression = pose.squash * 5.;
                let y = y - 36. + compression;
                let head = if pose.facing > 0. { 21. } else { 0. };
                let body = if pose.facing > 0. { 3. } else { 12. };
                let near = head + if pose.facing > 0. { 2. } else { 13. };
                let far = head + if pose.facing > 0. { 5. } else { 11. };
                parts.extend([
                    [x + body, y + 15., 27. + compression, 18. - compression, 9.],
                    [x + head, y + 7., 19., 20., 7.],
                    [x + far, y + 4., 3., 8., 1.5],
                    [x + near, y + 1., 4., 11., 2.],
                    [x + body + 4., y + 29. - compression, 6., 6. + pose.gait, 3.],
                    [
                        x + body + 19.,
                        y + 29. - compression,
                        6.,
                        6. - pose.gait,
                        3.,
                    ],
                ]);
                let tail = if pose.facing > 0. {
                    body + 2.
                } else {
                    body + 21.
                };
                for i in 0..5 {
                    parts.push([
                        x + tail - pose.facing * i as f32 * 3.,
                        y + 22. - i as f32 * 2.8 + if i == 0 { 0. } else { pose.gait * 0.25 },
                        7.,
                        7.,
                        3.5,
                    ]);
                }
            }
            // Nonzero winding unions the overlapping parts into one translucent fill.
            let mut path = gpui_kit::PathBuilder::fill().with_style(gpui_kit::PathStyle::Fill(
                gpui_kit::FillOptions::default().with_fill_rule(gpui_kit::FillRule::NonZero),
            ));
            for [x, y, w, h, radius] in parts {
                let r = radius.min(w * 0.5).min(h * 0.5);
                let mut points = Vec::with_capacity(52);
                for (cx, cy, angle) in [
                    (x + w - r, y + r, -std::f32::consts::FRAC_PI_2),
                    (x + w - r, y + h - r, 0.),
                    (x + r, y + h - r, std::f32::consts::FRAC_PI_2),
                    (x + r, y + r, std::f32::consts::PI),
                ] {
                    for i in 0..=12 {
                        let a = angle + i as f32 / 12. * std::f32::consts::FRAC_PI_2;
                        points.push(
                            bounds.origin
                                + gpui_kit::point(
                                    px((cx + r * a.cos()) * scale),
                                    px((cy + r * a.sin()) * scale),
                                ),
                        );
                    }
                }
                path.add_polygon(&points, true);
            }
            if let Ok(path) = path.build() {
                window.paint_path(path, gpui_kit::Hsla::from(rgb(ink)).opacity(opacity));
            }
        },
    )
    .absolute()
    .inset_0()
}

fn course_buildings(
    run: &Run,
    camera: f32,
    viewport: (f32, f32),
    drift: f32,
    scale: f32,
    omit_last: bool,
    cache: &mut neighborhood::FacadeArt,
) -> gpui_kit::Div {
    let (w, h) = viewport;
    let mut scene = div().absolute().inset_0();
    for (i, roof) in run.roofs.iter().enumerate() {
        let x = roof.x - camera;
        if x + roof.width < -80. || x > w + 80. || (omit_last && i + 1 == run.roofs.len()) {
            continue;
        }
        let (identity_level, seed) = run.roof_identity(i);
        let neighborhood = Neighborhood::for_level(identity_level);
        scene = scene.child(div().absolute().left(px(x * scale)).child(NativeArt {
            quads: cache.surface(neighborhood, *roof, seed, run.lit[i]),
        }));
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
                    seed * 7 + row * 3 + col,
                    run.lit[i],
                    drift,
                    (neighborhood, cache),
                    scale,
                ));
            }
        }
        if i != run.roofs.len() - 1 && !(i == 0 && run.level > 0) {
            // Chimneys stay behind the cat and do not change the collision surface.
            let (brick, coping) = neighborhood.chimney_colors(seed, run.lit[i]);
            scene = scene
                .child(block(x + 17., roof.y - 32., 23., 32., brick, scale))
                .child(block(x + 14., roof.y - 34., 29., 5., coping, scale));
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
        if run.has_lantern(i) {
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
                scene =
                    scene.child(circle(lamp - 20., roof.y - 60., 43., GOLD, scale).opacity(0.055));
            }
        }
    }
    scene
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
        self.facade_art.resize(h, scale);
        let run = &self.run;
        let drift = if self.reduced_motion(cx) {
            1.2
        } else {
            run.time + self.slide.as_ref().map_or(0., |slide| slide.time)
        };
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
            .on_action(cx.listener(Self::dismiss_options))
            .on_mouse_down(
                gpui_kit::MouseButton::Left,
                cx.listener(|this, _, window, cx| window.focus(&this.focus, cx)),
            );

        if run.home && run.level + 1 == LEVELS {
            let scene = scene
                .flex()
                .items_center()
                .justify_center()
                .child(self.results(cx, width, height));
            return self.with_options(scene, window, cx);
        }

        scene = scene.child(backdrop(
            w,
            h,
            camera + course_progress(run.level, START_X),
            drift,
            scale,
            run.level,
        ));

        if let Some(slide) = &self.slide {
            scene = scene.child(course_buildings(
                &slide.previous,
                camera - slide.offset(),
                (w, h),
                drift,
                scale,
                true,
                &mut self.facade_art,
            ));
        }
        scene = scene.child(course_buildings(
            run,
            camera,
            (w, h),
            drift,
            scale,
            false,
            &mut self.facade_art,
        ));

        // Keep the visible outgoing course until it has slid off screen. Its
        // shared roof and coin are drawn exactly once by the incoming course.
        for (course, camera, outgoing) in self
            .slide
            .iter()
            .map(|slide| (&slide.previous, camera - slide.offset(), true))
            .chain(std::iter::once((run, camera, false)))
        {
            for (index, coin) in course
                .coins
                .iter()
                .enumerate()
                .filter(|(_, coin)| !coin.collected)
            {
                if outgoing && index + 1 == course.coins.len() {
                    continue;
                }
                let x = coin.x - camera;
                if !(-25. ..w + 25.).contains(&x) {
                    continue;
                }
                scene = scene.child(cat_coin(x, coin.y, scale, &self.coin_art));
            }
            for enemy in course.enemies.iter().filter(|enemy| !enemy.defeated) {
                let (x, y) = enemy.position(
                    &course.roofs,
                    course.time + self.slide.as_ref().map_or(0., |slide| slide.time),
                );
                if !(-60. ..w + 60.).contains(&(x - camera)) {
                    continue;
                }
                scene = scene.child(enemy_sprite(enemy, x - camera, y, drift, scale));
            }
        }

        let slide_sleeping = self.slide.as_ref().is_some_and(LevelSlide::sleeping);
        let sleeping = run.home || slide_sleeping;
        let last = run.roofs.last().unwrap();
        if (-160. ..w + 160.).contains(&(run.destination() - camera)) {
            scene = scene.child(shelter(
                run.destination() - camera,
                last.y,
                run.level + 1 == LEVELS,
                !run.home,
                Neighborhood::for_level(run.level)
                    .checkpoint_palette(run.roof_identity(run.roofs.len() - 1).1),
                scale,
            ));
        }
        if run.level > 0 && (-110. ..w + 110.).contains(&(START_X - camera)) {
            scene = scene.child(shelter(
                START_X - camera,
                run.roofs[0].y,
                false,
                !slide_sleeping,
                Neighborhood::for_level(run.level - 1).checkpoint_palette(run.roof_identity(0).1),
                scale,
            ));
        }

        if self.preferences.ghost
            && self.arrival.is_none()
            && self.slide.is_none()
            && let Some(ghost) = self
                .ghost_replay
                .as_ref()
                .and_then(|replay| replay.frame_at(run.elapsed))
        {
            if let Some(gap) = ghost_gap(run, ghost, camera, w) {
                scene = scene.child(ghost_gap_indicator(gap, height));
            } else {
                scene = scene.child(ghost_sprite(ghost.pose, camera, scale));
            }
        }

        let airborne = run.ground.is_none();
        let gait = if self.reduced_motion(cx) {
            0.
        } else if airborne {
            1.5
        } else {
            (run.time * 26.).sin() * (run.vx.abs() / SPEED) * 2.5
        };
        if let Some(time) = self.arrival {
            scene = scene.child(arrival::sequence(time, camera, scale));
        } else if sleeping {
            scene = scene
                .child(resting_cat(run.x - camera, run.y, 0x05070d, scale))
                .child(vector_art(
                    art::sleep_paths(run.x - camera, run.y, drift, 1.),
                    scale,
                ));
        } else {
            scene = scene
                .child(
                    circle(run.x - camera - 17., run.y - 3., 34., 0x070a12, scale)
                        .h(px(5. * scale))
                        .opacity(if airborne { 0.18 } else { 0.45 }),
                )
                .child(player_sprite(
                    run.ghost_frame().pose,
                    camera,
                    gait,
                    if self.reduced_motion(cx) {
                        0.
                    } else {
                        run.squash
                    },
                    if run.invulnerable == 0. {
                        1.
                    } else if self.reduced_motion(cx) {
                        0.65
                    } else if ((run.time * 12.) as usize).is_multiple_of(2) {
                        0.5
                    } else {
                        1.
                    },
                    scale,
                ));
        }
        if run.level == 0 {
            if self.arrival.is_none() {
                scene = scene.child(arrival::empty_pack(camera, scale));
            }
            if !self.started {
                scene = scene.child(arrival::caption());
            }
        }
        let score = run.score();
        scene = scene
            .child(
                h_flex()
                    .absolute()
                    .top(px(19.))
                    .left(px(20.))
                    .gap(px(18.))
                    .child(stat_counter(
                        vector_art(art::stat_paths(StatIcon::Coin, 0., 0., 20., SKY), 1.),
                        score.coins,
                    ))
                    .child(stat_counter(
                        vector_art(art::stat_paths(StatIcon::Deaths, 0., 0., 20., SKY), 1.),
                        score.deaths,
                    ))
                    .child(stat_counter(
                        vector_art(art::stat_paths(StatIcon::Kills, 0., 0., 20., SKY), 1.),
                        score.kills,
                    )),
            )
            .child(
                div()
                    .absolute()
                    .top(px(16.))
                    .left_0()
                    .w_full()
                    .flex()
                    .justify_center()
                    .child(
                        div()
                            .font_family(crate::rooftops_score::POSTER_FONT)
                            .text_size(px(70.))
                            .line_height(px(87.5))
                            .font_semibold()
                            .text_color(rgb(art::PURPLE))
                            .child(score.time_label()),
                    ),
            );
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
        self.with_options(scene, window, cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[gpui_kit::test]
    fn options_freeze_play_and_transitions_and_resume_without_held_inputs(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        let (view, cx) = cx.add_window_view(Rooftops::new);
        cx.update(|_, cx| {
            view.update(cx, |game, _| {
                let start = Instant::now();
                game.begin_play(start);
                game.active_frame(start + std::time::Duration::from_secs(2), true);
                game.run.input.right = true;
                game.run.buffer = 0.1;
                game.set_options_open(true);
                let pose = (game.run.x, game.run.y, game.run.time, game.camera);
                assert!(
                    game.active_frame(start + std::time::Duration::from_secs(20), true)
                        .is_none()
                );
                assert_eq!(game.run.elapsed, 2.);
                assert_eq!((game.run.x, game.run.y, game.run.time, game.camera), pose);
                assert!(!game.run.input.right && !game.run.input.jump);
                assert_eq!(game.run.buffer, 0.);
                game.set_options_open(false);
                game.active_frame(start + std::time::Duration::from_secs(30), true);
                game.active_frame(start + std::time::Duration::from_secs(31), true);
                assert_eq!(game.run.elapsed, 3.);
                game.run.home = true;
                game.continue_from_home();
                game.set_options_open(true);
                game.active_frame(start + std::time::Duration::from_secs(40), true);
                assert_eq!(game.slide.as_ref().unwrap().time, 0.);
                game.preferences.animations = false;
                game.finish_slide();
                assert_eq!(game.camera, 0.);
                game.set_options_open(false);
                assert!(game.slide.is_none());
            })
        });
    }

    #[gpui_kit::test]
    fn options_button_and_controls_work_without_advancing_the_cat(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        use gpui_kit::{Modifiers, MouseButton};
        cx.update(gpui_kit::init);
        let (view, cx) = cx.add_window_view(Rooftops::new);
        cx.update(|window, cx| {
            let focus = view.read(cx).focus.clone();
            window.focus(&focus, cx);
            window.draw(cx).clear(cx);
        });
        for selector in [
            "rooftops-options",
            "rooftops-music",
            "rooftops-effects",
            "rooftops-ghost",
            "rooftops-animations",
            "rooftops-continue",
        ] {
            if selector == "rooftops-music" || selector == "rooftops-effects" {
                cx.update(|window, cx| {
                    let focus = view.read(cx).volume_focus
                        [usize::from(selector == "rooftops-effects")]
                    .clone();
                    window.focus(&focus, cx);
                });
                cx.simulate_keystrokes("home");
                cx.update(|window, cx| window.draw(cx).clear(cx));
                continue;
            }
            let button = cx
                .debug_bounds(selector)
                .unwrap_or_else(|| panic!("missing {selector}"));
            cx.simulate_mouse_down(button.center(), MouseButton::Left, Modifiers::default());
            cx.simulate_mouse_up(button.center(), MouseButton::Left, Modifiers::default());
            cx.update(|window, cx| window.draw(cx).clear(cx));
        }
        cx.update(|_, cx| {
            let game = view.read(cx);
            assert!(!game.options_open);
            assert_eq!(
                game.preferences,
                Preferences {
                    music_volume: 0,
                    effects_volume: 0,
                    ghost: false,
                    animations: false
                }
            );
            assert_eq!(game.run.x, START_X);
            assert_eq!(game.run.jumps, 0);
            assert_eq!(game.run.elapsed, 0.);
            assert!(
                game.arrival.is_none(),
                "disabling animations skips the opening"
            );
            assert!(game.run.lit[0]);
        });
    }

    #[gpui_kit::test]
    fn slider_labels_and_padding_never_preview_audio(cx: &mut gpui_kit::TestAppContext) {
        use gpui_kit::{Modifiers, MouseButton};
        cx.update(gpui_kit::init);
        let (view, cx) = cx.add_window_view(Rooftops::new);
        cx.update(|window, cx| {
            window.activate_window();
            view.update(cx, |game, cx| {
                game.set_options_open(true);
                window.focus(&game.options_focus, cx);
                cx.notify();
            });
        });
        cx.executor().run_until_parked();
        for id in ["rooftops-music", "rooftops-effects"] {
            cx.update(|window, cx| window.draw(cx).clear(cx));
            let bounds = cx.debug_bounds(id).unwrap();
            for point in [
                gpui_kit::point(bounds.left() + px(15.), bounds.top() + px(12.)),
                gpui_kit::point(bounds.center().x, bounds.top() + px(2.)),
            ] {
                cx.simulate_mouse_down(point, MouseButton::Left, Modifiers::default());
                cx.update(|window, cx| {
                    window.draw(cx).clear(cx);
                    assert!(
                        !view.read(cx).music_preview,
                        "holding a label or outer padding must not audition music"
                    );
                });
                cx.simulate_mouse_up(point, MouseButton::Left, Modifiers::default());
                cx.update(|window, cx| {
                    window.draw(cx).clear(cx);
                    assert!(
                        view.read(cx).audio.test_preview() <= 1,
                        "a label click does not change effects volume or beep"
                    );
                });
            }
        }
    }

    #[cfg(target_os = "macos")]
    #[gpui_kit::test]
    fn reopening_after_closing_options_restores_cat_and_movement(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        let (root, cx) = cx.add_window_view(|window, cx| {
            let game = cx.new(|cx| Rooftops::new(window, cx));
            gpui_kit::base::Root::new(game, window, cx)
        });
        let game = cx.update(|window, cx| {
            let game = root.read(cx).view().clone().downcast::<Rooftops>().unwrap();
            game.update(cx, |game, cx| {
                game.set_options_open(true);
                window.focus(&game.volume_focus[0], cx);
                cx.notify();
            });
            window.draw(cx).clear(cx);
            game
        });
        assert!(!cx.simulate_close());
        cx.update(|window, cx| {
            window.activate_window();
            window.draw(cx).clear(cx);
        });
        cx.executor().run_until_parked();
        cx.simulate_keystrokes("right");
        cx.update(|window, cx| {
            let game = game.read(cx);
            assert!(
                game.arrival.is_none() && game.run.input.right && game.started,
                "reopening must restore game focus so arrows reveal and move the cat"
            );
            assert!(game.focus.is_focused(window));
            assert!(!game.options_open && !game.music_preview);
            assert!(game.effects_adjustment.is_none());
        });
    }

    #[gpui_kit::test]
    fn dragging_music_previews_only_music_and_releasing_outside_keeps_play_frozen(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        use gpui_kit::{Modifiers, MouseButton};
        cx.update(gpui_kit::init);
        let (view, cx) = cx.add_window_view(Rooftops::new);
        let track = cx.update(|window, cx| {
            view.update(cx, |game, cx| {
                game.run.elapsed = 13.;
                window.activate_window();
                game.set_options_open(true);
                window.focus(&game.options_focus, cx);
                cx.notify();
            });
            window.draw(cx).clear(cx);
            view.read(cx).music_slider.read(cx).bounds()
        });
        cx.executor().run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let point = gpui_kit::point(track.left() + track.size.width * 0.75, track.center().y);
        cx.simulate_mouse_down(point, MouseButton::Left, Modifiers::default());
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let game = view.read(cx);
            assert!(game.music_preview && game.options_open);
            assert_eq!(
                game.audio.test_output(),
                (true, 75, 0),
                "preview excludes effects and intro thrusters"
            );
            assert_eq!(game.run.elapsed, 13.);
        });
        let outside = gpui_kit::point(px(2.), px(2.));
        cx.simulate_mouse_up(outside, MouseButton::Left, Modifiers::default());
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let game = view.read(cx);
            assert!(!game.music_preview);
            assert!(!game.audio.test_output().0);
            assert!(game.options_open);
            assert_eq!(game.run.elapsed, 13.);
        });
    }

    #[gpui_kit::test]
    fn music_handle_endcaps_start_preview(cx: &mut gpui_kit::TestAppContext) {
        use gpui_kit::{Modifiers, MouseButton};
        cx.update(gpui_kit::init);
        let (view, cx) = cx.add_window_view(Rooftops::new);
        cx.update(|window, cx| {
            window.activate_window();
            view.update(cx, |game, _| game.set_options_open(true));
        });
        cx.executor().run_until_parked();
        for value in [0, 100] {
            let bar = cx.update(|window, cx| {
                view.update(cx, |game, cx| {
                    game.preferences.music_volume = value;
                    game.music_slider.update(cx, |state, cx| {
                        state.set_value(f32::from(value), window, cx)
                    });
                    window.focus(&game.options_focus, cx);
                    cx.notify();
                });
                window.draw(cx).clear(cx);
                view.read(cx).music_slider.read(cx).bounds()
            });
            let point = gpui_kit::point(
                if value == 0 {
                    bar.left() - px(7.)
                } else {
                    bar.right() + px(7.)
                },
                bar.center().y,
            );
            cx.simulate_mouse_down(point, MouseButton::Left, Modifiers::default());
            cx.update(|_, cx| {
                assert!(
                    view.read(cx).music_preview,
                    "the actual handle overhang is interactive"
                )
            });
            cx.simulate_mouse_up(
                gpui_kit::point(px(2.), px(2.)),
                MouseButton::Left,
                Modifiers::default(),
            );
            cx.update(|_, cx| assert!(!view.read(cx).music_preview));
        }
    }

    #[gpui_kit::test]
    fn effects_release_requires_changed_volume_inside_outside_and_muted(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        use gpui_kit::{Modifiers, MouseButton};
        cx.update(gpui_kit::init);
        let (view, cx) = cx.add_window_view(Rooftops::new);
        cx.update(|window, cx| {
            window.activate_window();
            view.update(cx, |game, _| {
                game.set_options_open(true);
                game.run.elapsed = 13.;
            });
        });
        cx.executor().run_until_parked();
        for (value, outside) in [(55, false), (20, false), (80, true), (0, false)] {
            let track = cx.update(|window, cx| {
                view.update(cx, |game, cx| {
                    game.preferences.effects_volume = 55;
                    game.effects_slider
                        .update(cx, |state, cx| state.set_value(55., window, cx));
                    window.focus(&game.options_focus, cx);
                    cx.notify();
                });
                window.draw(cx).clear(cx);
                view.read(cx).effects_slider.read(cx).bounds()
            });
            let point = gpui_kit::point(
                track.left() + track.size.width * (f32::from(value) / 100.),
                track.center().y,
            );
            cx.simulate_mouse_down(point, MouseButton::Left, Modifiers::default());
            cx.update(|window, cx| {
                window.draw(cx).clear(cx);
                assert!(
                    view.read(cx).audio.test_preview() <= 1,
                    "no beep during adjustment"
                );
            });
            cx.simulate_mouse_up(
                if outside {
                    gpui_kit::point(px(2.), px(2.))
                } else {
                    point
                },
                MouseButton::Left,
                Modifiers::default(),
            );
            cx.update(|window, cx| {
                window.draw(cx).clear(cx);
                let game = view.read(cx);
                assert_eq!(game.preferences.effects_volume, value);
                assert_eq!(
                    game.audio.test_preview(),
                    if value == 55 { 1 } else { u32::from(value) + 1 },
                    "release auditions only a changed volume; zero stays silent"
                );
                assert!(!game.audio.test_output().0, "background score stays held");
                assert_eq!(game.run.elapsed, 13.);
                assert!(game.options_open);
            });
        }
    }

    #[gpui_kit::test]
    fn options_keep_keyboard_focus_inside_and_escape_returns_to_the_game(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        cx.update(|cx| {
            gpui_kit::init(cx);
            crate::application_menu::init(cx);
        });
        let (root, cx) = cx.add_window_view(|window, cx| {
            let game = cx.new(|cx| Rooftops::new(window, cx));
            window.activate_window();
            gpui_kit::base::Root::new(game, window, cx)
        });
        let game = cx.update(|window, cx| {
            let game = root.read(cx).view().clone().downcast::<Rooftops>().unwrap();
            game.update(cx, |game, cx| {
                game.run.elapsed = 13.;
                window.activate_window();
                game.set_options_open(true);
                window.focus(&game.options_focus, cx);
                cx.notify();
            });
            window.draw(cx).clear(cx);
            game
        });
        for _ in 0..6 {
            if cx.update(|window, cx| game.read(cx).volume_focus[0].is_focused(window)) {
                break;
            }
            cx.simulate_keystrokes("tab");
            cx.update(|window, cx| {
                window.draw(cx).clear(cx);
            });
        }
        cx.update(|window, cx| assert!(game.read(cx).volume_focus[0].is_focused(window)));
        cx.simulate_keystrokes("right");
        cx.update(|_, cx| assert_eq!(game.read(cx).preferences.music_volume, 76));
        // The release must still end audition after Tab moves focus off Music.
        cx.simulate_keystrokes("tab tab");
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.simulate_event(KeyUpEvent {
            keystroke: gpui_kit::Keystroke::parse("right").unwrap(),
        });
        cx.update(|_, cx| assert!(!game.read(cx).music_preview));
        cx.update(|window, cx| {
            let focus = game.read(cx).volume_focus[1].clone();
            window.focus(&focus, cx);
            window.draw(cx).clear(cx);
        });
        cx.simulate_keystrokes("right");
        cx.update(|_, cx| assert!(game.read(cx).audio.test_preview() <= 1));
        cx.simulate_keystrokes("tab");
        cx.simulate_event(KeyUpEvent {
            keystroke: gpui_kit::Keystroke::parse("right").unwrap(),
        });
        cx.update(|_, cx| {
            assert_eq!(
                game.read(cx).audio.test_preview(),
                57,
                "effects audition survives focus moving before key release"
            )
        });

        for _ in 0..8 {
            cx.simulate_keystrokes("tab");
            cx.update(|window, cx| {
                window.draw(cx).clear(cx);
                assert!(game.read(cx).options_focus.contains_focused(window, cx));
            });
        }
        cx.simulate_keystrokes("escape");
        cx.update(|_, cx| {
            assert!(!game.read(cx).options_open);
            assert_eq!(
                game.read(cx).run.elapsed,
                13.,
                "Escape must dismiss options before closing the game"
            );
        });
    }

    #[test]
    fn offscreen_ghost_gap_updates_in_meters_and_hides_when_visible() {
        let mut player = Run::new();
        player.x = 600.;
        let mut ghost = player.ghost_frame();
        ghost.pose.x = 1500.;
        let gap = ghost_gap(&player, ghost, 300., 900.).unwrap();
        assert!(!gap.ahead);
        assert_eq!(gap.meters, 900. / WORLD_UNITS_PER_METER);
        player.x += WORLD_UNITS_PER_METER;
        let closer = ghost_gap(&player, ghost, 364., 900.).unwrap();
        assert_eq!(closer.meters, gap.meters - 1.);
        assert!(ghost_gap(&player, ghost, 600., 900.).is_none());
        // Even a partially visible tail suppresses the edge indicator.
        ghost.pose.x = 300. + 900. + 27.;
        assert!(ghost_gap(&player, ghost, 300., 900.).is_none());
        ghost.pose.x += 0.25;
        assert!(ghost_gap(&player, ghost, 300., 900.).is_some());

        player.x = 1800.;
        ghost.pose.x = 1400.;
        let ahead = ghost_gap(&player, ghost, 1500., 900.).unwrap();
        assert!(ahead.ahead);
        assert_eq!(ahead.meters, 400. / WORLD_UNITS_PER_METER);
        ghost.pose.x += WORLD_UNITS_PER_METER;
        assert_eq!(
            ghost_gap(&player, ghost, 1500., 900.).unwrap().meters,
            ahead.meters - 1.
        );
    }

    #[test]
    fn ghost_distance_is_continuous_across_level_boundaries_and_ignores_jumps() {
        let mut player = Run::new();
        player.x = player.destination() - WORLD_UNITS_PER_METER;
        let mut ghost = Run::for_level(1).ghost_frame();
        let gap = ghost_gap(&player, ghost, 0., 900.).unwrap();
        assert!(!gap.ahead);
        assert_eq!(gap.meters, 1.);
        ghost.pose.y -= 120.;
        player.y -= 80.;
        assert_eq!(ghost_gap(&player, ghost, 0., 900.).unwrap().meters, 1.);

        ghost = player.ghost_frame();
        ghost.pose.x = player.destination();
        player = Run::for_level(1);
        player.x = START_X + WORLD_UNITS_PER_METER;
        let gap = ghost_gap(&player, ghost, 0., 900.).unwrap();
        assert!(
            gap.ahead,
            "a player in the next level is ahead despite smaller local coordinates"
        );
        assert_eq!(gap.meters, 1.);
        assert_eq!(
            course_progress(0, Run::new().destination()),
            course_progress(1, START_X)
        );
        assert_eq!(
            course_progress(1, Run::for_level(1).destination()),
            course_progress(2, START_X)
        );
    }

    #[gpui_kit::test]
    fn ghost_uses_only_the_fastest_completed_run_before_the_current_run(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        let (view, cx) = cx.add_window_view(Rooftops::new);
        cx.update(|_, cx| {
            view.update(cx, |game, cx| {
                let finish_run = |game: &mut Rooftops, cx: &mut Context<Rooftops>, seconds: f64| {
                    for level in 0..LEVELS {
                        game.run.elapsed = seconds * (level + 1) as f64 / LEVELS as f64;
                        game.run.x = game.run.destination();
                        game.run.home = true;
                        game.recording.capture(game.run.ghost_frame());
                        game.record_completion(cx);
                        if level + 1 < LEVELS {
                            game.continue_from_home();
                        }
                    }
                };
                assert!(game.ghost_replay.is_none(), "first run has no ghost");
                finish_run(game, cx, 30.);
                assert_eq!(game.best_replay.as_ref().unwrap().seconds, 30.);
                assert!(
                    game.ghost_replay.is_none(),
                    "current finish must not replace its own opponent"
                );
                game.continue_from_home();
                let opponent = game.ghost_replay.clone().unwrap();
                assert_eq!(opponent.seconds, 30.);
                finish_run(game, cx, 45.);
                assert_eq!(game.best_replay.as_ref().unwrap().seconds, 30.);
                game.continue_from_home();
                finish_run(game, cx, 24.);
                assert_eq!(game.best_replay.as_ref().unwrap().seconds, 24.);
                assert!(
                    Arc::ptr_eq(game.ghost_replay.as_ref().unwrap(), &opponent),
                    "a faster current run cannot swap the opponent mid-race"
                );
                game.continue_from_home();
                assert_eq!(game.ghost_replay.as_ref().unwrap().seconds, 24.);
                assert_eq!(game.run.elapsed, 0.);
                assert!(
                    game.recording.finish(game.run.ghost_frame()).is_none(),
                    "a partial run cannot be saved as a replay"
                );
                game.reset_progress(cx);
                assert!(!game.has_completed());
                assert!(game.best_replay.is_none() && game.ghost_replay.is_none());
                assert!(game.run_personal_best.is_none());
                assert_eq!(game.run.level, 0);
                assert_eq!(game.run.elapsed, 0.);
                assert_eq!(game.run.coin_count(), 0);
            });
        });
    }

    #[gpui_kit::test]
    fn play_again_button_and_space_restart_completed_runs(cx: &mut gpui_kit::TestAppContext) {
        use gpui_kit::{Modifiers, MouseButton};
        cx.update(gpui_kit::init);
        let (view, cx) = cx.add_window_view(Rooftops::new);
        for click_button in [true, false] {
            cx.update(|window, cx| {
                view.update(cx, |game, cx| {
                    game.run = Run::for_level(LEVELS - 1);
                    game.run.home = true;
                    game.run.elapsed = 143.6;
                    game.run.banked_coins = 41;
                    game.personal_best = Some(138.2);
                    game.run_personal_best = game.personal_best;
                    game.camera = 3000.;
                    game.save_message = Some("Saved previous score".into());
                    window.focus(&game.focus, cx);
                    cx.notify();
                });
                window.draw(cx).clear(cx);
            });
            let gear = cx
                .debug_bounds("rooftops-options")
                .expect("results retain the options control");
            cx.update(|window, _| {
                assert!(gear.left() >= px(0.) && gear.right() <= window.viewport_size().width);
                assert!(gear.top() >= px(0.) && gear.bottom() <= window.viewport_size().height);
            });
            if click_button {
                let button = cx
                    .debug_bounds("play-rooftops-again")
                    .expect("play-again button must be visible");
                let download = cx
                    .debug_bounds("download-rooftops-scorecard")
                    .expect("scorecard button must be visible");
                let exit = cx
                    .debug_bounds("exit-rooftops")
                    .expect("Exit must be visible");
                assert_eq!(button.size.height, download.size.height);
                assert_eq!(button.size.height, exit.size.height);
                assert_eq!(button.origin.y, download.origin.y);
                assert_eq!(button.origin.y, exit.origin.y);
                assert!(exit.left() > button.right());
                assert!(exit.right() < download.left());
                cx.simulate_mouse_down(button.center(), MouseButton::Left, Modifiers::default());
                cx.simulate_mouse_up(button.center(), MouseButton::Left, Modifiers::default());
            } else {
                cx.simulate_keystrokes("space");
            }
            cx.update(|window, cx| {
                let game = view.read(cx);
                assert_eq!(game.run.level, 0);
                assert!(!game.run.home);
                assert_eq!(game.run.coin_count(), 0);
                assert_eq!(game.run.elapsed, 0.);
                assert!(!game.started, "restarting waits for gameplay input");
                assert_eq!(game.camera, 0.);
                assert!(game.save_message.is_none());
                assert_eq!(game.personal_best, Some(138.2));
                assert!(game.focus.is_focused(window));
            });
            cx.update(|_, cx| {
                view.update(cx, |game, _| {
                    let now = Instant::now();
                    assert!(game.active_frame(now, true).is_none());
                    assert!(
                        game.active_frame(now + std::time::Duration::from_secs(10), true)
                            .is_none()
                    );
                    assert_eq!(game.run.elapsed, 0., "idle time after Play again is free");
                });
            });
        }
    }

    #[gpui_kit::test]
    fn personal_best_requires_all_levels_and_survives_restarts(cx: &mut gpui_kit::TestAppContext) {
        cx.update(gpui_kit::init);
        let (view, cx) = cx.add_window_view(Rooftops::new);
        cx.update(|_, cx| {
            view.update(cx, |game, cx| {
                game.run.elapsed = 50.;
                game.record_completion(cx);
                assert_eq!(game.personal_best, None);
                game.run.home = true;
                game.record_completion(cx);
                assert_eq!(game.personal_best, None);
                game.run.level = LEVELS - 1;
                game.record_completion(cx);
                assert_eq!(game.personal_best, Some(50.));
                assert!(
                    game.score().personal_best.is_none(),
                    "first run has no prior record"
                );
                game.continue_from_home();
                assert_eq!(game.run.elapsed, 0.);
                assert_eq!(game.personal_best, Some(50.));
                for (seconds, expected, show_footer) in
                    [(65., 50., true), (50., 50., false), (42., 42., false)]
                {
                    game.run.level = LEVELS - 1;
                    game.run.home = true;
                    game.run.elapsed = seconds;
                    game.record_completion(cx);
                    assert_eq!(game.personal_best, Some(expected));
                    let footer = crate::rooftops_score::poster_text(game.score())
                        .into_iter()
                        .find(|text| text.label.starts_with("personal best:"));
                    assert_eq!(footer.is_some(), show_footer);
                    if let Some(footer) = footer {
                        assert_eq!(footer.label, "personal best: 0:50");
                    }
                    game.continue_from_home();
                    assert_eq!(game.run_personal_best, Some(expected));
                }
            });
        });
    }

    #[cfg(target_os = "macos")]
    #[gpui_kit::test]
    fn closing_resets_the_game_and_retains_the_same_window(cx: &mut gpui_kit::TestAppContext) {
        cx.update(gpui_kit::init);
        let (view, cx) = cx.add_window_view(Rooftops::new);
        cx.update(|_, cx| {
            view.update(cx, |game, _| {
                game.run.elapsed = 123.;
                game.run.input.right = true;
                game.camera = 500.;
                game.save_message = Some("Old score".into());
            });
        });
        for _ in 0..3 {
            assert!(!cx.simulate_close());
            cx.update(|_, cx| {
                view.update(cx, |game, _| {
                    assert_eq!(game.run.elapsed, 0.);
                    assert!(!game.run.input.right);
                    assert_eq!(game.camera, 0.);
                    assert!(game.save_message.is_none());
                });
            });
        }
    }

    #[gpui_kit::test]
    fn play_time_excludes_pauses_and_completion(cx: &mut gpui_kit::TestAppContext) {
        cx.update(gpui_kit::init);
        let (view, cx) = cx.add_window_view(Rooftops::new);
        cx.update(|_, cx| {
            view.update(cx, |game, _| {
                let start = Instant::now();
                game.begin_play(start);
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

    #[gpui_kit::test]
    fn checkpoint_house_relighting_is_silent_and_intro_apartments_open_audibly(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        let (view, cx) = cx.add_window_view(Rooftops::new);
        cx.update(|_, cx| {
            view.update(cx, |game, _| {
                let start = Instant::now();
                game.sync_audio(true);
                game.arrival = Some(arrival::LANDING - 0.025);
                game.last_frame = Some(start);
                game.active_frame(start + std::time::Duration::from_millis(50), true);
                assert!(game.run.lit[0]);
                assert!(
                    game.audio.test_cues() > 0,
                    "touchdown opens the apartment shutters"
                );
                for skip in [false, true] {
                    game.audio.suspend();
                    game.arrival = None;
                    game.run = Run::new();
                    game.run.home = true;
                    game.continue_from_home();
                    game.sync_audio(true);
                    if !skip {
                        game.slide.as_mut().unwrap().time = LevelSlide::PAN - 0.025;
                        game.last_frame = Some(start);
                        game.active_frame(start + std::time::Duration::from_millis(50), true);
                        assert!(!game.slide.as_ref().unwrap().sleeping());
                        assert_eq!(
                            game.audio.test_cues(),
                            0,
                            "the house relights without opening apartments"
                        );
                    }
                    game.finish_slide();
                    assert_eq!(
                        game.audio.test_cues(),
                        0,
                        "finishing or skipping a slide is silent"
                    );
                }
            })
        });
    }

    #[gpui_kit::test]
    fn shutters_sound_only_when_landing_opens_closed_apartment_windows(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        let (view, cx) = cx.add_window_view(Rooftops::new);
        cx.update(|window, cx| {
            window.activate_window();
            view.update(cx, |game, _| {
                game.arrival = None;
                game.started = true;
                game.run = Run::new();
                game.run.enemies.clear();
                game.run.coins.clear();
            });
        });
        cx.executor().run_until_parked();
        for was_open in [false, true] {
            cx.update(|window, cx| {
                view.update(cx, |game, cx| {
                    game.audio.suspend();
                    game.run.x = game.run.roofs[1].x + 24.;
                    game.run.y = game.run.roofs[1].y - 0.2;
                    game.run.vy = 40.;
                    game.run.ground = None;
                    game.last_frame = Some(Instant::now() - std::time::Duration::from_millis(20));
                    window.focus(&game.focus, cx);
                    cx.notify();
                });
                // Advance through the production render path, where GPUI permits
                // the animation-frame request made by live gameplay.
                window.draw(cx).clear(cx);
                let game = view.read(cx);
                assert_eq!(game.run.ground, Some(1));
                assert!(game.run.lit[1]);
                assert_eq!(
                    game.audio.test_cues() > 0,
                    !was_open,
                    "landing on an already-open building is silent"
                );
            });
        }
    }

    #[gpui_kit::test]
    fn arrival_pauses_and_finishes_before_the_race_and_ghost_clock(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        let (view, cx) = cx.add_window_view(Rooftops::new);
        cx.update(|_, cx| {
            view.update(cx, |game, _| {
                let start = Instant::now();
                assert!(
                    !game.run.lit[0],
                    "the first roof starts dark before touchdown"
                );
                game.active_frame(start, true);
                game.active_frame(start + std::time::Duration::from_millis(50), true);
                assert!(!game.run.lit[0]);
                let progress = game.arrival;
                game.active_frame(start + std::time::Duration::from_secs(10), false);
                game.active_frame(start + std::time::Duration::from_secs(20), true);
                assert_eq!(game.arrival, progress, "focus loss pauses the arrival");
                for step in 1..=60 {
                    game.active_frame(
                        start + std::time::Duration::from_millis(20_000 + step * 50),
                        true,
                    );
                    assert_eq!(
                        game.run.lit[0],
                        game.arrival.is_none_or(|time| time >= arrival::LANDING)
                    );
                    if game.arrival.is_none() {
                        break;
                    }
                }
                assert!(game.arrival.is_none());
                assert!(game.run.lit[0], "touchdown lights the apartments");
                assert_eq!(game.run.elapsed, 0.);
                assert_eq!(game.run.time, 0.);
                assert_eq!(game.run.x, START_X);
                assert_eq!(game.run.ground, Some(0));
                assert_eq!(game.run.jumps, 0);
                assert_eq!(game.run.returns, 0);
                assert_eq!(game.run.coin_count(), 0);
                assert_eq!(game.run.ghost_frame().seconds, 0.);
                let resume = start + std::time::Duration::from_secs(30);
                game.active_frame(resume, true);
                game.active_frame(resume + std::time::Duration::from_secs(1), true);
                assert_eq!(game.run.elapsed, 0., "landing does not start the clock");
                assert!(!game.started);
                game.begin_play(resume + std::time::Duration::from_secs(2));
                game.active_frame(resume + std::time::Duration::from_secs(3), true);
                assert_eq!(game.run.elapsed, 1.);
            });
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
    fn arrival_requires_the_doorway_and_snaps_to_a_settled_sleeping_pose() {
        for level in 0..LEVELS {
            let mut run = Run::for_level(level);
            let last = run.roofs.len() - 1;
            run.x = run.destination() - 14.;
            run.y = run.roofs[last].y;
            run.ground = Some(last);
            run.step();
            assert!(!run.home, "landing on the building is not arrival");

            run.x = run.destination() + 8.;
            run.y -= 30.;
            run.ground = None;
            run.step();
            assert!(!run.home, "crossing the doorway in the air is not arrival");

            run.x = run.destination() - 1.;
            run.y = run.roofs[last].y;
            run.ground = Some(last);
            run.vx = SPEED;
            run.vy = 0.;
            run.input.right = true;
            run.step();
            assert!(run.home);
            assert_eq!(run.x, run.destination());
            assert_eq!((run.vx, run.vy, run.squash, run.buffer), (0., 0., 0., 0.));
            assert!(run.ghost_frame().pose.home);
            let x = run.x;
            run.step();
            assert_eq!(run.x, x);
        }
    }

    #[test]
    fn next_level_starts_on_the_same_roof_with_the_same_apartments_and_props() {
        for level in 0..LEVELS - 1 {
            let mut run = Run::for_level(level);
            let last = run.roofs.len() - 1;
            let roof = run.roofs[last];
            let identity = run.roof_identity(last);
            let door = run.destination();
            run.time = 25.;
            run.home = true;
            run.jump_down();
            assert_eq!(run.roofs[0].x - START_X, roof.x - door);
            assert_eq!(run.roofs[0].y, roof.y);
            assert_eq!(run.roofs[0].width, roof.width);
            assert_eq!(run.roof_identity(0), identity);
            assert!(!run.has_lantern(0));
            assert_eq!(run.y, roof.y);
            assert_eq!(run.time, 25., "residents keep their animation phase");
            assert!(run.lit[0]);
            assert!(!run.home);
            assert_eq!(run.ground, Some(0));
        }
    }

    #[test]
    fn doorway_coins_carry_over_without_being_awarded_twice() {
        for collected in [false, true] {
            let mut run = Run::new();
            run.coins.last_mut().unwrap().collected = collected;
            run.home = true;
            run.jump_down();
            assert_eq!(run.coin_count(), usize::from(collected));
            let roof = run.roofs[0];
            assert_eq!(
                run.coins.iter().any(|coin| coin.x < roof.x + roof.width),
                !collected
            );
        }
    }

    #[test]
    fn only_intermediate_checkpoint_roofs_have_lanterns() {
        for level in 0..LEVELS {
            let run = Run::for_level(level);
            assert!(!run.has_lantern(0));
            assert!(!run.has_lantern(run.roofs.len() - 1));
            assert!(run.has_lantern(run.checkpoint_spacing()));
            assert!(!run.has_lantern(1));
        }
    }

    #[gpui_kit::test]
    fn slide_keeps_the_roof_in_place_then_wakes_the_cat_without_race_time(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        let (view, cx) = cx.add_window_view(Rooftops::new);
        cx.update(|_, cx| {
            view.update(cx, |game, _| {
                game.begin_play(Instant::now());
                game.run.x = game.run.destination();
                game.run.home = true;
                game.run.elapsed = 42.;
                game.camera = game.run.roofs.last().unwrap().x + REST_ROOF_WIDTH - 900.;
                let screen_x = game.run.x - game.camera;
                let skyline_camera = game.camera;
                game.continue_from_home();
                assert_eq!(game.run.x - game.camera, screen_x);
                assert_eq!(game.camera + course_progress(1, START_X), skyline_camera);
                assert!(game.slide.as_ref().unwrap().sleeping());
                let start = Instant::now();
                game.active_frame(start, true);
                for frame in 1..=10 {
                    assert!(
                        game.active_frame(
                            start + std::time::Duration::from_millis(frame * 50),
                            true
                        )
                        .is_none()
                    );
                }
                let camera = game.camera;
                game.active_frame(start + std::time::Duration::from_secs(1), false);
                game.active_frame(start + std::time::Duration::from_secs(20), true);
                assert_eq!(game.camera, camera, "focus loss pauses the pan");
                for frame in 1..=15 {
                    game.active_frame(
                        start + std::time::Duration::from_millis(20_000 + frame * 50),
                        true,
                    );
                }
                assert_eq!(game.camera, 0.);
                assert!(
                    !game.slide.as_ref().unwrap().sleeping(),
                    "wake after the pan"
                );
                for frame in 16..=22 {
                    game.active_frame(
                        start + std::time::Duration::from_millis(20_000 + frame * 50),
                        true,
                    );
                    if game.slide.is_none() {
                        break;
                    }
                }
                assert!(game.slide.is_none());
                assert_eq!(game.run.elapsed, 42.);
                assert_eq!(game.run.ghost_frame().seconds, 42.);
                assert_eq!(game.run.x, START_X);
                assert_eq!(game.run.jumps, 0);
                let resume = start + std::time::Duration::from_secs(30);
                game.active_frame(resume, true);
                game.active_frame(resume + std::time::Duration::from_secs(1), true);
                assert_eq!(game.run.elapsed, 43.);
            });
        });
    }

    #[gpui_kit::test]
    fn either_movement_arrow_skips_the_slide_and_moves_the_awake_cat(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        let (view, cx) = cx.add_window_view(Rooftops::new);
        for key in ["left", "right"] {
            cx.update(|window, cx| {
                view.update(cx, |game, cx| {
                    game.run = Run::new();
                    game.run.home = true;
                    game.continue_from_home();
                    assert!(game.slide.is_some());
                    window.focus(&game.focus, cx);
                    game.key_down(
                        &KeyDownEvent {
                            keystroke: gpui_kit::Keystroke::parse(key).unwrap(),
                            is_held: false,
                            prefer_character_input: false,
                        },
                        window,
                        cx,
                    );
                    assert!(game.slide.is_none());
                    assert_eq!(game.camera, 0.);
                    assert!(!game.run.home);
                    game.run.step();
                    assert!(if key == "left" {
                        game.run.x < START_X
                    } else {
                        game.run.x > START_X
                    });
                    assert_eq!(game.run.ghost_frame().seconds, 0.);
                });
            });
        }
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
        cx.update(|_, cx| {
            assert!(
                view.read(cx).arrival.is_none(),
                "movement skips the arrival"
            );
            assert!(view.read(cx).run.lit[0], "skipping lights the first roof");
            assert!(view.read(cx).started, "movement starts the clock");
            assert!(view.read(cx).run.input.right);
        });
        cx.simulate_event(KeyUpEvent {
            keystroke: gpui_kit::Keystroke::parse("right").unwrap(),
        });
        cx.update(|_, cx| {
            assert!(
                !view.read(cx).run.input.right,
                "key release must clear movement"
            );
            view.update(cx, |game, _| {
                game.arrival = Some(0.6);
                game.run.lit[0] = false;
                game.started = false;
            });
        });
        cx.simulate_keystrokes("space");
        cx.update(|_, cx| {
            view.update(cx, |game, _| {
                assert!(game.arrival.is_none(), "Space skips the arrival too");
                assert!(game.run.lit[0]);
                assert!(game.started, "jumping starts the clock too");
                game.run.step();
                assert!(game.run.vy < 0.);
            })
        });
    }
}

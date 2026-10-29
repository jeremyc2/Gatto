use super::*;
use gpui_kit::base::{Checkbox, CheckboxState, FocusTrapElement as _};
use gpui_kit::component::{
    Icon, IconName,
    slider::{Slider, SliderEvent, SliderValue},
};

#[derive(Clone, Copy)]
pub(super) enum Preference {
    Ghost,
    Animations,
}

#[derive(Clone, Copy)]
pub(super) enum Volume {
    Music,
    Effects,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Adjustment {
    Pointer,
    Keyboard,
}

#[derive(Clone, Copy)]
pub(super) struct EffectsAdjustment {
    pub kind: Adjustment,
    pub initial_volume: u8,
}

impl Rooftops {
    pub(super) fn set_options_open(&mut self, open: bool) {
        self.music_preview = false;
        self.effects_adjustment = None;
        self.options_open = open;
        self.run.input = Input::default();
        self.run.buffer = 0.;
        self.last_frame = None;
        self.remainder = 0.;
        self.audio.suspend();
    }

    fn toggle_options(
        &mut self,
        _: &gpui_kit::ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.options_open {
            self.persist_options(cx, 0);
        }
        if !self.options_open {
            self.music_slider.update(cx, |state, cx| {
                state.set_value(f32::from(self.preferences.music_volume), window, cx)
            });
            self.effects_slider.update(cx, |state, cx| {
                state.set_value(f32::from(self.preferences.effects_volume), window, cx)
            });
        }
        self.set_options_open(!self.options_open);
        window.focus(
            if self.options_open {
                &self.options_focus
            } else {
                &self.focus
            },
            cx,
        );
        cx.notify();
    }

    pub(super) fn dismiss_options(
        &mut self,
        _: &crate::application_menu::DismissWindow,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.options_open {
            self.persist_options(cx, 0);
            self.set_options_open(false);
            window.focus(&self.focus, cx);
            cx.notify();
        } else {
            cx.propagate();
        }
    }

    fn change_preference(&mut self, preference: Preference, enabled: bool, cx: &mut Context<Self>) {
        match preference {
            Preference::Ghost => self.preferences.ghost = enabled,
            Preference::Animations => {
                self.preferences.animations = enabled;
                if !enabled {
                    self.finish_arrival();
                    self.finish_slide();
                }
            }
        }
        self.preferences_dirty = true;
        self.persist_options(cx, 0);
        cx.notify();
    }

    pub(super) fn volume_changed(
        &mut self,
        volume: Volume,
        event: &SliderEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.options_open {
            return;
        }
        let (value, release) = match event {
            SliderEvent::Change(SliderValue::Single(value)) => (*value, false),
            SliderEvent::Release(SliderValue::Single(value)) => (*value, true),
            _ => return,
        };
        if matches!(volume, Volume::Music) {
            self.music_preview = !release && self.options_open && window.is_window_active();
        }
        let value = value.round().clamp(0., 100.) as u8;
        let current = match volume {
            Volume::Music => &mut self.preferences.music_volume,
            Volume::Effects => &mut self.preferences.effects_volume,
        };
        let changed = *current != value;
        if changed {
            *current = value;
            self.preferences_dirty = true;
            self.audio.retry();
        }
        if self.options_open && matches!(volume, Volume::Music) {
            self.audio.suspend();
            if self.music_preview {
                self.sync_music_preview(true);
            }
        }
        self.persist_options(cx, if release { 0 } else { 250 });
        cx.notify();
    }

    pub(super) fn sync_music_preview(&self, active: bool) {
        self.audio.sync(
            audio::Scene {
                active,
                level: if self.run.home && self.run.level + 1 == LEVELS {
                    3
                } else {
                    self.run.level
                },
                resting: self.run.home || self.slide.is_some(),
                thruster: false,
            },
            Preferences {
                effects_volume: 0,
                ..self.preferences
            },
        );
    }

    fn end_music_preview(&mut self) {
        if self.music_preview {
            self.music_preview = false;
            self.audio.suspend();
        }
    }

    fn finish_effects_adjustment(
        &mut self,
        kind: Adjustment,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        let Some(edit) = self.effects_adjustment.filter(|edit| edit.kind == kind) else {
            return;
        };
        self.effects_adjustment = None;
        if self.options_open
            && window.is_window_active()
            && self.preferences.effects_volume != edit.initial_volume
        {
            self.audio.preview_effects(self.preferences.effects_volume);
        }
        self.persist_options(cx, 0);
        cx.notify();
    }

    fn persist_options(&mut self, cx: &mut Context<Self>, delay_ms: u64) {
        if !self.preferences_dirty {
            return;
        }
        self.preferences_message = None;
        let Some(path) = &self.best_path else {
            return;
        };
        let preferences = self.preferences;
        match GameData::preferences_write(path, preferences) {
            Ok(write) => {
                let debounce = cx
                    .background_executor()
                    .timer(std::time::Duration::from_millis(delay_ms));
                cx.spawn(async move |this, cx| {
                    debounce.await;
                    if !write.is_current() {
                        return;
                    }
                    let result = cx.background_spawn(async move { write.save() }).await;
                    let _ = this.update(cx, |this, cx| {
                        if this.preferences == preferences {
                            match result {
                                Ok(true) => {
                                    this.preferences_dirty = false;
                                    this.preferences_message = None;
                                }
                                Ok(false) => return,
                                Err(error) => {
                                    eprintln!("Could not save Rooftops options: {error}");
                                    this.preferences_message = Some(
                                        "Couldn't save changes. Adjust an option to try again."
                                            .into(),
                                    );
                                }
                            }
                            cx.notify();
                        }
                    });
                })
                .detach();
            }
            Err(error) => {
                eprintln!("Could not save Rooftops options: {error}");
                self.preferences_message =
                    Some("Couldn't save changes. Adjust an option to try again.".into());
            }
        }
    }

    fn volume_row(
        &self,
        volume: Volume,
        label: &'static str,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let (state, focus, value, id) = match volume {
            Volume::Music => (
                &self.music_slider,
                &self.volume_focus[0],
                self.preferences.music_volume,
                "rooftops-music",
            ),
            Volume::Effects => (
                &self.effects_slider,
                &self.volume_focus[1],
                self.preferences.effects_volume,
                "rooftops-effects",
            ),
        };
        let pointer_slider = state.clone();
        v_flex()
            .id(id)
            .debug_selector(move || id.into())
            .track_focus(focus)
            .tab_index(0)
            .tab_stop(true)
            .gap(px(8.))
            // Enclose the handle's 8px overhang without moving the label/track.
            .mx(-px(2.))
            .px(px(8.))
            .py(px(6.))
            .rounded(px(5.))
            .child(
                h_flex()
                    .justify_between()
                    .text_color(rgb(if focus.is_focused(window) {
                        GOLD
                    } else {
                        0xf3ddba
                    }))
                    .text_size(px(16.))
                    .child(label)
                    .child(div().text_size(px(12.)).child(if value == 0 {
                        "Off".into()
                    } else {
                        format!("{value}%")
                    })),
            )
            .child(
                Slider::new(state)
                    .bg(rgb(art::PURPLE))
                    .text_color(rgb(GOLD))
                    .h(px(24.)),
            )
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                let current = match volume {
                    Volume::Music => this.preferences.music_volume,
                    Volume::Effects => this.preferences.effects_volume,
                };
                let next = match event.keystroke.key.as_str() {
                    "left" | "down" => current.saturating_sub(if event.keystroke.modifiers.shift {
                        5
                    } else {
                        1
                    }),
                    "right" | "up" => current
                        .saturating_add(if event.keystroke.modifiers.shift {
                            5
                        } else {
                            1
                        })
                        .min(100),
                    "home" => 0,
                    "end" => 100,
                    _ => return,
                };
                if matches!(volume, Volume::Effects) {
                    if this
                        .effects_adjustment
                        .is_none_or(|edit| edit.kind != Adjustment::Keyboard)
                    {
                        this.effects_adjustment = Some(EffectsAdjustment {
                            kind: Adjustment::Keyboard,
                            initial_volume: this.preferences.effects_volume,
                        });
                    }
                    this.music_preview = false;
                    this.audio.suspend();
                }
                let state = match volume {
                    Volume::Music => this.music_slider.clone(),
                    Volume::Effects => this.effects_slider.clone(),
                };
                state.update(cx, |state, cx| state.set_value(f32::from(next), window, cx));
                this.volume_changed(
                    volume,
                    &SliderEvent::Change(SliderValue::Single(f32::from(next))),
                    window,
                    cx,
                );
                window.prevent_default();
                cx.stop_propagation();
            }))
            .capture_any_mouse_down(cx.listener(
                move |this, event: &gpui_kit::MouseDownEvent, _, cx| {
                    if event.button != gpui_kit::MouseButton::Left || !this.options_open {
                        return;
                    }
                    // Match the native 24px track hit area and 16px thumb,
                    // including the thumb's overhang at either endpoint.
                    let bar = pointer_slider.read(cx).bounds();
                    let hit = gpui_kit::Bounds::new(
                        gpui_kit::point(bar.left() - px(8.), bar.center().y - px(12.)),
                        gpui_kit::size(bar.size.width + px(16.), px(24.)),
                    );
                    if !hit.contains(&event.position) {
                        return;
                    }
                    this.audio.suspend();
                    this.music_preview = matches!(volume, Volume::Music) && this.options_open;
                    this.effects_adjustment =
                        matches!(volume, Volume::Effects).then_some(EffectsAdjustment {
                            kind: Adjustment::Pointer,
                            initial_volume: this.preferences.effects_volume,
                        });
                    if this.music_preview {
                        this.sync_music_preview(true);
                    }
                    cx.notify();
                },
            ))
            .capture_any_mouse_up(cx.listener(
                |this, event: &gpui_kit::MouseUpEvent, window, cx| {
                    if event.button != gpui_kit::MouseButton::Left {
                        return;
                    }
                    this.end_music_preview();
                    this.finish_effects_adjustment(Adjustment::Pointer, window, cx);
                    cx.notify();
                },
            ))
            .on_mouse_up_out(
                gpui_kit::MouseButton::Left,
                cx.listener(|this, _, window, cx| {
                    this.end_music_preview();
                    this.finish_effects_adjustment(Adjustment::Pointer, window, cx);
                    cx.notify();
                }),
            )
    }

    fn option_row(
        &self,
        id: &'static str,
        label: &'static str,
        preference: Preference,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let checked = match preference {
            Preference::Ghost => self.preferences.ghost,
            Preference::Animations => self.preferences.animations,
        };
        let focus = &self.option_controls_focus[preference as usize];
        let view = cx.entity().downgrade();
        let mut indicator = div()
            .w(px(17.))
            .h(px(17.))
            .rounded(px(3.))
            .border_1()
            .border_color(rgb(art::PURPLE))
            .bg(rgb(if checked { art::PURPLE } else { SKY }))
            .flex()
            .items_center()
            .justify_center()
            .flex_shrink_0();
        if checked {
            indicator = indicator.child(
                Icon::new(IconName::Check)
                    .size(px(13.))
                    .text_color(rgb(SKY)),
            );
        }
        Checkbox::new(id)
            .debug_selector(move || id.into())
            .accessibility_label(label)
            .checked(checked)
            .track_focus(focus)
            .h_flex()
            .items_center()
            .gap(px(12.))
            .py(px(8.))
            .px(px(6.))
            .w_full()
            .rounded(px(5.))
            .child(indicator)
            .child(
                div()
                    .text_color(rgb(if focus.is_focused(window) {
                        GOLD
                    } else {
                        0xf3ddba
                    }))
                    .text_size(px(16.))
                    .child(label),
            )
            .on_change(move |state, _, _, cx| {
                let _ = view.update(cx, |this, cx| {
                    this.change_preference(preference, state == CheckboxState::Checked, cx);
                });
            })
    }

    pub(super) fn with_options(
        &self,
        mut scene: gpui_kit::Stateful<gpui_kit::Div>,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> gpui_kit::Stateful<gpui_kit::Div> {
        if !self.options_open {
            return scene.child(
                div()
                    .absolute()
                    .top(px(18.))
                    .right(px(20.))
                    .w(px(40.))
                    .h(px(40.))
                    .child(
                        Button::new("rooftops-options")
                            .debug_selector(|| "rooftops-options".into())
                            .icon(IconName::Settings)
                            .accessibility_label("Options")
                            .tooltip("Options")
                            .custom(
                                ButtonCustomVariant::new(cx)
                                    .color(rgb(SKY).into())
                                    .foreground(rgb(art::PURPLE).into())
                                    .hover(rgb(0x302d4a).into()),
                            )
                            .w(px(40.))
                            .h(px(40.))
                            .rounded_full()
                            .on_click(cx.listener(Self::toggle_options)),
                    ),
            );
        }
        let mut card = v_flex()
            .track_focus(&self.options_focus)
            .capture_key_up(cx.listener(|this, event: &KeyUpEvent, window, cx| {
                if matches!(
                    event.keystroke.key.as_str(),
                    "left" | "right" | "up" | "down" | "home" | "end"
                ) {
                    this.end_music_preview();
                    this.finish_effects_adjustment(Adjustment::Keyboard, window, cx);
                    this.persist_options(cx, 0);
                    cx.notify();
                }
            }))
            .w(px(330.))
            .max_w_full()
            .p(px(26.))
            .gap(px(12.))
            .rounded(px(14.))
            .bg(rgb(0x202236))
            .border_1()
            .border_color(rgb(0x756176))
            .child(
                div()
                    .text_size(px(28.))
                    .font_semibold()
                    .text_color(rgb(GOLD))
                    .child("Rooftops"),
            )
            .child(self.volume_row(Volume::Music, "Music", window, cx))
            .child(self.volume_row(Volume::Effects, "Sound effects", window, cx))
            .child(self.option_row("rooftops-ghost", "Ghost cat", Preference::Ghost, window, cx))
            .child(self.option_row(
                "rooftops-animations",
                "Animations",
                Preference::Animations,
                window,
                cx,
            ));
        if let Some(message) = self.audio.error() {
            card = card.child(
                div()
                    .text_sm()
                    .text_color(rgb(0xffbdab))
                    .child(message.to_owned()),
            );
        }
        if let Some(message) = &self.preferences_message {
            card = card.child(
                div()
                    .text_sm()
                    .text_color(rgb(0xffbdab))
                    .child(message.clone()),
            );
        }
        card = card.child(
            score_button("rooftops-continue", "Continue", cx)
                .debug_selector(|| "rooftops-continue".into())
                .w_full()
                .on_click(cx.listener(Self::toggle_options)),
        );
        scene = scene.child(
            div()
                .absolute()
                .inset_0()
                .occlude()
                .bg(rgb(SKY).opacity(0.82))
                .flex()
                .items_center()
                .justify_center()
                .p(px(20.))
                .on_mouse_down(gpui_kit::MouseButton::Left, |_, _, cx| {
                    cx.stop_propagation()
                })
                .child(card.focus_trap("rooftops-options-trap", &self.options_focus)),
        );
        scene
    }
}

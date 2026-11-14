use super::*;
use crate::settings::UI_FONT_SIZES;
use gpui_kit::{
    component::{Selectable as _, button::ButtonVariants as _, slider::Slider},
    relative,
};

const MIN_SIZE: u8 = UI_FONT_SIZES[0];
const MAX_SIZE: u8 = UI_FONT_SIZES[UI_FONT_SIZES.len() - 1];
const SIZE_STEP: u8 = UI_FONT_SIZES[1] - MIN_SIZE;

fn stop_position(points: u8) -> f32 {
    f32::from(points - MIN_SIZE) / f32::from(MAX_SIZE - MIN_SIZE)
}

pub(super) fn init_keybindings(cx: &mut App) {
    // Use the window's view so the shortcuts also work when no control has focus.
    cx.intercept_keystrokes(|event, window, cx| {
        let modifiers = event.keystroke.modifiers;
        if !crate::theme::primary_modifier(modifiers) || modifiers.number_of_modifiers() != 1 {
            return;
        }
        let direction = match event.keystroke.key.as_str() {
            "=" => 1.,
            "-" => -1.,
            _ => return,
        };
        let Some(view) = window
            .root::<gpui_kit::base::Root>()
            .flatten()
            .and_then(|root| root.read(cx).view().clone().downcast::<UploaderApp>().ok())
        else {
            return;
        };
        view.update(cx, |app, cx| {
            if !app.recording_global_shortcut {
                app.step_interface_size(direction, window, cx);
                cx.stop_propagation();
            }
        });
    })
    .detach();
}

impl UploaderApp {
    fn step_interface_size(&self, direction: f32, window: &mut Window, cx: &mut Context<Self>) {
        self.ui_size_slider.update(cx, |slider, cx| {
            let current = slider.value().end();
            let next = (current + direction * slider.step_value())
                .clamp(slider.min_value(), slider.max_value());
            if current != next {
                slider.set_value(next, window, cx);
            }
        });
    }

    pub(super) fn new_ui_size_slider(
        points: u8,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<SliderState> {
        let slider = cx.new(|_| {
            SliderState::new()
                .min(f32::from(MIN_SIZE))
                .max(f32::from(MAX_SIZE))
                .step(f32::from(SIZE_STEP))
                .default_value(f32::from(points))
        });
        cx.observe_in(&slider, window, |this, slider, window, cx| {
            let state = slider.read(cx);
            let points = state.value().end() as u8;
            // GPUI rounds the value during dragging, but moves the thumb
            // continuously unless we synchronize it with the selected stop.
            if state.percentage().end != stop_position(points) {
                slider.update(cx, |state, cx| {
                    state.set_value(f32::from(points), window, cx);
                });
            }
            if this.settings.ui_font_size == points {
                return;
            }
            this.settings.ui_font_size = points;
            theme::set_ui_font_size(points, cx);
            if this.persist_preferences
                && let Err(error) = this.settings.save()
            {
                window.push_notification(
                    Notification::error(format!("Could not save interface size: {error}")),
                    cx,
                );
            }
            cx.notify();
        })
        .detach();
        slider
    }

    pub(super) fn render_ui_size_setting(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_2()
            .rounded_lg()
            .border_1()
            .border_color(cx.theme().border)
            .p_4()
            .child(div().font_semibold().text_sm().child("Base font size"))
            .child(
                v_flex()
                    .w_full()
                    .max_w(SETTINGS_CONTROL_WIDTH)
                    .self_start()
                    .px_6()
                    .child(
                        Slider::new(&self.ui_size_slider)
                            .bg(cx.theme().primary)
                            .text_color(cx.theme().primary_foreground)
                            .accessibility_label("Base font size in points"),
                    )
                    .child(div().relative().h_7().children(UI_FONT_SIZES.map(|points| {
                        let slider = self.ui_size_slider.clone();
                        v_flex()
                            .absolute()
                            .left(relative(stop_position(points)))
                            .ml(-rems(1.5))
                            .w_12()
                            .child(
                                div()
                                    .w_px()
                                    .h_1()
                                    .self_center()
                                    .bg(cx.theme().muted_foreground.opacity(0.55)),
                            )
                            .child(
                                Button::new(("interface-size", u32::from(points)))
                                    .debug_selector(move || format!("interface-size-{points}"))
                                    .ghost()
                                    .xsmall()
                                    .w_full()
                                    .tab_stop(true)
                                    .selected(self.settings.ui_font_size == points)
                                    .label(format!("{points} pt"))
                                    .on_click(move |_, window, cx| {
                                        slider.update(cx, |state, cx| {
                                            state.set_value(f32::from(points), window, cx);
                                        });
                                    }),
                            )
                    }))),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::{Modifiers, TestAppContext};

    #[gpui_kit::test]
    fn interface_size_shortcuts_use_the_same_stops_on_both_pages(cx: &mut TestAppContext) {
        let (view, cx) = super::super::tests::upload_window(cx);
        cx.update(|window, cx| {
            super::super::init_keybindings(cx);
            window.activate_window();
            view.update(cx, |app, cx| app.focus_handle.focus(window, cx));
            window.draw(cx).clear(cx);
        });
        cx.simulate_keystrokes("cmd-= cmd-=");
        cx.update(|window, cx| {
            assert_eq!(view.read(cx).settings.ui_font_size, 20);
            view.update(cx, |app, cx| {
                app.show_settings(cx);
                app.owner_input
                    .update(cx, |input, cx| input.focus(window, cx));
            });
            window.draw(cx).clear(cx);
        });
        cx.simulate_keystrokes("cmd-= cmd-= cmd-=");
        cx.update(|window, cx| {
            assert_eq!(view.read(cx).settings.ui_font_size, 24);
            assert_eq!(view.read(cx).ui_size_slider.read(cx).value().end(), 24.);
            window.blur(cx);
        });
        cx.simulate_keystrokes("cmd-- cmd-- cmd-- cmd-- cmd--");
        cx.update(|_, cx| {
            assert_eq!(view.read(cx).settings.ui_font_size, 16);
            assert_eq!(view.read(cx).ui_size_slider.read(cx).value().end(), 16.);
        });
    }

    #[gpui_kit::test]
    fn interface_size_snaps_and_rescales_the_window_before_mouse_up(cx: &mut TestAppContext) {
        let (view, cx) = super::super::tests::upload_window(cx);
        cx.update(|window, cx| {
            view.update(cx, |app, cx| app.show_settings(cx));
            window.draw(cx).clear(cx);
        });
        let slider = cx.update(|_, cx| view.read(cx).ui_size_slider.clone());
        let track = cx.update(|_, cx| slider.read(cx).bounds());
        let mods = Modifiers::default();
        let start = point(track.left() + track.size.width * 0.30, track.center().y);
        cx.simulate_mouse_down(start, MouseButton::Left, mods);
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            assert_eq!(window.rem_size(), px(18.));
            assert_eq!(slider.read(cx).percentage().end, 0.25);
        });
        let track = cx.update(|_, cx| slider.read(cx).bounds());
        let end = point(track.left() + track.size.width * 0.78, track.center().y);
        cx.simulate_mouse_move(end, MouseButton::Left, mods);
        cx.simulate_mouse_move(end + point(px(1.), px(0.)), MouseButton::Left, mods);
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            assert_eq!(window.rem_size(), px(22.));
            assert_eq!(slider.read(cx).percentage().end, 0.75);
        });
        cx.simulate_mouse_up(end, MouseButton::Left, mods);

        let largest = cx.debug_bounds("interface-size-24").unwrap();
        let previous = cx.debug_bounds("interface-size-16").unwrap();
        cx.simulate_click(largest.center(), mods);
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            assert_eq!(window.rem_size(), px(24.));
            assert_eq!(view.read(cx).settings.ui_font_size, 24);
        });
        let smallest = cx.debug_bounds("interface-size-16").unwrap();
        assert!(smallest.size.height > previous.size.height);
        cx.simulate_click(smallest.center(), mods);
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            assert_eq!(window.rem_size(), px(16.));
            assert_eq!(view.read(cx).settings.ui_font_size, 16);
        });
    }
}

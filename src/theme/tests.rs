use super::*;
use gpui_kit::base::Root;
use gpui_kit::component::{Disableable as _, v_flex};
use gpui_kit::{AppContext as _, Context, Modifiers, Render, TestAppContext};

struct HoverControls;
impl Render for HoverControls {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl gpui_kit::IntoElement {
        v_flex()
            .p_4()
            .gap_4()
            .items_start()
            .child(
                Button::new("hover-button")
                    .app_secondary()
                    .label("Button")
                    .debug_selector(|| "hover-button".into()),
            )
            .child(
                AppCheckbox::new("hover-checkbox")
                    .accessibility_label("Checkbox")
                    .debug_selector(|| "hover-checkbox".into()),
            )
            .child(
                AppCheckbox::new("checked-checkbox")
                    .checked(true)
                    .accessibility_label("Checked")
                    .debug_selector(|| "checked-checkbox".into()),
            )
            .child(
                AppCheckbox::new("disabled-checkbox")
                    .disabled(true)
                    .accessibility_label("Disabled")
                    .debug_selector(|| "disabled-checkbox".into()),
            )
            .child(
                Button::new("disabled-button")
                    .app_secondary()
                    .label("Disabled button")
                    .disabled(true)
                    .debug_selector(|| "disabled-button".into()),
            )
    }
}

#[gpui_kit::test]
fn hover_brightens_control_borders_and_backgrounds_without_affecting_disabled_controls(
    cx: &mut TestAppContext,
) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        apply_dark_theme(cx);
        cx.set_reduce_motion(true);
    });
    let (_, cx) = cx.add_window_view(|window, cx| {
        let view = cx.new(|_| HoverControls);
        Root::new(view, window, cx)
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    for selector in [
        "hover-button",
        "hover-checkbox",
        "checked-checkbox",
        "disabled-checkbox",
        "disabled-button",
    ] {
        let bounds = cx.debug_bounds(selector).unwrap();
        cx.simulate_mouse_move(bounds.center(), None, Modifiers::default());
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let quads = window
                .painted_quads()
                .into_iter()
                .filter(|quad| {
                    quad.bounds.map(|value| px(value.0 / window.scale_factor())) == bounds
                })
                .collect::<Vec<_>>();
            let highlighted = quads
                .iter()
                .any(|quad| quad.border_color == rgb(HOVER_BORDER).into());
            assert_eq!(highlighted, !selector.starts_with("disabled"), "{selector}");
            if matches!(selector, "hover-button" | "hover-checkbox") {
                assert!(
                    quads
                        .iter()
                        .any(|quad| quad.background == rgb(HOVER_BACKGROUND).into()),
                    "{selector} must also brighten its fill"
                );
            }
        });
    }
}

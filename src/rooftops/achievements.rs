use super::*;
use gpui_kit::base::FocusTrapElement as _;
use gpui_kit::component::scroll::ScrollableElement as _;

mod artwork;

impl Rooftops {
    pub(super) fn open_achievements(
        &mut self,
        _: &gpui_kit::ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.achievements_open = true;
        self.last_frame = None;
        window.focus(&self.achievements_focus, cx);
        cx.notify();
    }

    pub(super) fn close_achievements(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.achievements_open = false;
        self.last_frame = None;
        window.focus(&self.focus, cx);
        cx.notify();
    }

    pub(super) fn with_achievements(
        &self,
        scene: gpui_kit::Stateful<gpui_kit::Div>,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> gpui_kit::Stateful<gpui_kit::Div> {
        let unlocked = Achievement::ALL
            .iter()
            .filter(|&&award| self.achievements.contains(award))
            .count();
        let columns = if window.viewport_size().width >= px(510.) {
            2
        } else {
            1
        };
        let mut grid = v_flex().gap_3().pb_2();
        for row in Achievement::ALL.chunks(columns) {
            let mut cards = h_flex().gap_3().items_stretch();
            for &award in row {
                cards = cards.child(award_card(award, self.achievements.contains(award)));
            }
            grid = grid.child(cards);
        }

        let panel = v_flex()
            .id("rooftops-achievements-modal")
            .track_focus(&self.achievements_focus)
            .w(px(610.))
            .h(px(536.))
            .max_w_full()
            .max_h_full()
            .p(px(20.))
            .gap_4()
            .rounded(px(16.))
            .bg(rgb(0x181b2c))
            .border_1()
            .border_color(rgb(0x75627a))
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .gap_3()
                    .flex_shrink_0()
                    .child(
                        v_flex()
                            .gap_1()
                            .child(
                                div()
                                    .font_family("Georgia")
                                    .font_bold()
                                    .text_size(px(29.))
                                    .text_color(rgb(GOLD))
                                    .child("Achievements"),
                            )
                            .child(div().text_sm().text_color(rgb(0xc4bdd5)).child(format!(
                                "{unlocked} of {} unlocked",
                                Achievement::ALL.len()
                            ))),
                    )
                    .child(
                        menu::menu_button("rooftops-achievements-close", "Close", cx).on_click(
                            cx.listener(|this, _, window, cx| this.close_achievements(window, cx)),
                        ),
                    ),
            )
            .child(
                v_flex()
                    .id("rooftops-achievements-grid")
                    .debug_selector(|| "rooftops-achievements-grid".into())
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scrollbar()
                    .child(grid),
            );

        scene.child(
            div()
                .absolute()
                .inset_0()
                .occlude()
                .bg(rgb(SKY).opacity(0.88))
                .flex()
                .items_center()
                .justify_center()
                .p(px(16.))
                .on_mouse_down(gpui_kit::MouseButton::Left, |_, _, cx| {
                    cx.stop_propagation()
                })
                .child(panel.focus_trap("rooftops-achievements-trap", &self.achievements_focus)),
        )
    }
}

fn award_card(award: Achievement, unlocked: bool) -> gpui_kit::Div {
    let art = artwork::medal(award, unlocked);
    let illustration = gpui_kit::canvas(
        |_, _, _| (),
        move |bounds, _, window, _| art.paint_at(bounds.origin, [0., 0.], 1., window),
    )
    .w(px(artwork::WIDTH))
    .h(px(artwork::HEIGHT))
    .flex_shrink_0();

    let status = div()
        .mt_auto()
        .px_2()
        .py_0p5()
        .rounded_full()
        .bg(rgb(if unlocked { 0xf0c17d } else { 0x2a2f45 }))
        .text_xs()
        .font_semibold()
        .text_color(rgb(if unlocked { 0x2e2031 } else { 0xaeb4ce }))
        .child(if unlocked { "Unlocked" } else { "Locked" });

    v_flex()
        .flex_1()
        .min_w_0()
        .gap_1()
        .items_center()
        .text_center()
        .rounded(px(10.))
        .bg(rgb(if unlocked { 0x342c3e } else { 0x202337 }))
        .border_1()
        .border_color(rgb(if unlocked { 0xc79765 } else { 0x373b51 }))
        .p_3()
        .child(illustration)
        .child(
            div()
                .font_semibold()
                .text_size(px(15.))
                .text_color(rgb(if unlocked { 0xffe1b1 } else { 0xe1d9e9 }))
                .child(award.label()),
        )
        .child(
            div()
                .text_size(px(12.))
                .line_height(px(16.))
                .text_color(rgb(0xd0c7db))
                .child(award.description()),
        )
        .child(status)
}

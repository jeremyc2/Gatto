use gpui_kit::{
    App, Context, FocusHandle, Focusable, InteractiveElement as _, IntoElement, ParentElement as _,
    Render, Styled as _, Window,
    component::scroll::ScrollableElement as _,
    component::{
        ActiveTheme as _, Sizable as _, StyledExt as _, WindowExt as _, button::Button,
        clipboard::Clipboard, h_flex, notification::Notification, v_flex,
    },
    div,
};

use crate::diagnostics::Diagnostics;

pub struct LogViewer {
    diagnostics: Diagnostics,
    focus_handle: FocusHandle,
    lines: Vec<String>,
}

impl LogViewer {
    pub fn new(diagnostics: Diagnostics, cx: &mut Context<Self>) -> Self {
        let lines = diagnostics.lines();
        Self {
            diagnostics,
            focus_handle: cx.focus_handle(),
            lines,
        }
    }

    fn refresh(&mut self, _: &gpui_kit::ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.lines = self.diagnostics.lines();
        cx.notify();
    }
}

impl Focusable for LogViewer {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for LogViewer {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entries = if self.lines.is_empty() {
            vec!["No diagnostic events have been recorded yet.".to_owned()]
        } else {
            self.lines.clone()
        };
        let log_text = self.diagnostics.text();
        let diagnostics = self.diagnostics.clone();
        let view = cx.entity().downgrade();

        v_flex()
            .track_focus(&self.focus_handle)
            .size_full()
            .min_w_0()
            .overflow_hidden()
            .bg(cx.theme().background)
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .flex_wrap()
                    .gap_x_4()
                    .gap_y_3()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .px_5()
                    .py_4()
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .gap_1()
                            .child(div().text_lg().font_semibold().child("Application logs"))
                            .child(
                                div()
                                    .max_w_full()
                                    .min_w_0()
                                    .text_xs()
                                    .whitespace_normal()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("Stored only while the app is running. Tokens and image data are excluded."),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                Button::new("refresh-logs")
                                    .small()
                                    .outline()
                                    .label("Refresh")
                                    .on_click(cx.listener(Self::refresh)),
                            )
                            .child(
                                Clipboard::new("copy-all-logs")
                                    .small()
                                    .value(log_text)
                                    .tooltip("Copy all application logs")
                                    .accessibility_label("Copy all application logs")
                                    .on_copied(move |_, window, cx| {
                                        diagnostics
                                            .info("Copied application log to the clipboard.");
                                        let lines = diagnostics.lines();
                                        let _ = view.update(cx, |this, cx| {
                                            this.lines = lines;
                                            cx.notify();
                                        });
                                        window.push_notification(
                                            Notification::success(
                                                "Application logs copied to the clipboard.",
                                            ),
                                            cx,
                                        );
                                    }),
                            ),
                    ),
            )
            .child(
                v_flex()
                    .id("application-log-content")
                    .flex_1()
                    .min_w_0()
                    .min_h_0()
                    .overflow_x_hidden()
                    .p_5()
                    .gap_1()
                    .font_family(cx.theme().mono_font_family.clone())
                    .text_xs()
                    .overflow_y_scrollbar()
                    .children(entries.into_iter().map(|entry| {
                        div()
                            .w_full()
                            .max_w_full()
                            .min_w_0()
                            .whitespace_normal()
                            .rounded_sm()
                            .bg(cx.theme().muted.opacity(0.18))
                            .px_2()
                            .py_1()
                            .child(entry)
                    })),
            )
    }
}

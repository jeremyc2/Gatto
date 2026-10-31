use gpui_kit::component::{ActiveTheme as _, Disableable, Icon, checkbox::Checkbox};
use gpui_kit::{
    App, ElementId, InteractiveElement, Interactivity, IntoElement, ParentElement as _, RenderOnce,
    SharedString, StyleRefinement, Styled, Window, div, prelude::FluentBuilder as _, px, rgb,
};

/// Keep the shared checkbox's semantics and focus treatment, adding the app's
/// hover colors over its indicator without changing its label or hit target.
#[derive(IntoElement)]
pub struct AppCheckbox {
    inner: Checkbox,
    checked: bool,
    labeled: bool,
    disabled: bool,
}

impl AppCheckbox {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            inner: Checkbox::new(id),
            checked: false,
            labeled: false,
            disabled: false,
        }
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self.inner = self.inner.checked(checked);
        self
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.labeled = true;
        self.inner = self.inner.label(label.into());
        self
    }

    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.inner = self.inner.accessibility_label(label);
        self
    }

    pub fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.inner = self.inner.tooltip(tooltip);
        self
    }

    pub fn on_click(mut self, callback: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.inner = self.inner.on_click(callback);
        self
    }
}

impl Disableable for AppCheckbox {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self.inner = self.inner.disabled(disabled);
        self
    }
}

impl InteractiveElement for AppCheckbox {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.inner.interactivity()
    }
}

impl Styled for AppCheckbox {
    fn style(&mut self) -> &mut StyleRefinement {
        self.inner.style()
    }
}

impl RenderOnce for AppCheckbox {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let fill = if self.checked {
            cx.theme().primary_hover
        } else {
            rgb(super::HOVER_BACKGROUND).into()
        };
        let foreground = cx.theme().primary_foreground;
        div()
            .relative()
            .min_w_0()
            .group("app-checkbox-hover")
            .child(self.inner)
            .when(!self.disabled, |root| {
                root.child(
                    div()
                        .absolute()
                        .left_0()
                        .top(px(if self.labeled { 2. } else { 0. }))
                        .size_4()
                        .rounded(px(4.))
                        .border_1()
                        .border_color(gpui_kit::transparent_black())
                        .text_color(gpui_kit::transparent_black())
                        .group_hover("app-checkbox-hover", |indicator| {
                            indicator
                                .bg(fill)
                                .border_color(rgb(super::HOVER_BORDER))
                                .text_color(foreground)
                        })
                        .when(self.checked, |indicator| {
                            indicator.child(
                                div()
                                    .absolute()
                                    .left_px()
                                    .top_px()
                                    .size(px(12.))
                                    .opacity(0.)
                                    .group_hover("app-checkbox-hover", |mark| mark.opacity(1.))
                                    .child(
                                        Icon::new(gpui_kit::assets::IconName::Check)
                                            .size(px(12.))
                                            .text_color(foreground),
                                    ),
                            )
                        }),
                )
            })
    }
}

use gpui_kit::component::{
    ActiveTheme as _, Disableable, Icon, Selectable, Sizable, Size,
    button::{Button, ButtonCustomVariant, ButtonVariant, ButtonVariants},
    menu::DropdownMenu,
};
use gpui_kit::{
    AnyElement, App, ClickEvent, InteractiveElement, Interactivity, IntoElement, ParentElement,
    RenderOnce, SharedString, StyleRefinement, Styled, Window,
};

/// Apply the app's button colors while retaining the component's state handling.
#[derive(IntoElement)]
pub struct AppButton {
    inner: Button,
    disabled: bool,
    native_hover: bool,
}

impl AppButton {
    pub(super) fn new(inner: Button, native_hover: bool) -> Self {
        Self {
            inner,
            disabled: false,
            native_hover,
        }
    }
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.inner = self.inner.label(label);
        self
    }
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.inner = self.inner.icon(Into::<Icon>::into(icon));
        self
    }
    pub fn tooltip(mut self, text: impl Into<SharedString>) -> Self {
        self.inner = self.inner.tooltip(text);
        self
    }
    pub fn accessibility_label(mut self, text: impl Into<SharedString>) -> Self {
        self.inner = self.inner.accessibility_label(text);
        self
    }
    pub fn loading(mut self, loading: bool) -> Self {
        self.inner = self.inner.loading(loading);
        self
    }
    pub fn on_click(
        mut self,
        callback: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.inner = self.inner.on_click(callback);
        self
    }
}

impl Disableable for AppButton {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self.inner = self.inner.disabled(disabled);
        self
    }
}
impl Sizable for AppButton {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.inner = self.inner.with_size(size);
        self
    }
}
impl ButtonVariants for AppButton {
    fn with_variant(mut self, variant: ButtonVariant) -> Self {
        self.native_hover = true;
        self.inner = self.inner.with_variant(variant);
        self
    }
}
impl Selectable for AppButton {
    fn selected(mut self, selected: bool) -> Self {
        self.inner = self.inner.selected(selected);
        self
    }
    fn is_selected(&self) -> bool {
        self.inner.is_selected()
    }
    fn open(mut self, open: bool) -> Self {
        self.inner = self.inner.open(open);
        self
    }
    fn is_open(&self) -> bool {
        self.inner.is_open()
    }
}
impl InteractiveElement for AppButton {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.inner.interactivity()
    }
}
impl Styled for AppButton {
    fn style(&mut self) -> &mut StyleRefinement {
        self.inner.style()
    }
}
impl ParentElement for AppButton {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.inner.extend(elements);
    }
}
impl DropdownMenu for AppButton {}

impl RenderOnce for AppButton {
    fn render(mut self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        if !self.native_hover {
            let colors = ButtonCustomVariant::new(cx)
                .color(cx.theme().input)
                .foreground(cx.theme().foreground)
                .hover(gpui_kit::rgb(super::HOVER_BACKGROUND).into())
                .active(cx.theme().primary_active);
            self.inner = self.inner.custom(colors);
            if !self.disabled {
                self.inner = self
                    .inner
                    .bg(cx.theme().input_background())
                    .border_color(cx.theme().input);
            }
        }
        self.inner
    }
}

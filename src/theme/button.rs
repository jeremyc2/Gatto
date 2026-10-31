use gpui_kit::component::{
    ActiveTheme as _, Disableable, Icon, Selectable, Sizable, Size,
    button::{Button, ButtonCustomVariant, ButtonVariant, ButtonVariants},
    menu::DropdownMenu,
};
use gpui_kit::{
    AnyElement, App, Bounds, ClickEvent, Corners, Element, ElementId, GlobalElementId, Hitbox,
    InspectorElementId, InteractiveElement, Interactivity, IntoElement, LayoutId, ParentElement,
    Pixels, RenderOnce, SharedString, StyleRefinement, Styled, Window, px,
};

/// Preserve the shared button's behavior and layout, and paint the app's stronger
/// hover border while leaving the component's state handling intact.
#[derive(IntoElement)]
pub struct AppButton {
    inner: Button,
    disabled: bool,
    loading: bool,
    native_hover: bool,
}

impl AppButton {
    pub(super) fn new(inner: Button, native_hover: bool) -> Self {
        Self {
            inner,
            disabled: false,
            loading: false,
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
        self.loading = loading;
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
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
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
        let corners = &self.inner.style().corner_radii;
        let radius = cx.theme().radius.into();
        let corners = Corners {
            top_left: corners.top_left.unwrap_or(radius),
            top_right: corners.top_right.unwrap_or(radius),
            bottom_left: corners.bottom_left.unwrap_or(radius),
            bottom_right: corners.bottom_right.unwrap_or(radius),
        }
        .to_pixels(window.rem_size());
        HoverBorder {
            inner: self.inner.into_any_element(),
            enabled: !self.disabled && !self.loading,
            corners,
        }
    }
}

/// An element decorator adds no layout node or click handler around the button.
struct HoverBorder {
    inner: AnyElement,
    enabled: bool,
    corners: Corners<Pixels>,
}
impl IntoElement for HoverBorder {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl Element for HoverBorder {
    type RequestLayoutState = ();
    type PrepaintState = Hitbox;
    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        (self.inner.request_layout(window, cx), ())
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> Hitbox {
        let hitbox = window.insert_hitbox(bounds, gpui_kit::HitboxBehavior::Normal);
        self.inner.prepaint(window, cx);
        hitbox
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        hitbox: &mut Hitbox,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.inner.paint(window, cx);
        if self.enabled && hitbox.is_hovered(window) {
            window.paint_quad(gpui_kit::quad(
                bounds,
                self.corners,
                gpui_kit::transparent_black(),
                px(1.),
                gpui_kit::rgb(super::HOVER_BORDER),
                gpui_kit::BorderStyle::default(),
            ));
        }
    }
}

use gpui_kit::component::{
    ActiveTheme as _, Sizable as _, Size, Theme, ThemeMode,
    button::{Button, ButtonVariant, ButtonVariants as _},
    dialog::{DialogAction, DialogClose, DialogFooter},
};
use gpui_kit::{
    App, Hsla, InteractiveElement as _, IntoElement as _, ParentElement as _, Styled as _, Window,
    div, px, rgb,
};

mod button;
mod checkbox;
pub use button::AppButton;
pub use checkbox::AppCheckbox;

pub(super) const HOVER_BACKGROUND: u32 = 0x2b3345;
pub(super) const HOVER_BORDER: u32 = 0xb0a6f7;

#[cfg(test)]
mod tests;

/// Opens every app window with shared native title-bar colors and a sidebar-style divider.
pub fn open_window<V: gpui_kit::Render>(
    options: gpui_kit::WindowOptions,
    cx: &mut App,
    build: impl FnOnce(&mut Window, &mut App) -> gpui_kit::Entity<V>,
) -> anyhow::Result<(gpui_kit::AnyWindowHandle, gpui_kit::Entity<V>)> {
    gpui_kit::open_window(options, cx, |window, cx| {
        let view = build(window, cx);
        crate::window_limits::style_window(&view, window, cx);
        view
    })
}

/// App actions share the landing page's geometry and outlined secondary style.
/// Annotation tools keep their own compact, selectable treatment.
pub trait AppButtonStyle {
    fn app_primary(self) -> AppButton;
    fn app_secondary(self) -> AppButton;
    fn app_icon(self) -> AppButton;
}

impl AppButtonStyle for Button {
    fn app_primary(self) -> AppButton {
        AppButton::new(
            self.primary().with_size(Size::Medium).min_w(px(80.)).px_3(),
            true,
        )
    }

    fn app_secondary(self) -> AppButton {
        AppButton::new(
            self.with_variant(ButtonVariant::Default)
                .outline()
                .with_size(Size::Medium)
                .min_w(px(80.))
                .px_3(),
            false,
        )
    }

    fn app_icon(self) -> AppButton {
        AppButton::new(
            self.with_variant(ButtonVariant::Default).outline().xsmall(),
            false,
        )
    }
}

/// Validation belongs to the dialog that owns the action, above its backdrop.
#[derive(Clone, Default)]
pub struct DialogFeedback(std::rc::Rc<std::cell::RefCell<Option<String>>>);

impl DialogFeedback {
    pub fn show(&self, error: impl std::fmt::Display, window: &mut Window) {
        *self.0.borrow_mut() = Some(error.to_string());
        window.refresh();
    }

    pub fn clear(&self, window: &mut Window) {
        if self.0.borrow_mut().take().is_some() {
            window.refresh();
        }
    }

    pub fn element(&self, cx: &App) -> Option<gpui_kit::AnyElement> {
        self.0.borrow().as_ref().map(|message| {
            div()
                .id("dialog-validation-error")
                .debug_selector(|| "dialog-validation-error".into())
                .text_sm()
                .text_color(cx.theme().danger)
                .child(message.clone())
                .into_any_element()
        })
    }
}

/// Fixed slots keep Base's full-size dialog wrappers from stretching the buttons.
pub fn confirmation_footer(id: &'static str, label: &'static str, width: f32) -> DialogFooter {
    DialogFooter::new()
        .child(
            div().w(px(88.)).h(px(32.)).flex_none().child(
                DialogClose::new().child(
                    Button::new("cancel")
                        .app_secondary()
                        .w(px(88.))
                        .h(px(32.))
                        .label("Cancel"),
                ),
            ),
        )
        .child(
            div().w(px(width)).h(px(32.)).flex_none().child(
                DialogAction::new().child(
                    Button::new(id)
                        .debug_selector(move || id.into())
                        .app_primary()
                        .w(px(width))
                        .h(px(32.))
                        .label(label),
                ),
            ),
        )
}

fn color(hex: u32) -> Hsla {
    rgb(hex).into()
}

/// Applies the production color tokens used by both the live app and documentation renderer.
pub fn apply_dark_theme(cx: &mut gpui_kit::App) {
    Theme::change(ThemeMode::Dark, None, cx);
    Theme::update(cx, |theme| {
        theme.background = color(0x0e1017);
        theme.border = color(0x2b2f45);
        theme.sidebar = color(0x171b25);
        theme.sidebar_accent = color(0x252b3b);
        theme.sidebar_border = color(0x2b3345);
        theme.primary = color(0x6c63ff);
        theme.primary_hover = color(0x8279ff);
        theme.primary_active = color(0x574edb);
        theme.primary_foreground = color(0xffffff);
        theme.button_primary = color(0x6c63ff);
        theme.button_primary_hover = color(0x8279ff);
        theme.button_hover = color(HOVER_BACKGROUND);
        theme.secondary_hover = color(HOVER_BACKGROUND);
        theme.button_primary_active = color(0x574edb);
        theme.button_primary_foreground = color(0xffffff);
        theme.ring = color(0x8b7bff);
        theme.link = color(0x7fb2ff);
        theme.success = color(0x3ddc97);
        theme.warning = color(0xffb454);
        theme.danger = color(0xff6b81);
    });
}

/// Forces native macOS surfaces to use the same dark appearance as the GPUI content.
#[cfg(target_os = "macos")]
pub fn force_dark_appearance() {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSAppearance, NSAppearanceNameDarkAqua, NSApplication};

    if let Some(main_thread) = MainThreadMarker::new() {
        let appearance = unsafe { NSAppearance::appearanceNamed(NSAppearanceNameDarkAqua) };
        NSApplication::sharedApplication(main_thread).setAppearance(appearance.as_deref());
    }
}

#[cfg(not(target_os = "macos"))]
pub fn force_dark_appearance() {}

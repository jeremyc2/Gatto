use gpui_kit::component::{
    Sizable as _, Size, Theme, ThemeMode,
    button::{Button, ButtonVariant, ButtonVariants as _},
    dialog::{DialogAction, DialogClose, DialogFooter},
};
use gpui_kit::{Hsla, ParentElement as _, Styled as _, div, px, rgb};

/// App actions share the landing page's geometry and outlined secondary style.
/// Annotation tools keep their own compact, selectable treatment.
pub trait AppButtonStyle {
    fn app_primary(self) -> Self;
    fn app_secondary(self) -> Self;
    fn app_icon(self) -> Self;
}

impl AppButtonStyle for Button {
    fn app_primary(self) -> Self {
        self.primary().with_size(Size::Medium).min_w(px(80.)).px_3()
    }

    fn app_secondary(self) -> Self {
        self.with_variant(ButtonVariant::Default)
            .outline()
            .with_size(Size::Medium)
            .min_w(px(80.))
            .px_3()
    }

    fn app_icon(self) -> Self {
        self.with_variant(ButtonVariant::Default).outline().xsmall()
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
        theme.primary = color(0x6c63ff);
        theme.primary_hover = color(0x8279ff);
        theme.primary_active = color(0x574edb);
        theme.primary_foreground = color(0xffffff);
        theme.button_primary = color(0x6c63ff);
        theme.button_primary_hover = color(0x8279ff);
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

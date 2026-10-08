use anyhow::{Context as _, Result};
use tray_icon::{
    Icon, TrayIcon, TrayIconBuilder,
    menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuAction {
    Open,
    Settings,
    Quit,
}

pub struct MenuBar {
    _tray: TrayIcon,
    open_id: MenuId,
    settings_id: MenuId,
    quit_id: MenuId,
}

impl MenuBar {
    pub fn install() -> Result<(Self, async_channel::Receiver<MenuEvent>)> {
        let menu = Menu::new();
        let open = MenuItem::new("Open App", true, None);
        let settings = MenuItem::new("App Preferences", true, None);
        let separator = PredefinedMenuItem::separator();
        let quit = MenuItem::new("Quit", true, None);
        menu.append_items(&[&open, &settings, &separator, &quit])
            .context("Could not create the menu bar menu")?;

        let icon = app_glyph_icon()?;
        let tray_builder = TrayIconBuilder::new()
            .with_tooltip("GitHub Image Upload")
            .with_menu(Box::new(menu));
        #[cfg(target_os = "macos")]
        let tray_builder = tray_builder.with_icon_templated(icon);
        #[cfg(not(target_os = "macos"))]
        let tray_builder = tray_builder.with_icon(icon);
        let tray = tray_builder
            .build()
            .context("Could not create the menu bar icon")?;

        let (sender, receiver) = async_channel::unbounded();
        MenuEvent::set_event_handler(Some(move |event| {
            let _ = sender.try_send(event);
        }));

        Ok((
            Self {
                _tray: tray,
                open_id: open.id().clone(),
                settings_id: settings.id().clone(),
                quit_id: quit.id().clone(),
            },
            receiver,
        ))
    }

    pub fn action_for(&self, event: &MenuEvent) -> Option<MenuAction> {
        if event.id == self.open_id {
            Some(MenuAction::Open)
        } else if event.id == self.settings_id {
            Some(MenuAction::Settings)
        } else if event.id == self.quit_id {
            Some(MenuAction::Quit)
        } else {
            None
        }
    }
}

fn app_glyph_icon() -> Result<Icon> {
    static GLYPH: &[u8] = include_bytes!("../packaging/menu-bar-icon.png");

    let glyph = ::image::load_from_memory_with_format(GLYPH, ::image::ImageFormat::Png)
        .context("Could not decode the menu bar icon")?
        .into_rgba8();
    let (width, height) = glyph.dimensions();
    Icon::from_rgba(glyph.into_raw(), width, height).context("Could not create the menu bar icon")
}

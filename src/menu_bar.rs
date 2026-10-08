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
        let settings = MenuItem::new("Settings…", true, None);
        let separator = PredefinedMenuItem::separator();
        let quit = MenuItem::new("Quit", true, None);
        menu.append_items(&[&open, &settings, &separator, &quit])
            .context("Could not create the menu bar menu")?;

        let icon = github_mark_icon()?;
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

fn github_mark_icon() -> Result<Icon> {
    const SIZE: u32 = 22;
    let mut rgba = vec![0_u8; (SIZE * SIZE * 4) as usize];
    let center = 10.5_f32;

    for y in 0..SIZE {
        for x in 0..SIZE {
            let xf = x as f32;
            let yf = y as f32;
            let head = ((xf - center) / 7.4).powi(2) + ((yf - 10.0) / 6.5).powi(2) <= 1.0;
            let left_ear = yf <= 6.0 && (4.0..=9.0).contains(&xf) && yf >= xf - 4.0;
            let right_ear = yf <= 6.0 && (12.0..=17.0).contains(&xf) && yf >= 17.0 - xf;
            let body = (13.0..=20.5).contains(&yf) && (xf - center).abs() <= 4.6;
            let left_leg = yf >= 18.0 && (5.0..=10.5).contains(&xf);
            let right_leg = yf >= 18.0 && (10.5..=16.0).contains(&xf);
            let tail =
                xf <= 6.0 && (13.0..=17.0).contains(&yf) && (yf - (17.8 - xf * 0.55)).abs() <= 1.4;
            if head || left_ear || right_ear || body || left_leg || right_leg || tail {
                let index = ((y * SIZE + x) * 4) as usize;
                rgba[index..index + 4].copy_from_slice(&[0, 0, 0, 255]);
            }
        }
    }

    Icon::from_rgba(rgba, SIZE, SIZE).context("Could not create the GitHub menu bar icon")
}

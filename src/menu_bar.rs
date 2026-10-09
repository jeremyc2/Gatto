use std::{cell::RefCell, rc::Rc};

use anyhow::{Context as _, Result};
use tray_icon::{
    Icon, TrayIcon, TrayIconBuilder,
    menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuAction {
    Open,
    PreviewFromClipboard,
    QuickCopy,
    Settings,
    Quit,
}

pub struct MenuBar {
    _tray: TrayIcon,
    menu: Menu,
    open_id: MenuId,
    paste_preview_id: MenuId,
    paste_url_id: MenuId,
    paste_items: [MenuItem; 2],
    paste_separator: PredefinedMenuItem,
    paste_actions_visible: bool,
    settings_id: MenuId,
    quit_id: MenuId,
}

impl MenuBar {
    pub fn install(
        paste_actions_visible: bool,
    ) -> Result<(Self, async_channel::Receiver<MenuEvent>)> {
        let menu = Menu::new();
        let open = MenuItem::new("Open App", true, None);
        let paste_preview = MenuItem::new("Preview from Clipboard", true, None);
        let paste_url = MenuItem::new("Quick Copy", true, None);
        let settings = MenuItem::new("App Preferences", true, None);
        let paste_separator = PredefinedMenuItem::separator();
        let settings_separator = PredefinedMenuItem::separator();
        let quit_separator = PredefinedMenuItem::separator();
        let quit = MenuItem::new("Quit", true, None);
        menu.append(&open)
            .context("Could not create the menu bar menu")?;
        if paste_actions_visible {
            menu.append_items(&[&paste_separator, &paste_preview, &paste_url])
                .context("Could not create the Paste menu items")?;
        }
        menu.append_items(&[&settings_separator, &settings, &quit_separator, &quit])
            .context("Could not create the menu bar menu")?;

        let icon = app_glyph_icon()?;
        let tray_builder = TrayIconBuilder::new()
            .with_tooltip("Gatto")
            .with_menu(Box::new(menu.clone()));
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
                menu,
                open_id: open.id().clone(),
                paste_preview_id: paste_preview.id().clone(),
                paste_url_id: paste_url.id().clone(),
                paste_items: [paste_preview, paste_url],
                paste_separator,
                paste_actions_visible,
                settings_id: settings.id().clone(),
                quit_id: quit.id().clone(),
            },
            receiver,
        ))
    }

    pub fn action_for(&self, event: &MenuEvent) -> Option<MenuAction> {
        if event.id == self.open_id {
            Some(MenuAction::Open)
        } else if event.id == self.paste_preview_id {
            Some(MenuAction::PreviewFromClipboard)
        } else if event.id == self.paste_url_id {
            Some(MenuAction::QuickCopy)
        } else if event.id == self.settings_id {
            Some(MenuAction::Settings)
        } else if event.id == self.quit_id {
            Some(MenuAction::Quit)
        } else {
            None
        }
    }

    fn set_paste_actions_visible(&mut self, visible: bool) {
        if self.paste_actions_visible == visible {
            return;
        }
        let result = if visible {
            self.menu.insert_items(
                &[
                    &self.paste_separator,
                    &self.paste_items[0],
                    &self.paste_items[1],
                ],
                1,
            )
        } else {
            self.menu
                .remove(&self.paste_separator)
                .and_then(|_| self.menu.remove(&self.paste_items[0]))
                .and_then(|_| self.menu.remove(&self.paste_items[1]))
        };
        if result.is_ok() {
            self.paste_actions_visible = visible;
        }
    }
}

/// A UI-thread handle that lets the application add or remove the paste action
/// group as the pinned-repository setting changes.
#[derive(Clone, Default)]
pub struct MenuBarController {
    menu_bar: Rc<RefCell<Option<MenuBar>>>,
}

impl MenuBarController {
    pub fn attach(&self, menu_bar: MenuBar) {
        *self.menu_bar.borrow_mut() = Some(menu_bar);
    }

    pub fn action_for(&self, event: &MenuEvent) -> Option<MenuAction> {
        self.menu_bar
            .borrow()
            .as_ref()
            .and_then(|menu_bar| menu_bar.action_for(event))
    }

    pub fn set_paste_actions_visible(&self, visible: bool) {
        if let Some(menu_bar) = self.menu_bar.borrow_mut().as_mut() {
            menu_bar.set_paste_actions_visible(visible);
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

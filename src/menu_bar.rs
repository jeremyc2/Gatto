use crate::{desktop::Menu, global_shortcut::Shortcut, updater::UpdateState};
use anyhow::Result;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuAction {
    Open,
    AddFromClipboard,
    Settings,
    CreateComposition,
    Update,
    Quit,
}

pub struct MenuBar {
    menu: Menu,
}

impl MenuBar {
    pub fn install(
        add_from_clipboard_enabled: bool,
        add_from_clipboard_shortcut: Option<&str>,
    ) -> Result<(Self, async_channel::Receiver<u32>)> {
        let (menu, events) = Menu::new()?;
        let bar = Self { menu };
        bar.menu
            .add_from_clipboard_enabled(add_from_clipboard_enabled);
        bar.set_shortcut(add_from_clipboard_shortcut)?;
        Ok((bar, events))
    }
    fn set_shortcut(&self, shortcut: Option<&str>) -> Result<()> {
        if let Some(shortcut) = shortcut {
            let key = Shortcut::parse(shortcut)?;
            self.menu
                .shortcut(&key.canonical(), &key.equivalent, key.modifiers, key.numpad)
        } else {
            self.menu.shortcut("", "", 0, false)
        }
    }
}

#[derive(Clone, Default)]
pub struct MenuBarController {
    menu_bar: Rc<RefCell<Option<MenuBar>>>,
    update_generation: Rc<Cell<u32>>,
    add_from_clipboard_busy: Rc<Cell<bool>>,
}
impl MenuBarController {
    pub fn add_from_clipboard_busy(&self) -> bool {
        self.add_from_clipboard_busy.get()
    }
    pub fn add_from_clipboard_busy_flag(&self) -> Rc<Cell<bool>> {
        self.add_from_clipboard_busy.clone()
    }
    pub fn set_add_from_clipboard_shortcut(&self, shortcut: Option<&str>) -> Result<()> {
        if let Some(bar) = self.menu_bar.borrow().as_ref() {
            bar.set_shortcut(shortcut)?;
        }
        Ok(())
    }
    pub fn attach(&self, menu_bar: MenuBar) {
        *self.menu_bar.borrow_mut() = Some(menu_bar);
    }
    pub fn action_for(&self, event: &u32) -> Option<MenuAction> {
        if self.menu_bar.borrow().is_none() {
            return None;
        }
        match event {
            1 => Some(MenuAction::Open),
            3 => Some(MenuAction::AddFromClipboard),
            4 => Some(MenuAction::Settings),
            id if *id == self.update_identifier() => Some(MenuAction::Update),
            6 => Some(MenuAction::Quit),
            7 => Some(MenuAction::CreateComposition),
            _ => None,
        }
    }
    pub fn set_update_state(&self, state: UpdateState) {
        if let Some(bar) = self.menu_bar.borrow().as_ref() {
            bar.menu.update_state(
                match state {
                    UpdateState::Idle => 0,
                    UpdateState::Available => 1,
                    UpdateState::Installing => 2,
                    UpdateState::Failed => 3,
                },
                self.update_identifier(),
            );
        }
    }
    fn update_identifier(&self) -> u32 {
        (self.update_generation.get() << 8) | 5
    }
    pub fn invalidate_update_offer(&self) {
        self.update_generation
            .set(self.update_generation.get().wrapping_add(1) & 0x00ff_ffff);
        self.set_update_state(UpdateState::Idle);
    }
    pub fn set_add_from_clipboard_enabled(&self, visible: bool) {
        if let Some(bar) = self.menu_bar.borrow().as_ref() {
            bar.menu.add_from_clipboard_enabled(visible);
        }
    }
}

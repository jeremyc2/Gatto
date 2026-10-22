use crate::{desktop::Menu, global_shortcut::Shortcut, updater::UpdateState};
use anyhow::Result;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuAction {
    Open,
    StageFromClipboard,
    QuickCopy,
    Settings,
    Update,
    Quit,
}

pub struct MenuBar {
    menu: Menu,
}

impl MenuBar {
    pub fn install(
        paste_actions_visible: bool,
        quick_copy_shortcut: Option<&str>,
    ) -> Result<(Self, async_channel::Receiver<u32>)> {
        let (menu, events) = Menu::new()?;
        let bar = Self { menu };
        bar.menu.paste_visible(paste_actions_visible);
        bar.set_shortcut(quick_copy_shortcut)?;
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
}
impl MenuBarController {
    pub fn set_quick_copy_shortcut(&self, shortcut: Option<&str>) -> Result<()> {
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
            2 => Some(MenuAction::StageFromClipboard),
            3 => Some(MenuAction::QuickCopy),
            4 => Some(MenuAction::Settings),
            id if *id == self.update_identifier() => Some(MenuAction::Update),
            6 => Some(MenuAction::Quit),
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
    pub fn set_paste_actions_visible(&self, visible: bool) {
        if let Some(bar) = self.menu_bar.borrow().as_ref() {
            bar.menu.paste_visible(visible);
        }
    }
}

//! Rust owns the status item, menu, target and channel for exactly one lifetime.
use anyhow::{Context as _, Result};
use objc2::{
    AnyThread as _, DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send,
    rc::Retained, sel,
};
use objc2_app_kit::{
    NSEventModifierFlags, NSImage, NSMenu, NSMenuItem, NSStatusBar, NSStatusItem,
    NSVariableStatusItemLength,
};
use objc2_foundation::{NSData, NSObject, NSObjectProtocol, NSSize, NSString};
use std::{cell::Cell, ffi::CString};

define_class!(
    // SAFETY: NSObject has no subclassing requirements. Ivars own the sender;
    // callbacks enqueue IDs and never borrow a live GPUI view.
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = async_channel::Sender<u32>]
    #[name = "GattoMenuTarget"]
    struct MenuTarget;

    // SAFETY: NSObjectProtocol has no extra requirements.
    unsafe impl NSObjectProtocol for MenuTarget {}

    impl MenuTarget {
        // SAFETY: AppKit's target/action signature passes the activating item.
        #[unsafe(method(activate:))]
        fn activate(&self, item: &NSMenuItem) {
            let _ = self.ivars().try_send(item.tag() as u32);
        }
    }
);

pub struct Menu {
    main_thread: MainThreadMarker,
    status: Retained<NSStatusItem>,
    menu: Retained<NSMenu>,
    _target: Retained<MenuTarget>,
    open: Retained<NSMenuItem>,
    preview: Retained<NSMenuItem>,
    quick: Retained<NSMenuItem>,
    preferences: Retained<NSMenuItem>,
    composition: Retained<NSMenuItem>,
    update: Retained<NSMenuItem>,
    quit: Retained<NSMenuItem>,
    paste_visible: Cell<bool>,
    update_visible: Cell<bool>,
}

impl Menu {
    pub fn new() -> Result<(Self, async_channel::Receiver<u32>)> {
        let main_thread = MainThreadMarker::new()
            .context("The menu bar must be created on the application main thread")?;
        let (sender, events) = async_channel::unbounded();
        let target = MenuTarget::alloc(main_thread).set_ivars(sender);
        // SAFETY: NSObject's initializer preserves our initialized Rust ivars.
        let target: Retained<MenuTarget> = unsafe { msg_send![super(target), init] };
        let data = NSData::with_bytes(include_bytes!("../../packaging/menu-bar-icon.png"));
        let image = NSImage::initWithData(NSImage::alloc(), &data)
            .context("Could not create the macOS menu bar icon")?;
        image.setTemplate(true);
        image.setSize(NSSize::new(
            18. * image.size().width / image.size().height,
            18.,
        ));
        let status =
            NSStatusBar::systemStatusBar().statusItemWithLength(NSVariableStatusItemLength);
        let Some(button) = status.button(main_thread) else {
            NSStatusBar::systemStatusBar().removeStatusItem(&status);
            anyhow::bail!("Could not create the macOS menu bar icon");
        };
        button.setImage(Some(&image));
        button.setToolTip(Some(&NSString::from_str("Gatto")));
        let menu = NSMenu::new(main_thread);
        menu.setAutoenablesItems(false);
        let item = |title: &str, key: &str, tag| {
            let item = NSMenuItem::new(main_thread);
            item.setTitle(&NSString::from_str(title));
            item.setKeyEquivalent(&NSString::from_str(key));
            item.setTag(tag);
            item.setKeyEquivalentModifierMask(if key.is_empty() {
                NSEventModifierFlags::empty()
            } else {
                NSEventModifierFlags::Command
            });
            // SAFETY: The target implements activate: with this exact signature.
            // Menu owns target until after it disconnects every item in Drop.
            unsafe {
                item.setTarget(Some(&target));
                item.setAction(Some(sel!(activate:)));
            }
            item
        };
        let this = Self {
            open: item("Open App", "", 1),
            preview: item("Draft from Clipboard", "v", 2),
            quick: item("Quick Copy", "", 3),
            preferences: item("App Preferences", ",", 4),
            composition: item("Side-by-Side Comparison", "", 7),
            update: item("↑ Update", "", 5),
            quit: item("Quit", "q", 6),
            main_thread,
            status,
            menu,
            _target: target,
            paste_visible: Cell::new(false),
            update_visible: Cell::new(false),
        };
        this.rebuild();
        this.status.setMenu(Some(&this.menu));
        Ok((this, events))
    }

    fn items(&self) -> [&NSMenuItem; 7] {
        [
            &self.open,
            &self.preview,
            &self.quick,
            &self.preferences,
            &self.composition,
            &self.update,
            &self.quit,
        ]
    }

    fn rebuild(&self) {
        self.menu.removeAllItems();
        self.menu.addItem(&self.open);
        self.menu.addItem(&self.composition);
        if self.update_visible.get() {
            self.menu.addItem(&self.update);
        }
        self.menu
            .addItem(&NSMenuItem::separatorItem(self.main_thread));
        self.menu.addItem(&self.preview);
        if self.paste_visible.get() {
            self.menu.addItem(&self.quick);
        }
        self.menu
            .addItem(&NSMenuItem::separatorItem(self.main_thread));
        self.menu.addItem(&self.preferences);
        self.menu
            .addItem(&NSMenuItem::separatorItem(self.main_thread));
        self.menu.addItem(&self.quit);
    }
    pub fn paste_visible(&self, visible: bool) {
        self.paste_visible.set(visible);
        self.rebuild();
    }
    pub fn update_state(&self, state: u32, identifier: u32) {
        self.update_visible.set(state != 0);
        self.update.setTag(identifier as isize);
        self.update.setTitle(&NSString::from_str(match state {
            2 => "Updating…",
            3 => "↻ Retry Update",
            _ => "↑ Update",
        }));
        self.update.setEnabled(state != 2);
        self.rebuild();
    }
    pub fn shortcut(
        &self,
        canonical: &str,
        equivalent: &str,
        modifiers: u32,
        numpad: bool,
    ) -> Result<()> {
        // Preserve the existing rejection of interior NULs, including callers
        // outside the canonical shortcut parser.
        CString::new(canonical)?;
        CString::new(equivalent)?;
        let mut flags = NSEventModifierFlags::empty();
        for (bit, flag) in [
            (1, NSEventModifierFlags::Command),
            (2, NSEventModifierFlags::Control),
            (4, NSEventModifierFlags::Option),
            (8, NSEventModifierFlags::Shift),
        ] {
            if modifiers & bit != 0 {
                flags |= flag;
            }
        }
        if numpad {
            flags |= NSEventModifierFlags::NumericPad;
        }
        self.quick.setKeyEquivalent(&NSString::from_str(equivalent));
        self.quick
            .setKeyEquivalentModifierMask(if equivalent.is_empty() {
                NSEventModifierFlags::empty()
            } else {
                flags
            });
        self.quick.setTitle(&NSString::from_str(
            &if !canonical.is_empty() && equivalent.is_empty() {
                format!("Quick Copy ({canonical})")
            } else {
                "Quick Copy".into()
            },
        ));
        Ok(())
    }
}
impl Drop for Menu {
    fn drop(&mut self) {
        self.status.setMenu(None);
        NSStatusBar::systemStatusBar().removeStatusItem(&self.status);
        for item in self.items() {
            // SAFETY: Disconnect unretained targets while target is still alive,
            // including any items temporarily retained by native menu tracking.
            unsafe {
                item.setTarget(None);
                item.setAction(None);
            }
        }
    }
}

#[cfg(feature = "walkthrough-screenshots")]
#[allow(dead_code)] // Called by the standalone main-thread harness.
pub(super) fn verify() -> Result<()> {
    let events = objc2::rc::autoreleasepool(|_| -> Result<_> {
        let (menu, events) = Menu::new()?;
        assert!(
            menu.status
                .button(menu.main_thread)
                .unwrap()
                .image()
                .unwrap()
                .isTemplate()
        );
        assert_eq!(menu.menu.numberOfItems(), 8);
        menu.paste_visible(true);
        assert_eq!(menu.menu.numberOfItems(), 9);
        assert_eq!(menu.preview.title().to_string(), "Draft from Clipboard");
        assert_eq!(menu.preview.keyEquivalent().to_string(), "v");
        menu.shortcut("Command+Shift+Numpad1", "1", 9, true)?;
        assert!(
            menu.quick
                .keyEquivalentModifierMask()
                .contains(NSEventModifierFlags::NumericPad)
        );
        menu.shortcut("", "", 0, false)?;
        assert_eq!(menu.quick.keyEquivalent().len(), 0);
        assert!(menu.quick.keyEquivalentModifierMask().is_empty());
        menu.shortcut("Command+Shift+MediaPlayPause", "", 9, false)?;
        assert!(menu.quick.title().to_string().contains("MediaPlayPause"));
        menu.update_state(2, 261);
        assert_eq!(menu.update.tag(), 261);
        assert!(!menu.update.isEnabled());
        assert_eq!(menu.menu.numberOfItems(), 10);
        menu.paste_visible(false);
        assert_eq!(menu.menu.numberOfItems(), 9);
        assert_eq!(menu.menu.itemAtIndex(2).as_deref(), Some(&*menu.update));
        menu.menu.performActionForItemAtIndex(1);
        assert_eq!(events.try_recv()?, 7);
        menu.menu.performActionForItemAtIndex(6);
        assert_eq!(events.try_recv()?, 4);
        menu.update_state(0, 517);
        assert_eq!(menu.update.tag(), 517);
        assert_eq!(menu.menu.numberOfItems(), 8);
        let retained_item = menu.open.clone();
        drop(menu);
        assert!(retained_item.target().is_none());
        assert!(retained_item.action().is_none());
        Ok(events)
    })?;
    assert!(events.is_closed());
    Ok(())
}

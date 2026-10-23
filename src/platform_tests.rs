#![deny(unsafe_op_in_unsafe_fn)]
// This standalone main-thread harness exercises the production native shortcut
// controller. Rust/GPUI test workers are not macOS's AppKit main thread.
#![allow(dead_code)]
mod desktop;
mod global_shortcut;

fn main() -> anyhow::Result<()> {
    if desktop::run_test_receiver() {
        return Ok(());
    }
    desktop::verify_desktop()?;
    use std::{cell::RefCell, rc::Rc};
    let observed = Rc::new(RefCell::new(Vec::new()));
    let capture = observed.clone();
    let controller = global_shortcut::GlobalShortcutController::new(move |value| {
        capture.borrow_mut().push(value.map(str::to_owned))
    });
    let _events = controller.install()?;
    controller.set_shortcut(Some("Command+Control+Shift+F19"))?;
    assert!(controller.is_registered_event(1));
    assert!(controller.set_shortcut(Some("Command+Shift+F24")).is_err());
    assert_eq!(
        controller.registered_shortcut().as_deref(),
        Some("Command+Control+Shift+F19")
    );
    assert!(controller.is_registered_event(1));
    assert_eq!(observed.borrow().len(), 1);
    controller.set_shortcut(None)?;
    assert!(!controller.is_registered_event(1));
    assert_eq!(observed.borrow().last(), Some(&None));
    controller.set_shortcut(Some("Command+Control+Shift+F19"))?;
    assert!(!controller.is_registered_event(1)); // Queued events from the old registration are stale.
    controller.set_shortcut(None)?;
    // Use uncommon chords so this can run alongside the installed app. Exercise
    // both direct replacement and the recorder's disable/register sequence.
    for shortcut in [
        "Command+Control+Shift+F19",
        "Command+Control+Shift+F18",
        "Command+Control+Shift+F19",
    ] {
        controller.set_shortcut(Some(shortcut))?;
        assert_eq!(controller.registered_shortcut().as_deref(), Some(shortcut));
    }
    controller.set_shortcut(None)?;
    controller.set_shortcut(Some("Command+Control+Shift+F19"))?;
    controller.set_shortcut(None)?;
    println!(
        "Native shortcut registration, failed replacement, disable and stale-event guards passed."
    );
    Ok(())
}

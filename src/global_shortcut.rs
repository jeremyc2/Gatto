use std::{cell::RefCell, rc::Rc};

use anyhow::{Context as _, Result, bail};
use global_hotkey::{
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
    hotkey::{Code, HotKey, Modifiers},
};
use gpui_kit::Keystroke;

pub const DEFAULT_QUICK_COPY_SHORTCUT: &str = "Command+Shift+U";

struct GlobalShortcutState {
    manager: GlobalHotKeyManager,
    registered: Option<HotKey>,
}

/// Owns the process-wide hotkey registration while remaining cloneable for UI callbacks.
#[derive(Clone, Default)]
pub struct GlobalShortcutController {
    state: Rc<RefCell<Option<GlobalShortcutState>>>,
}

impl GlobalShortcutController {
    /// Creates the native manager on the main thread and forwards pressed events
    /// into an async channel that can be consumed by GPUI's event loop.
    pub fn install(&self) -> Result<async_channel::Receiver<u32>> {
        let manager =
            GlobalHotKeyManager::new().context("Could not initialize macOS global shortcuts")?;
        let (sender, receiver) = async_channel::unbounded();
        GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
            if event.state() == HotKeyState::Pressed {
                let _ = sender.try_send(event.id());
            }
        }));
        *self.state.borrow_mut() = Some(GlobalShortcutState {
            manager,
            registered: None,
        });
        Ok(receiver)
    }

    /// Replaces the active shortcut without dropping the previous registration
    /// when the new combination is unavailable.
    pub fn set_shortcut(&self, shortcut: Option<&str>) -> Result<Option<String>> {
        let hotkey = shortcut.map(parse_shortcut).transpose()?;
        let mut state = self.state.borrow_mut();
        let Some(state) = state.as_mut() else {
            if hotkey.is_none() {
                return Ok(None);
            }
            bail!("The global shortcut service is unavailable");
        };

        if state.registered == hotkey {
            return Ok(hotkey.map(format_shortcut));
        }

        match hotkey {
            Some(new_hotkey) => {
                state.manager.register(new_hotkey).with_context(|| {
                    format!(
                        "Could not register {}. It may already be used by another app.",
                        format_shortcut(new_hotkey)
                    )
                })?;

                if let Some(previous) = state.registered
                    && let Err(error) = state.manager.unregister(previous)
                {
                    let _ = state.manager.unregister(new_hotkey);
                    return Err(error).context("Could not replace the previous global shortcut");
                }
                state.registered = Some(new_hotkey);
                Ok(Some(format_shortcut(new_hotkey)))
            }
            None => {
                if let Some(previous) = state.registered {
                    state
                        .manager
                        .unregister(previous)
                        .context("Could not disable the global shortcut")?;
                }
                state.registered = None;
                Ok(None)
            }
        }
    }

    pub fn is_registered_event(&self, id: u32) -> bool {
        self.state
            .borrow()
            .as_ref()
            .and_then(|state| state.registered)
            .is_some_and(|hotkey| hotkey.id() == id)
    }
}

pub fn canonicalize_shortcut(shortcut: &str) -> Result<String> {
    parse_shortcut(shortcut).map(format_shortcut)
}

pub fn shortcut_from_keystroke(keystroke: &Keystroke) -> Result<String> {
    let mut parts = Vec::new();
    if keystroke.modifiers.platform {
        parts.push("Command");
    }
    if keystroke.modifiers.control {
        parts.push("Control");
    }
    if keystroke.modifiers.alt {
        parts.push("Option");
    }
    if keystroke.modifiers.shift {
        parts.push("Shift");
    }

    let key = match keystroke.key.as_str() {
        "return" => "Enter",
        "space" | " " => "Space",
        key => key,
    };
    parts.push(key);
    canonicalize_shortcut(&parts.join("+"))
}

fn parse_shortcut(shortcut: &str) -> Result<HotKey> {
    let shortcut = shortcut.trim();
    if shortcut.is_empty() {
        bail!("Enter a keyboard shortcut.");
    }
    let hotkey = shortcut
        .parse::<HotKey>()
        .map_err(|_| anyhow::anyhow!("Use a shortcut such as Command+Shift+U."))?;
    let modifier_count = [
        Modifiers::SUPER,
        Modifiers::CONTROL,
        Modifiers::ALT,
        Modifiers::SHIFT,
    ]
    .into_iter()
    .filter(|modifier| hotkey.mods.contains(*modifier))
    .count();
    if modifier_count < 2 {
        bail!("Use at least two modifier keys so the shortcut does not interfere with typing.");
    }
    Ok(hotkey)
}

fn format_shortcut(hotkey: HotKey) -> String {
    let mut parts = Vec::new();
    if hotkey.mods.contains(Modifiers::SUPER) {
        parts.push("Command".to_owned());
    }
    if hotkey.mods.contains(Modifiers::CONTROL) {
        parts.push("Control".to_owned());
    }
    if hotkey.mods.contains(Modifiers::ALT) {
        parts.push("Option".to_owned());
    }
    if hotkey.mods.contains(Modifiers::SHIFT) {
        parts.push("Shift".to_owned());
    }
    parts.push(format_key(hotkey.key));
    parts.join("+")
}

fn format_key(key: Code) -> String {
    let name = key.to_string();
    name.strip_prefix("Key")
        .or_else(|| name.strip_prefix("Digit"))
        .unwrap_or(&name)
        .to_owned()
}

#[cfg(test)]
mod tests {
    use gpui_kit::{Keystroke, Modifiers as GpuiModifiers};

    use super::{DEFAULT_QUICK_COPY_SHORTCUT, canonicalize_shortcut, shortcut_from_keystroke};

    #[test]
    fn canonicalizes_a_user_friendly_shortcut() {
        assert_eq!(
            canonicalize_shortcut("cmd + shift + u").unwrap(),
            "Command+Shift+U"
        );
        assert_eq!(
            canonicalize_shortcut(DEFAULT_QUICK_COPY_SHORTCUT).unwrap(),
            "Command+Shift+U"
        );
    }

    #[test]
    fn rejects_shortcuts_that_could_interfere_with_typing() {
        assert!(canonicalize_shortcut("u").is_err());
        assert!(canonicalize_shortcut("command+u").is_err());
    }

    #[test]
    fn converts_a_recorded_keystroke_to_a_global_shortcut() {
        let keystroke = Keystroke {
            modifiers: GpuiModifiers {
                platform: true,
                shift: true,
                ..Default::default()
            },
            key: "u".into(),
            key_char: Some("U".into()),
        };

        assert_eq!(
            shortcut_from_keystroke(&keystroke).unwrap(),
            "Command+Shift+U"
        );
    }

    #[test]
    fn recorded_shortcuts_still_require_two_supported_modifiers() {
        let keystroke = Keystroke {
            modifiers: GpuiModifiers {
                platform: true,
                function: true,
                ..Default::default()
            },
            key: "u".into(),
            key_char: Some("u".into()),
        };

        assert!(shortcut_from_keystroke(&keystroke).is_err());
    }
}

use crate::desktop::Hotkey;
use anyhow::{Context as _, Result, bail};
use gpui_kit::Keystroke;
use std::{cell::RefCell, rc::Rc};

pub const DEFAULT_ADD_FROM_CLIPBOARD_SHORTCUT: &str = "Command+Shift+G";
type ShortcutObserver = Rc<dyn Fn(Option<&str>)>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Shortcut {
    pub key: String,
    pub modifiers: u32,
    pub equivalent: String,
    pub numpad: bool,
}
impl Shortcut {
    pub fn parse(value: &str) -> Result<Self> {
        let parts: Vec<_> = value.split('+').map(str::trim).collect();
        let Some((key, modifiers)) = parts.split_last() else {
            bail!("Enter a keyboard shortcut.");
        };
        let mut mask = 0_u32;
        for modifier in modifiers {
            mask |= match modifier.to_ascii_uppercase().as_str() {
                "COMMAND" | "CMD" | "SUPER" | "COMMANDORCONTROL" | "COMMANDORCTRL"
                | "CMDORCTRL" | "CMDORCONTROL" => 1,
                "CONTROL" | "CTRL" => 2,
                "OPTION" | "ALT" => 4,
                "SHIFT" => 8,
                _ => bail!("Use a shortcut such as Command+Shift+G."),
            };
        }
        if mask.count_ones() < 2 {
            bail!("Use at least two modifier keys so the shortcut does not interfere with typing.");
        }
        let upper = key.to_ascii_uppercase();
        let key = normalize_key(&upper)
            .ok_or_else(|| anyhow::anyhow!("Use a shortcut such as Command+Shift+G."))?;
        let numpad = key.starts_with("Numpad");
        let equivalent = menu_equivalent(&key);
        Ok(Self {
            key,
            modifiers: mask,
            equivalent,
            numpad,
        })
    }
    pub fn canonical(&self) -> String {
        let mut parts = Vec::new();
        for (bit, name) in [(1, "Command"), (2, "Control"), (4, "Option"), (8, "Shift")] {
            if self.modifiers & bit != 0 {
                parts.push(name);
            }
        }
        parts.push(
            self.key
                .strip_prefix("Key")
                .or_else(|| self.key.strip_prefix("Digit"))
                .unwrap_or(&self.key),
        );
        parts.join("+")
    }
}

fn normalize_key(key: &str) -> Option<String> {
    let letter = key.strip_prefix("KEY").unwrap_or(key);
    if letter.len() == 1 && letter.as_bytes()[0].is_ascii_uppercase() {
        return Some(format!("Key{letter}"));
    }
    let digit = key.strip_prefix("DIGIT").unwrap_or(key);
    if digit.len() == 1 && digit.as_bytes()[0].is_ascii_digit() {
        return Some(format!("Digit{digit}"));
    }
    if let Some(n) = key.strip_prefix('F').and_then(|v| v.parse::<u32>().ok())
        && (1..=24).contains(&n)
    {
        return Some(format!("F{n}"));
    }
    let num = key
        .strip_prefix("NUMPAD")
        .or_else(|| key.strip_prefix("NUM"));
    if let Some(num) = num {
        if num.len() == 1 && num.as_bytes()[0].is_ascii_digit() {
            return Some(format!("Numpad{num}"));
        }
        if let Some(name) = match num {
            "ADD" | "PLUS" => Some("Add"),
            "SUBTRACT" => Some("Subtract"),
            "MULTIPLY" => Some("Multiply"),
            "DIVIDE" => Some("Divide"),
            "DECIMAL" => Some("Decimal"),
            "ENTER" => Some("Enter"),
            "EQUAL" => Some("Equal"),
            _ => None,
        } {
            return Some(format!("Numpad{name}"));
        }
    }
    Some(
        match key {
            "BACKQUOTE" | "`" => "Backquote",
            "BACKSLASH" | "\\" => "Backslash",
            "BRACKETLEFT" | "[" => "BracketLeft",
            "BRACKETRIGHT" | "]" => "BracketRight",
            "COMMA" | "," => "Comma",
            "EQUAL" | "=" => "Equal",
            "MINUS" | "-" => "Minus",
            "PERIOD" | "." => "Period",
            "QUOTE" | "'" => "Quote",
            "SEMICOLON" | ";" => "Semicolon",
            "SLASH" | "/" => "Slash",
            "INTLBACKSLASH" => "IntlBackslash",
            "BACKSPACE" => "Backspace",
            "CAPSLOCK" => "CapsLock",
            "ENTER" | "RETURN" => "Enter",
            "SPACE" => "Space",
            "TAB" => "Tab",
            "DELETE" => "Delete",
            "END" => "End",
            "HOME" => "Home",
            "INSERT" => "Insert",
            "PAGEDOWN" => "PageDown",
            "PAGEUP" => "PageUp",
            "PRINTSCREEN" => "PrintScreen",
            "SCROLLLOCK" => "ScrollLock",
            "NUMLOCK" => "NumLock",
            "PAUSE" | "PAUSEBREAK" => "Pause",
            "ESCAPE" | "ESC" => "Escape",
            "ARROWDOWN" | "DOWN" => "ArrowDown",
            "ARROWUP" | "UP" => "ArrowUp",
            "ARROWLEFT" | "LEFT" => "ArrowLeft",
            "ARROWRIGHT" | "RIGHT" => "ArrowRight",
            "AUDIOVOLUMEDOWN" | "VOLUMEDOWN" => "AudioVolumeDown",
            "AUDIOVOLUMEUP" | "VOLUMEUP" => "AudioVolumeUp",
            "AUDIOVOLUMEMUTE" | "VOLUMEMUTE" => "AudioVolumeMute",
            "MEDIAPLAY" => "MediaPlay",
            "MEDIAPAUSE" => "MediaPause",
            "MEDIAPLAYPAUSE" => "MediaPlayPause",
            "MEDIASTOP" => "MediaStop",
            "MEDIATRACKNEXT" => "MediaTrackNext",
            "MEDIATRACKPREV" | "MEDIATRACKPREVIOUS" => "MediaTrackPrevious",
            _ => return None,
        }
        .into(),
    )
}
fn menu_equivalent(key: &str) -> String {
    if let Some(letter) = key.strip_prefix("Key") {
        return letter.to_ascii_lowercase();
    }
    if let Some(digit) = key.strip_prefix("Digit") {
        return digit.into();
    }
    if let Some(n) = key.strip_prefix('F').and_then(|v| v.parse::<u32>().ok()) {
        return char::from_u32(0xf704 + n - 1).unwrap().into();
    }
    if let Some(digit) = key.strip_prefix("Numpad")
        && digit.len() == 1
    {
        return digit.into();
    }
    match key {
        "Backquote" => "`",
        "Backslash" | "IntlBackslash" => "\\",
        "BracketLeft" => "[",
        "BracketRight" => "]",
        "Comma" => ",",
        "Equal" | "NumpadEqual" => "=",
        "Minus" | "NumpadSubtract" => "-",
        "Period" | "NumpadDecimal" => ".",
        "Quote" => "'",
        "Semicolon" => ";",
        "Slash" | "NumpadDivide" => "/",
        "NumpadAdd" => "+",
        "NumpadMultiply" => "*",
        "Enter" => "\r",
        "NumpadEnter" => "\u{3}",
        "Tab" => "\t",
        "Space" => " ",
        "Backspace" => "\u{8}",
        "Delete" => "\u{7f}",
        "Escape" => "\u{1b}",
        "ArrowUp" => "\u{f700}",
        "ArrowDown" => "\u{f701}",
        "ArrowLeft" => "\u{f702}",
        "ArrowRight" => "\u{f703}",
        "Insert" => "\u{f727}",
        "Home" => "\u{f729}",
        "End" => "\u{f72b}",
        "PageUp" => "\u{f72c}",
        "PageDown" => "\u{f72d}",
        _ => "",
    }
    .into()
}

struct Registered {
    shortcut: Shortcut,
    native: Hotkey,
    identifier: u32,
}
struct GlobalShortcutState {
    registered: Option<Registered>,
    sender: async_channel::Sender<u32>,
    next_id: u32,
}

#[derive(Clone, Default)]
pub struct GlobalShortcutController {
    state: Rc<RefCell<Option<GlobalShortcutState>>>,
    on_change: Option<ShortcutObserver>,
}
impl GlobalShortcutController {
    pub fn new(on_change: impl Fn(Option<&str>) + 'static) -> Self {
        Self {
            on_change: Some(Rc::new(on_change)),
            ..Self::default()
        }
    }
    pub fn registered_shortcut(&self) -> Option<String> {
        self.state
            .borrow()
            .as_ref()
            .and_then(|state| state.registered.as_ref())
            .map(|key| key.shortcut.canonical())
    }
    pub fn install(&self) -> Result<async_channel::Receiver<u32>> {
        let (sender, receiver) = async_channel::unbounded();
        *self.state.borrow_mut() = Some(GlobalShortcutState {
            registered: None,
            sender,
            next_id: 1,
        });
        Ok(receiver)
    }
    pub fn set_shortcut(&self, shortcut: Option<&str>) -> Result<Option<String>> {
        let shortcut = shortcut.map(Shortcut::parse).transpose()?;
        let mut registration = self.state.borrow_mut();
        let Some(state) = registration.as_mut() else {
            if shortcut.is_none() {
                return Ok(None);
            }
            bail!("The global shortcut service is unavailable");
        };
        if state.registered.as_ref().map(|key| &key.shortcut) == shortcut.as_ref() {
            return Ok(shortcut.map(|key| key.canonical()));
        }
        let candidate = if let Some(shortcut) = shortcut {
            let identifier = state.next_id;
            state.next_id = state.next_id.wrapping_add(1).max(1);
            let native = Hotkey::register(
                &shortcut.key,
                shortcut.modifiers,
                identifier,
                state.sender.clone(),
            )
            .with_context(|| {
                format!(
                    "Could not register {}. The previous shortcut remains active.",
                    shortcut.canonical()
                )
            })?;
            Some(Registered {
                shortcut,
                native,
                identifier,
            })
        } else {
            None
        };
        // Register first. If releasing the old key fails, dropping the candidate
        // rolls it back and the old state/observer remains unchanged.
        if let Some(previous) = state.registered.as_mut() {
            previous
                .native
                .unregister()
                .context("Could not replace the previous global shortcut")?;
        }
        state.registered = candidate;
        let canonical = state
            .registered
            .as_ref()
            .map(|key| key.shortcut.canonical());
        drop(registration);
        if let Some(on_change) = &self.on_change {
            on_change(canonical.as_deref());
        }
        Ok(canonical)
    }
    pub fn is_registered_event(&self, id: u32) -> bool {
        self.state
            .borrow()
            .as_ref()
            .and_then(|state| state.registered.as_ref())
            .is_some_and(|key| key.identifier == id)
    }
}

pub fn canonicalize_shortcut(shortcut: &str) -> Result<String> {
    Shortcut::parse(shortcut).map(|key| key.canonical())
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
    parts.push(match keystroke.key.as_str() {
        "return" => "Enter",
        "space" | " " => "Space",
        key => key,
    });
    canonicalize_shortcut(&parts.join("+"))
}
#[cfg(test)]
mod tests {
    use gpui_kit::{Keystroke, Modifiers as GpuiModifiers};

    use super::{
        DEFAULT_ADD_FROM_CLIPBOARD_SHORTCUT, canonicalize_shortcut, shortcut_from_keystroke,
    };

    #[test]
    fn stored_shortcuts_round_trip_aliases_keypad_function_and_media_keys() {
        for (alias, expected) in [
            ("CmdOrCtrl+alt+keyg", "Command+Option+G"),
            ("control+shift+digit5", "Control+Shift+5"),
            ("cmd+shift+numplus", "Command+Shift+NumpadAdd"),
            ("cmd+shift+num0", "Command+Shift+Numpad0"),
            ("cmd+shift+numenter", "Command+Shift+NumpadEnter"),
            ("cmd+shift+esc", "Command+Shift+Escape"),
            ("cmd+shift+down", "Command+Shift+ArrowDown"),
            ("cmd+shift+f24", "Command+Shift+F24"),
            ("cmd+shift+volumeup", "Command+Shift+AudioVolumeUp"),
            (
                "cmd+shift+mediatrackprev",
                "Command+Shift+MediaTrackPrevious",
            ),
            ("cmd+shift+pausebreak", "Command+Shift+Pause"),
        ] {
            assert_eq!(canonicalize_shortcut(alias).unwrap(), expected);
            assert_eq!(canonicalize_shortcut(expected).unwrap(), expected);
        }
        for invalid in [
            "cmd+cmd+g",
            "cmd+shift+",
            "cmd++g",
            "cmd+g+shift",
            "cmd+shift+unrecognized",
        ] {
            assert!(canonicalize_shortcut(invalid).is_err(), "{invalid}");
        }
        let top = super::Shortcut::parse("cmd+shift+1").unwrap();
        let pad = super::Shortcut::parse("cmd+shift+num1").unwrap();
        assert_eq!(top.equivalent, pad.equivalent);
        assert!(!top.numpad && pad.numpad);
        assert_ne!(
            super::Shortcut::parse("cmd+shift+Enter")
                .unwrap()
                .equivalent,
            super::Shortcut::parse("cmd+shift+NumpadEnter")
                .unwrap()
                .equivalent
        );
    }

    #[test]
    fn canonicalizes_a_user_friendly_shortcut() {
        assert_eq!(
            canonicalize_shortcut("cmd + shift + g").unwrap(),
            "Command+Shift+G"
        );
        assert_eq!(
            canonicalize_shortcut(DEFAULT_ADD_FROM_CLIPBOARD_SHORTCUT).unwrap(),
            "Command+Shift+G"
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

use anyhow::{Result, bail};
use chrono::{DateTime, Local};

pub fn format_local_timestamp(milliseconds: u64) -> Option<String> {
    if milliseconds == 0 || milliseconds > 253_402_300_799_999 {
        return None;
    }
    let date = DateTime::from_timestamp_millis(milliseconds as i64)?;
    Some(
        date.with_timezone(&Local)
            .format("%b %-d, %Y %H:%M")
            .to_string(),
    )
}

pub fn install_file_service() -> Result<()> {
    Ok(())
}

pub struct Hotkey;

impl Hotkey {
    pub fn register(_: &str, _: u32, _: u32, _: async_channel::Sender<u32>) -> Result<Self> {
        bail!("Global shortcuts are unavailable in the Windows preview.")
    }

    pub fn unregister(&mut self) -> Result<()> {
        Ok(())
    }
}

// The Windows preview uses its main window as the entry point.
pub struct Menu;

impl Menu {
    pub fn shortcut(&self, _: &str, _: &str, _: u32, _: bool) -> Result<()> {
        Ok(())
    }

    pub fn update_state(&self, _: u32, _: u32) {}

    pub fn add_from_clipboard_enabled(&self, _: bool) {}
}

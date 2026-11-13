use anyhow::{Context as _, Result, bail, ensure};
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

pub fn preview_video(path: &std::path::Path) -> Result<()> {
    use std::os::windows::ffi::OsStrExt as _;
    use windows::{
        Win32::{
            Foundation::{CloseHandle, WAIT_OBJECT_0},
            System::Threading::{INFINITE, WaitForSingleObject},
            UI::{
                Shell::{
                    SEE_MASK_FLAG_NO_UI, SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS,
                    SHELLEXECUTEINFOW, ShellExecuteExW,
                },
                WindowsAndMessaging::SW_SHOWNORMAL,
            },
        },
        core::PCWSTR,
    };
    let path: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
    let mut launch = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_FLAG_NO_UI | SEE_MASK_NOASYNC | SEE_MASK_NOCLOSEPROCESS,
        lpFile: PCWSTR(path.as_ptr()),
        nShow: SW_SHOWNORMAL.0,
        ..Default::default()
    };
    // SAFETY: The path and structure remain live through the call.
    unsafe { ShellExecuteExW(&mut launch) }.context("Could not open the default video player")?;
    if !launch.hProcess.is_invalid() {
        // SAFETY: ShellExecuteExW returned an owned process handle.
        let status = unsafe { WaitForSingleObject(launch.hProcess, INFINITE) };
        unsafe { CloseHandle(launch.hProcess) }
            .context("Could not close the video player handle")?;
        ensure!(
            status == WAIT_OBJECT_0,
            "Could not wait for the video player"
        );
    }
    Ok(())
}

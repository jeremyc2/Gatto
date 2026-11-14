use anyhow::{Context as _, Result, ensure};
use std::{ffi::OsString, os::windows::ffi::OsStringExt as _, path::PathBuf};
use windows::{
    Win32::{
        Foundation::ERROR_FILE_NOT_FOUND,
        System::{
            Com::CoTaskMemFree,
            Registry::{
                HKEY, HKEY_CURRENT_USER, KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ,
                RRF_RT_REG_SZ, RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegGetValueW,
                RegSetValueExW,
            },
        },
        UI::Shell::{FOLDERID_Downloads, KNOWN_FOLDER_FLAG, SHGetKnownFolderPath},
    },
    core::{PCWSTR, w},
};

const RUN_KEY: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
const APP: PCWSTR = w!("Gatto");

pub fn starts_at_login() -> Result<bool> {
    // SAFETY: Both registry strings are static; no data is copied when the
    // destination is null and only the required size is requested.
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            RUN_KEY,
            APP,
            RRF_RT_REG_SZ,
            None,
            None,
            Some(&mut 0),
        )
    };
    if status == ERROR_FILE_NOT_FOUND {
        return Ok(false);
    }
    status
        .ok()
        .context("Could not read the Gatto login entry")?;
    Ok(true)
}

pub fn set_start_at_login(enabled: bool) -> Result<()> {
    let mut key = HKEY::default();
    // SAFETY: key receives an owned handle and is closed after the write.
    unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            RUN_KEY,
            None,
            PCWSTR::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            None,
            &mut key,
            None,
        )
        .ok()
        .context("Could not open the Windows login registry key")?;
    }
    let result = (|| {
        if enabled {
            use std::os::windows::ffi::OsStrExt as _;
            let executable = std::env::current_exe().context("Could not locate Gatto")?;
            let command = ['"' as u16]
                .into_iter()
                .chain(executable.as_os_str().encode_wide())
                .chain(format!("\" {}\0", crate::settings::LOGIN_ARGUMENT).encode_utf16())
                .collect::<Vec<_>>();
            ensure!(
                command.len() <= 261,
                "Move Gatto to a shorter path to start it at login."
            );
            let bytes: Vec<_> = command.into_iter().flat_map(u16::to_le_bytes).collect();
            // SAFETY: The REG_SZ bytes are terminated and live through the call.
            unsafe { RegSetValueExW(key, APP, None, REG_SZ, Some(&bytes)) }.ok()?;
        } else {
            // SAFETY: The key is live and the value name is static.
            let status = unsafe { RegDeleteValueW(key, APP) };
            if status != ERROR_FILE_NOT_FOUND {
                status.ok()?;
            }
        }
        Ok::<_, anyhow::Error>(())
    })();
    // SAFETY: The handle was acquired above.
    let _ = unsafe { RegCloseKey(key) };
    result.context("Could not change the Gatto login entry")
}

pub fn downloads_directory() -> Result<PathBuf> {
    // SAFETY: The shell returns a task-allocated, null-terminated path.
    unsafe {
        let path = SHGetKnownFolderPath(&FOLDERID_Downloads, KNOWN_FOLDER_FLAG(0), None)
            .context("Downloads folder is unavailable")?;
        let directory = PathBuf::from(OsString::from_wide(path.as_wide()));
        CoTaskMemFree(Some(path.0.cast()));
        Ok(directory)
    }
}

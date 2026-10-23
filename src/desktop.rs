//! Main-thread-owned AppKit/Carbon adapters. Native callbacks only enqueue IDs;
//! they never reenter a GPUI borrow. No handle can cross a thread boundary.
use anyhow::{Context as _, Result, bail};
use objc2::MainThreadMarker;
mod file_service;
mod menu;
#[allow(unused_imports)] // Standalone platform harness uses the private menu checks.
pub use menu::Menu;

pub fn install_file_service() -> Result<()> {
    file_service::install(
        MainThreadMarker::new()
            .context("The file service must be installed on the application main thread")?,
    );
    Ok(())
}

use std::{
    ffi::{CString, c_char, c_void},
    ptr::NonNull,
};

unsafe extern "C" {
    fn GattoHotkeyCreate(
        key: *const c_char,
        modifiers: u32,
        identifier: u32,
        callback: extern "C" fn(*mut c_void, u32),
        context: *mut c_void,
        error: *mut i32,
    ) -> *mut c_void;
    fn GattoHotkeyDestroy(handle: *mut c_void) -> i32;
    fn GattoHotkeyDetach(handle: *mut c_void);
}

extern "C" fn send_event(context: *mut c_void, identifier: u32) {
    // SAFETY: Each native owner is destroyed/detached before its boxed sender.
    let sender = unsafe { &*context.cast::<async_channel::Sender<u32>>() };
    let _ = sender.try_send(identifier);
}

pub struct Hotkey {
    handle: Option<NonNull<c_void>>,
    _sender: Box<async_channel::Sender<u32>>,
    _main_thread: MainThreadMarker,
}
impl Hotkey {
    pub fn register(
        key: &str,
        modifiers: u32,
        identifier: u32,
        sender: async_channel::Sender<u32>,
    ) -> Result<Self> {
        let main_thread = MainThreadMarker::new()
            .context("Shortcuts must be registered on the application main thread")?;
        let mut sender = Box::new(sender);
        let key = CString::new(key)?;
        let mut error = 0;
        // SAFETY: Construction is main-thread-only and the boxed callback stays
        // alive until successful unregistration or explicit native detachment.
        let handle = unsafe {
            GattoHotkeyCreate(
                key.as_ptr(),
                modifiers,
                identifier,
                send_event,
                (&mut *sender as *mut async_channel::Sender<u32>).cast(),
                &mut error,
            )
        };
        let Some(handle) = NonNull::new(handle) else {
            bail!(
                "macOS could not register this shortcut (status {error}). It may be unavailable or require permission for media keys."
            );
        };
        Ok(Self {
            handle: Some(handle),
            _sender: sender,
            _main_thread: main_thread,
        })
    }
    pub fn unregister(&mut self) -> Result<()> {
        if let Some(handle) = self.handle {
            // SAFETY: This owns the live registration. Failure leaves it owned.
            let status = unsafe { GattoHotkeyDestroy(handle.as_ptr()) };
            if status != 0 {
                bail!("Could not unregister the macOS shortcut (status {status})");
            }
            self.handle = None;
        }
        Ok(())
    }
}
impl Drop for Hotkey {
    fn drop(&mut self) {
        if self.unregister().is_err()
            && let Some(handle) = self.handle
        {
            // SAFETY: A failed OS cleanup must never leave a callback referring
            // to freed Rust memory. Native code leaks only the detached owner.
            unsafe { GattoHotkeyDetach(handle.as_ptr()) }
        }
    }
}

#[cfg(feature = "walkthrough-screenshots")]
#[allow(dead_code)] // Only the standalone main-thread harness calls this.
pub fn run_test_receiver() -> bool {
    file_service::checks::run_receiver()
}
#[cfg(feature = "walkthrough-screenshots")]
#[allow(dead_code)] // Only the standalone main-thread harness calls this.
pub fn verify_desktop() -> Result<()> {
    file_service::checks::verify()?;
    menu::verify()?;
    println!("Rust file service, menu lifecycle and action delivery passed.");
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn background_construction_rejects_main_thread_objects_before_native_registration() {
        std::thread::spawn(|| {
            assert!(super::Menu::new().is_err());
            let (sender, _events) = async_channel::unbounded();
            assert!(super::Hotkey::register("KeyG", 9, 1, sender).is_err());
            assert!(super::install_file_service().is_err());
        })
        .join()
        .unwrap();
    }
}

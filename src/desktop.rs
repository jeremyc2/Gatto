//! Main-thread-owned AppKit/Carbon adapters. Native callbacks only enqueue IDs;
//! they never reenter a GPUI borrow. No handle can cross a thread boundary.
use anyhow::{Result, bail};
use std::{
    ffi::{CString, c_char, c_void},
    marker::PhantomData,
    ptr::NonNull,
    rc::Rc,
};

unsafe extern "C" {
    fn GattoInstallFileService();
    fn GattoMenuCreate(
        bytes: *const u8,
        length: usize,
        callback: extern "C" fn(*mut c_void, u32),
        context: *mut c_void,
    ) -> *mut c_void;
    fn GattoMenuDestroy(handle: *mut c_void);
    fn GattoMenuPasteVisible(handle: *mut c_void, visible: bool);
    fn GattoMenuUpdate(handle: *mut c_void, state: u32, identifier: u32);
    fn GattoMenuShortcut(
        handle: *mut c_void,
        canonical: *const c_char,
        equivalent: *const c_char,
        modifiers: u32,
        numpad: bool,
    );
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

/// Called once on GPUI's application thread after its open-URL handler is ready.
pub fn install_file_service() {
    // SAFETY: AppKit owns the provider for the process lifetime. It forwards only
    // file URLs to GPUI's existing delegate and retains no Rust pointers.
    unsafe { GattoInstallFileService() }
}

extern "C" fn send_event(context: *mut c_void, identifier: u32) {
    // SAFETY: Each native owner is destroyed/detached before its boxed sender.
    let sender = unsafe { &*context.cast::<async_channel::Sender<u32>>() };
    let _ = sender.try_send(identifier);
}

pub struct Menu {
    handle: NonNull<c_void>,
    _sender: Box<async_channel::Sender<u32>>,
    _main_thread: PhantomData<Rc<()>>,
}

impl Menu {
    pub fn new() -> Result<(Self, async_channel::Receiver<u32>)> {
        let (sender, receiver) = async_channel::unbounded();
        let mut sender = Box::new(sender);
        let png = include_bytes!("../packaging/menu-bar-icon.png");
        // SAFETY: The image is borrowed only during construction. The sender's
        // heap address stays stable until after native destruction in Drop.
        let handle = unsafe {
            GattoMenuCreate(
                png.as_ptr(),
                png.len(),
                send_event,
                (&mut *sender as *mut async_channel::Sender<u32>).cast(),
            )
        };
        let Some(handle) = NonNull::new(handle) else {
            bail!("Could not create the macOS menu bar icon");
        };
        Ok((
            Self {
                handle,
                _sender: sender,
                _main_thread: PhantomData,
            },
            receiver,
        ))
    }
    pub fn paste_visible(&self, visible: bool) {
        // SAFETY: The main-thread handle remains owned by self.
        unsafe { GattoMenuPasteVisible(self.handle.as_ptr(), visible) }
    }
    pub fn update_state(&self, state: u32, identifier: u32) {
        // SAFETY: The main-thread handle remains owned by self.
        unsafe { GattoMenuUpdate(self.handle.as_ptr(), state, identifier) }
    }
    pub fn shortcut(
        &self,
        canonical: &str,
        equivalent: &str,
        modifiers: u32,
        numpad: bool,
    ) -> Result<()> {
        let canonical = CString::new(canonical)?;
        let equivalent = CString::new(equivalent)?;
        // SAFETY: Native setters copy both strings and retain no Rust borrows.
        unsafe {
            GattoMenuShortcut(
                self.handle.as_ptr(),
                canonical.as_ptr(),
                equivalent.as_ptr(),
                modifiers,
                numpad,
            )
        }
        Ok(())
    }
}
impl Drop for Menu {
    fn drop(&mut self) {
        // SAFETY: Remove all callback sources before the sender is freed.
        unsafe { GattoMenuDestroy(self.handle.as_ptr()) }
    }
}

pub struct Hotkey {
    handle: Option<NonNull<c_void>>,
    _sender: Box<async_channel::Sender<u32>>,
    _main_thread: PhantomData<Rc<()>>,
}
impl Hotkey {
    pub fn register(
        key: &str,
        modifiers: u32,
        identifier: u32,
        sender: async_channel::Sender<u32>,
    ) -> Result<Self> {
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
            _main_thread: PhantomData,
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

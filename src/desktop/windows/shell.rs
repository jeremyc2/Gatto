//! The hidden shell window runs on GPUI's UI thread. It owns the tray and
//! hotkeys and receives bounded activation messages from subsequent launches.
use crate::open_request::OpenRequest;
use anyhow::{Context as _, Result, bail, ensure};
use async_channel::Sender;
use std::{
    cell::RefCell,
    collections::HashMap,
    ffi::OsString,
    os::windows::ffi::OsStringExt as _,
    time::{Duration, Instant},
};
use windows::{
    Win32::{
        Foundation::{
            CloseHandle, ERROR_ALREADY_EXISTS, ERROR_SUCCESS, GetLastError, HANDLE, HWND, LPARAM,
            LRESULT, POINT, SetLastError, WPARAM,
        },
        System::{
            DataExchange::COPYDATASTRUCT, LibraryLoader::GetModuleHandleW, Threading::CreateMutexW,
        },
        UI::{
            Input::KeyboardAndMouse::{
                HOT_KEY_MODIFIERS, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT, RegisterHotKey,
                UnregisterHotKey,
            },
            Shell::{
                NIF_ICON, NIF_MESSAGE, NIF_SHOWTIP, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_SETVERSION,
                NIN_SELECT, NOTIFYICON_VERSION_4, NOTIFYICONDATAW, Shell_NotifyIconW,
            },
            WindowsAndMessaging::*,
        },
    },
    core::{HSTRING, PCWSTR, w},
};

const INSTANCE: PCWSTR = w!("Local\\Gatto.Desktop.78623da0");
const CLASS: PCWSTR = w!("Gatto.Desktop.78623da0");
const ACTIVATION: usize = 0x4741_5454;
const MAX_ACTIVATION_BYTES: usize = 65_536;
const TRAY_MESSAGE: u32 = WM_APP + 1;
const NIN_KEYSELECT: u32 = NIN_SELECT | 1;

#[derive(Default)]
struct State {
    hwnd: HWND,
    open: Option<Sender<Vec<OpenRequest>>>,
    menu: Option<Sender<u32>>,
    hotkeys: HashMap<i32, (u32, Sender<u32>)>,
    next_hotkey: i32,
    shortcut: String,
    clipboard_enabled: bool,
    taskbar_created: u32,
    icon: HICON,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::default();
}

struct Instance(HANDLE);
impl Drop for Instance {
    fn drop(&mut self) {
        // SAFETY: Our owned mutex handle releases the instance reservation.
        let _ = unsafe { CloseHandle(self.0) };
    }
}

pub struct Desktop {
    hwnd: HWND,
    _instance: Instance,
}

impl Desktop {
    pub fn start(args: &[OsString], open: Sender<Vec<OpenRequest>>) -> Result<Option<Self>> {
        // SAFETY: The mutex's name is static. Keep its handle until the window is destroyed.
        let (instance, existing) = unsafe {
            SetLastError(ERROR_SUCCESS);
            let handle = CreateMutexW(None, false, INSTANCE)?;
            (Instance(handle), GetLastError() == ERROR_ALREADY_EXISTS)
        };
        if existing {
            if args == ["--login"] {
                return Ok(None);
            }
            forward(args)?;
            return Ok(None);
        }

        // SAFETY: The registered procedure has Win32's window callback ABI. This
        // top-level window stays hidden but receives Explorer restart broadcasts.
        let (hwnd, taskbar_created) = unsafe {
            let module = GetModuleHandleW(None)?.into();
            let class = WNDCLASSW {
                lpfnWndProc: Some(window_proc),
                hInstance: module,
                lpszClassName: CLASS,
                ..Default::default()
            };
            ensure!(
                RegisterClassW(&class) != 0,
                "Could not register the Gatto shell window"
            );
            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                CLASS,
                PCWSTR::null(),
                WINDOW_STYLE(0),
                0,
                0,
                0,
                0,
                None,
                None,
                Some(module),
                None,
            )?;
            (hwnd, RegisterWindowMessageW(w!("TaskbarCreated")))
        };
        STATE.with_borrow_mut(|state| {
            state.hwnd = hwnd;
            state.open = Some(open.clone());
            state.taskbar_created = taskbar_created;
        });
        if !args.is_empty() && args != ["--login"] {
            let _ = open.try_send(OpenRequest::from_windows_args(args));
        }
        Ok(Some(Self {
            hwnd,
            _instance: instance,
        }))
    }
}

impl Drop for Desktop {
    fn drop(&mut self) {
        // SAFETY: This thread created and owns the window.
        let _ = unsafe { DestroyWindow(self.hwnd) };
    }
}

fn forward(args: &[OsString]) -> Result<()> {
    use std::os::windows::ffi::OsStrExt as _;
    let wire: Vec<u16> = args
        .iter()
        .flat_map(|arg| arg.encode_wide().chain([0]))
        .collect();
    ensure!(
        wire.len() * 2 <= MAX_ACTIVATION_BYTES,
        "Too many files to send to Gatto at once"
    );
    let deadline = Instant::now() + Duration::from_secs(5);
    let hwnd = loop {
        // SAFETY: The class points to a static wide string.
        if let Ok(hwnd) = unsafe { FindWindowW(CLASS, PCWSTR::null()) } {
            break hwnd;
        }
        ensure!(
            Instant::now() < deadline,
            "Gatto is already running but did not respond. Try again shortly."
        );
        std::thread::sleep(Duration::from_millis(25));
    };
    let data = COPYDATASTRUCT {
        dwData: ACTIVATION,
        cbData: (wire.len() * 2) as u32,
        lpData: if wire.is_empty() {
            std::ptr::null_mut()
        } else {
            wire.as_ptr().cast_mut().cast()
        },
    };
    let mut response = 0;
    // SAFETY: Both the structure and its read-only buffer remain live until Win32
    // returns. The other process receives a copy. Permission only allows foregrounding.
    let delivered = unsafe {
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid != 0 {
            let _ = AllowSetForegroundWindow(pid);
        }
        SendMessageTimeoutW(
            hwnd,
            WM_COPYDATA,
            WPARAM(0),
            LPARAM(&data as *const _ as isize),
            SMTO_ABORTIFHUNG | SMTO_BLOCK,
            5000,
            Some(&mut response),
        )
    };
    ensure!(
        delivered.0 != 0 && response == 1,
        "The running copy of Gatto did not accept this launch. Try again shortly."
    );
    Ok(())
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_COPYDATA if lparam.0 != 0 => {
            // SAFETY: Win32 provides a COPYDATASTRUCT and read-only data valid for
            // this callback. Validate the length before reading the buffer.
            let data = unsafe { &*(lparam.0 as *const COPYDATASTRUCT) };
            let size = data.cbData as usize;
            if data.dwData != ACTIVATION || size > MAX_ACTIVATION_BYTES || size % 2 != 0 {
                return LRESULT(0);
            }
            let args = if size == 0 {
                vec![]
            } else if !data.lpData.is_null() {
                let bytes = unsafe { std::slice::from_raw_parts(data.lpData.cast::<u8>(), size) };
                let wire: Vec<_> = bytes
                    .chunks_exact(2)
                    .map(|b| u16::from_le_bytes([b[0], b[1]]))
                    .collect();
                if wire.last() != Some(&0) {
                    return LRESULT(0);
                }
                wire[..wire.len() - 1]
                    .split(|u| *u == 0)
                    .map(OsString::from_wide)
                    .collect()
            } else {
                return LRESULT(0);
            };
            let sender = STATE.with_borrow(|state| state.open.clone());
            LRESULT(sender.is_some_and(|sender| {
                sender
                    .try_send(OpenRequest::from_windows_args(&args))
                    .is_ok()
            }) as isize)
        }
        WM_HOTKEY => {
            STATE.with_borrow(|state| {
                if let Some((event, sender)) = state.hotkeys.get(&(wparam.0 as i32)) {
                    let _ = sender.try_send(*event);
                }
            });
            LRESULT(0)
        }
        TRAY_MESSAGE => {
            let event = {
                let notification = (lparam.0 & 0xffff) as u32;
                // Shell uses WM_CONTEXTMENU for version 4 and WM_RBUTTONUP
                // on shells that declined the version upgrade.
                if matches!(notification, WM_CONTEXTMENU | WM_RBUTTONUP) {
                    popup(hwnd, (notification == WM_CONTEXTMENU).then_some(wparam))
                } else if matches!(notification, NIN_SELECT | NIN_KEYSELECT | WM_LBUTTONUP) {
                    1
                } else {
                    0
                }
            };
            if event != 0 {
                STATE.with_borrow(|state| {
                    if let Some(sender) = &state.menu {
                        let _ = sender.try_send(event);
                    }
                });
            }
            LRESULT(0)
        }
        _ if STATE.with_borrow(|state| {
            state.taskbar_created != 0 && state.taskbar_created == message
        }) =>
        {
            let icon = STATE.with_borrow(|state| state.icon);
            if !icon.is_invalid() {
                let _ = show_icon(hwnd, icon);
            }
            LRESULT(0)
        }
        // SAFETY: Pass unchanged arguments back to Windows for unhandled messages.
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

fn icon_data(hwnd: HWND) -> NOTIFYICONDATAW {
    NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: 1,
        ..Default::default()
    }
}

fn show_icon(hwnd: HWND, icon: HICON) -> Result<()> {
    let mut data = icon_data(hwnd);
    data.hIcon = icon;
    data.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP | NIF_SHOWTIP;
    data.uCallbackMessage = TRAY_MESSAGE;
    for (dest, src) in data.szTip.iter_mut().zip("Gatto".encode_utf16()) {
        *dest = src;
    }
    // SAFETY: data and its handles remain valid for the shell calls.
    ensure!(
        unsafe { Shell_NotifyIconW(NIM_ADD, &data) }.as_bool(),
        "Could not add the Gatto tray icon"
    );
    data.Anonymous.uVersion = NOTIFYICON_VERSION_4;
    let _ = unsafe { Shell_NotifyIconW(NIM_SETVERSION, &data) };
    Ok(())
}

pub struct Menu;

impl Menu {
    pub fn new() -> Result<(Self, async_channel::Receiver<u32>)> {
        let (sender, receiver) = async_channel::unbounded();
        let hwnd = STATE.with_borrow(|state| state.hwnd);
        ensure!(
            !hwnd.is_invalid(),
            "Gatto's Windows shell window is unavailable"
        );
        let icon = load_app_icon(0, 0)?;
        if let Err(error) = show_icon(hwnd, icon) {
            // SAFETY: load_app_icon returns an owned icon.
            let _ = unsafe { DestroyIcon(icon) };
            return Err(error);
        }
        STATE.with_borrow_mut(|state| {
            state.menu = Some(sender);
            state.icon = icon;
        });
        Ok((Self, receiver))
    }

    pub fn shortcut(&self, label: &str, _: &str, _: u32, _: bool) -> Result<()> {
        STATE.with_borrow_mut(|state| {
            state.shortcut = label.replace("Control", "Ctrl").replace("Option", "Alt")
        });
        Ok(())
    }

    pub fn update_state(&self, _: u32, _: u32) {}

    pub fn add_from_clipboard_enabled(&self, enabled: bool) {
        STATE.with_borrow_mut(|state| state.clipboard_enabled = enabled);
    }
}

impl Drop for Menu {
    fn drop(&mut self) {
        let (hwnd, icon) = STATE.with_borrow_mut(|state| {
            state.menu = None;
            (state.hwnd, std::mem::take(&mut state.icon))
        });
        // SAFETY: Remove the shell's reference before destroying the owned icon.
        unsafe {
            let _ = Shell_NotifyIconW(NIM_DELETE, &icon_data(hwnd));
            let _ = DestroyIcon(icon);
        }
    }
}

fn popup(hwnd: HWND, anchor: Option<WPARAM>) -> u32 {
    let (clipboard, shortcut) =
        STATE.with_borrow(|state| (state.clipboard_enabled, state.shortcut.clone()));
    let clipboard_label = if shortcut.is_empty() {
        "Add from Clipboard".to_owned()
    } else {
        format!("Add from Clipboard\t{shortcut}")
    };
    // SAFETY: The menu is destroyed on every path after successful creation.
    unsafe {
        let Ok(menu) = CreatePopupMenu() else {
            return 0;
        };
        for (id, label, flags) in [
            (1, "Open Gatto", MF_STRING),
            (
                3,
                clipboard_label.as_str(),
                if clipboard { MF_STRING } else { MF_GRAYED },
            ),
            (7, "Create side-by-side comparison", MF_STRING),
            (4, "App preferences", MF_STRING),
            (8, "Check for Windows updates…", MF_STRING),
            (9, "About Gatto", MF_STRING),
            (0, "", MF_SEPARATOR),
            (6, "Quit Gatto", MF_STRING),
        ] {
            let _ = AppendMenuW(menu, flags, id, &HSTRING::from(label));
        }
        let mut point = POINT::default();
        if let Some(WPARAM(value)) = anchor {
            point.x = value as i16 as i32;
            point.y = (value >> 16) as i16 as i32;
        }
        if anchor.is_none() || (point.x == -1 && point.y == -1) {
            let _ = GetCursorPos(&mut point);
        }
        let _ = SetForegroundWindow(hwnd);
        let command = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON,
            point.x,
            point.y,
            None,
            hwnd,
            None,
        );
        let _ = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0));
        let _ = DestroyMenu(menu);
        command.0 as u32
    }
}

pub struct Hotkey(Option<i32>);

impl Hotkey {
    pub fn register(key: &str, modifiers: u32, event: u32, sender: Sender<u32>) -> Result<Self> {
        ensure!(
            modifiers & 1 == 0,
            "Use Ctrl, Alt, or Shift for Windows global shortcuts."
        );
        let key = virtual_key(key)
            .context("Use a letter, number, or supported keyboard key for the Windows shortcut.")?;
        let mut flags = MOD_NOREPEAT;
        for (bit, flag) in [(2, MOD_CONTROL), (4, MOD_ALT), (8, MOD_SHIFT)] {
            if modifiers & bit != 0 {
                flags |= flag;
            }
        }
        STATE.with_borrow_mut(|state| {
            ensure!(
                !state.hwnd.is_invalid(),
                "Gatto's Windows shell window is unavailable"
            );
            // App-owned Win32 hotkey IDs are 0–0xBFFF. Keep native IDs
            // separate from the shared controller's monotonically increasing event IDs.
            for _ in 0..0xbfff {
                state.next_hotkey = state.next_hotkey % 0xbfff + 1;
                let id = state.next_hotkey;
                if let std::collections::hash_map::Entry::Vacant(slot) = state.hotkeys.entry(id) {
                    // SAFETY: Register against our UI-thread window, using an unused ID.
                    unsafe {
                        RegisterHotKey(Some(state.hwnd), id, HOT_KEY_MODIFIERS(flags.0), key)
                    }
                    .context(
                        "Windows could not reserve that shortcut. It may already be in use.",
                    )?;
                    slot.insert((event, sender));
                    return Ok(Self(Some(id)));
                }
            }
            bail!("No Windows hotkey IDs available")
        })
    }

    pub fn unregister(&mut self) -> Result<()> {
        if let Some(id) = self.0 {
            STATE.with_borrow_mut(|state| {
                // SAFETY: This registration was created on this UI thread.
                unsafe { UnregisterHotKey(Some(state.hwnd), id) }?;
                state.hotkeys.remove(&id);
                Ok::<_, windows::core::Error>(())
            })?;
            self.0 = None;
        }
        Ok(())
    }
}

impl Drop for Hotkey {
    fn drop(&mut self) {
        // TLS can already be tearing down at process exit.
        let _ = STATE.try_with(|_| self.unregister());
    }
}

fn virtual_key(key: &str) -> Option<u32> {
    use windows::Win32::UI::Input::KeyboardAndMouse::*;
    if let Some(key) = key
        .strip_prefix("Key")
        .or_else(|| key.strip_prefix("Digit"))
        && let [ascii] = key.as_bytes()
        && ascii.is_ascii_alphanumeric()
    {
        return Some(u32::from(*ascii));
    }
    if let Some(n) = key.strip_prefix('F').and_then(|n| n.parse::<u32>().ok()) {
        // F12 is reserved for the debugger even when one isn't running.
        return ((1..=24).contains(&n) && n != 12).then(|| u32::from(VK_F1.0) + n - 1);
    }
    if let Some([digit]) = key.strip_prefix("Numpad").map(str::as_bytes)
        && digit.is_ascii_digit()
    {
        return Some(u32::from(VK_NUMPAD0.0) + u32::from(digit - b'0'));
    }
    Some(u32::from(
        match key {
            "Backquote" => VK_OEM_3,
            "Backslash" => VK_OEM_5,
            "IntlBackslash" => VK_OEM_102,
            "BracketLeft" => VK_OEM_4,
            "BracketRight" => VK_OEM_6,
            "Comma" => VK_OEM_COMMA,
            "Equal" => VK_OEM_PLUS,
            "Minus" => VK_OEM_MINUS,
            "Period" => VK_OEM_PERIOD,
            "Quote" => VK_OEM_7,
            "Semicolon" => VK_OEM_1,
            "Slash" => VK_OEM_2,
            "Backspace" => VK_BACK,
            "Enter" => VK_RETURN,
            "Space" => VK_SPACE,
            "Tab" => VK_TAB,
            "Delete" => VK_DELETE,
            "End" => VK_END,
            "Home" => VK_HOME,
            "Insert" => VK_INSERT,
            "PageDown" => VK_NEXT,
            "PageUp" => VK_PRIOR,
            "Escape" => VK_ESCAPE,
            "ArrowDown" => VK_DOWN,
            "ArrowUp" => VK_UP,
            "ArrowLeft" => VK_LEFT,
            "ArrowRight" => VK_RIGHT,
            "NumpadAdd" => VK_ADD,
            "NumpadSubtract" => VK_SUBTRACT,
            "NumpadMultiply" => VK_MULTIPLY,
            "NumpadDivide" => VK_DIVIDE,
            "NumpadDecimal" => VK_DECIMAL,
            _ => return None,
        }
        .0,
    ))
}

fn load_app_icon(width: i32, height: i32) -> Result<HICON> {
    // SAFETY: Windows copies the compiled resource into a new icon.
    let module = unsafe { GetModuleHandleW(None) }?;
    let resource = PCWSTR(101_usize as *const u16); // MAKEINTRESOURCEW(101)
    let icon = unsafe {
        LoadImageW(
            Some(module.into()),
            resource,
            IMAGE_ICON,
            width,
            height,
            LR_DEFAULTSIZE,
        )
    }?;
    Ok(HICON(icon.0))
}

pub fn set_window_icon(window: &gpui_kit::Window) {
    use raw_window_handle::RawWindowHandle;
    let Ok(handle) = raw_window_handle::HasWindowHandle::window_handle(window) else {
        return;
    };
    let RawWindowHandle::Win32(raw) = handle.as_raw() else {
        return;
    };
    let hwnd = HWND(raw.hwnd.get() as *mut _);
    // Keep these icons for the thread lifetime since any GPUI window can use them.
    thread_local! {
        static ICONS: (Option<HICON>, Option<HICON>) = (
            load_app_icon(32, 32).ok(), load_app_icon(16, 16).ok()
        );
    }
    ICONS.with(|icons| {
        for (which, icon) in [(ICON_BIG, icons.0), (ICON_SMALL, icons.1)] {
            if let Some(icon) = icon {
                // SAFETY: The window and icon handles remain valid after the call.
                unsafe {
                    SendMessageW(
                        hwnd,
                        WM_SETICON,
                        Some(WPARAM(which as usize)),
                        Some(LPARAM(icon.0 as isize)),
                    )
                };
            }
        }
    });
}

pub fn launch_error(error: &anyhow::Error) {
    // SAFETY: The text is kept alive through the modal message box.
    unsafe {
        MessageBoxW(
            None,
            &HSTRING::from(format!("{error:#}")),
            w!("Gatto"),
            MB_OK | MB_ICONERROR,
        )
    };
}

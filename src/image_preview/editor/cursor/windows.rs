//! Honor the editor cursor for client-area mouse messages and let Windows and
//! GPUI handle non-client resize, typing, and all other cursors.
use super::*;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::{cell::RefCell, collections::HashMap};
use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    Graphics::Gdi::{
        BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateBitmap, CreateDIBSection, DIB_RGB_COLORS,
        DeleteObject,
    },
    UI::{
        HiDpi::GetDpiForWindow,
        Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass},
        WindowsAndMessaging::{
            CreateIconIndirect, DestroyCursor, HCURSOR, HTCLIENT, ICONINFO, SetCursor,
            WM_NCDESTROY, WM_SETCURSOR,
        },
    },
};

const SUBCLASS: usize = 1;

struct Cursor(HCURSOR);
impl Drop for Cursor {
    fn drop(&mut self) {
        // SAFETY: This is the owned result of CreateIconIndirect.
        let _ = unsafe { DestroyCursor(self.0) };
    }
}

#[derive(Default)]
struct Cursors {
    current: Option<(SelectionCursor, u32)>,
    cache: HashMap<(SelectionCursor, u32), Cursor>,
}

thread_local! {
    static WINDOWS: RefCell<HashMap<isize, Cursors>> = RefCell::default();
}

fn hwnd(window: &Window) -> Option<HWND> {
    let RawWindowHandle::Win32(raw) = HasWindowHandle::window_handle(window).ok()?.as_raw() else {
        return None;
    };
    Some(HWND(raw.hwnd.get() as *mut _))
}

pub fn set(window: &Window, kind: SelectionCursor, _: CursorStyle) {
    if !window.is_window_active() || matches!(kind, SelectionCursor::Arrow | SelectionCursor::Move)
    {
        clear(window);
        return;
    }
    let Some(hwnd) = hwnd(window) else { return };
    WINDOWS.with_borrow_mut(|windows| {
        let cursors = if let std::collections::hash_map::Entry::Occupied(entry) =
            windows.entry(hwnd.0 as isize)
        {
            entry.into_mut()
        } else {
            // SAFETY: This callback and the window both run on the GPUI UI thread.
            if !unsafe { SetWindowSubclass(hwnd, Some(subclass), SUBCLASS, 0) }.as_bool() {
                return;
            }
            windows.entry(hwnd.0 as isize).or_default()
        };
        // SAFETY: Query the DPI of the live window. Bound image allocation.
        let dpi = unsafe { GetDpiForWindow(hwnd) }.clamp(96, 384);
        let key = (kind, dpi);
        if !cursors.cache.contains_key(&key) {
            let Some(cursor) = make(kind, (CURSOR_SIZE as u32 * dpi + 48) / 96) else {
                return;
            };
            // Bound native handles. Switch away before dropping the active cursor.
            if cursors.cache.len() == 96 {
                unsafe { SetCursor(Some(cursor.0)) };
                cursors.cache.clear();
            }
            cursors.cache.insert(key, cursor);
        }
        let cursor = cursors.cache[&key].0;
        cursors.current = Some(key);
        // SAFETY: The cache holds the cursor while any window message can use it.
        unsafe { SetCursor(Some(cursor)) };
    });
}

pub fn clear(window: &Window) {
    let Some(hwnd) = hwnd(window) else { return };
    WINDOWS.with_borrow_mut(|windows| {
        if let Some(cursors) = windows.get_mut(&(hwnd.0 as isize)) {
            cursors.current = None;
        }
    });
}

unsafe extern "system" fn subclass(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _: usize,
    _: usize,
) -> LRESULT {
    if message == WM_SETCURSOR && (lparam.0 & 0xffff) as u32 == HTCLIENT {
        let cursor = WINDOWS.with_borrow(|windows| {
            let state = windows.get(&(hwnd.0 as isize))?;
            Some(state.cache.get(&state.current?)?.0)
        });
        if let Some(cursor) = cursor {
            // SAFETY: The cursor is retained in this window's cache.
            unsafe { SetCursor(Some(cursor)) };
            return LRESULT(1);
        }
    } else if message == WM_NCDESTROY {
        // SAFETY: This is the last message for this HWND. Remove our hook.
        let _ = unsafe { RemoveWindowSubclass(hwnd, Some(subclass), SUBCLASS) };
        WINDOWS.with_borrow_mut(|windows| {
            windows.remove(&(hwnd.0 as isize));
        });
    }
    // SAFETY: Forward the original arguments to the next handler.
    unsafe { DefSubclassProc(hwnd, message, wparam, lparam) }
}

fn make(kind: SelectionCursor, size: u32) -> Option<Cursor> {
    let image = cursor_image(kind)?;
    let mut scaled = tiny_skia::Pixmap::new(size, size)?;
    let transform = tiny_skia::Transform::from_scale(
        size as f32 / image.width() as f32,
        size as f32 / image.height() as f32,
    );
    scaled.draw_pixmap(
        0,
        0,
        image.as_ref(),
        &tiny_skia::PixmapPaint {
            quality: tiny_skia::FilterQuality::Bicubic,
            ..Default::default()
        },
        transform,
        None,
    );
    let size = size as i32;
    let mut info = BITMAPINFO::default();
    info.bmiHeader = BITMAPINFOHEADER {
        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: size,
        biHeight: -size,
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB.0,
        ..Default::default()
    };
    let mut bits = std::ptr::null_mut();
    // SAFETY: The DIB stores size × size BGRA pixels and takes ownership of
    // its pixel buffer. CreateIconIndirect copies the source bitmaps.
    unsafe {
        let color = CreateDIBSection(None, &info, DIB_RGB_COLORS, &mut bits, None, 0).ok()?;
        let mask_bytes = vec![0_u8; ((size as usize).div_ceil(16) * 2) * size as usize];
        let mask = CreateBitmap(size, size, 1, 1, Some(mask_bytes.as_ptr().cast()));
        let result = if !mask.is_invalid() && !bits.is_null() {
            let target = std::slice::from_raw_parts_mut(bits.cast::<u8>(), scaled.data().len());
            for (bgra, rgba) in target
                .chunks_exact_mut(4)
                .zip(scaled.data().chunks_exact(4))
            {
                bgra.copy_from_slice(&[rgba[2], rgba[1], rgba[0], rgba[3]]);
            }
            CreateIconIndirect(&ICONINFO {
                fIcon: false.into(),
                xHotspot: size as u32 / 2,
                yHotspot: size as u32 / 2,
                hbmMask: mask,
                hbmColor: color,
            })
            .ok()
            .map(|cursor| Cursor(HCURSOR(cursor.0)))
        } else {
            None
        };
        if !mask.is_invalid() {
            let _ = DeleteObject(mask.into());
        }
        let _ = DeleteObject(color.into());
        result
    }
}

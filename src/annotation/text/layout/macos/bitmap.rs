//! Fallible equivalent of the locked core-graphics bitmap constructor, whose
//! safe constructor asserts on native allocation failure. Native owns all bytes.
use core_foundation::base::{CFType, TCFType};
use core_graphics::{context::CGContext, sys};
use std::{ffi::c_void, ptr::NonNull};

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGColorSpaceCreateDeviceRGB() -> *mut sys::CGColorSpace;
    fn CGBitmapContextCreate(
        data: *mut c_void,
        width: usize,
        height: usize,
        bits_per_component: usize,
        bytes_per_row: usize,
        space: *mut sys::CGColorSpace,
        bitmap_info: u32,
    ) -> *mut sys::CGContext;
    fn CGContextRelease(context: *mut sys::CGContext);
}

struct CreatedContext(NonNull<sys::CGContext>);
impl Drop for CreatedContext {
    fn drop(&mut self) {
        // SAFETY: Balances this constructor's create reference exactly once.
        unsafe {
            CGContextRelease(self.0.as_ptr());
        }
    }
}

pub(super) fn rgba(width: u32, height: u32) -> Option<CGContext> {
    use core_graphics::base::{kCGBitmapByteOrder32Big, kCGImageAlphaPremultipliedLast};
    if width == 0 || height == 0 {
        return None;
    }
    let row = (width as usize).checked_mul(4)?;
    row.checked_mul(height as usize)?;
    // SAFETY: Create returns an owned color space or null. CGColorSpace is a
    // Core Foundation type; CFType releases it even if context creation fails.
    let space = unsafe { NonNull::new(CGColorSpaceCreateDeviceRGB())? };
    let space = unsafe { CFType::wrap_under_create_rule(space.as_ptr().cast()) };
    // SAFETY: Null data asks Core Graphics to allocate and own its storage.
    // Dimensions/stride are checked, format is 8-bit premultiplied RGBA, and the
    // context retains the color space independently of our local CFType owner.
    let raw = unsafe {
        CGBitmapContextCreate(
            std::ptr::null_mut(),
            width as usize,
            height as usize,
            8,
            row,
            space.as_CFTypeRef().cast_mut().cast(),
            kCGBitmapByteOrder32Big | kCGImageAlphaPremultipliedLast,
        )
    };
    let created = CreatedContext(NonNull::new(raw)?);
    // SAFETY: This known-live context is retained by the wrapper; CreatedContext
    // then releases the original create reference. No Rust allocation is borrowed.
    let context = unsafe { CGContext::from_existing_context_ptr(created.0.as_ptr()) };
    // Initialize every byte before glyph drawing/copying, including transparent
    // margins; never depend on native allocator contents or expose stale data.
    use core_graphics::geometry::{CGPoint, CGRect, CGSize};
    context.clear_rect(CGRect::new(
        &CGPoint::new(0., 0.),
        &CGSize::new(width.into(), height.into()),
    ));
    Some(context)
}

#[cfg(test)]
mod tests {
    #[test]
    fn rejects_empty_bitmaps_and_owns_a_tightly_packed_rgba_buffer() {
        assert!(super::rgba(0, 2).is_none());
        assert!(super::rgba(2, 0).is_none());
        assert!(super::rgba(u32::MAX, u32::MAX).is_none());
        let mut bitmap = super::rgba(5, 3).unwrap();
        assert_eq!(bitmap.width(), 5);
        assert_eq!(bitmap.bytes_per_row(), 20);
        assert_eq!(bitmap.data(), &[0; 60]);
    }
}

#[cfg(test)]
mod rendering_tests {
    #[test]
    fn native_owned_color_glyph_pixels_match_the_previous_borrowed_buffer() {
        use core_graphics::{
            base::{kCGBitmapByteOrder32Big, kCGImageAlphaPremultipliedLast},
            color_space::CGColorSpace,
            context::CGContext,
            geometry::CGPoint,
        };
        let face = core_text::font::new_from_name("AppleColorEmoji", 56.).unwrap();
        for text in ["🐈", "🦊", "✨"] {
            let characters = text.encode_utf16().collect::<Vec<_>>();
            let mut glyphs = vec![0; characters.len()];
            // SAFETY: Both arrays cover exactly the supplied UTF-16 count and
            // remain live exclusively throughout the synchronous font lookup.
            unsafe {
                face.get_glyphs_for_characters(
                    characters.as_ptr(),
                    glyphs.as_mut_ptr(),
                    characters.len() as _,
                );
            }
            assert_ne!(glyphs[0], 0);
            let mut expected = tiny_skia::Pixmap::new(128, 128).unwrap();
            // Reference implementation from before the migration. Its context
            // is consumed by draw_glyphs before Rust reads/drops the allocation.
            let reference = CGContext::create_bitmap_context(
                Some(expected.data_mut().as_mut_ptr().cast()),
                128,
                128,
                8,
                512,
                &CGColorSpace::create_device_rgb(),
                kCGBitmapByteOrder32Big | kCGImageAlphaPremultipliedLast,
            );
            face.draw_glyphs(&glyphs[..1], &[CGPoint::new(20., 60.)], reference);
            let mut actual = super::rgba(128, 128).unwrap();
            face.draw_glyphs(&glyphs[..1], &[CGPoint::new(20., 60.)], actual.clone());
            assert!(
                expected
                    .data()
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .any(|pixel| pixel[3] > 0)
            );
            assert_eq!(actual.data(), expected.data(), "{text}");
        }
    }
}

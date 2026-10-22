use super::*;

impl ImagePreview {
    pub(in crate::image_preview) fn preview_cursor_layer(
        &self,
        mut fallback: CursorStyle,
        window: &Window,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let active = self.editor.canvas_hover;
        if let Some(input) = &self.editor.editing {
            fallback = if input.read(cx).contains(window.mouse_position()) {
                CursorStyle::IBeam
            } else {
                CursorStyle::Arrow
            };
        }
        let kind = if self.editor.editing.is_none()
            && self.editor.tool == Tool::Select
            && self.drag.is_none()
            && !self.editor.space
        {
            self.editor.selection_cursor
        } else {
            SelectionCursor::Arrow
        };
        canvas(
            |_, _, _| (),
            move |_, _, window, cx| {
                if active {
                    // Paint after all controls: a descendant's default cursor must not
                    // override the operation beneath the pointer on the artwork.
                    window.set_window_cursor_style(fallback);
                    #[cfg(target_os = "macos")]
                    window.defer(cx, move |window, _| native::set(window, kind, fallback));
                }
                #[cfg(not(target_os = "macos"))]
                let _ = (kind, cx);
            },
        )
        .absolute()
        .inset_0()
    }
    pub(in crate::image_preview) fn sync_selection_cursor(
        &self,
        window: &Window,
        fallback: CursorStyle,
        cx: &mut Context<Self>,
    ) {
        if self.editor.editing.is_some() {
            return;
        }
        #[cfg(target_os = "macos")]
        {
            let cursor =
                if self.editor.tool == Tool::Select && self.drag.is_none() && !self.editor.space {
                    self.editor.selection_cursor
                } else {
                    SelectionCursor::Arrow
                };
            // Run after GPUI has applied its standard cursor for the completed frame.
            window.defer(cx, move |window, _| native::set(window, cursor, fallback));
        }
        #[cfg(not(target_os = "macos"))]
        let _ = (window, fallback, cx);
    }
}

#[cfg(target_os = "macos")]
mod native {
    use super::*;
    use objc2::{AnyThread as _, rc::Retained};
    use objc2_app_kit::{NSCursor, NSImage};
    use objc2_foundation::{NSData, NSPoint, NSSize};
    use raw_window_handle::RawWindowHandle;
    use std::{
        cell::{Cell, RefCell},
        collections::HashMap,
    };

    thread_local! {
        static CURSORS: RefCell<HashMap<SelectionCursor, Option<Retained<NSCursor>>>> = RefCell::default();
        static CUSTOM_ACTIVE: Cell<bool> = const { Cell::new(false) };
    }
    pub fn set(window: &Window, kind: SelectionCursor, fallback: CursorStyle) {
        if !window.is_window_active()
            || !raw_window_handle::HasWindowHandle::window_handle(window)
                .is_ok_and(|h| matches!(h.as_raw(), RawWindowHandle::AppKit(_)))
        {
            return;
        }
        if matches!(kind, SelectionCursor::Arrow | SelectionCursor::Move) {
            if CUSTOM_ACTIVE.replace(false) {
                let cursor = match fallback {
                    CursorStyle::Crosshair => NSCursor::crosshairCursor(),
                    CursorStyle::OpenHand => NSCursor::openHandCursor(),
                    CursorStyle::ClosedHand => NSCursor::closedHandCursor(),
                    CursorStyle::IBeam => NSCursor::IBeamCursor(),
                    _ => NSCursor::arrowCursor(),
                };
                cursor.set();
            }
            return;
        };
        CURSORS.with_borrow_mut(|cursors| {
            // Angles are whole degrees, so this cache has at most 360 rotate and
            // 180 resize images. Create only the orientations actually used.
            if let Some(cursor) = cursors.entry(kind).or_insert_with(|| make(kind)) {
                cursor.set();
                CUSTOM_ACTIVE.set(true);
            }
        });
    }
    fn make(kind: SelectionCursor) -> Option<Retained<NSCursor>> {
        let bytes = cursor_image(kind)?.encode_png().ok()?;
        let data = NSData::with_bytes(&bytes);
        let image = NSImage::initWithData(NSImage::alloc(), &data)?;
        image.setSize(NSSize::new(CURSOR_SIZE, CURSOR_SIZE));
        Some(NSCursor::initWithImage_hotSpot(
            NSCursor::alloc(),
            &image,
            NSPoint::new(CURSOR_SIZE / 2., CURSOR_SIZE / 2.),
        ))
    }
}

#[cfg(any(target_os = "macos", test))]
const CURSOR_SIZE: f64 = 24.;

/// Small closed arrow silhouettes: dark interior, thin white outline, dark outer keyline.
#[cfg(any(target_os = "macos", test))]
pub(super) fn cursor_image(kind: SelectionCursor) -> Option<tiny_skia::Pixmap> {
    use tiny_skia::{FillRule, LineJoin, Paint, PathBuilder, Pixmap, Stroke, Transform};
    let mut p = PathBuilder::new();
    match kind {
        SelectionCursor::Rotate(_) => {
            p.move_to(2.5, 7.);
            p.line_to(8.5, 1.5);
            p.line_to(8.5, 5.);
            p.cubic_to(15.5, 5., 19., 8.5, 19., 15.5);
            p.line_to(22.5, 15.5);
            p.line_to(17., 21.5);
            p.line_to(11.5, 15.5);
            p.line_to(15., 15.5);
            p.cubic_to(15., 11., 12.5, 9., 8.5, 9.);
            p.line_to(8.5, 12.5);
        }
        SelectionCursor::Resize(_) => {
            for (i, (x, y)) in [
                (2.5, 12.),
                (8.25, 6.5),
                (8.25, 10.),
                (15.75, 10.),
                (15.75, 6.5),
                (21.5, 12.),
                (15.75, 17.5),
                (15.75, 14.),
                (8.25, 14.),
                (8.25, 17.5),
            ]
            .into_iter()
            .enumerate()
            {
                if i == 0 {
                    p.move_to(x, y);
                } else {
                    p.line_to(x, y);
                }
            }
        }
        SelectionCursor::Arrow | SelectionCursor::Move => return None,
    }
    p.close();
    let mut path = p.finish()?;
    let angle = match kind {
        SelectionCursor::Rotate(angle) | SelectionCursor::Resize(angle) => angle,
        _ => unreachable!(),
    };
    path = path.transform(Transform::from_rotate_at(f32::from(angle), 12., 12.))?;
    // Leave room for sharp outlined tips, including their outer contrast keyline.
    path = path.transform(Transform::from_row(0.82, 0., 0., 0.82, 2.16, 2.16))?;
    // A two-times representation keeps the 24-point AppKit cursor crisp on Retina.
    let mut image = Pixmap::new(48, 48)?;
    let transform = Transform::from_scale(2., 2.);
    let mut paint = Paint::default();
    paint.set_color_rgba8(18, 18, 18, 255);
    image.stroke_path(
        &path,
        &paint,
        &Stroke {
            width: 2.3,
            line_join: LineJoin::Miter,
            ..Default::default()
        },
        transform,
        None,
    );
    image.fill_path(&path, &paint, FillRule::Winding, transform, None);
    paint.set_color_rgba8(255, 255, 255, 255);
    image.stroke_path(
        &path,
        &paint,
        &Stroke {
            width: 1.1,
            line_join: LineJoin::Miter,
            ..Default::default()
        },
        transform,
        None,
    );
    Some(image)
}

#[cfg(all(feature = "walkthrough-screenshots", target_os = "macos"))]
impl ImagePreview {
    #[allow(dead_code)] // Used by the walkthrough screenshot binary.
    pub fn write_walkthrough_cursors(output: &std::path::Path) -> anyhow::Result<()> {
        let kinds = [
            SelectionCursor::Rotate(270),
            SelectionCursor::Rotate(0),
            SelectionCursor::Rotate(90),
            SelectionCursor::Rotate(180),
            SelectionCursor::Resize(0),
            SelectionCursor::Resize(17),
            SelectionCursor::Resize(45),
            SelectionCursor::Resize(73),
            SelectionCursor::Resize(90),
            SelectionCursor::Resize(135),
        ];
        let width = kinds.len() as u32 * 80;
        let mut sheet = image::RgbaImage::new(width, 320);
        for (row, background) in [
            [17, 17, 17, 255],
            [255, 92, 0, 255],
            [67, 143, 255, 255],
            [244, 244, 245, 255],
        ]
        .into_iter()
        .enumerate()
        {
            for y in row * 80..(row + 1) * 80 {
                for x in 0..width {
                    sheet.put_pixel(x, y as u32, image::Rgba(background));
                }
            }
            for (col, kind) in kinds.into_iter().enumerate() {
                let icon = cursor_image(kind).unwrap();
                let decoded = image::load_from_memory(&icon.encode_png().unwrap())?.to_rgba8();
                image::imageops::overlay(
                    &mut sheet,
                    &decoded,
                    (col * 80 + 16) as i64,
                    (row * 80 + 16) as i64,
                );
            }
        }
        sheet.save(output)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outlined_cursor_images_are_compact_and_do_not_clip_their_tips() {
        assert_eq!(CURSOR_SIZE, 24.);
        assert!(cursor_image(SelectionCursor::Move).is_none());
        assert!(cursor_image(SelectionCursor::Arrow).is_none());
        for kind in (0..360)
            .map(SelectionCursor::Rotate)
            .chain((0..180).map(SelectionCursor::Resize))
        {
            let image = cursor_image(kind).unwrap();
            assert_eq!((image.width(), image.height()), (48, 48));
            let mut light = 0;
            let mut dark = 0;
            let mut extent = (48usize, 48usize, 0usize, 0usize);
            for (index, pixel) in image.data().as_chunks::<4>().0.iter().enumerate() {
                let (x, y) = (index % 48, index / 48);
                if x == 0 || y == 0 || x == 47 || y == 47 {
                    assert_eq!(pixel[3], 0, "{kind:?}: the tip is clipped at {x},{y}");
                }
                if pixel[3] == 255 {
                    light += usize::from(pixel[0] > 230);
                    dark += usize::from(pixel[0] < 30);
                    extent.0 = extent.0.min(x);
                    extent.1 = extent.1.min(y);
                    extent.2 = extent.2.max(x);
                    extent.3 = extent.3.max(y);
                }
            }
            assert!(
                light > 8 && dark > 20,
                "{kind:?}: needs both a white outline and dark body"
            );
            match kind {
                SelectionCursor::Resize(0) => assert!(extent.2 - extent.0 > extent.3 - extent.1),
                SelectionCursor::Resize(90) => assert!(extent.3 - extent.1 > extent.2 - extent.0),
                _ => {}
            }
        }
    }
}

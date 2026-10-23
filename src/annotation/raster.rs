//! Bounded, composed textures for objects that need bitmap glyphs or group opacity.
use super::*;
use gpui_kit::{Bounds, Corners, RenderImage, Window, point, px, size};

#[derive(Clone)]
pub(super) struct RasterPreview {
    pub texture: Arc<RenderImage>,
    bounds: Rect,
}

impl RasterPreview {
    pub fn translate(&mut self, dx: f32, dy: f32) {
        self.bounds.x += dx;
        self.bounds.y += dy;
    }

    pub fn new(mark: &Mark) -> Option<Self> {
        let b = mark.visual_bounds();
        let pad = (mark.style.font_size / 12.)
            .max(mark.style.width * 5.)
            .max(4. * mark.style.scale);
        // Two pixels per annotation unit, capped independently of the source image.
        let scale = (2. / mark.style.scale).min(2044. / (b.w.max(b.h) + 2. * pad));
        let bounds = Rect {
            x: b.x - pad,
            y: b.y - pad,
            w: b.w + 2. * pad,
            h: b.h + 2. * pad,
        };
        let w = (bounds.w * scale).ceil() as u32 + 2;
        let h = (bounds.h * scale).ceil() as u32 + 2;
        let mut image = Pixmap::new(w, h)?;
        mark.paint_layer(
            &mut image,
            mark.frame().transform().post_concat(Transform::from_row(
                scale,
                0.,
                0.,
                scale,
                1. - bounds.x * scale,
                1. - bounds.y * scale,
            )),
        );
        // Convert the completed layer in place before handing it to GPUI.
        // No second RGBA buffer is needed for channel order or straight alpha.
        for source in image.data_mut().as_chunks_mut::<4>().0 {
            let c = tiny_skia::PremultipliedColorU8::from_rgba(
                source[0], source[1], source[2], source[3],
            )?
            .demultiply();
            *source = [
                c.blue(),
                c.green(),
                c.red(),
                (f32::from(c.alpha()) * mark.style.opacity).round() as u8,
            ];
        }
        let pixels = image::RgbaImage::from_raw(w, h, image.take())?;
        Some(Self {
            texture: Arc::new(RenderImage::new(vec![image::Frame::new(pixels)])),
            bounds: Rect {
                x: bounds.x - 1. / scale,
                y: bounds.y - 1. / scale,
                w: w as f32 / scale,
                h: h as f32 / scale,
            },
        })
    }

    pub fn paint(&self, origin: Pos, scale: f32, window: &mut Window) {
        let b = self.bounds;
        let bounds = Bounds::new(
            point(px(origin.x + b.x * scale), px(origin.y + b.y * scale)),
            size(px(b.w * scale), px(b.h * scale)),
        );
        let _ = window.paint_image(
            bounds,
            bounds,
            Corners::default(),
            self.texture.clone(),
            0,
            false,
        );
    }
}

//! Fixed comparison captions share the freeform text layout and input, while
//! their reserved footer remains protected from annotations.
use super::*;

pub const MAX_WIDTH: u32 = 3840;
pub const MAX_HEIGHT: u32 = 2160;
pub const CAPTION_GAP: f32 = 16.;
/// Leave room below GitHub's 10 MB image limit; every saved edit is checked again.
pub const INITIAL_PNG_BUDGET: usize = 9_000_000;
pub const SAVE_PNG_BUDGET: usize = 9_500_000;
pub const LABEL_FONT_SIZE: f32 = 32.;
pub const BACKGROUND: [u8; 4] = [24, 27, 39, 255];

#[derive(Clone, Debug, PartialEq)]
pub struct Comparison {
    /// Image columns can be wider than their source images. Both columns have
    /// the same top and height, anchoring captions to one shared baseline.
    pub panels: [Rect; 2],
    pub labels: [String; 2],
}
impl Comparison {
    pub fn new(panels: [Rect; 2]) -> Self {
        Self {
            panels,
            labels: ["Before".into(), "After".into()],
        }
    }
    pub fn label_scale(&self) -> f32 {
        ((self.panels[1].x + self.panels[1].w + self.panels[0].x) / 1920.)
            .max(self.panels[0].h / 1000.)
            .clamp(1., 2.)
    }
    pub fn label(&self, index: usize) -> String {
        self.labels[index].replace("\r\n", "\n").replace('\r', "\n")
    }

    pub fn label_mark(&self, index: usize) -> Mark {
        let panel = self.panels[index];
        let mut mark = Mark::new(
            Tool::Text,
            Pos::new(
                panel.x,
                panel.y + panel.h + CAPTION_GAP * self.label_scale(),
            ),
            Style {
                font_size: LABEL_FONT_SIZE * self.label_scale(),
                ..Default::default()
            },
        );
        mark.wrap = Some(panel.w);
        // Fixed left/top anchor: unlike a freeform drawing, captions never move
        // when glyph metrics or trailing whitespace change.
        mark.text = self.label(index);
        mark
    }
    pub fn label_marks(&self) -> Vec<Mark> {
        (0..2).map(|i| self.label_mark(i)).collect()
    }
    pub fn caption_rect(&self, index: usize) -> Rect {
        let mut rect = self.label_mark(index).bounds();
        rect.h = self
            .label_marks()
            .iter()
            .map(|mark| mark.bounds().h)
            .fold(0., f32::max);
        rect
    }
    pub fn caption_band(&self) -> Rect {
        self.caption_rect(0).union(self.caption_rect(1))
    }
    fn footer_background(&self) -> Rect {
        let band = self.caption_band();
        Rect {
            x: 0.,
            y: band.y,
            w: self.panels[1].x + self.panels[1].w + self.panels[0].x,
            h: band.h + 16.,
        }
    }
    pub(super) fn paint(
        &self,
        target: &mut Pixmap,
        transform: Transform,
        layer: &mut Option<Pixmap>,
    ) -> Result<()> {
        let band = self.footer_background();
        let mut fill = Paint::default();
        fill.set_color_rgba8(BACKGROUND[0], BACKGROUND[1], BACKGROUND[2], BACKGROUND[3]);
        target.fill_rect(
            tiny_skia::Rect::from_xywh(band.x, band.y, band.w, band.h).unwrap(),
            &fill,
            transform,
            None,
        );
        for mark in self.label_marks() {
            mark.paint_onto(target, transform, layer)?;
        }
        Ok(())
    }
    pub(super) fn paint_caption_region(
        &self,
        origin: Pos,
        scale: f32,
        window: &mut gpui_kit::Window,
    ) {
        let rect = self.footer_background();
        window.paint_quad(gpui_kit::fill(
            gpui_kit::Bounds::new(
                gpui_kit::point(
                    gpui_kit::px(origin.x + rect.x * scale),
                    gpui_kit::px(origin.y + rect.y * scale),
                ),
                gpui_kit::size(gpui_kit::px(rect.w * scale), gpui_kit::px(rect.h * scale)),
            ),
            gpui_kit::rgb(0x181b27),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn document() -> Document {
        let mut document = Document::from_image(image::RgbaImage::from_pixel(
            856,
            380,
            image::Rgba(BACKGROUND),
        ));
        document.comparison = Some(Comparison::new([
            Rect {
                x: 16.,
                y: 64.,
                w: 400.,
                h: 300.,
            },
            Rect {
                x: 440.,
                y: 64.,
                w: 400.,
                h: 300.,
            },
        ]));
        document
    }
    #[test]
    fn captions_wrap_below_both_images_and_protect_the_shared_row() {
        let mut document = document();
        let comparison = document.comparison.as_mut().unwrap();
        comparison.labels[0] =
            "A caption that wraps naturally across several lines\nAn explicit second paragraph"
                .into();
        let rect = comparison.caption_rect(0);
        assert_eq!(rect.y, comparison.caption_rect(1).y);
        assert!(rect.y >= comparison.panels[0].y + comparison.panels[0].h);
        assert!(rect.h > LABEL_FONT_SIZE * 2.);
        assert_eq!(comparison.label_mark(0).text, comparison.labels[0]);
        let mut mark = Mark::new(
            Tool::Rectangle,
            Pos::new(rect.x, rect.y),
            Style {
                fill: Fill::Solid,
                color: 11,
                ..Default::default()
            },
        );
        mark.points[1] = Pos::new(rect.x + rect.w, rect.y + rect.h);
        document.marks.push(mark);
        let (bytes, bounds) = document.png(None).unwrap();
        let pixels = image::load_from_memory(&bytes).unwrap().to_rgba8();
        assert_eq!(
            pixels
                .get_pixel(
                    (rect.x - bounds.x + rect.w - 2.) as u32,
                    (rect.y - bounds.y + rect.h - 2.) as u32
                )
                .0,
            BACKGROUND
        );
        assert!(pixels.height() as f32 >= rect.y + rect.h);
        assert!(
            pixels
                .enumerate_pixels()
                .any(|(_, y, p)| y as f32 >= rect.y && p.0[..3] == COLORS[0])
        );
    }
    #[test]
    fn byte_budget_reduces_noisy_images_but_keeps_compressible_detail_and_original_pixels() {
        let mut seed = 123_u32;
        let noisy = image::RgbaImage::from_fn(2600, 1500, |_, _| {
            let mut next = || {
                seed ^= seed << 13;
                seed ^= seed >> 17;
                seed ^= seed << 5;
                seed as u8
            };
            image::Rgba([next(), next(), next(), 255])
        });
        let document = Document::from_image(noisy);
        let (bytes, size, _) = document.comparison_png(INITIAL_PNG_BUDGET).unwrap();
        assert!(bytes.len() <= INITIAL_PNG_BUDGET);
        assert!(
            size.0 < 2600 && size.1 < 1500,
            "fixture must exercise byte-driven reduction"
        );
        assert_eq!(
            document.base.dimensions(),
            (2600, 1500),
            "retain the original for future saves"
        );
        let decoded = image::load_from_memory(&bytes).unwrap();
        assert_eq!((decoded.width(), decoded.height()), size);
        let flat = Document::from_image(image::RgbaImage::from_pixel(
            2600,
            1500,
            image::Rgba(BACKGROUND),
        ));
        assert_eq!(
            flat.comparison_png(INITIAL_PNG_BUDGET).unwrap().1,
            (2600, 1500)
        );
    }
    #[test]
    fn comparison_encoding_preserves_translucent_annotations_outside_the_base() {
        let mut document = document();
        let mut mark = Mark::new(
            Tool::Rectangle,
            Pos::new(-100., -80.),
            Style {
                fill: Fill::Solid,
                opacity: 0.5,
                ..Default::default()
            },
        );
        mark.points[1] = Pos::new(-20., -10.);
        document.marks.push(mark);
        let expected =
            image::load_from_memory(&document.render(None).unwrap().0.encode_png().unwrap())
                .unwrap()
                .to_rgba8();
        let actual = image::load_from_memory(&document.png(None).unwrap().0)
            .unwrap()
            .to_rgba8();
        assert_eq!(actual, expected);
    }
    #[test]
    fn annotation_expansion_still_exports_within_budget_and_keeps_comparison_metadata() {
        let mut document = document();
        let mut mark = Mark::new(Tool::Arrow, Pos::new(-100., -80.), Style::default());
        mark.points[1] = Pos::new(3000., 1600.);
        document.marks.push(mark);
        let output = image::load_from_memory(&document.png(None).unwrap().0).unwrap();
        assert!(output.width() <= MAX_WIDTH && output.height() <= MAX_HEIGHT);
        assert_eq!(document.base.dimensions(), (856, 380));
        assert_eq!(
            document.comparison.as_ref().unwrap().labels,
            ["Before", "After"]
        );
    }
}

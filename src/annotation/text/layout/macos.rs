//! CoreText owns both line breaking and glyph placement, including kerning.
use super::*;
mod bitmap;
use core_foundation::{
    attributed_string::CFAttributedStringRef,
    base::{CFIndex, CFTypeRef},
};
use unicode_segmentation::UnicodeSegmentation;

#[link(name = "CoreText", kind = "framework")]
unsafe extern "C" {
    fn CTTypesetterCreateWithAttributedString(string: CFAttributedStringRef) -> CFTypeRef;
    fn CTTypesetterSuggestLineBreak(typesetter: CFTypeRef, start: CFIndex, width: f64) -> CFIndex;
}

pub(super) fn line_ranges(text: &str, size: f32, wrap: Option<f32>) -> Vec<Range<usize>> {
    use core_foundation::{
        attributed_string::CFMutableAttributedString,
        base::{CFRange, CFType, TCFType},
        string::CFString,
    };
    use core_text::{font::new_from_name, string_attributes::kCTFontAttributeName};
    let Some(width) = wrap.filter(|_| !text.is_empty()) else {
        return std::iter::once(0..text.len()).collect();
    };
    let font = new_from_name("Arial", f64::from(size)).expect("macOS provides Arial");
    let mut string = CFMutableAttributedString::new();
    string.replace_str(&CFString::new(text), CFRange::init(0, 0));
    unsafe {
        string.set_attribute(
            CFRange::init(0, string.char_len()),
            kCTFontAttributeName,
            &font,
        );
    }
    let raw = unsafe { CTTypesetterCreateWithAttributedString(string.as_concrete_TypeRef()) };
    if raw.is_null() {
        return std::iter::once(0..text.len()).collect();
    }
    // Core Foundation's owning wrapper releases the native typesetter on every exit.
    let typesetter = unsafe { CFType::wrap_under_create_rule(raw) };
    let mut utf16 = 0;
    let mut boundaries = Vec::new();
    for (byte, cluster) in text.grapheme_indices(true) {
        boundaries.push((utf16, byte));
        utf16 += cluster.encode_utf16().count() as CFIndex;
    }
    boundaries.push((utf16, text.len()));
    let mut ranges = Vec::new();
    let mut start = 0;
    while start + 1 < boundaries.len() {
        let count = unsafe {
            CTTypesetterSuggestLineBreak(
                typesetter.as_CFTypeRef(),
                boundaries[start].0,
                f64::from(width),
            )
        };
        let target = boundaries[start].0 + count;
        let end = (start + 1..boundaries.len())
            .take_while(|i| boundaries[*i].0 <= target)
            .last()
            .unwrap_or(start + 1);
        ranges.push(boundaries[start].1..boundaries[end].1);
        start = end;
    }
    ranges
}
pub(super) fn shape_line(text: &str, size: f32, line_height: f32) -> Line {
    use core_foundation::{
        attributed_string::CFMutableAttributedString,
        base::{CFRange, TCFType},
        string::CFString,
    };
    use core_graphics::{geometry::CGAffineTransform, path::CGPathElementType as Segment};
    use core_text::{
        font::{CTFont, new_from_name},
        line::CTLine,
        string_attributes::kCTFontAttributeName,
    };
    let face = new_from_name("Arial", f64::from(size)).expect("macOS provides Arial");
    let mut string = CFMutableAttributedString::new();
    string.replace_str(&CFString::new(text), CFRange::init(0, 0));
    unsafe {
        string.set_attribute(
            CFRange::init(0, string.char_len()),
            kCTFontAttributeName,
            &face,
        );
    }
    let line = CTLine::new_with_attributed_string(string.as_concrete_TypeRef());
    let metrics = line.get_typographic_bounds();
    let baseline =
        (line_height - (face.ascent() + face.descent()) as f32) / 2. + face.ascent() as f32;
    let mut parts = TextParts::default();
    let mut bitmaps = Vec::new();
    for run in line.glyph_runs().iter() {
        let run_face = run
            .attributes()
            .and_then(|attributes| {
                let key = unsafe { CFString::wrap_under_get_rule(kCTFontAttributeName) };
                attributes
                    .find(&key)
                    .and_then(|value| value.downcast::<CTFont>())
            })
            .unwrap_or_else(|| face.clone());
        for (glyph, at) in run.glyphs().iter().zip(run.positions().iter()) {
            let builder = RefCell::new(PathBuilder::new());
            let Ok(path) = run_face
                .create_path_for_glyph(*glyph, &CGAffineTransform::new(1., 0., 0., 1., 0., 0.))
            else {
                parts.push(None, Pos::default());
                if let Some(mut glyph) = bitmap_glyph(&run_face, *glyph) {
                    glyph.bounds.x += at.x as f32;
                    glyph.bounds.y += baseline - at.y as f32;
                    bitmaps.push(glyph);
                }
                continue;
            };
            path.apply(&|segment| {
                let mut builder = builder.borrow_mut();
                let mut points = [Pos::default(); 3];
                for (point, p) in points.iter_mut().zip(segment.points()) {
                    *point = Pos::new(p.x as f32, -p.y as f32);
                }
                match segment.element_type {
                    Segment::MoveToPoint => builder.move_to(points[0].x, points[0].y),
                    Segment::AddLineToPoint => builder.line_to(points[0].x, points[0].y),
                    Segment::AddQuadCurveToPoint => {
                        builder.quad_to(points[0].x, points[0].y, points[1].x, points[1].y)
                    }
                    Segment::AddCurveToPoint => builder.cubic_to(
                        points[0].x,
                        points[0].y,
                        points[1].x,
                        points[1].y,
                        points[2].x,
                        points[2].y,
                    ),
                    Segment::CloseSubpath => builder.close(),
                }
            });
            parts.push(
                builder.into_inner().finish(),
                Pos::new(at.x as f32, baseline - at.y as f32),
            );
        }
    }
    let (path, parts) = parts.finish();
    let mut utf16 = 0;
    let mut carets = Vec::new();
    for (i, c) in text.char_indices() {
        carets.push((i, line.get_string_offset_for_string_index(utf16) as f32));
        utf16 += c.len_utf16() as isize;
    }
    carets.push((
        text.len(),
        line.get_string_offset_for_string_index(utf16) as f32,
    ));
    Line {
        width: metrics.width as f32,
        path,
        parts,
        range: 0..text.len(),
        carets,
        bitmaps,
    }
}

/// Color/bitmap fonts have no CGPath. Rasterize only those glyphs, retaining
/// CoreText's chosen fallback font, baseline, color, and advance positions.
fn bitmap_glyph(face: &core_text::font::CTFont, glyph: u16) -> Option<BitmapGlyph> {
    use core_graphics::geometry::CGPoint;
    let b = face.get_bounding_rects_for_glyphs(
        core_text::font_descriptor::kCTFontOrientationDefault,
        &[glyph],
    );
    if b.size.width <= 0. || b.size.height <= 0. {
        return None;
    }
    let x = b.origin.x.floor() - 1.;
    let y = b.origin.y.floor() - 1.;
    let width = (b.origin.x + b.size.width).ceil() + 1. - x;
    let height = (b.origin.y + b.size.height).ceil() + 1. - y;
    let scale = 2f64.min(512. / width.max(height));
    let w = (width * scale).ceil() as u32;
    let h = (height * scale).ceil() as u32;
    // Core Graphics owns its drawing buffer. Never let a native context retain
    // a raw pointer into a Rust allocation whose borrow it cannot express.
    let mut context = bitmap::rgba(w, h)?;
    context.scale(scale, scale);
    context.translate(-x, -y);
    face.draw_glyphs(&[glyph], &[CGPoint::new(0., 0.)], context.clone());
    let image = Pixmap::from_vec(context.data().to_vec(), tiny_skia::IntSize::from_wh(w, h)?)?;
    // CoreText's bitmap glyph drawing already supplies top-to-bottom image rows.
    Some(BitmapGlyph {
        image: Arc::new(image),
        bounds: Rect {
            x: x as f32,
            y: (-y - f64::from(h) / scale) as f32,
            w: (f64::from(w) / scale) as f32,
            h: (f64::from(h) / scale) as f32,
        },
    })
}

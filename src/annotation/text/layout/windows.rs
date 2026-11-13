//! Shape Windows annotation text once for preview, editing, and export.
use super::*;
use cosmic_text::{
    Attrs, Buffer, Command, Cursor, Family, FontSystem, Metrics, Shaping, SwashCache, SwashContent,
    Wrap,
};

struct Fonts {
    system: FontSystem,
    rasterizer: SwashCache,
}

thread_local! {
    static FONTS: RefCell<Fonts> = RefCell::new(Fonts {
        system: FontSystem::new(),
        rasterizer: SwashCache::new(),
    });
}

fn layout(
    system: &mut FontSystem,
    text: &str,
    size: f32,
    height: f32,
    width: Option<f32>,
) -> Buffer {
    let mut buffer = Buffer::new(system, Metrics::new(size, height));
    buffer.set_wrap(if width.is_some() {
        Wrap::WordOrGlyph
    } else {
        Wrap::None
    });
    buffer.set_size(width, None);
    buffer.set_text(
        text,
        &Attrs::new().family(Family::Name("Arial")),
        Shaping::Advanced,
        None,
    );
    buffer.shape_until_scroll(system, false);
    buffer
}

pub(super) fn line_ranges(text: &str, size: f32, width: Option<f32>) -> Vec<Range<usize>> {
    if width.is_none() || text.is_empty() {
        return std::iter::once(0..text.len()).collect();
    }
    FONTS.with_borrow_mut(|fonts| {
        let buffer = layout(&mut fonts.system, text, size, line_height(size), width);
        let starts = buffer
            .layout_runs()
            .skip(1)
            .filter_map(|run| run.glyphs.iter().map(|glyph| glyph.start).min());
        let mut offsets = vec![0, text.len()];
        offsets.extend(starts);
        offsets.sort_unstable();
        offsets.dedup();
        offsets.windows(2).map(|pair| pair[0]..pair[1]).collect()
    })
}

pub(super) fn shape_line(text: &str, size: f32, height: f32) -> Line {
    FONTS.with_borrow_mut(|fonts| {
        let Fonts { system, rasterizer } = fonts;
        let buffer = layout(system, text, size, height, None);
        let mut parts = TextParts::default();
        let mut bitmaps = Vec::new();
        let mut carets = Vec::new();
        let mut width = 0.;
        if let Some(run) = buffer.layout_runs().next() {
            width = run.line_w;
            carets.extend(
                text.char_indices()
                    .map(|(byte, _)| byte)
                    .chain([text.len()])
                    .map(|byte| {
                        (
                            byte,
                            run.cursor_position(&Cursor::new(0, byte)).unwrap_or(width),
                        )
                    }),
            );
            for glyph in run.glyphs {
                let physical = glyph.physical((0., run.line_y), 1.);
                let at = Pos::new(
                    glyph.x + glyph.font_size * glyph.x_offset,
                    run.line_y + glyph.y - glyph.font_size * glyph.y_offset,
                );
                if let Some(image) = rasterizer.get_image_uncached(system, physical.cache_key)
                    && image.content == SwashContent::Color
                    && let Some(size) =
                        tiny_skia::IntSize::from_wh(image.placement.width, image.placement.height)
                {
                    let bounds = Rect {
                        x: (physical.x + image.placement.left) as f32,
                        y: (physical.y - image.placement.top) as f32,
                        w: size.width() as f32,
                        h: size.height() as f32,
                    };
                    let mut bytes = image.data;
                    for rgba in bytes.chunks_exact_mut(4) {
                        for channel in 0..3 {
                            rgba[channel] =
                                (u16::from(rgba[channel]) * u16::from(rgba[3]) / 255) as u8;
                        }
                    }
                    if let Some(image) = Pixmap::from_vec(bytes, size) {
                        parts.push(None, at);
                        bitmaps.push(BitmapGlyph {
                            image: Arc::new(image),
                            bounds,
                        });
                        continue;
                    }
                }
                let outline = rasterizer.get_outline_commands_uncached(system, physical.cache_key);
                let mut path = PathBuilder::new();
                for command in outline.as_deref().unwrap_or_default() {
                    match *command {
                        Command::MoveTo(p) => path.move_to(p.x, -p.y),
                        Command::LineTo(p) => path.line_to(p.x, -p.y),
                        Command::QuadTo(a, b) => path.quad_to(a.x, -a.y, b.x, -b.y),
                        Command::CurveTo(a, b, c) => path.cubic_to(a.x, -a.y, b.x, -b.y, c.x, -c.y),
                        Command::Close => path.close(),
                    }
                }
                parts.push(path.finish(), at);
            }
        }
        if carets.is_empty() {
            carets.push((0, 0.));
        }
        let (path, parts) = parts.finish();
        Line {
            width,
            path,
            parts,
            range: 0..text.len(),
            carets,
            bitmaps,
        }
    })
}

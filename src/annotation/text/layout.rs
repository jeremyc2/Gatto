//! One CoreText line-box layout for editing, geometry, native painting, and export.
use super::*;
use std::ops::Range;
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
};
mod macos;
use macos::{line_ranges, shape_line};

#[derive(Clone)]
pub(super) struct Line {
    pub width: f32,
    pub path: Option<Path>,
    parts: Vec<TextPart>,
    range: Range<usize>,
    carets: Vec<(usize, f32)>,
    bitmaps: Vec<BitmapGlyph>,
}

#[derive(Clone)]
struct BitmapGlyph {
    image: Arc<Pixmap>,
    bounds: Rect,
}

#[derive(Clone)]
struct TextPart {
    path: Path,
    origin: Pos,
}

// Keep words together, with a bounded fragment for unusually long unbroken text.
// Native meshes can then reuse unchanged geometry instead of tessellating a paragraph.
#[derive(Default)]
struct TextParts {
    parts: Vec<TextPart>,
    current: PathBuilder,
    origin: Option<Pos>,
    glyphs: usize,
}
impl TextParts {
    fn push(&mut self, path: Option<Path>, at: Pos) {
        let Some(path) = path else {
            self.flush();
            return;
        };
        if self.glyphs == 16 {
            self.flush();
        }
        let origin = *self.origin.get_or_insert(at);
        if let Some(path) =
            path.transform(Transform::from_translate(at.x - origin.x, at.y - origin.y))
        {
            self.current.push_path(&path);
        }
        self.glyphs += 1;
    }
    fn flush(&mut self) {
        if let Some(path) = std::mem::take(&mut self.current).finish() {
            self.parts.push(TextPart {
                path,
                origin: self.origin.unwrap(),
            });
        }
        self.origin = None;
        self.glyphs = 0;
    }
    fn finish(mut self) -> (Option<Path>, Vec<TextPart>) {
        self.flush();
        let mut combined = PathBuilder::new();
        for part in &self.parts {
            if let Some(path) = part
                .path
                .clone()
                .transform(Transform::from_translate(part.origin.x, part.origin.y))
            {
                combined.push_path(&path);
            }
        }
        (combined.finish(), self.parts)
    }
}

pub(crate) struct Layout {
    pub width: f32,
    pub height: f32,
    pub line_height: f32,
    lines: Vec<Line>,
}

type Key = (String, u32, Option<u32>, u32);
thread_local! {
    static CACHE: RefCell<HashMap<Key, Arc<Layout>>> = RefCell::default();
    static CACHE_BYTES: Cell<usize> = const { Cell::new(0) };
}

#[derive(Default)]
struct ShapedLines {
    entries: HashMap<(String, u32), Line>,
    bytes: usize,
}
thread_local! {
    static SHAPED_LINES: RefCell<ShapedLines> = RefCell::default();
}
impl Line {
    fn estimated_bytes(&self) -> usize {
        self.bitmaps
            .iter()
            .map(|glyph| glyph.image.data().len())
            .sum::<usize>()
            + self.carets.len() * std::mem::size_of::<(usize, f32)>()
            + self
                .path
                .as_ref()
                .map_or(0, |path| path.points().len() * 12)
            + self
                .parts
                .iter()
                .map(|part| part.path.points().len() * 12)
                .sum::<usize>()
    }
}
fn shaped_line(text: &str, size: f32, line_height: f32) -> Line {
    let key = (text.to_owned(), size.to_bits());
    SHAPED_LINES.with(|cache| {
        if let Some(line) = cache.borrow().entries.get(&key) {
            return line.clone();
        }
        let line = shape_line(text, size, line_height);
        let bytes = text.len() + line.estimated_bytes();
        const BUDGET: usize = 4 * 1024 * 1024;
        if bytes <= BUDGET {
            let mut cache = cache.borrow_mut();
            if cache.entries.len() >= 512 || cache.bytes + bytes > BUDGET {
                cache.entries.clear();
                cache.bytes = 0;
            }
            cache.entries.insert(key, line.clone());
            cache.bytes += bytes;
        }
        line
    })
}

pub fn line_height(size: f32) -> f32 {
    (size * 1.35).round().max(1.)
}

impl Layout {
    #[cfg(test)]
    pub fn get(text: &str, size: f32, wrap: Option<f32>) -> Arc<Self> {
        Self::get_scaled(text, size, wrap, 1.)
    }

    pub fn get_scaled(text: &str, size: f32, wrap: Option<f32>, scale: f32) -> Arc<Self> {
        // Measure in annotation units so line rounding, wrapping, glyphs, and
        // caret positions stay identical at every creation zoom.
        let size = size / scale;
        let wrap = wrap.map(|w| w / scale);
        let wrap = wrap.map(|w| w.floor().max(16.));
        let key = (
            text.to_owned(),
            size.to_bits(),
            wrap.map(f32::to_bits),
            scale.to_bits(),
        );
        CACHE.with(|cache| {
            if let Some(layout) = cache.borrow().get(&key) {
                return layout.clone();
            }
            let mut layout = Self::build(text, size, wrap);
            if scale != 1. {
                layout.width *= scale;
                layout.height *= scale;
                layout.line_height *= scale;
                for line in &mut layout.lines {
                    line.width *= scale;
                    for glyph in &mut line.bitmaps {
                        glyph.bounds.x *= scale;
                        glyph.bounds.y *= scale;
                        glyph.bounds.w *= scale;
                        glyph.bounds.h *= scale;
                    }
                    line.path = line
                        .path
                        .take()
                        .and_then(|p| p.transform(Transform::from_scale(scale, scale)));
                    for part in &mut line.parts {
                        part.path = part
                            .path
                            .clone()
                            .transform(Transform::from_scale(scale, scale))
                            .unwrap();
                        part.origin.x *= scale;
                        part.origin.y *= scale;
                    }
                    for (_, x) in &mut line.carets {
                        *x *= scale;
                    }
                }
            }
            let layout = Arc::new(layout);
            let mut cache = cache.borrow_mut();
            let estimate = text.len() * 2
                + layout
                    .lines
                    .iter()
                    .map(Line::estimated_bytes)
                    .sum::<usize>();
            const BUDGET: usize = 8 * 1024 * 1024;
            CACHE_BYTES.with(|bytes| {
                if cache.len() >= 128 || bytes.get() + estimate > BUDGET {
                    cache.clear();
                    bytes.set(0);
                }
                if estimate <= BUDGET {
                    cache.insert(key, layout.clone());
                    bytes.set(bytes.get() + estimate);
                }
            });
            layout
        })
    }

    fn build(text: &str, size: f32, wrap: Option<f32>) -> Self {
        let mut lines = Vec::new();
        let line_height = line_height(size);
        let mut paragraph_start = 0;
        for paragraph in text.split('\n') {
            for range in line_ranges(paragraph, size, wrap) {
                let mut line = shaped_line(&paragraph[range.clone()], size, line_height);
                line.range = paragraph_start + range.start..paragraph_start + range.end;
                lines.push(line);
            }
            paragraph_start += paragraph.len() + 1;
        }
        Self {
            width: lines
                .iter()
                .map(|line| line.width)
                .fold(0., f32::max)
                .max(1.),
            height: line_height * lines.len() as f32,
            line_height,
            lines,
        }
    }

    pub(crate) fn parts(&self, width: f32, align: Align) -> impl Iterator<Item = (&Path, Pos)> {
        self.lines.iter().enumerate().flat_map(move |(row, line)| {
            let x = align.offset((width - line.width).max(0.));
            let y = row as f32 * self.line_height;
            line.parts
                .iter()
                .map(move |part| (&part.path, Pos::new(x + part.origin.x, y + part.origin.y)))
        })
    }

    pub fn path(&self, width: f32, align: Align) -> Option<Path> {
        let mut builder = PathBuilder::new();
        for (row, line) in self.lines.iter().enumerate() {
            if let Some(path) = &line.path
                && let Some(path) = path.clone().transform(Transform::from_translate(
                    align.offset((width - line.width).max(0.)),
                    row as f32 * self.line_height,
                ))
            {
                builder.push_path(&path);
            }
        }
        builder.finish()
    }
    pub fn has_bitmaps(&self) -> bool {
        self.lines.iter().any(|line| !line.bitmaps.is_empty())
    }
    pub fn paint_bitmaps(
        &self,
        target: &mut Pixmap,
        width: f32,
        align: Align,
        transform: Transform,
    ) {
        for (row, line) in self.lines.iter().enumerate() {
            let x = align.offset((width - line.width).max(0.));
            let y = row as f32 * self.line_height;
            for glyph in &line.bitmaps {
                let b = glyph.bounds;
                target.draw_pixmap(
                    0,
                    0,
                    glyph.image.as_ref().as_ref(),
                    &tiny_skia::PixmapPaint::default(),
                    Transform::from_row(
                        b.w / glyph.image.width() as f32,
                        0.,
                        0.,
                        b.h / glyph.image.height() as f32,
                        x + b.x,
                        y + b.y,
                    )
                    .post_concat(transform),
                    None,
                );
            }
        }
    }
    /// Character advance boxes include letter holes and a small halo, while
    /// empty rows, alignment padding, and long spaces remain transparent.
    pub fn hit(&self, text: &str, p: Pos, width: f32, align: Align, padding: f32) -> bool {
        self.lines.iter().enumerate().any(|(row, line)| {
            let ink = line
                .path
                .as_ref()
                .map(|path| {
                    let b = path.bounds();
                    Rect {
                        x: b.x(),
                        y: b.y(),
                        w: b.width(),
                        h: b.height(),
                    }
                })
                .into_iter()
                .chain(line.bitmaps.iter().map(|glyph| glyph.bounds))
                .reduce(Rect::union);
            let Some(ink) = ink else {
                return false;
            };
            let y = row as f32 * self.line_height + ink.y;
            let offset = align.offset((width - line.width).max(0.));
            if !(Rect {
                x: offset,
                y,
                w: line.width,
                h: ink.h,
            })
            .contains(p, padding)
            {
                return false;
            }
            line.carets.windows(2).any(|pair| {
                let start = line.range.start + pair[0].0;
                let end = line.range.start + pair[1].0;
                if text
                    .get(start..end)
                    .is_none_or(|s| s.chars().all(char::is_whitespace))
                {
                    return false;
                }
                Rect {
                    x: offset + pair[0].1.min(pair[1].1),
                    y,
                    w: (pair[1].1 - pair[0].1).abs(),
                    h: ink.h,
                }
                .contains(p, padding)
            })
        })
    }
    pub fn caret(&self, index: usize, width: f32, align: Align) -> Pos {
        let row = self
            .lines
            .iter()
            .rposition(|line| line.range.start <= index)
            .unwrap_or(0);
        let line = &self.lines[row];
        let relative = index.saturating_sub(line.range.start);
        let x = line
            .carets
            .iter()
            .rev()
            .find(|(i, _)| *i <= relative)
            .map_or(0., |(_, x)| *x);
        Pos::new(
            align.offset((width - line.width).max(0.)) + x,
            row as f32 * self.line_height,
        )
    }
    pub fn index_at(&self, p: Pos, width: f32, align: Align) -> usize {
        let row = ((p.y / self.line_height).floor().max(0.) as usize).min(self.lines.len() - 1);
        let line = &self.lines[row];
        let x = p.x - align.offset((width - line.width).max(0.));
        line.range.start
            + line
                .carets
                .iter()
                .min_by(|a, b| (a.1 - x).abs().total_cmp(&(b.1 - x).abs()))
                .map_or(0, |(i, _)| *i)
    }
    pub fn selection(&self, range: Range<usize>, width: f32, align: Align) -> Vec<Rect> {
        self.lines
            .iter()
            .enumerate()
            .filter_map(|(row, line)| {
                if range.end <= line.range.start || range.start > line.range.end {
                    return None;
                }
                let at = |index: usize| {
                    line.carets
                        .iter()
                        .rev()
                        .find(|(i, _)| *i <= index.saturating_sub(line.range.start))
                        .map_or(0., |(_, x)| *x)
                };
                let x = at(range.start.max(line.range.start));
                let end = at(range.end.min(line.range.end));
                Some(Rect {
                    x: align.offset((width - line.width).max(0.)) + x.min(end),
                    y: row as f32 * self.line_height,
                    w: (end - x).abs().max(2.),
                    h: self.line_height,
                })
            })
            .collect()
    }
}

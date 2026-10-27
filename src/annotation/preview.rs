//! Cached native geometry and incremental pixelation tiles. No PNG encoding during interaction.
use super::*;
use gpui_kit::{Bounds, Pixels, Window, point, px, rgba, size};

#[derive(Default, Clone)]
pub struct PreviewCache {
    marks: Vec<CachedMark>,
    comparison: Option<Comparison>,
    comparison_start: usize,
    hover: Option<super::pixelation::HoverPreview>,
    pixelation: super::pixelation::PixelationPreview,
    retired: Vec<Arc<gpui_kit::RenderImage>>,
}
#[derive(Clone)]
struct CachedMark {
    mark: Mark,
    frame: Frame,
    paths: NativePaths,
    text: Vec<TextMesh>,
    raster: Option<super::raster::RasterPreview>,
}
#[derive(Clone)]
struct TextMesh {
    mesh: Arc<NativePaths>,
    origin: Pos,
}
impl PreviewCache {
    pub fn retired_images(&mut self, all: bool) -> Vec<Arc<gpui_kit::RenderImage>> {
        if all {
            self.retired.extend(
                self.marks
                    .iter_mut()
                    .filter_map(|mark| mark.raster.take())
                    .map(|raster| raster.texture),
            );
            self.marks.clear();
        }
        let mut retired = std::mem::take(&mut self.retired);
        retired.extend(self.pixelation.retired_images(all));
        retired
    }
    pub fn update(&mut self, doc: &Document) {
        if self.hover.as_ref().is_none_or(|hover| !hover.matches(doc)) {
            self.hover = Some(super::pixelation::HoverPreview::new(doc));
        }
        self.pixelation.update(doc);
        self.comparison = doc.comparison.clone();
        self.comparison_start = doc.marks.len();
        let labels = doc
            .comparison
            .as_ref()
            .map_or_else(Vec::new, Comparison::label_marks);
        let marks = doc.marks.iter().chain(&labels);
        let count = doc.marks.len() + labels.len();
        for (i, mark) in marks.enumerate() {
            if let Some(old) = self.marks.get_mut(i) {
                if old.mark == *mark {
                    continue;
                }
                if let Some(raster) = &mut old.raster {
                    let delta = Pos::new(
                        mark.points[0].x - old.mark.points[0].x,
                        mark.points[0].y - old.mark.points[0].y,
                    );
                    let mut moved = old.mark.clone();
                    moved.translate(delta.x, delta.y);
                    if moved == *mark {
                        raster.translate(delta.x, delta.y);
                        old.mark = mark.clone();
                        old.frame = mark.frame();
                        continue;
                    }
                }
                let mut old_style = old.mark.style.clone();
                old_style.opacity = mark.style.opacity;
                if old.raster.is_none()
                    && mark.style.opacity == 1.
                    && old.mark.tool == mark.tool
                    && old.mark.points == mark.points
                    && old.mark.number == mark.number
                    && old.mark.text == mark.text
                    && old.mark.wrap == mark.wrap
                    && old.mark.label_position == mark.label_position
                    && old.mark.flip_x == mark.flip_x
                    && old.mark.flip_y == mark.flip_y
                    && old_style == mark.style
                {
                    old.mark = mark.clone();
                    old.frame.rotation = mark.rotation;
                    continue;
                }
            }
            let paths = body_paths(mark);
            let text = text_meshes(mark);
            let cached = CachedMark {
                mark: mark.clone(),
                frame: mark.frame(),
                paths,
                text,
                raster: if !matches!(mark.tool, Tool::Redact | Tool::Marker)
                    && (mark.style.opacity < 1. || mark.has_bitmap_text())
                {
                    super::raster::RasterPreview::new(mark)
                } else {
                    None
                },
            };
            if i < self.marks.len() {
                if let Some(raster) = self.marks[i].raster.take() {
                    self.retired.push(raster.texture);
                }
                self.marks[i] = cached;
            } else {
                self.marks.push(cached);
            }
        }
        if self.marks.len() > count {
            self.retired.extend(
                self.marks
                    .drain(count..)
                    .filter_map(|mark| mark.raster)
                    .map(|raster| raster.texture),
            );
        }
    }
    pub fn paint(&self, origin: Pos, scale: f32, brush: Option<(Pos, f32)>, window: &mut Window) {
        let at = |p: Pos| point(px(origin.x + p.x * scale), px(origin.y + p.y * scale));
        // Pixelation is a source-image effect, below every annotation regardless of creation order.
        self.pixelation.paint(origin, scale, window);
        if let Some((center, diameter)) = brush
            && let Some(hover) = &self.hover
        {
            hover.paint(center, diameter * scale / 2., origin, scale, window);
        }
        for (index, cached) in self
            .marks
            .iter()
            .enumerate()
            .filter(|(_, cached)| cached.mark.tool != Tool::Redact)
        {
            if index == self.comparison_start
                && let Some(comparison) = &self.comparison
            {
                comparison.paint_caption_region(origin, scale, window);
            }
            if let Some(raster) = &cached.raster {
                raster.paint(origin, scale, window);
                continue;
            }
            let center = cached.frame.center();
            let (sin, cos) = cached.frame.rotation.sin_cos();
            let at = |p: Pos| {
                at(Pos::new(
                    center.x + (p.x - center.x) * cos - (p.y - center.y) * sin,
                    center.y + (p.x - center.x) * sin + (p.y - center.y) * cos,
                ))
            };
            for (path, color, opacity) in &cached.paths {
                let mut path = path.clone();
                let bounds = cached.frame.map_bounds(Rect {
                    x: path.bounds.origin.x.as_f32(),
                    y: path.bounds.origin.y.as_f32(),
                    w: path.bounds.size.width.as_f32(),
                    h: path.bounds.size.height.as_f32(),
                });
                path.bounds = Bounds::new(
                    point(
                        px(origin.x + bounds.x * scale),
                        px(origin.y + bounds.y * scale),
                    ),
                    size(px(bounds.w * scale), px(bounds.h * scale)),
                );
                for vertex in &mut path.vertices {
                    vertex.xy_position = at(Pos::new(
                        vertex.xy_position.x.as_f32(),
                        vertex.xy_position.y.as_f32(),
                    ));
                }
                let alpha = (cached.mark.style.opacity * opacity * 255.).round() as u32;
                window.paint_path(
                    path,
                    rgba(
                        (u32::from(color[0]) << 24)
                            | (u32::from(color[1]) << 16)
                            | (u32::from(color[2]) << 8)
                            | alpha,
                    ),
                );
            }
            let clip = Bounds::new(point(px(0.), px(0.)), window.viewport_size())
                .intersect(&window.content_mask().bounds);
            for (path, color, opacity) in
                text_layers(&cached.text, at, |bounds| bounds.intersects(&clip))
            {
                let alpha = (cached.mark.style.opacity * opacity * 255.).round() as u32;
                window.paint_path(
                    path,
                    rgba(
                        (u32::from(color[0]) << 24)
                            | (u32::from(color[1]) << 16)
                            | (u32::from(color[2]) << 8)
                            | alpha,
                    ),
                );
            }
        }
    }
}
type NativePath = (gpui_kit::Path<Pixels>, [u8; 3], f32);
type NativePaths = Vec<NativePath>;
fn body_paths(mark: &Mark) -> NativePaths {
    if mark.tool == Tool::Marker {
        return mark
            .marker_layers()
            .into_iter()
            .filter_map(|(path, color)| Some((native_path(&path)?, color, 1.)))
            .collect();
    }
    let color = COLORS[mark.style.color];
    if mark.tool == Tool::Text {
        return Vec::new();
    }
    let path = mark.body_path();
    if mark.is_line_dot() {
        let result: NativePaths = path
            .as_ref()
            .and_then(native_path)
            .into_iter()
            .map(|p| (p, color, 1.))
            .collect();
        return result;
    }
    if mark.tool == Tool::Redact {
        return Vec::new();
    }
    let stroke = Stroke {
        width: mark.style.width,
        line_cap: LineCap::Round,
        line_join: tiny_skia::LineJoin::Round,
        ..Default::default()
    };
    let mut result = Vec::new();
    let mut combined = PathBuilder::new();
    if mark.tool.is_shape() && mark.style.fill != Fill::None {
        let body = mark.body_path().unwrap();
        if let Some(fill) = native_path(&body) {
            let alpha = if mark.style.fill == Fill::Solid {
                1.
            } else {
                40. / 255.
            };
            result.push((fill, mark.style.fill_color(), alpha));
        }
        if mark.style.fill == Fill::Hatch
            && let Some(hatch) = shapes::hatch_path(&body, mark.bounds(), mark.style.scale)
                .and_then(|path| {
                    path.stroke(
                        &Stroke {
                            width: (mark.style.width * 0.4).max(mark.style.scale),
                            ..Default::default()
                        },
                        1.,
                    )
                })
                .and_then(|path| native_path(&path))
        {
            result.push((hatch, color, 150. / 255.));
        }
    }
    let dashed = mark
        .style
        .dash
        .pattern(mark.style.width)
        .and_then(|dash| path.as_ref().and_then(|path| path.dash(&dash, 1.)));
    if let Some(stroked) = dashed
        .as_ref()
        .or(path.as_ref())
        .and_then(|path| path.stroke(&stroke, 1.))
    {
        combined.push_path(&stroked);
    }
    if mark.tool == Tool::Arrow {
        for (tip, toward, cap) in [
            (
                mark.points[0],
                mark.points.get(2).copied().unwrap_or(mark.points[1]),
                mark.style.start,
            ),
            (
                mark.points[1],
                mark.points.get(2).copied().unwrap_or(mark.points[0]),
                mark.style.end,
            ),
        ] {
            if let Some(cap_path) = cap_path(tip, toward, cap, mark.style.width) {
                if cap == Cap::Arrow {
                    if let Some(stroked) = cap_path.stroke(&stroke, 1.) {
                        combined.push_path(&stroked);
                    }
                } else {
                    combined.push_path(&cap_path);
                }
            }
        }
    }
    if let Some(symbol) = mark.box_symbol() {
        combined.push_path(&symbol);
    }
    if let Some(path) = combined.finish().and_then(|path| native_path(&path)) {
        result.push((path, color, 1.));
    }
    result
}

mod text_mesh;

fn text_meshes(mark: &Mark) -> Vec<TextMesh> {
    if mark.text.is_empty() {
        return Vec::new();
    }
    let Some(bounds) = mark.label_edit_bounds() else {
        return Vec::new();
    };
    mark.text_layout()
        .parts(bounds.w, mark.text_align())
        .map(|(part, origin)| TextMesh {
            mesh: text_mesh::get(
                part,
                mark.text_font_size(),
                mark.style.scale,
                mark.label_color(),
            ),
            origin: Pos::new(bounds.x + origin.x, bounds.y + origin.y),
        })
        .collect()
}

// Transform only visible text and submit one outline plus one ink layer.
// Keeping shared local meshes avoids copying an entire paragraph on each edit.
fn text_layers(
    parts: &[TextMesh],
    at: impl Fn(Pos) -> gpui_kit::Point<Pixels>,
    visible: impl Fn(Bounds<Pixels>) -> bool,
) -> NativePaths {
    let mut layers: [Option<NativePath>; 2] = [None, None];
    for part in parts {
        for (i, (source, color, opacity)) in part.mesh.iter().enumerate() {
            let b = source.bounds;
            let at = |p: Pos| at(Pos::new(p.x + part.origin.x, p.y + part.origin.y));
            let corners = Rect {
                x: b.origin.x.as_f32(),
                y: b.origin.y.as_f32(),
                w: b.size.width.as_f32(),
                h: b.size.height.as_f32(),
            }
            .corners()
            .map(at);
            let mut bounds = Bounds::new(corners[0], size(px(0.), px(0.)));
            for corner in &corners[1..] {
                bounds = bounds.union(&Bounds::new(*corner, size(px(0.), px(0.))));
            }
            if !visible(bounds) {
                continue;
            }
            let layer = if part.mesh.len() == 2 { i } else { 1 };
            let (path, _, _) = layers[layer].get_or_insert_with(|| {
                let mut path = gpui_kit::Path::new(bounds.origin);
                path.bounds = bounds;
                (path, *color, *opacity)
            });
            path.bounds = path.bounds.union(&bounds);
            path.vertices
                .extend(source.vertices.iter().map(|vertex| gpui_kit::PathVertex {
                    xy_position: at(Pos::new(
                        vertex.xy_position.x.as_f32(),
                        vertex.xy_position.y.as_f32(),
                    )),
                    st_position: vertex.st_position,
                    content_mask: vertex.content_mask,
                }));
        }
    }
    layers.into_iter().flatten().collect()
}

#[cfg(test)]
fn paths(mark: &Mark) -> NativePaths {
    let mut paths = body_paths(mark);
    paths.extend(text_layers(
        &text_meshes(mark),
        |p| point(px(p.x), px(p.y)),
        |_| true,
    ));
    paths
}

fn outlined_text(
    path: &Path,
    font_size: f32,
    scale: f32,
    color: [u8; 3],
) -> Vec<(gpui_kit::Path<Pixels>, [u8; 3], f32)> {
    let edge = if color.iter().map(|c| u32::from(*c)).sum::<u32>() < 280 {
        255
    } else {
        0
    };
    let mut result = Vec::new();
    if let Some(outline) = path.stroke(
        &Stroke {
            width: (font_size / 12.).max(2. * scale),
            line_join: tiny_skia::LineJoin::Round,
            ..Default::default()
        },
        1.,
    ) && let Some(outline) = native_path(&outline)
    {
        result.push((outline, [edge; 3], 1.));
    }
    if let Some(path) = native_path(path) {
        result.push((path, color, 1.));
    }
    result
}

/// Swatches use the same transparent, soft, solid, and diagonal-hatch treatments as shapes.
pub fn fill_icon(fill: Fill, color: gpui_kit::Hsla) -> impl gpui_kit::IntoElement {
    use gpui_kit::Styled as _;
    gpui_kit::canvas(
        move |bounds, _, _| {
            let edge = (bounds.size.width.as_f32().min(bounds.size.height.as_f32()) - 6.).max(1.);
            let b = Rect {
                x: bounds.origin.x.as_f32() + (bounds.size.width.as_f32() - edge) / 2.,
                y: bounds.origin.y.as_f32() + (bounds.size.height.as_f32() - edge) / 2.,
                w: edge,
                h: edge,
            };
            let Some(body) = shapes::shape_path(Tool::Rectangle, b, 1.) else {
                return Vec::new();
            };
            let mut paths = Vec::new();
            if fill != Fill::None
                && let Some(path) = native_path(&body)
            {
                paths.push((path, if fill == Fill::Solid { 1. } else { 40. / 255. }));
            }
            if fill == Fill::Hatch {
                let mut hatch = PathBuilder::new();
                for offset in [-12f32, -6., 0., 6., 12.] {
                    let offset = offset * edge / 18.;
                    let (x, y) = (b.x + offset.max(0.), b.y + (-offset).max(0.));
                    let length = edge - offset.abs();
                    hatch.move_to(x, y);
                    hatch.line_to(x + length, y + length);
                }
                if let Some(path) = hatch
                    .finish()
                    .and_then(|path| {
                        path.stroke(
                            &Stroke {
                                width: 1.5,
                                ..Default::default()
                            },
                            1.,
                        )
                    })
                    .and_then(|path| native_path(&path))
                {
                    paths.push((path, 150. / 255.));
                }
            }
            if let Some(path) = body
                .stroke(
                    &Stroke {
                        width: 1.5,
                        ..Default::default()
                    },
                    1.,
                )
                .and_then(|path| native_path(&path))
            {
                paths.push((path, 1.));
            }
            paths
        },
        move |_, paths, window, _| {
            for (path, alpha) in paths {
                let mut color = color;
                color.a *= alpha;
                window.paint_path(path, color);
            }
        },
    )
    .size(px(20.))
}

fn native_path(path: &Path) -> Option<gpui_kit::Path<Pixels>> {
    let mut builder = gpui_kit::PathBuilder::fill().with_style(gpui_kit::PathStyle::Fill(
        gpui_kit::FillOptions::default().with_fill_rule(gpui_kit::FillRule::NonZero),
    ));
    let at = |p: tiny_skia::Point| point(px(p.x), px(p.y));
    for segment in path.segments() {
        match segment {
            tiny_skia::PathSegment::MoveTo(p) => builder.move_to(at(p)),
            tiny_skia::PathSegment::LineTo(p) => builder.line_to(at(p)),
            tiny_skia::PathSegment::QuadTo(c, p) => builder.curve_to(at(p), at(c)),
            tiny_skia::PathSegment::CubicTo(a, b, p) => {
                builder.cubic_bezier_to(at(p), at(a), at(b))
            }
            tiny_skia::PathSegment::Close => builder.close(),
        }
    }
    builder.build().ok()
}

/// Match the other toolbar glyphs' 16-pixel square and stroke weight.
pub fn shape_icon(tool: Tool, color: gpui_kit::Hsla) -> impl gpui_kit::IntoElement {
    use gpui_kit::Styled as _;
    gpui_kit::canvas(
        move |bounds, _, _| {
            let edge = bounds
                .size
                .width
                .as_f32()
                .min(bounds.size.height.as_f32())
                .min(16.);
            let width = edge * if tool == Tool::Oval { 0.375 } else { 0.75 };
            let height = edge * 0.75;
            shapes::shape_path(
                tool,
                Rect {
                    x: bounds.origin.x.as_f32() + (bounds.size.width.as_f32() - width) / 2.,
                    y: bounds.origin.y.as_f32() + (bounds.size.height.as_f32() - height) / 2.,
                    w: width,
                    h: height,
                },
                1.,
            )
            .and_then(|path| {
                path.stroke(
                    &Stroke {
                        width: 4. / 3.,
                        line_join: tiny_skia::LineJoin::Round,
                        line_cap: LineCap::Round,
                        ..Default::default()
                    },
                    1.,
                )
            })
            .and_then(|path| native_path(&path))
        },
        move |_, path, window, _| {
            if let Some(path) = path {
                window.paint_path(path, color);
            }
        },
    )
    .size(px(16.))
    .flex_shrink_0()
}

/// A continuous freehand stroke, matching the toolbar's 16-pixel vector glyphs.
pub fn scribble_icon(color: gpui_kit::Hsla) -> impl gpui_kit::IntoElement {
    use gpui_kit::Styled as _;
    gpui_kit::canvas(
        move |bounds, _, _| {
            let mut path = PathBuilder::new();
            path.move_to(2., 15.);
            path.cubic_to(8., 9., 15., 1., 17., 3.);
            path.cubic_to(20., 6., 3., 18., 6., 21.);
            path.cubic_to(8., 23., 15., 11., 17., 13.);
            path.cubic_to(19., 15., 11., 22., 15., 22.);
            path.cubic_to(18., 22., 20., 17., 22., 19.);
            path.finish()
                .and_then(|path| {
                    path.stroke(
                        &Stroke {
                            width: 2.,
                            line_join: tiny_skia::LineJoin::Round,
                            line_cap: LineCap::Round,
                            ..Default::default()
                        },
                        1.,
                    )
                })
                .and_then(|path| {
                    path.transform(
                        Transform::from_scale(2. / 3., 2. / 3.)
                            .post_translate(bounds.origin.x.as_f32(), bounds.origin.y.as_f32()),
                    )
                })
                .and_then(|path| native_path(&path))
        },
        move |_, path, window, _| {
            if let Some(path) = path {
                window.paint_path(path, color);
            }
        },
    )
    .size(px(16.))
    .flex_shrink_0()
}

/// A circular, colored photo with a pixelated edge, matching the round brush.
pub fn pixelate_icon() -> impl gpui_kit::IntoElement {
    use gpui_kit::Styled as _;
    static ICON: OnceLock<Arc<gpui_kit::RenderImage>> = OnceLock::new();
    let icon = ICON
        .get_or_init(|| {
            let mut photo = image::RgbaImage::from_fn(64, 48, |x, y| {
                let t = y as f32 / 48.;
                let texture = ((x.wrapping_mul(73) ^ y.wrapping_mul(131)) % 9) as f32 - 4.;
                let mut color = [55. + 90. * t, 114. + 77. * t, 162. + 55. * t];
                let glow = (-(x as f32 - 46.).powi(2) / 32. - (y as f32 - 11.).powi(2) / 32.).exp();
                for (c, target) in [250., 228., 173.].into_iter().enumerate() {
                    color[c] = color[c] * (1. - glow) + target * glow;
                }
                if y as f32 > 25. + 6. * (x as f32 * 0.12).sin() + 3. * (x as f32 * 0.31).cos() {
                    color = [92. + texture, 119. + texture, 125. + texture];
                }
                if y as f32 > 35. + 6. * (x as f32 * 0.09 + 2.).sin() {
                    color = [25. + texture, 65. + texture, 61. + texture];
                }
                image::Rgba([color[0] as u8, color[1] as u8, color[2] as u8, 255])
            });
            for y in (0..48).step_by(8) {
                for x in (32..64).step_by(8) {
                    let color = *photo.get_pixel(x + 4, y + 4);
                    for yy in y..y + 8 {
                        for xx in x..x + 8 {
                            photo.put_pixel(xx, yy, color);
                        }
                    }
                }
            }
            for p in photo.pixels_mut() {
                p.0.swap(0, 2);
            }
            Arc::new(gpui_kit::RenderImage::new(vec![image::Frame::new(photo)]))
        })
        .clone();
    gpui_kit::canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            let diameter = bounds.size.width.min(bounds.size.height);
            let picture = Bounds::new(
                point(
                    bounds.origin.x + (bounds.size.width - diameter) / 2.,
                    bounds.origin.y + (bounds.size.height - diameter) / 2.,
                ),
                size(diameter, diameter),
            );
            let _ = window.paint_image(
                picture,
                picture,
                gpui_kit::Corners::all(diameter / 2.),
                icon.clone(),
                0,
                false,
            );
        },
    )
    .size(px(24.))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marker_preview_uses_vector_paths_for_badge_and_two_digit_number() {
        let mut doc = Document::from_image(image::RgbaImage::new(100, 100));
        let mut mark = Mark::new(Tool::Marker, Pos::new(50., 50.), Style::default());
        mark.number = Some(99);
        doc.marks.push(mark);
        let mut cache = PreviewCache::default();
        cache.update(&doc);
        assert!(
            cache.marks[0].raster.is_none(),
            "Markers must stay vector at any zoom"
        );
        assert!(
            cache.marks[0].paths.len() >= 3,
            "Circle borders and numeral need vector geometry"
        );
    }

    #[test]
    fn composed_textures_reuse_moves_and_retire_on_change_removal_and_close() {
        let mut doc =
            Document::decode(include_bytes!("../../packaging/menu-bar-icon.png")).unwrap();
        let mut mark = Mark::new(
            Tool::Rectangle,
            Pos::new(40., 40.),
            Style {
                fill: Fill::Solid,
                opacity: 0.5,
                ..Default::default()
            },
        );
        mark.points[1] = Pos::new(200., 200.);
        doc.marks.push(mark);
        let mut cache = PreviewCache::default();
        cache.update(&doc);
        let first = cache.marks[0].raster.as_ref().unwrap().texture.clone();
        cache.update(&doc);
        doc.marks[0].translate(20., -10.);
        cache.update(&doc);
        assert!(Arc::ptr_eq(
            &first,
            &cache.marks[0].raster.as_ref().unwrap().texture
        ));
        assert!(cache.retired_images(false).is_empty());
        doc.marks[0].style.opacity = 0.25;
        cache.update(&doc);
        let retired = cache.retired_images(false);
        assert_eq!(retired.len(), 1);
        assert!(Arc::ptr_eq(&first, &retired[0]));
        doc.marks.clear();
        cache.update(&doc);
        assert_eq!(cache.retired_images(false).len(), 1);
        doc.marks.push(Mark::new(
            Tool::Line,
            Pos::default(),
            Style {
                opacity: 0.5,
                ..Default::default()
            },
        ));
        cache.update(&doc);
        assert_eq!(cache.retired_images(true).len(), 1);
        assert!(cache.retired_images(true).is_empty());
    }

    #[test]
    fn text_meshes_reuse_unchanged_words_when_typing_moving_and_reflowing() {
        for tool in [Tool::Text, Tool::Rectangle, Tool::Arrow, Tool::Line] {
            let mut mark = Mark::new(tool, Pos::new(50., 50.), Style::default());
            mark.points[1] = Pos::new(350., 200.);
            mark.wrap = (tool == Tool::Text).then_some(180.);
            mark.set_text("Repeated words".into());
            let before = text_meshes(&mark);
            mark.set_text("Repeated words change and wrap".into());
            let after = text_meshes(&mark);
            assert!(Arc::ptr_eq(&before[0].mesh, &after[0].mesh), "{tool:?}");
            mark.translate(120., -80.);
            let moved = text_meshes(&mark);
            assert!(Arc::ptr_eq(&after[0].mesh, &moved[0].mesh));
            assert!(moved[0].origin.distance(after[0].origin) > 100.);
            mark.style.font_size *= 1.5;
            let resized = text_meshes(&mark);
            assert!(!Arc::ptr_eq(&moved[0].mesh, &resized[0].mesh));
        }
    }

    #[test]
    fn long_unbroken_text_reuses_bounded_geometry_and_skips_offscreen_letters() {
        let mut mark = Mark::new(Tool::Text, Pos::new(20., 20.), Style::default());
        mark.set_text("A".repeat(512));
        let parts = text_meshes(&mark);
        assert!(parts.len() > 1);
        assert!(Arc::ptr_eq(&parts[0].mesh, &parts[1].mesh));
        let at = |p: Pos| point(px(p.x), px(p.y));
        let all = text_layers(&parts, at, |_| true);
        let viewport = Bounds::new(point(px(0.), px(0.)), size(px(250.), px(100.)));
        let visible = text_layers(&parts, at, |b| b.intersects(&viewport));
        assert_eq!(all.len(), 2);
        assert_eq!(visible.len(), 2);
        let vertices = |paths: &NativePaths| {
            paths
                .iter()
                .map(|(p, _, _)| p.vertices.len())
                .sum::<usize>()
        };
        assert!(vertices(&visible) < vertices(&all) / 8);
        assert!(
            text_layers(&parts, at, |b| b.intersects(&Bounds::new(
                point(px(-100.), px(-100.)),
                size(px(10.), px(10.))
            )))
            .is_empty()
        );
    }

    #[test]
    fn reflected_rounded_geometry_refreshes_the_cache_and_matches_export() {
        let mut mark = Mark::new(
            Tool::ArrowRight,
            Pos::new(40., 40.),
            Style {
                fill: Fill::Solid,
                color: 7,
                ..Default::default()
            },
        );
        mark.points[1] = Pos::new(240., 160.);
        let mut doc = Document {
            annotation_scale: OnceLock::new(),
            next_marker: 1,
            comparison: None,
            base: Arc::new(image::RgbaImage::new(300, 200)),
            marks: vec![mark],
            undo: Vec::new(),
            redo: Vec::new(),
            mosaic: Arc::default(),
            source_png: Arc::default(),
        };
        let mut cache = PreviewCache::default();
        let left = Pos::new(100.5, 64.5);
        let right = Pos::new(180.5, 64.5);
        for (x, y) in [
            (false, false),
            (true, false),
            (false, true),
            (true, true),
            (false, false),
        ] {
            doc.marks[0].flip_x = x;
            doc.marks[0].flip_y = y;
            cache.update(&doc);
            assert_eq!(covers(&cache.marks[0].paths, left), x);
            assert_eq!(covers(&cache.marks[0].paths, right), !x);
            let (png, bounds) = doc.png(None).unwrap();
            let output = image::load_from_memory(&png).unwrap().to_rgba8();
            for (p, filled) in [(left, x), (right, !x)] {
                let pixel = output.get_pixel((p.x - bounds.x) as u32, (p.y - bounds.y) as u32);
                assert_eq!(pixel[3] == 255, filled);
                if filled {
                    assert_eq!(&pixel.0[..3], &doc.marks[0].style.fill_color());
                }
            }
        }
    }

    #[test]
    fn diagonal_multiline_arrow_labels_preserve_visible_shafts_at_both_ends() {
        let mut mark = Mark::new(
            Tool::Arrow,
            Pos::new(50., 350.),
            Style {
                width: 5.,
                start: Cap::Circle,
                end: Cap::Arrow,
                ..Default::default()
            },
        );
        mark.style.font_size = 44.;
        mark.points[1] = Pos::new(350., 150.);
        mark.set_text("Hello, this is\nthe end\nof glory\nand I'm holding\nof\na\nwith you".into());
        let mesh = paths(&mark);
        let doc = Document {
            annotation_scale: OnceLock::new(),
            next_marker: 1,
            comparison: None,
            base: Arc::new(image::RgbaImage::from_pixel(
                480,
                540,
                image::Rgba([40, 43, 49, 255]),
            )),
            marks: vec![mark.clone()],
            undo: Vec::new(),
            redo: Vec::new(),
            mosaic: Arc::default(),
            source_png: Arc::default(),
        };
        let (png, bounds) = doc.png(None).unwrap();
        let exported = image::load_from_memory(&png).unwrap().to_rgba8();
        let length = mark.points[0].distance(mark.points[1]);
        for fraction in [15. / length, 1. - 26. / length] {
            let p = mark.arrow_at(fraction);
            assert!(
                covers(&mesh[..1], p),
                "Label erased the shaft near {fraction}: {p:?}, label {:?}",
                mark.label_bounds()
            );
            let pixel = exported.get_pixel(
                (p.x - bounds.x).round() as u32,
                (p.y - bounds.y).round() as u32,
            );
            assert_eq!(
                &pixel.0[..3],
                &COLORS[mark.style.color],
                "Export also preserves the shaft"
            );
        }
    }

    #[test]
    fn arrow_labels_cut_the_shaft_without_painting_over_the_source_image() {
        for curved in [false, true] {
            let mut mark = Mark::new(
                Tool::Arrow,
                Pos::new(50., 150.),
                Style {
                    width: 8.,
                    ..Default::default()
                },
            );
            mark.points[1] = Pos::new(450., 150.);
            if curved {
                mark.points.push(Pos::new(200., 290.));
            }
            mark.text = "Label".into();
            let mesh = paths(&mark);
            let base = Arc::new(image::RgbaImage::from_fn(500, 400, |x, y| {
                image::Rgba([(x % 191) as u8, (y % 213) as u8, 70, 255])
            }));
            let doc = Document {
                annotation_scale: OnceLock::new(),
                next_marker: 1,
                comparison: None,
                base: base.clone(),
                marks: vec![mark.clone()],
                undo: Vec::new(),
                redo: Vec::new(),
                mosaic: Arc::default(),
                source_png: Arc::default(),
            };
            let (png, b) = doc.png(None).unwrap();
            let output = image::load_from_memory(&png).unwrap().to_rgba8();
            let cutout = mark.arrow_label_cutout().unwrap();
            let p = Pos::new((cutout.x + 1.).floor(), (cutout.y + cutout.h / 2.).floor());
            assert!(!covers(&mesh, p), "Label padding must stay transparent");
            assert_eq!(
                output.get_pixel((p.x - b.x) as u32, (p.y - b.y) as u32),
                base.get_pixel(p.x as u32, p.y as u32)
            );
            assert!(mark.label_path().is_some());
            assert!(mark.hit(mark.label_bounds().unwrap().center(), 0.));
        }
        // An initially tiny arrow grows to preserve both its label and shaft.
        let mut short = Mark::new(Tool::Arrow, Pos::new(50., 50.), Style::default());
        short.points[1] = Pos::new(55., 50.);
        short.set_text("Many words on a very short arrow".into());
        assert!(short.label_path().is_some());
        assert!(short.body_path().is_some());
        assert!(!paths(&short).is_empty());
    }

    fn covers(paths: &[(gpui_kit::Path<Pixels>, [u8; 3], f32)], p: Pos) -> bool {
        paths.iter().any(|(path, _, _)| {
            path.vertices.as_chunks::<3>().0.iter().any(|triangle| {
                let at = |i: usize| {
                    Pos::new(
                        triangle[i].xy_position.x.as_f32(),
                        triangle[i].xy_position.y.as_f32(),
                    )
                };
                let cross = |a: Pos, b: Pos| (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x);
                let signs = [
                    cross(at(0), at(1)),
                    cross(at(1), at(2)),
                    cross(at(2), at(0)),
                ];
                signs.iter().all(|value| *value >= -0.001)
                    || signs.iter().all(|value| *value <= 0.001)
            })
        })
    }
    #[test]
    fn coincident_line_endpoints_paint_a_filled_dot_in_preview_and_export() {
        for width in [8., 16., 32., 64.] {
            for dash in [Dash::Solid, Dash::Dashed, Dash::Dotted] {
                let at = Pos::new(100., 100.);
                let mark = Mark::new(
                    Tool::Line,
                    at,
                    Style {
                        width,
                        dash,
                        color: 5,
                        ..Default::default()
                    },
                );
                let mesh = paths(&mark);
                let doc = Document {
                    annotation_scale: OnceLock::new(),
                    next_marker: 1,
                    comparison: None,
                    base: Arc::new(image::RgbaImage::new(200, 200)),
                    marks: vec![mark.clone()],
                    undo: Vec::new(),
                    redo: Vec::new(),
                    mosaic: Arc::default(),
                    source_png: Arc::default(),
                };
                let (png, bounds) = doc.png(None).unwrap();
                let output = image::load_from_memory(&png).unwrap().to_rgba8();
                assert_eq!(bounds.x, 0.);
                assert_eq!(bounds.y, 0.);
                for p in [at, Pos::new(at.x + width * 0.25, at.y)] {
                    assert!(
                        covers(&mesh, p),
                        "Missing native dot at {p:?}, {width}, {dash:?}"
                    );
                    let pixel = output.get_pixel(p.x as u32, p.y as u32);
                    assert_eq!(&pixel.0[..3], &COLORS[5]);
                    assert_eq!(pixel[3], 255);
                    assert!(mark.hit(p, 0.));
                }
                let outside = Pos::new(at.x + width / 2. + 2., at.y);
                assert!(!covers(&mesh, outside));
                assert_eq!(output.get_pixel(outside.x as u32, outside.y as u32)[3], 0);
                assert!(!mark.hit(outside, 0.));
            }
        }
    }
    #[test]
    fn box_symbols_share_the_border_tone_and_dash_in_preview_and_export() {
        for tool in [Tool::XBox, Tool::CheckBox] {
            for fill in [Fill::None, Fill::Soft, Fill::Solid, Fill::Hatch] {
                for dash in [Dash::Solid, Dash::Dashed, Dash::Dotted] {
                    for color in [0, 7, 11] {
                        let mut mark = Mark::new(
                            tool,
                            Pos::new(40., 40.),
                            Style {
                                fill,
                                dash,
                                color,
                                width: 8.,
                                ..Default::default()
                            },
                        );
                        mark.points[1] = Pos::new(240., 240.);
                        let mesh = paths(&mark);
                        let stroke = COLORS[color];
                        let doc = Document {
                            annotation_scale: OnceLock::new(),
                            next_marker: 1,
                            comparison: None,
                            base: Arc::new(image::RgbaImage::new(280, 280)),
                            marks: vec![mark],
                            undo: Vec::new(),
                            redo: Vec::new(),
                            mosaic: Arc::default(),
                            source_png: Arc::default(),
                        };
                        let (png, _) = doc.png(None).unwrap();
                        let output = image::load_from_memory(&png).unwrap().to_rgba8();
                        let segments = if tool == Tool::XBox {
                            [((0., 0.), (1., 1.)), ((1., 0.), (0., 1.))]
                        } else {
                            [((0.3, 0.52), (0.47, 0.75)), ((0.47, 0.75), (0.76, 0.28))]
                        };
                        let mut painted = 0;
                        for (a, b) in segments {
                            let at = |(x, y): (f32, f32)| Pos::new(40. + 200. * x, 40. + 200. * y);
                            for step in 5..95 {
                                let p = at(a).lerp(at(b), step as f32 / 100.);
                                let p = Pos::new(p.x.floor() + 0.5, p.y.floor() + 0.5);
                                if output.get_pixel(p.x as u32, p.y as u32).0
                                    == [stroke[0], stroke[1], stroke[2], 255]
                                {
                                    painted += 1;
                                    assert!(
                                        mesh.iter().any(|path| path.1 == stroke
                                            && covers(std::slice::from_ref(path), p)),
                                        "{tool:?} {fill:?} {dash:?} at {p:?}: preview must match the symbol's exported stroke"
                                    );
                                }
                            }
                        }
                        if dash == Dash::Solid {
                            assert!(
                                painted >= 175,
                                "{tool:?} {fill:?}: incomplete solid symbol ({painted})"
                            );
                        } else {
                            assert!(
                                (20..165).contains(&painted),
                                "{tool:?} {fill:?} {dash:?}: missing dash gaps ({painted})"
                            );
                        }
                        if tool == Tool::XBox {
                            assert!(
                                doc.marks[0].hit(Pos::new(52., 52.), 1.),
                                "X reaches the corners"
                            );
                            if dash == Dash::Solid {
                                assert_eq!(
                                    output.get_pixel(52, 52).0,
                                    [stroke[0], stroke[1], stroke[2], 255]
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn every_shape_uses_matching_darker_fills_and_lighter_strokes_for_every_palette_color() {
        for tool in SHAPES.into_iter().filter(|tool| tool.is_shape()) {
            for (color, stroke) in COLORS.into_iter().enumerate() {
                let mut mark = Mark::new(
                    tool,
                    Pos::new(40., 40.),
                    Style {
                        fill: Fill::Solid,
                        color,
                        width: 8.,
                        ..Default::default()
                    },
                );
                mark.points[1] = Pos::new(240., 240.);
                let fill = mark.style.fill_color();
                assert!(
                    fill.iter()
                        .zip(stroke)
                        .all(|(fill, stroke)| *fill <= stroke)
                );
                assert_ne!(fill, stroke);
                let mesh = paths(&mark);
                let fill_paths: NativePaths =
                    mesh.iter().filter(|p| p.1 == fill).cloned().collect();
                let stroke_paths: NativePaths =
                    mesh.iter().filter(|p| p.1 == stroke).cloned().collect();
                assert!(!fill_paths.is_empty() && !stroke_paths.is_empty());
                for rotation in [0., 0.7] {
                    mark.rotation = rotation;
                    let doc = Document {
                        annotation_scale: OnceLock::new(),
                        next_marker: 1,
                        comparison: None,
                        base: Arc::new(image::RgbaImage::new(280, 280)),
                        marks: vec![mark.clone()],
                        undo: Vec::new(),
                        redo: Vec::new(),
                        mosaic: Arc::default(),
                        source_png: Arc::default(),
                    };
                    let (png, bounds) = doc.png(None).unwrap();
                    let output = image::load_from_memory(&png).unwrap().to_rgba8();
                    let mut painted_fill = false;
                    let mut painted_stroke = false;
                    for y in (42..240).step_by(5) {
                        for x in (42..240).step_by(5) {
                            let p = Pos::new(x as f32, y as f32);
                            let world = mark.frame().to_world(p);
                            let pixel = output
                                .get_pixel((world.x - bounds.x) as u32, (world.y - bounds.y) as u32)
                                .0;
                            if pixel == [fill[0], fill[1], fill[2], 255] && covers(&fill_paths, p) {
                                painted_fill = true;
                            }
                            if pixel == [stroke[0], stroke[1], stroke[2], 255]
                                && covers(&stroke_paths, p)
                            {
                                painted_stroke = true;
                            }
                        }
                    }
                    assert!(
                        painted_fill && painted_stroke,
                        "{tool:?}, color {color}, rotation {rotation}: preview and export need both tones"
                    );
                }
            }
        }
    }
    #[test]
    fn filled_directional_shapes_have_matching_native_borders() {
        let make = |tool| {
            let mut mark = Mark::new(
                tool,
                Pos::new(40., 40.),
                Style {
                    fill: Fill::Solid,
                    width: 16.,
                    ..Default::default()
                },
            );
            mark.points[1] = Pos::new(280., 200.);
            mark
        };
        let right = make(Tool::ArrowRight);
        let right_mesh = paths(&right);
        for tool in [Tool::ArrowLeft, Tool::ArrowUp, Tool::ArrowDown] {
            let mark = make(tool);
            let mesh = paths(&mark);
            let doc = Document {
                annotation_scale: OnceLock::new(),
                next_marker: 1,
                comparison: None,
                base: Arc::new(image::RgbaImage::new(320, 240)),
                marks: vec![mark],
                undo: Vec::new(),
                redo: Vec::new(),
                mosaic: Arc::default(),
                source_png: Arc::default(),
            };
            let (png, bounds) = doc.png(None).unwrap();
            let output = image::load_from_memory(&png).unwrap().to_rgba8();
            // The shaft's inner border must be as solid as the center, in every direction.
            for t in [0.1, 0.2, 0.35, 0.5] {
                let p = Pos::new(40. + 240. * t, 40. + 160. * 0.3 + 3.);
                let x = (p.x - 40.) / 240.;
                let y = (p.y - 40.) / 160.;
                let (x, y) = match tool {
                    Tool::ArrowLeft => (1. - x, y),
                    Tool::ArrowUp => (y, 1. - x),
                    Tool::ArrowDown => (y, x),
                    _ => unreachable!(),
                };
                let q = Pos::new(40. + 240. * x, 40. + 160. * y);
                assert!(covers(&right_mesh, p));
                assert_eq!(
                    output.get_pixel((q.x - bounds.x) as u32, (q.y - bounds.y) as u32)[3],
                    255
                );
                assert!(
                    covers(&mesh, q),
                    "{tool:?}: native border has a cutout at {q:?}"
                );
            }
        }
    }
    #[test]
    fn rotating_cached_objects_and_labels_matches_rotating_the_exported_pixels() {
        for tool in [Tool::Rectangle, Tool::Text, Tool::Pen, Tool::Heart] {
            let mut mark = Mark::new(
                tool,
                Pos::new(40., 65.),
                Style {
                    width: 8.,
                    font_size: 20.,
                    fill: Fill::Hatch,
                    ..Default::default()
                },
            );
            mark.points[1] = Pos::new(200., 175.);
            if tool == Tool::Pen {
                mark.points.push(Pos::new(160., 70.));
            }
            mark.text = "Rotate this label".into();
            let center = mark.bounds().center();
            mark.translate(120. - center.x, 120. - center.y);
            let mut doc = Document {
                annotation_scale: OnceLock::new(),
                next_marker: 1,
                comparison: None,
                base: Arc::new(image::RgbaImage::new(240, 240)),
                marks: vec![mark],
                undo: Vec::new(),
                redo: Vec::new(),
                mosaic: Arc::default(),
                source_png: Arc::default(),
            };
            let mut cache = PreviewCache::default();
            cache.update(&doc);
            let mesh = cache.marks[0].paths.as_ptr();
            let (png, bounds) = doc.png(None).unwrap();
            assert_eq!(
                bounds,
                Rect {
                    x: 0.,
                    y: 0.,
                    w: 240.,
                    h: 240.
                }
            );
            let original = image::load_from_memory(&png).unwrap().to_rgba8();
            doc.marks[0].rotation = std::f32::consts::FRAC_PI_2;
            cache.update(&doc);
            assert_eq!(
                cache.marks[0].paths.as_ptr(),
                mesh,
                "Rotation reuses tessellated geometry"
            );
            let (png, bounds) = doc.png(None).unwrap();
            assert_eq!(
                bounds,
                Rect {
                    x: 0.,
                    y: 0.,
                    w: 240.,
                    h: 240.
                }
            );
            let rotated = image::load_from_memory(&png).unwrap().to_rgba8();
            let expected = image::imageops::rotate90(&original);
            let difference: u64 = expected
                .pixels()
                .zip(rotated.pixels())
                .map(|(a, b)| a[3].abs_diff(b[3]) as u64)
                .sum();
            // Curve flattening can change edge antialiasing after a transform.
            let ink: u64 = expected.pixels().map(|p| u64::from(p[3])).sum();
            assert!(
                difference < ink / 100 + 500,
                "{tool:?}: rotation diverges from export by {difference}"
            );
            // Preview geometry uses the same frame, including both outline and text paths.
            let frame = cache.marks[0].frame;
            let mut geometry = cache.marks[0].paths.clone();
            geometry.extend(text_layers(
                &cache.marks[0].text,
                |p| point(px(p.x), px(p.y)),
                |_| true,
            ));
            for y in (20..220).step_by(5) {
                for x in (20..220).step_by(5) {
                    let p = Pos::new(x as f32 + 0.5, y as f32 + 0.5);
                    let covered = covers(&geometry, frame.to_local(p));
                    if rotated.get_pixel(x, y)[3] == 255 {
                        assert!(
                            covered,
                            "{tool:?}: native geometry misses rotated export at {p:?}"
                        );
                    }
                }
            }
        }
    }
    #[test]
    fn arrow_shafts_do_not_protrude_beyond_triangle_tips() {
        for width in [8., 32., 64.] {
            for curved in [false, true] {
                let a = Pos::new(100., 150.);
                let b = Pos::new(500., 150.);
                let control = Pos::new(300., 300.);
                let mut mark = Mark::new(
                    Tool::Arrow,
                    a,
                    Style {
                        width,
                        start: Cap::Triangle,
                        end: Cap::Triangle,
                        ..Default::default()
                    },
                );
                mark.points[1] = b;
                if curved {
                    mark.points.push(control);
                }
                let mesh = paths(&mark);
                let doc = Document {
                    annotation_scale: OnceLock::new(),
                    next_marker: 1,
                    comparison: None,
                    base: Arc::new(image::RgbaImage::new(600, 400)),
                    marks: vec![mark],
                    undo: Vec::new(),
                    redo: Vec::new(),
                    mosaic: Arc::default(),
                    source_png: Arc::default(),
                };
                let (png, bounds) = doc.png(None).unwrap();
                let output = image::load_from_memory(&png).unwrap().to_rgba8();
                for (tip, toward) in [
                    (a, if curved { control } else { b }),
                    (b, if curved { control } else { a }),
                ] {
                    let length = tip.distance(toward);
                    let p = Pos::new(
                        tip.x + (tip.x - toward.x) / length * width * 0.25,
                        tip.y + (tip.y - toward.y) / length * width * 0.25,
                    );
                    assert!(
                        !covers(&mesh, p),
                        "Shaft protrudes beyond {tip:?}, width {width}, curved {curved}"
                    );
                    assert_eq!(
                        output.get_pixel((p.x - bounds.x) as u32, (p.y - bounds.y) as u32)[3],
                        0,
                        "Exported shaft protrudes beyond the cap"
                    );
                }
            }
        }
    }
    #[test]
    fn filled_arrow_caps_have_no_cutout_where_the_shaft_overlaps() {
        for (a, b) in [
            (Pos::new(20., 120.), Pos::new(220., 20.)),
            (Pos::new(220., 20.), Pos::new(20., 120.)),
        ] {
            for cap in [Cap::Triangle, Cap::Circle] {
                let mut mark = Mark::new(
                    Tool::Arrow,
                    a,
                    Style {
                        start: cap,
                        end: cap,
                        width: 8.,
                        ..Default::default()
                    },
                );
                mark.points[1] = b;
                let preview = paths(&mark);
                for t in [0.01, 0.03, 0.06, 0.94, 0.97, 0.99] {
                    assert!(
                        covers(&preview, a.lerp(b, t)),
                        "{cap:?} cap has a cutout at {t}, {a:?} → {b:?}"
                    );
                }
            }
        }
    }
    struct MosaicScene {
        cache: std::rc::Rc<std::cell::RefCell<PreviewCache>>,
        frames: std::rc::Rc<std::cell::Cell<usize>>,
        hover: bool,
    }
    impl gpui_kit::Render for MosaicScene {
        fn render(
            &mut self,
            _: &mut Window,
            _: &mut gpui_kit::Context<Self>,
        ) -> impl gpui_kit::IntoElement {
            use gpui_kit::Styled as _;
            let cache = self.cache.clone();
            let frames = self.frames.clone();
            let hover = self.hover;
            gpui_kit::canvas(
                |_, _, _| (),
                move |_, _, window, _| {
                    frames.set(frames.get() + 1);
                    let brush =
                        hover.then(|| (Pos::new(400. + (frames.get() % 24) as f32, 300.), 1280.));
                    cache.borrow().paint(Pos::new(0., 0.), 0.4, brush, window);
                },
            )
            .size_full()
        }
    }
    #[gpui_kit::test]
    fn pixelation_and_hover_keep_native_frame_building_interactive(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        use gpui_kit::test::TestWindowExt as _;
        let mut doc = Document {
            annotation_scale: OnceLock::new(),
            next_marker: 1,
            comparison: None,
            base: Arc::new(image::RgbaImage::from_fn(2400, 1600, |x, y| {
                image::Rgba([(x % 255) as u8, (y % 255) as u8, 100, 255])
            })),
            marks: Vec::new(),
            undo: Vec::new(),
            redo: Vec::new(),
            mosaic: Arc::default(),
            source_png: Arc::default(),
        };
        for origin in [Pos::new(0., 0.), Pos::new(800., 400.)] {
            let mut mark = Mark::new(Tool::Redact, origin, Style::default());
            mark.points[1] = Pos::new(origin.x + 1600., origin.y + 1200.);
            doc.marks.push(mark);
        }
        let mut cache = PreviewCache::default();
        cache.update(&doc);
        let frames = std::rc::Rc::new(std::cell::Cell::new(0));
        let capture = frames.clone();
        let (view, cx) = cx.add_window_view(|_, _| MosaicScene {
            cache: std::rc::Rc::new(std::cell::RefCell::new(cache)),
            frames: capture,
            hover: false,
        });
        cx.update(|window, cx| window.render_frame(cx));
        let before = frames.get();
        let started = std::time::Instant::now();
        for _ in 0..24 {
            cx.update(|window, cx| window.render_frame(cx));
        }
        let elapsed = started.elapsed();
        eprintln!("24 native frames with two pixelation regions: {elapsed:?}");
        assert!(
            frames.get() >= before + 24,
            "Every frame must paint the actual cached pixelation path"
        );
        assert!(
            elapsed < std::time::Duration::from_millis(500),
            "Pixelation blocks interactive rendering: {elapsed:?}"
        );
        cx.update(|_, cx| {
            view.update(cx, |scene, cx| {
                scene.hover = true;
                cx.notify();
            })
        });
        // Warm the shared color cache, then move a maximum-size hover preview every frame.
        cx.update(|window, cx| window.render_frame(cx));
        let started = std::time::Instant::now();
        for _ in 0..24 {
            cx.update(|window, cx| window.render_frame(cx));
        }
        let elapsed = started.elapsed();
        eprintln!("24 native frames with a moving pixelation hover: {elapsed:?}");
        assert!(
            elapsed < std::time::Duration::from_millis(500),
            "Hover preview blocks rendering: {elapsed:?}"
        );
    }
}

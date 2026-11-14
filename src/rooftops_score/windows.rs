use super::*;
use anyhow::Context as _;
use tiny_skia::{FillRule, Paint, PathBuilder, Pixmap, Rect, Transform};

const SCALE: Transform = Transform {
    sx: 4.,
    ky: 0.,
    kx: 0.,
    sy: 4.,
    tx: 0.,
    ty: 0.,
};

pub(crate) fn png_bytes(score: Score) -> anyhow::Result<Vec<u8>> {
    let mut pixmap = Pixmap::new(3600, 2160).context("Could not allocate the score image")?;
    let mut shapes = poster_shapes().into_iter();
    draw_shape(
        &mut pixmap,
        shapes.next().context("Missing poster background")?,
    );
    draw_artwork(&mut pixmap, art::full_moon_paths(654., 130., 122.));
    for shape in shapes {
        draw_shape(&mut pixmap, shape);
    }
    draw_artwork(&mut pixmap, poster_paths(1.2));
    for label in poster_text(score) {
        let (width, path) =
            crate::annotation::score_label(&label.label, label.size * 4., label.bold);
        if let Some(path) = path {
            let x = label.x * 4. + label.centered_width.map_or(0., |w| (w * 4. - width) * 0.5);
            pixmap.fill_path(
                &path,
                &paint(label.ink, 1.),
                FillRule::Winding,
                Transform::from_translate(x, label.y * 4.),
                None,
            );
        }
    }
    pixmap.encode_png().context("Could not encode the PNG")
}

fn paint(ink: u32, opacity: f32) -> Paint<'static> {
    let mut paint = Paint::default();
    paint.set_color_rgba8(
        (ink >> 16) as u8,
        (ink >> 8) as u8,
        ink as u8,
        (opacity * 255.).round() as u8,
    );
    paint
}

fn draw_shape(pixmap: &mut Pixmap, shape: PosterShape) {
    let [x, y, w, h] = shape.rect;
    let paint = paint(shape.ink, shape.opacity);
    let radius = shape.radius.min(w / 2.).min(h / 2.);
    if radius <= 0. {
        if let Some(rect) = Rect::from_xywh(x, y, w, h) {
            pixmap.fill_rect(rect, &paint, SCALE, None);
        }
        return;
    }
    let mut path = PathBuilder::new();
    path.move_to(x + radius, y);
    path.line_to(x + w - radius, y);
    path.quad_to(x + w, y, x + w, y + radius);
    path.line_to(x + w, y + h - radius);
    path.quad_to(x + w, y + h, x + w - radius, y + h);
    path.line_to(x + radius, y + h);
    path.quad_to(x, y + h, x, y + h - radius);
    path.line_to(x, y + radius);
    path.quad_to(x, y, x + radius, y);
    path.close();
    if let Some(path) = path.finish() {
        pixmap.fill_path(&path, &paint, FillRule::Winding, SCALE, None);
    }
}

fn draw_artwork(pixmap: &mut Pixmap, paths: Vec<art::ArtPath>) {
    for shape in paths {
        let mut points = shape.points.into_iter();
        let Some([x, y]) = points.next() else {
            continue;
        };
        let mut path = PathBuilder::new();
        path.move_to(x, y);
        for [x, y] in points {
            path.line_to(x, y);
        }
        path.close();
        if let Some(path) = path.finish() {
            pixmap.fill_path(
                &path,
                &paint(shape.ink, shape.opacity),
                FillRule::Winding,
                SCALE,
                None,
            );
        }
    }
}

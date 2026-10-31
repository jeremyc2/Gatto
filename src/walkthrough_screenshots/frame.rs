//! Documentation presentation is separate from the unframed native captures.
use std::{fs, path::Path};

use anyhow::{Context as _, Result};
use image::{Rgba, RgbaImage};

pub(super) fn write_documentation_images(directory: &Path) -> Result<()> {
    let destination = directory.join("framed");
    fs::create_dir_all(&destination)?;
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let screenshot = name.starts_with(|c: char| c.is_ascii_digit())
            || name.starts_with("image-editor")
            || name == "comparison-editor.png"
            || name.starts_with("rooftops-");
        if entry.file_type()?.is_file() && name.ends_with(".png") && screenshot {
            write_documentation_image(&entry.path(), &destination.join(name.as_ref()))?;
        }
    }
    Ok(())
}

pub(super) fn write_documentation_image(source: &Path, destination: &Path) -> Result<()> {
    rounded_frame(&image::open(source)?.to_rgba8())
        .save(destination)
        .with_context(|| format!("Could not frame {}", source.display()))
}

fn rounded_frame(source: &RgbaImage) -> RgbaImage {
    // Native captures are 2×: this renders a one-point white edge and 16-point corners.
    const BORDER: u32 = 2;
    const RADIUS: f32 = 32.;
    let width = source.width() + BORDER * 2;
    let height = source.height() + BORDER * 2;
    RgbaImage::from_fn(width, height, |x, y| {
        let outer = coverage(x as f32, y as f32, width, height, RADIUS);
        let inner = coverage(
            x as f32 - BORDER as f32,
            y as f32 - BORDER as f32,
            source.width(),
            source.height(),
            RADIUS - BORDER as f32,
        );
        let pixel = if inner > 0. {
            *source.get_pixel(x - BORDER, y - BORDER)
        } else {
            Rgba([0; 4])
        };
        let edge = (outer - inner).max(0.);
        let content = inner * f32::from(pixel[3]) / 255.;
        let alpha = edge + content;
        if alpha == 0. {
            return Rgba([0; 4]);
        }
        Rgba([
            ((255. * edge + f32::from(pixel[0]) * content) / alpha).round() as u8,
            ((255. * edge + f32::from(pixel[1]) * content) / alpha).round() as u8,
            ((255. * edge + f32::from(pixel[2]) * content) / alpha).round() as u8,
            (alpha * 255.).round() as u8,
        ])
    })
}

fn coverage(x: f32, y: f32, width: u32, height: u32, radius: f32) -> f32 {
    if x < 0. || y < 0. || x >= width as f32 || y >= height as f32 {
        return 0.;
    }
    let radius = radius.min(width as f32 / 2.).min(height as f32 / 2.);
    let x = x + 0.5;
    let y = y + 0.5;
    let dx = x - x.clamp(radius, width as f32 - radius);
    let dy = y - y.clamp(radius, height as f32 - radius);
    (radius + 0.5 - dx.hypot(dy)).clamp(0., 1.)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_has_transparent_corners_white_edges_and_unchanged_content() {
        let color = Rgba([14, 16, 23, 255]);
        let source = RgbaImage::from_pixel(100, 80, color);
        let framed = rounded_frame(&source);
        assert_eq!(framed.dimensions(), (104, 84));
        assert_eq!(framed.get_pixel(0, 0)[3], 0);
        assert_eq!(*framed.get_pixel(50, 0), Rgba([255; 4]));
        assert_eq!(*framed.get_pixel(50, 83), Rgba([255; 4]));
        assert_eq!(*framed.get_pixel(50, 2), color);
        assert_eq!(*framed.get_pixel(50, 40), color);
        assert!((0..32).any(|x| (1..255).contains(&framed.get_pixel(x, 0)[3])));
    }
}

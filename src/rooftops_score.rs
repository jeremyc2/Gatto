use std::path::{Path, PathBuf};

pub(crate) const POSTER_FONT: &str = "Helvetica Neue";

#[derive(Clone, Copy)]
pub(crate) struct Score {
    pub coins: usize,
    pub deaths: usize,
    pub kills: usize,
    pub seconds: f64,
}

pub(crate) struct PosterShape {
    pub rect: [f32; 4],
    pub ink: u32,
    pub opacity: f32,
    pub radius: f32,
}

pub(crate) struct PosterText {
    pub label: String,
    pub x: f32,
    pub y: f32,
    pub size: f32,
    pub ink: u32,
    pub bold: bool,
}

// One authored poster is shared by the GPUI results and the downloadable PNG.
// Coordinates are in a 900 × 540 design space.
pub(crate) fn poster_shapes() -> Vec<PosterShape> {
    let mut shapes = Vec::with_capacity(240);
    let mut add = |rect, ink, opacity, radius| {
        shapes.push(PosterShape {
            rect,
            ink,
            opacity,
            radius,
        })
    };
    add([0., 0., 900., 540.], 0x171a2d, 1., 0.);
    for (rect, ink, opacity, radius) in [
        ([613., 88., 204., 204.], 0xe9bc83, 0.025, 102.),
        ([635., 110., 160., 160.], 0xe9bc83, 0.05, 80.),
        ([654., 130., 122., 122.], 0xf3ddba, 1., 61.),
        ([672., 148., 23., 23.], 0xd8b98e, 0.3, 11.5),
        ([728., 185., 32., 32.], 0xd8b98e, 0.25, 16.),
    ] {
        add(rect, ink, opacity, radius);
    }
    for i in 0..40 {
        let x = ((i * 137 + 33) % 820) as f32 + 40.;
        let y = ((i * 47 + 13) % 240) as f32 + 38.;
        // Give the large headline a quiet, uncluttered area.
        if x < 550. && y < 175. {
            continue;
        }
        let d = if i % 7 == 0 { 2.5 } else { 1.3 };
        add([x, y, d, d], 0xf3ddba, 0.25, d * 0.5);
    }
    for layer in 0..2 {
        for i in 0..12 {
            let x = i as f32 * 82. - 18. + layer as f32 * 29.;
            let roof = 245. + ((i * 31 + layer * 19) % 65) as f32;
            let ink = if layer == 0 { 0x24273c } else { 0x303044 };
            add([x, roof, 73., 365. - roof], ink, 1., 2.);
            add([x + 10., roof - 7., 50., 8.], ink, 1., 0.);
            if i % 3 == 0 {
                add([x + 24., roof - 25., 15., 20.], ink, 1., 2.);
            }
            for row in 0..3 {
                for col in 0..3 {
                    let y = roof + 13. + row as f32 * 17.;
                    if y > 352. {
                        continue;
                    }
                    add(
                        [x + 12. + col as f32 * 18., y, 5., 8.],
                        0xe2be91,
                        if (i + row + col) % 3 == 0 { 0.7 } else { 0.12 },
                        0.5,
                    );
                }
            }
        }
    }
    // A black cat surveying the illuminated city from its own roof.
    add([636., 277., 151., 78.], 0x393447, 1., 0.);
    add([630., 270., 163., 7.], 0x967784, 1., 2.);
    add([648., 292., 23., 42.], 0xedd0a3, 1., 2.);
    add([683., 292., 23., 42.], 0xd9b48c, 1., 2.);
    add([718., 292., 23., 42.], 0xedd0a3, 1., 2.);
    add([753., 292., 23., 42.], 0xd9b48c, 1., 2.);
    add([686., 216., 51., 54.], 0x05070d, 1., 22.);
    add([710., 190., 35., 37.], 0x05070d, 1., 12.);
    // Overlapping ears at the back of the head keep the seated cat in profile.
    add([720., 183., 6., 16.], 0x05070d, 1., 2.5);
    add([713., 177., 8., 22.], 0x05070d, 1., 3.);
    add([730., 205., 7., 3.], 0xa8f0ce, 1., 1.5);
    add([720., 248., 9., 22.], 0x05070d, 1., 4.);
    for (x, y) in [
        (691., 252.),
        (681., 250.),
        (672., 245.),
        (665., 238.),
        (663., 230.),
        (665., 222.),
    ] {
        add([x, y, 13., 13.], 0x05070d, 1., 6.5);
    }
    // Warm hairline border and a quiet statistics panel.
    for rect in [
        [20., 20., 860., 0.8],
        [20., 519., 860., 0.8],
        [20., 20., 0.8, 500.],
        [879., 20., 0.8, 500.],
    ] {
        add(rect, 0xc5a07b, 0.35, 0.);
    }
    add([36., 354., 828., 142.], 0x222338, 1., 8.);
    add([52., 354., 796., 1.], 0xd8b48c, 0.55, 0.);
    for x in [258., 458., 658.] {
        add([x, 378., 1., 94.], 0xb4938a, 0.18, 0.);
    }
    for x in [810., 826., 842.] {
        add([x, 48., 6., 6.], 0xf3ddba, 1., 3.);
    }
    shapes
}

pub(crate) fn poster_text(score: Score) -> Vec<PosterText> {
    let mut text = vec![
        PosterText {
            label: "ROOFTOPS".into(),
            x: 44.,
            y: 42.,
            size: 14.,
            ink: 0xd4b89c,
            bold: true,
        },
        PosterText {
            label: "Moonlit mischief".into(),
            x: 41.,
            y: 76.,
            size: 52.,
            ink: 0xf3ddba,
            bold: true,
        },
        PosterText {
            label: "GATTO".into(),
            x: 44.,
            y: 502.,
            size: 10.,
            ink: 0x96899f,
            bold: false,
        },
    ];
    for (x, label, value) in [
        (58., "COINS", score.coins.to_string()),
        (280., "KILLS", score.kills.to_string()),
        (480., "DEATHS", score.deaths.to_string()),
        (680., "TIME", score.time_label()),
    ] {
        text.push(PosterText {
            label: label.into(),
            x,
            y: 376.,
            size: 12.,
            ink: 0xb9a5b4,
            bold: true,
        });
        text.push(PosterText {
            // Keep long runs and large totals inside their statistics column.
            size: 52_f32.min(170. / (value.chars().count() as f32 * 0.7)),
            label: value,
            x: x - 2.,
            y: 399.,
            ink: 0xf3ddba,
            bold: true,
        });
    }
    text
}

impl Score {
    pub fn time_label(self) -> String {
        let seconds = self.seconds.max(0.).floor() as u64;
        if seconds >= 3600 {
            format!(
                "{}:{:02}:{:02}",
                seconds / 3600,
                seconds / 60 % 60,
                seconds % 60
            )
        } else {
            format!("{}:{:02}", seconds / 60, seconds % 60)
        }
    }

    pub fn save(self) -> anyhow::Result<PathBuf> {
        save_in(self, &downloads_directory()?)
    }
}

fn save_in(score: Score, directory: &Path) -> anyhow::Result<PathBuf> {
    use std::io::Write as _;
    #[cfg(target_os = "macos")]
    let bytes = objc2::rc::autoreleasepool(|_| png_bytes(score))?;
    #[cfg(not(target_os = "macos"))]
    let bytes = png_bytes(score)?;
    // Create atomically so repeated saves never overwrite an earlier run.
    for suffix in 0..10000 {
        let name = if suffix == 0 {
            "Rooftops-score.png".to_owned()
        } else {
            format!("Rooftops-score-{suffix}.png")
        };
        let path = directory.join(name);
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut file) => {
                if let Err(error) = file.write_all(&bytes) {
                    let _ = std::fs::remove_file(&path);
                    return Err(error.into());
                }
                return Ok(path);
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.into()),
        }
    }
    anyhow::bail!("No available score filename")
}

#[cfg(target_os = "macos")]
fn downloads_directory() -> anyhow::Result<PathBuf> {
    use anyhow::Context as _;
    use objc2_foundation::{
        NSSearchPathDirectory, NSSearchPathDomainMask, NSSearchPathForDirectoriesInDomains,
    };
    let paths = NSSearchPathForDirectoriesInDomains(
        NSSearchPathDirectory::DownloadsDirectory,
        NSSearchPathDomainMask::UserDomainMask,
        true,
    );
    Ok(PathBuf::from(
        paths
            .firstObject()
            .context("Downloads folder is unavailable")?
            .to_string(),
    ))
}

#[cfg(target_os = "macos")]
pub(crate) fn png_bytes(score: Score) -> anyhow::Result<Vec<u8>> {
    use anyhow::Context as _;
    use objc2::AnyThread as _;
    use objc2_app_kit::{
        NSBezierPath, NSBitmapImageFileType, NSBitmapImageRep, NSColor, NSDeviceRGBColorSpace,
        NSFont, NSFontAttributeName, NSForegroundColorAttributeName, NSGraphicsContext,
        NSStringDrawing,
    };
    use objc2_foundation::{NSDictionary, NSPoint, NSRect, NSSize, NSString};

    // Render directly at four times the authored size; never upscale a screenshot.
    const EXPORT_SCALE: f64 = 4.;
    const EXPORT_HEIGHT: f64 = 2160.;

    fn color(rgb: u32, opacity: f64) -> objc2::rc::Retained<NSColor> {
        NSColor::colorWithSRGBRed_green_blue_alpha(
            ((rgb >> 16) & 255) as f64 / 255.,
            ((rgb >> 8) & 255) as f64 / 255.,
            (rgb & 255) as f64 / 255.,
            opacity,
        )
    }
    fn rect(shape: PosterShape) {
        let [x, y, w, h] = shape.rect.map(|v| v as f64 * EXPORT_SCALE);
        let radius = shape.radius as f64 * EXPORT_SCALE;
        color(shape.ink, shape.opacity as f64).set();
        NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(
            NSRect::new(NSPoint::new(x, EXPORT_HEIGHT - y - h), NSSize::new(w, h)),
            radius,
            radius,
        )
        .fill();
    }
    fn text(label: &str, x: f64, y: f64, size: f64, ink: u32, bold: bool) {
        let name = if bold {
            "HelveticaNeue-Bold"
        } else {
            "HelveticaNeue"
        };
        let font =
            NSFont::fontWithName_size(&NSString::from_str(name), size).unwrap_or_else(|| {
                if bold {
                    NSFont::boldSystemFontOfSize(size)
                } else {
                    NSFont::systemFontOfSize(size)
                }
            });
        let color = color(ink, 1.);
        // The documented AppKit attribute keys take NSFont and NSColor objects.
        unsafe {
            let attributes = NSDictionary::from_slices(
                &[NSFontAttributeName, NSForegroundColorAttributeName],
                &[
                    &*font as &objc2::runtime::AnyObject,
                    &*color as &objc2::runtime::AnyObject,
                ],
            );
            NSString::from_str(label).drawAtPoint_withAttributes(
                NSPoint::new(x, EXPORT_HEIGHT - y - size),
                Some(&attributes),
            );
        }
    }
    // Null planes let AppKit own the interleaved RGBA bitmap storage.
    let bitmap = unsafe {
        NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bytesPerRow_bitsPerPixel(
            NSBitmapImageRep::alloc(), std::ptr::null_mut(), 3600, 2160, 8, 4, true, false, NSDeviceRGBColorSpace, 0, 0,
        )
    }.context("Could not allocate the score image")?;
    let context = NSGraphicsContext::graphicsContextWithBitmapImageRep(&bitmap)
        .context("Could not draw the score image")?;
    struct SavedGraphicsState;
    impl Drop for SavedGraphicsState {
        fn drop(&mut self) {
            NSGraphicsContext::restoreGraphicsState_class();
        }
    }
    NSGraphicsContext::saveGraphicsState_class();
    let saved_state = SavedGraphicsState;
    context.setShouldAntialias(true);
    NSGraphicsContext::setCurrentContext(Some(&context));
    for shape in poster_shapes() {
        rect(shape);
    }
    for item in poster_text(score) {
        text(
            &item.label,
            item.x as f64 * EXPORT_SCALE,
            item.y as f64 * EXPORT_SCALE,
            item.size as f64 * EXPORT_SCALE,
            item.ink,
            item.bold,
        );
    }
    drop(saved_state);
    // PNG accepts an empty properties dictionary for lossless default encoding.
    let png = unsafe {
        bitmap.representationUsingType_properties(NSBitmapImageFileType::PNG, &NSDictionary::new())
    }
    .context("Could not encode the PNG")?;
    Ok(png.to_vec())
}

#[cfg(not(target_os = "macos"))]
fn downloads_directory() -> anyhow::Result<PathBuf> {
    anyhow::bail!("Score export requires macOS")
}
#[cfg(not(target_os = "macos"))]
pub(crate) fn png_bytes(_: Score) -> anyhow::Result<Vec<u8>> {
    anyhow::bail!("Score export requires macOS")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(target_os = "macos")]
    #[gpui_kit::test]
    fn export_writes_a_readable_png_without_overwriting_previous_scores(
        _: &mut gpui_kit::TestAppContext,
    ) {
        let directory =
            std::env::temp_dir().join(format!("gatto-score-test-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let score = Score {
            coins: 41,
            deaths: 6,
            kills: 13,
            seconds: 143.6,
        };
        let export_directory = directory.clone();
        let first = std::thread::spawn(move || {
            objc2::rc::autoreleasepool(|_| save_in(score, &export_directory))
        })
        .join()
        .unwrap()
        .unwrap();
        let second = save_in(score, &directory).unwrap();
        assert_ne!(first, second);
        let bytes = std::fs::read(&first).unwrap();
        let decoded = image::load_from_memory(&bytes).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (3600, 2160));
        assert_eq!(bytes, std::fs::read(&second).unwrap());
        for path in [first, second] {
            std::fs::remove_file(path).unwrap();
        }
        std::fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn time_formats_minutes_and_hours() {
        let score = Score {
            coins: 0,
            deaths: 0,
            kills: 0,
            seconds: 125.9,
        };
        assert_eq!(score.time_label(), "2:05");
        assert_eq!(
            Score {
                seconds: 3661.,
                ..score
            }
            .time_label(),
            "1:01:01"
        );
    }
}

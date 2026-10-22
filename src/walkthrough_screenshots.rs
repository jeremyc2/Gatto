#![allow(dead_code)]

mod annotation;
mod app;
mod application_menu;
mod compression;
mod diagnostics;
mod github;
mod global_shortcut;
mod image_preview;
mod log_viewer;
mod menu_bar;
mod model;
mod rooftops;
mod rooftops_score;
mod settings;
mod theme;
mod updater;
mod video;
mod window_limits;

#[cfg(target_os = "macos")]
use std::{path::PathBuf, sync::Arc};

#[cfg(target_os = "macos")]
use anyhow::{Context as _, Result};
#[cfg(target_os = "macos")]
use gpui_kit::test::TestWindowExt as _;
#[cfg(target_os = "macos")]
use gpui_kit::{
    AppContext as _, Bounds, HeadlessAppContext, WindowBounds, WindowOptions, px, size,
};

#[cfg(target_os = "macos")]
use crate::app::{UploaderApp, WalkthroughScenario};

#[cfg(target_os = "macos")]
fn main() {
    if let Err(error) = generate() {
        eprintln!("Could not generate walkthrough screenshots: {error:#}");
        std::process::exit(1);
    }
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("Walkthrough screenshots can only be generated on macOS.");
    std::process::exit(1);
}

#[cfg(target_os = "macos")]
fn generate() -> Result<()> {
    let output_directory = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .context("Pass the output directory as the first argument")?;
    std::fs::create_dir_all(&output_directory).with_context(|| {
        format!(
            "Could not create screenshot directory {}",
            output_directory.display()
        )
    })?;

    if std::env::args().any(|arg| arg == "--check-pixelation-edges") {
        return check_pixelation_edges(&output_directory);
    }
    if std::env::args().any(|arg| arg == "--check-annotation-rendering") {
        return check_annotation_rendering(&output_directory);
    }

    for scenario in WalkthroughScenario::ALL {
        let output = output_directory.join(scenario.file_name());
        render_scenario(scenario, &output)?;
        println!("Wrote {}", output.display());
    }

    for level in 0..rooftops::LEVELS {
        let output = output_directory.join(format!("rooftops-{}.png", level + 1));
        render_rooftops(&output, level)?;
        println!("Wrote {}", output.display());
    }

    let editor_output = output_directory.join("image-editor.png");
    render_image_editor(&editor_output)?;
    println!("Wrote {}", editor_output.display());
    image_preview::ImagePreview::write_walkthrough_cursors(
        &output_directory.join("annotation-cursors.png"),
    )?;

    let score = rooftops_score::Score {
        coins: 41,
        deaths: 6,
        kills: 13,
        seconds: 143.6,
    };
    let output = output_directory.join("rooftops-score.png");
    std::fs::write(&output, rooftops_score::png_bytes(score)?)?;
    println!("Wrote {}", output.display());
    Ok(())
}

/// Compare real Metal preview pixels with saved output away from antialiased edges.
#[cfg(target_os = "macos")]
fn check_annotation_rendering(output: &std::path::Path) -> Result<()> {
    use annotation::{Document, Mark, Pos, PreviewCache, Style, Tool};
    use gpui_kit::{IntoElement, Render, Styled as _, Window, canvas};
    use std::{cell::RefCell, rc::Rc};

    struct Scene(Rc<RefCell<PreviewCache>>);
    impl Render for Scene {
        fn render(&mut self, _: &mut Window, _: &mut gpui_kit::Context<Self>) -> impl IntoElement {
            let cache = self.0.clone();
            canvas(
                |_, _, _| (),
                move |bounds, _, window, _| {
                    window.paint_quad(gpui_kit::fill(bounds, gpui_kit::rgb(0x506478)));
                    cache.borrow().paint(Pos::default(), 1., None, window);
                },
            )
            .size_full()
        }
    }
    let mut bytes = std::io::Cursor::new(Vec::new());
    image::RgbaImage::from_pixel(400, 300, image::Rgba([80, 100, 120, 255]))
        .write_to(&mut bytes, image::ImageFormat::Png)?;
    let mut doc = Document::decode(bytes.get_ref())?;
    let cache = Rc::new(RefCell::new(PreviewCache::default()));
    let platform = gpui_kit::platform::current_platform(true);
    let mut cx = HeadlessAppContext::with_platform(
        platform.text_system(),
        Arc::new(gpui_kit::assets::AllAssets),
        gpui_kit::platform::current_headless_renderer,
    );
    let handle = cx.open_window(size(px(400.), px(300.)), |_, cx| {
        cx.new(|_| Scene(cache.clone()))
    })?;
    for (name, tool, text, opacity, rotation) in [
        ("translucent-shape", Tool::Rectangle, "Label", 0.5, 0.),
        ("translucent-text", Tool::Text, "Outlined", 0.5, 0.),
        ("rotated-text", Tool::Text, "Rotated", 0.25, 0.3),
        ("emoji-text", Tool::Text, "A😀B", 1., 0.),
        ("emoji-shape", Tool::Rectangle, "👩🏽‍💻", 0.5, 0.2),
        ("emoji-arrow", Tool::Arrow, "🇺🇸", 0.75, 0.),
    ] {
        let mut mark = Mark::new(
            tool,
            Pos::new(60., 60.),
            Style {
                width: 8.,
                color: 6,
                fill: annotation::Fill::Solid,
                opacity,
                ..Default::default()
            },
        );
        mark.points[1] = Pos::new(260., 230.);
        mark.style.font_size = 44.;
        mark.rotation = rotation;
        mark.set_text(text.into());
        doc.marks = vec![mark];
        cache.borrow_mut().update(&doc);
        cx.update_window(*handle, |_, window, cx| window.render_frame(cx))?;
        cx.run_until_parked();
        let preview = cx.capture_screenshot(handle.into())?;
        let (png, bounds) = doc.png(None)?;
        let exported = image::load_from_memory(&png)?.to_rgba8();
        preview.save(output.join(format!("{name}-preview.png")))?;
        exported.save(output.join(format!("{name}-export.png")))?;
        let factor = preview.width() as f32 / 400.;
        let mut checked = 0;
        let mut changed = 0;
        for y in 2..298u32 {
            for x in 2..398u32 {
                let ex = (x as f32 - bounds.x) as u32;
                let ey = (y as f32 - bounds.y) as u32;
                let expected = exported.get_pixel(ex, ey);
                if expected.0 == [80, 100, 120, 255]
                    || !(-1..=1).all(|dy| {
                        (-1..=1).all(|dx| {
                            exported.get_pixel((ex as i32 + dx) as u32, (ey as i32 + dy) as u32)
                                == expected
                        })
                    })
                {
                    continue;
                }
                checked += 1;
                let actual = preview.get_pixel(
                    ((x as f32 + 0.5) * factor) as u32,
                    ((y as f32 + 0.5) * factor) as u32,
                );
                if expected.0[..3]
                    .iter()
                    .zip(&actual.0[..3])
                    .any(|(a, b)| a.abs_diff(*b) > 3)
                {
                    changed += 1;
                }
            }
        }
        anyhow::ensure!(checked > 20, "{name}: insufficient painted pixels");
        anyhow::ensure!(
            changed == 0,
            "{name}: {changed}/{checked} interior pixels differ between preview and export"
        );
        println!("{name}: {checked} interior pixels match saved output");
    }
    Ok(())
}

/// A flat image must remain flat after pixelation, including filtered brush edges
/// and tile boundaries. Exercise the real Metal renderer at fractional zooms.
#[cfg(target_os = "macos")]
fn check_pixelation_edges(output: &std::path::Path) -> Result<()> {
    use annotation::{Document, Mark, Pos, PreviewCache, Style, Tool};
    use gpui_kit::{IntoElement, Render, Styled as _, Window, canvas};
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };

    struct Scene {
        cache: Rc<RefCell<PreviewCache>>,
        scale: Rc<Cell<f32>>,
    }
    impl Render for Scene {
        fn render(&mut self, _: &mut Window, _: &mut gpui_kit::Context<Self>) -> impl IntoElement {
            let cache = self.cache.clone();
            let scale = self.scale.get();
            canvas(
                |_, _, _| (),
                move |bounds, _, window, _| {
                    window.paint_quad(gpui_kit::fill(bounds, gpui_kit::rgb(0x506478)));
                    cache.borrow().paint(
                        Pos::new(450.25 - 256. * scale, 350.5 - 256. * scale),
                        scale,
                        None,
                        window,
                    );
                },
            )
            .size_full()
        }
    }
    let base = image::RgbaImage::from_pixel(512, 512, image::Rgba([80, 100, 120, 255]));
    let mut bytes = std::io::Cursor::new(Vec::new());
    base.write_to(&mut bytes, image::ImageFormat::Png)?;
    let mut doc = Document::decode(bytes.get_ref())?;
    let mut stroke = Mark::new(
        Tool::Redact,
        Pos::new(180., 200.),
        Style {
            width: 54.,
            ..Default::default()
        },
    );
    stroke.points[1] = Pos::new(340., 300.);
    let cache = Rc::new(RefCell::new(PreviewCache::default()));
    let platform = gpui_kit::platform::current_platform(true);
    let mut cx = HeadlessAppContext::with_platform(
        platform.text_system(),
        Arc::new(gpui_kit::assets::AllAssets),
        gpui_kit::platform::current_headless_renderer,
    );
    let zoom = Rc::new(Cell::new(3.25));
    let handle = cx.open_window(size(px(900.), px(700.)), |_, cx| {
        cx.new(|_| Scene {
            cache: cache.clone(),
            scale: zoom.clone(),
        })
    })?;
    for scale in [3.25, 2.4, 0.6] {
        doc.marks.clear();
        cache.borrow_mut().update(&doc);
        zoom.set(scale);
        cx.update_window(*handle, |_, window, cx| window.render_frame(cx))?;
        cx.run_until_parked();
        let before = cx.capture_screenshot(handle.into())?;
        doc.marks.push(stroke.clone());
        cache.borrow_mut().update(&doc);
        cx.update_window(*handle, |_, window, cx| window.render_frame(cx))?;
        cx.run_until_parked();
        let after = cx.capture_screenshot(handle.into())?;
        after.save(output.join(format!("pixelation-edges-{scale}.png")))?;
        let changed = before
            .pixels()
            .zip(after.pixels())
            .filter(|(a, b)| {
                a.0[..3]
                    .iter()
                    .zip(&b.0[..3])
                    .any(|(a, b)| a.abs_diff(*b) > 2)
            })
            .count();
        anyhow::ensure!(
            changed == 0,
            "Pixelation darkens {changed} pixels of a flat image at {scale}× zoom"
        );
        println!("Pixelation edges stay flat at {scale}× zoom");
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn render_rooftops(output: &std::path::Path, level: usize) -> Result<()> {
    let platform = gpui_kit::platform::current_platform(true);
    let mut cx = HeadlessAppContext::with_platform(
        platform.text_system(),
        Arc::new(gpui_kit::assets::AllAssets),
        gpui_kit::platform::current_headless_renderer,
    );
    let (handle, _) = cx.update(|cx| {
        gpui_kit::init(cx);
        gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds {
                    origin: Default::default(),
                    size: size(px(960.), px(600.)),
                })),
                focus: false,
                show: false,
                ..Default::default()
            },
            cx,
            |window, cx| cx.new(|cx| rooftops::Rooftops::for_preview(window, cx, level)),
        )
    })?;
    cx.update_window(handle, |_, window, cx| window.render_frame(cx))?;
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| window.render_frame(cx))?;
    cx.capture_screenshot(handle)
        .context("GPUI's Metal screenshot renderer is unavailable")?
        .save(output)?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn render_scenario(scenario: WalkthroughScenario, output: &std::path::Path) -> Result<()> {
    let platform = gpui_kit::platform::current_platform(true);
    let mut cx = HeadlessAppContext::with_platform(
        platform.text_system(),
        Arc::new(gpui_kit::assets::AllAssets),
        gpui_kit::platform::current_headless_renderer,
    );
    cx.update(|cx| {
        gpui_kit::init(cx);
        theme::apply_dark_theme(cx);
        app::init_keybindings(cx);
        image_preview::init_keybindings(cx);
    });

    let (window_handle, _) = cx.update(|cx| {
        gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds {
                    origin: Default::default(),
                    size: size(px(620.), px(780.)),
                })),
                focus: false,
                show: false,
                ..Default::default()
            },
            cx,
            |window, cx| {
                window.set_window_title("Gatto");
                cx.new(|cx| UploaderApp::for_walkthrough(scenario, window, cx))
            },
        )
    })?;

    cx.update_window(window_handle, |_, window, cx| window.render_frame(cx))?;
    cx.run_until_parked();
    cx.update_window(window_handle, |_, window, cx| window.render_frame(cx))?;
    let image = cx
        .capture_screenshot(window_handle)
        .context("GPUI's Metal screenshot renderer is unavailable")?;
    image
        .save(output)
        .with_context(|| format!("Could not write {}", output.display()))?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn render_image_editor(output: &std::path::Path) -> Result<()> {
    let platform = gpui_kit::platform::current_platform(true);
    let mut cx = HeadlessAppContext::with_platform(
        platform.text_system(),
        Arc::new(gpui_kit::assets::AllAssets),
        gpui_kit::platform::current_headless_renderer,
    );
    cx.update(|cx| {
        gpui_kit::init(cx);
        image_preview::init_keybindings(cx);
        theme::apply_dark_theme(cx);
    });
    let (handle, preview) = cx.update(|cx| {
        gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds {
                    origin: Default::default(),
                    size: size(px(1000.), px(760.)),
                })),
                focus: false,
                show: false,
                ..Default::default()
            },
            cx,
            |_, cx| cx.new(image_preview::ImagePreview::for_walkthrough),
        )
    })?;
    cx.update_window(handle, |_, window, cx| window.render_frame(cx))?;
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| window.render_frame(cx))?;
    cx.capture_screenshot(handle)
        .context("GPUI's Metal screenshot renderer is unavailable")?
        .save(output)?;
    for variant in [
        "marquee",
        "group",
        "shapes",
        "shape-properties",
        "x-box",
        "checkbox",
        "brush",
        "revert",
        "zoom",
        "exit",
        "saving",
        "text-editing",
        "text-finished",
    ] {
        cx.update_window(handle, |_, window, cx| {
            use gpui_kit::InputEvent as _;
            match variant {
                "marquee" => {
                    window.dispatch_event(
                        gpui_kit::MouseDownEvent {
                            position: gpui_kit::point(px(100.), px(160.)),
                            button: gpui_kit::MouseButton::Left,
                            modifiers: Default::default(),
                            click_count: 1,
                            first_mouse: false,
                        }
                        .to_platform_input(),
                        cx,
                    );
                    window.dispatch_event(
                        gpui_kit::MouseMoveEvent {
                            position: gpui_kit::point(px(780.), px(610.)),
                            pressed_button: Some(gpui_kit::MouseButton::Left),
                            modifiers: Default::default(),
                        }
                        .to_platform_input(),
                        cx,
                    );
                }
                "group" => {
                    window.dispatch_event(
                        gpui_kit::MouseUpEvent {
                            position: gpui_kit::point(px(780.), px(610.)),
                            button: gpui_kit::MouseButton::Left,
                            modifiers: Default::default(),
                            click_count: 1,
                        }
                        .to_platform_input(),
                        cx,
                    );
                    preview.update(cx, |preview, cx| preview.select_walkthrough_group(true, cx));
                }
                "shapes" => {
                    preview.update(cx, |preview, cx| {
                        preview.select_walkthrough_group(false, cx)
                    });
                    window.click("annotation-shapes-overflow", cx);
                }
                "shape-properties" => window.click("annotation-shape-picker", cx),
                "x-box" => {
                    window.click("annotation-shapes-overflow", cx);
                    window.click("annotation-shape-picker", cx);
                    window.click(("annotation-shape", 10usize), cx);
                    window.click(("annotation-fill", 2usize), cx);
                }
                "checkbox" => {
                    window.click("annotation-shape-picker", cx);
                    window.click(("annotation-shape", 11usize), cx);
                }
                "brush" => {
                    window.click("annotation-shapes-overflow", cx);
                    window.click(("annotation-tool", 8usize), cx);
                    window.click("pixelate-size", cx);
                    window.dispatch_event(
                        gpui_kit::MouseMoveEvent {
                            position: gpui_kit::point(px(560.), px(470.)),
                            pressed_button: None,
                            modifiers: Default::default(),
                        }
                        .to_platform_input(),
                        cx,
                    );
                }
                "revert" => window.click("annotation-revert", cx),
                "zoom" => {
                    window.click("cancel", cx);
                    window.click("preview-fit", cx);
                }
                "exit" => {
                    window.within("popup-menu").click(3usize, cx);
                    preview.update(cx, |preview, cx| preview.confirm_exit(window, cx));
                }
                "saving" => {
                    window.click("cancel", cx);
                    window.click("annotation-apply", cx);
                }
                "text-editing" | "text-finished" => {
                    preview.update(cx, |preview, cx| {
                        preview.walkthrough_text_edit(variant == "text-finished", window, cx)
                    });
                }
                _ => unreachable!(),
            }
            window.render_frame(cx);
        })?;
        cx.run_until_parked();
        cx.update_window(handle, |_, window, cx| window.render_frame(cx))?;
        cx.capture_screenshot(handle)
            .context("GPUI's Metal screenshot renderer is unavailable")?
            .save(output.with_file_name(format!("image-editor-{variant}.png")))?;
    }
    Ok(())
}

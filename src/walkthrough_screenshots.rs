#![deny(unsafe_op_in_unsafe_fn)]
#![allow(dead_code)]

mod annotation;
mod app;
mod application_menu;
mod compression;
mod desktop;
mod diagnostics;
#[cfg(target_os = "macos")]
#[path = "walkthrough_screenshots/frame.rs"]
mod frame;
mod github;
mod global_shortcut;
mod image_composer;
mod image_preview;
mod log_viewer;
mod menu_bar;
mod model;
mod projects;
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

    generate_captures(&output_directory)?;
    frame::write_documentation_images(&output_directory)
}

#[cfg(target_os = "macos")]
fn generate_captures(output_directory: &std::path::Path) -> Result<()> {
    if std::env::args().any(|arg| arg == "--audit-rooftops" || arg == "--audit-rooftops-gpu") {
        return audit_rooftops(output_directory);
    }
    if std::env::args().any(|arg| arg == "--rooftops-form-edges") {
        for value in [0, 100] {
            render_rooftops(
                &output_directory.join(format!("rooftops-options-{value}.png")),
                RooftopsCapture::OptionsAt(value),
            )?;
        }
        return render_respawn_probe(&output_directory.join("rooftops-respawn-probe.png"));
    }
    if std::env::args().any(|arg| arg == "--rooftops-options") {
        return render_rooftops(
            &output_directory.join("rooftops-options.png"),
            RooftopsCapture::Options,
        );
    }
    if std::env::args().any(|arg| arg == "--check-rooftops-audio") {
        return rooftops::audio::check_output();
    }
    if std::env::args().any(|arg| arg == "--rooftops-effects") {
        return rooftops::audio::write_effect_previews(output_directory);
    }
    if std::env::args().any(|arg| arg == "--rooftops-audio") {
        return rooftops::audio::write_previews(output_directory);
    }
    if std::env::args().any(|arg| arg == "--rooftops-arrival") {
        for (index, time) in [0.6, 1.4, 2.6].into_iter().enumerate() {
            render_rooftops(
                &output_directory.join(format!("rooftops-arrival-{}.png", index + 1)),
                RooftopsCapture::Arrival(time),
            )?;
        }
        return Ok(());
    }
    if std::env::args().any(|arg| arg == "--rooftops-preferences") {
        return render_scenario(
            WalkthroughScenario::Preferences,
            &output_directory.join("rooftops-preferences.png"),
        );
    }
    if std::env::args().any(|arg| arg == "--rooftops-continuity") {
        for level in 0..rooftops::LEVELS - 1 {
            for (name, time) in [
                ("sleep", None),
                ("slide-start", Some(0.)),
                ("slide-middle", Some(0.575)),
                ("wake", Some(1.2)),
                ("start", Some(1.5)),
            ] {
                render_rooftops(
                    &output_directory.join(format!("rooftops-{}-{name}.png", level + 1)),
                    RooftopsCapture::Checkpoint(level, time),
                )?;
            }
        }
        render_rooftops(
            &output_directory.join("rooftops-home.png"),
            RooftopsCapture::Checkpoint(2, None),
        )?;
        return Ok(());
    }
    if std::env::args().any(|arg| arg == "--rooftops-only") {
        for level in 0..rooftops::LEVELS {
            render_rooftops(
                &output_directory.join(format!("rooftops-{}.png", level + 1)),
                RooftopsCapture::Level(level, level == 2),
            )?;
        }
        render_rooftops(
            &output_directory.join("rooftops-phase-3.png"),
            RooftopsCapture::Level(2, false),
        )?;
        render_rooftops(
            &output_directory.join("rooftops-behind.png"),
            RooftopsCapture::Gap(false),
        )?;
        render_rooftops(
            &output_directory.join("rooftops-ahead.png"),
            RooftopsCapture::Gap(true),
        )?;
        let score = rooftops_score::Score {
            coins: 41,
            deaths: 6,
            kills: 21,
            seconds: 143.6,
            personal_best: Some(138.2),
        };
        std::fs::write(
            output_directory.join("rooftops-score.png"),
            rooftops_score::png_bytes(score)?,
        )?;
        return Ok(());
    }
    if std::env::args().any(|arg| arg == "--markers-only") {
        return render_image_editor(&output_directory.join("image-editor.png"));
    }

    if std::env::args().any(|arg| arg == "--projects-only") {
        for scenario in [
            WalkthroughScenario::Project,
            WalkthroughScenario::AttachmentDetails,
            WalkthroughScenario::AttachmentReorder,
            WalkthroughScenario::SidebarCollapsed,
            WalkthroughScenario::EmptyProjects,
            WalkthroughScenario::ProjectRename,
        ] {
            render_scenario(scenario, &output_directory.join(scenario.file_name()))?;
        }
        return Ok(());
    }

    if std::env::args().any(|arg| arg == "--comparison-editor-only") {
        return render_comparison(output_directory, true);
    }

    // Additional comparison diagnostics are opt-in.
    if std::env::args().any(|arg| arg == "--comparison-only") {
        return render_comparison(output_directory, false);
    }
    if std::env::args().any(|arg| arg == "--check-pixelation-edges") {
        return check_pixelation_edges(output_directory);
    }
    if std::env::args().any(|arg| arg == "--check-annotation-rendering") {
        return check_annotation_rendering(output_directory);
    }

    for scenario in WalkthroughScenario::ALL {
        let output = output_directory.join(scenario.file_name());
        render_scenario(scenario, &output)?;
        println!("Wrote {}", output.display());
    }

    for level in 0..rooftops::LEVELS {
        let output = output_directory.join(format!("rooftops-{}.png", level + 1));
        render_rooftops(&output, RooftopsCapture::Level(level, level == 2))?;
        println!("Wrote {}", output.display());
    }
    for (name, level) in [("rooftops-checkpoint.png", 0), ("rooftops-home.png", 2)] {
        let output = output_directory.join(name);
        render_rooftops(&output, RooftopsCapture::Checkpoint(level, None))?;
        println!("Wrote {}", output.display());
    }
    let neighborhood_output = output_directory.join("rooftops-phase-3.png");
    render_rooftops(&neighborhood_output, RooftopsCapture::Level(2, false))?;
    println!("Wrote {}", neighborhood_output.display());
    render_rooftops(
        &output_directory.join("rooftops-options.png"),
        RooftopsCapture::Options,
    )?;

    let editor_output = output_directory.join("image-editor.png");
    render_image_editor(&editor_output)?;
    println!("Wrote {}", editor_output.display());
    render_comparison(output_directory, true)?;
    println!(
        "Wrote {}",
        output_directory.join("comparison-editor.png").display()
    );
    image_preview::ImagePreview::write_walkthrough_cursors(
        &output_directory.join("annotation-cursors.png"),
    )?;

    let score = rooftops_score::Score {
        coins: 41,
        deaths: 6,
        kills: 21,
        seconds: 143.6,
        personal_best: Some(138.2),
    };
    let output = output_directory.join("rooftops-score.png");
    std::fs::write(&output, rooftops_score::png_bytes(score)?)?;
    println!("Wrote {}", output.display());
    Ok(())
}

/// Exercise the complete picker-to-single-editor path with the native renderer.
#[cfg(target_os = "macos")]
fn render_comparison(output: &std::path::Path, editor_only: bool) -> Result<()> {
    use gpui_kit::Focusable as _;
    let intermediate = tempfile::tempdir()?;
    let diagnostic_output = if editor_only {
        intermediate.path()
    } else {
        output
    };
    let platform = gpui_kit::platform::current_platform(true);
    let mut cx = HeadlessAppContext::with_platform(
        platform.text_system(),
        Arc::new(gpui_kit::assets::AllAssets),
        gpui_kit::platform::current_headless_renderer,
    );
    cx.update(|cx| {
        gpui_kit::init(cx);
        theme::apply_dark_theme(cx);
        image_preview::init_keybindings(cx);
        cx.set_reduce_motion(true);
    });
    let (picker, view) = cx.update(|cx| {
        crate::theme::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                    Default::default(),
                    size(
                        px(image_composer::WINDOW_WIDTH),
                        px(image_composer::WINDOW_HEIGHT),
                    ),
                ))),
                show: false,
                focus: false,
                ..Default::default()
            },
            cx,
            |window, cx| cx.new(|cx| image_composer::ImageComposer::new(window, cx)),
        )
    })?;
    let mut heading_bounds = None;
    for populated in [false, true] {
        if populated {
            cx.update_window(picker, |_, window, cx| {
                view.update(cx, |view, cx| view.load_walkthrough(0, window, cx))
            })?;
        }
        cx.run_until_parked();
        cx.update_window(picker, |_, window, cx| {
            window.render_frame(cx);
            let bounds = window.find("comparison-heading").bounds();
            if let Some(previous) = heading_bounds {
                assert_eq!(bounds, previous);
            }
            heading_bounds = Some(bounds);
        })?;
        cx.capture_screenshot(picker)?
            .save(diagnostic_output.join(if populated {
                "comparison-picker.png"
            } else {
                "comparison-empty.png"
            }))?;
    }
    let (main, main_view) = cx.update(|cx| {
        crate::theme::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                    Default::default(),
                    size(px(620.), px(780.)),
                ))),
                show: false,
                focus: false,
                ..Default::default()
            },
            cx,
            |window, cx| {
                cx.new(|cx| {
                    UploaderApp::for_walkthrough(WalkthroughScenario::Preferences, window, cx)
                })
            },
        )
    })?;
    let main_capture = main_view.clone();
    cx.update(|cx| {
        cx.subscribe(
            &view,
            move |_, event: &image_composer::ComparisonRequested, cx| {
                main.update(cx, |_, window, cx| {
                    main_capture.update(cx, |app, cx| {
                        app.begin_comparison(event.0.clone(), window, cx).unwrap();
                    })
                })
                .unwrap();
            },
        )
        .detach()
    });
    let start = std::time::Instant::now();
    cx.update_window(picker, |_, window, cx| {
        view.update(cx, |view, cx| view.load_walkthrough(1, window, cx))
    })?;
    cx.run_until_parked();
    cx.update_window(main, |_, window, cx| window.render_frame(cx))?;
    cx.capture_screenshot(main)?
        .save(diagnostic_output.join("comparison-main.png"))?;
    let attachment = cx
        .update(|cx| main_view.read(cx).walkthrough_comparison())
        .context("Automatic comparison did not stage an attachment")?;
    println!(
        "Combine two walkthrough images: {:.1} ms; {:?}; {} bytes",
        start.elapsed().as_secs_f64() * 1000.,
        attachment.image().unwrap().pixel_size,
        attachment.len()
    );
    std::fs::write(
        diagnostic_output.join("comparison-output.png"),
        &attachment.image().unwrap().bytes,
    )?;
    let id = attachment.id;
    let (editor, preview) = cx.update(|cx| {
        crate::theme::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                    Default::default(),
                    size(px(1060.), px(740.)),
                ))),
                show: false,
                focus: false,
                ..Default::default()
            },
            cx,
            |window, cx| {
                let preview =
                    cx.new(|cx| image_preview::ImagePreview::new(vec![attachment], id, cx));
                window.focus(&preview.read(cx).focus_handle(cx), cx);
                preview
            },
        )
    })?;
    cx.update_window(editor, |_, window, cx| window.render_frame(cx))?;
    cx.run_until_parked();
    for (index, caption) in [(0_usize, "orange cat"), (1, "black cat")] {
        cx.update_window(editor, |_, window, cx| {
            window.activate_window();
            window.double_click(("comparison-caption", index), cx);
        })?;
        cx.run_until_parked();
        cx.update_window(editor, |_, window, cx| {
            window.input(caption, cx);
            window.press("enter", cx);
            window.render_frame(cx);
        })?;
        cx.run_until_parked();
    }
    cx.update_window(editor, |_, window, cx| window.render_frame(cx))?;
    cx.capture_screenshot(editor)?
        .save(output.join("comparison-editor.png"))?;
    if editor_only {
        return Ok(());
    }
    let idle = cx.update_window(editor, |_, window, _| {
        window.find(("comparison-caption", 0_usize)).bounds()
    })?;
    cx.update_window(editor, |_, window, cx| {
        window.activate_window();
        window.double_click(("comparison-caption", 0_usize), cx);
    })?;
    cx.run_until_parked();
    cx.update_window(editor, |_, window, cx| {
        window.render_frame(cx);
        anyhow::ensure!(
            window.find(("comparison-caption", 0_usize)).bounds() == idle,
            "editing must preserve caption bounds"
        );
        Ok::<_, anyhow::Error>(())
    })??;
    cx.capture_screenshot(editor)?
        .save(output.join("comparison-caption-selected.png"))?;
    cx.update_window(editor, |_, window, cx| {
        window.input("Orange gardener", cx);
        window.press("enter", cx);
        window.render_frame(cx);
    })?;
    cx.run_until_parked();
    cx.update_window(editor, |_, window, cx| window.render_frame(cx))?;
    cx.capture_screenshot(editor)?
        .save(output.join("comparison-renamed.png"))?;
    cx.update_window(editor, |_, window, cx| {
        window.double_click(("comparison-caption", 0_usize), cx);
    })?;
    cx.run_until_parked();
    cx.update_window(editor, |_, window, cx| {
        window.input("Orange gardener", cx);
        window.press("end", cx);
        window.press("shift-enter", cx);
        window.input(
            "A caption that wraps naturally without covering either image.",
            cx,
        );
        window.render_frame(cx);
    })?;
    cx.run_until_parked();
    cx.update_window(editor, |_, window, cx| window.render_frame(cx))?;
    cx.capture_screenshot(editor)?
        .save(output.join("comparison-caption-multiline.png"))?;
    cx.update_window(editor, |_, window, cx| {
        window.press("enter", cx);
        window.render_frame(cx);
    })?;
    cx.run_until_parked();
    cx.update_window(editor, |_, window, cx| window.render_frame(cx))?;
    cx.capture_screenshot(editor)?
        .save(output.join("comparison-caption-wrapped.png"))?;
    let source = |name: &str| {
        model::StagedAttachment::from_path(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("packaging")
                .join(name),
            Default::default(),
        )
    };
    let small =
        image_composer::combine([source("menu-bar-icon.png")?, source("app-cat-pixel.png")?])?;
    let id = small.id;
    cx.update_window(editor, |_, window, cx| {
        preview.update(cx, |preview, cx| preview.show(vec![small], id, cx));
        window.render_frame(cx);
    })?;
    cx.run_until_parked();
    cx.update_window(editor, |_, window, cx| {
        window.double_click(("comparison-caption", 0_usize), cx)
    })?;
    cx.run_until_parked();
    cx.update_window(editor, |_, window, cx| {
        window.input(
            "A longer caption beneath a much smaller image, independent of its original size.",
            cx,
        );
        window.press("enter", cx);
        window.render_frame(cx);
    })?;
    cx.run_until_parked();
    cx.update_window(editor, |_, window, cx| window.render_frame(cx))?;
    cx.capture_screenshot(editor)?
        .save(output.join("comparison-small-images.png"))?;
    drop(preview);
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
enum RooftopsCapture {
    Options,
    OptionsAt(u8),
    Level(usize, bool),
    Gap(bool),
    Arrival(f32),
    Checkpoint(usize, Option<f32>),
}

#[cfg(target_os = "macos")]
fn render_rooftops(output: &std::path::Path, capture: RooftopsCapture) -> Result<()> {
    let check_form = matches!(&capture, RooftopsCapture::OptionsAt(_));
    let platform = gpui_kit::platform::current_platform(true);
    let mut cx = HeadlessAppContext::with_platform(
        platform.text_system(),
        Arc::new(gpui_kit::assets::AllAssets),
        gpui_kit::platform::current_headless_renderer,
    );
    let (handle, _) = cx.update(|cx| {
        gpui_kit::init(cx);
        crate::theme::open_window(
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
            |window, cx| {
                cx.new(|cx| match capture {
                    RooftopsCapture::Options => rooftops::Rooftops::for_options_preview(window, cx),
                    RooftopsCapture::OptionsAt(value) => {
                        rooftops::Rooftops::for_options_endpoint_preview(window, cx, value)
                    }
                    RooftopsCapture::Checkpoint(level, time) => {
                        rooftops::Rooftops::for_checkpoint_preview(window, cx, level, time)
                    }
                    RooftopsCapture::Gap(ahead) => {
                        rooftops::Rooftops::for_gap_preview(window, cx, ahead)
                    }
                    RooftopsCapture::Level(level, show_results) => {
                        rooftops::Rooftops::for_preview(window, cx, level, show_results)
                    }
                    RooftopsCapture::Arrival(time) => {
                        rooftops::Rooftops::for_arrival_preview(window, cx, time)
                    }
                })
            },
        )
    })?;
    cx.update_window(handle, |_, window, cx| window.render_frame(cx))?;
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| window.render_frame(cx))?;
    let image = cx
        .capture_screenshot(handle)
        .context("GPUI's Metal screenshot renderer is unavailable")?;
    image.save(output)?;
    if check_form {
        let ratio = image.width() as f32 / 960.;
        let x = (342. * ratio) as u32;
        let border = ((190. * ratio) as u32..(220. * ratio) as u32)
            .any(|y| image.get_pixel(x, y).0[..3] == [139, 123, 255]);
        anyhow::ensure!(!border, "Outer form border intersects the slider endpoint");
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn render_scenario(scenario: WalkthroughScenario, output: &std::path::Path) -> Result<()> {
    // Keep main-window examples near the app's 600px minimum height. Preferences
    // and the mixed batch need more room to keep their relevant controls visible.
    let rooftops_preferences = std::env::args().any(|arg| arg == "--rooftops-preferences");
    let (width, height) = match scenario {
        WalkthroughScenario::Preferences if rooftops_preferences => (900., 1750.),
        WalkthroughScenario::MixedUploadProgress => (1024., 768.),
        WalkthroughScenario::Preferences => (800., 780.),
        WalkthroughScenario::AttachmentReorder | WalkthroughScenario::SidebarCollapsed => {
            (800., 760.)
        }
        WalkthroughScenario::VideoCompressing => (800., 640.),
        _ => (800., 600.),
    };
    let platform = gpui_kit::platform::current_platform(true);
    let mut cx = HeadlessAppContext::with_platform(
        platform.text_system(),
        Arc::new(gpui_kit::assets::AllAssets),
        gpui_kit::platform::current_headless_renderer,
    );
    cx.update(|cx| {
        gpui_kit::init(cx);
        cx.set_reduce_motion(true);
        theme::apply_dark_theme(cx);
        app::init_keybindings(cx);
        image_preview::init_keybindings(cx);
    });

    let (window_handle, _) = cx.update(|cx| {
        crate::theme::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds {
                    origin: Default::default(),
                    size: size(px(width), px(height)),
                })),
                focus: false,
                show: false,
                ..Default::default()
            },
            cx,
            |window, cx| {
                window.set_window_title("Gatto");
                cx.new(|cx| {
                    if rooftops_preferences {
                        UploaderApp::for_rooftops_preferences(window, cx)
                    } else {
                        UploaderApp::for_walkthrough(scenario, window, cx)
                    }
                })
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
        crate::theme::open_window(
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
    let variants = if std::env::args().any(|arg| arg == "--markers-only") {
        vec!["markers"]
    } else {
        vec![
            "text-editing",
            "shapes",
            "markers",
            "group",
            "brush",
            "exit",
        ]
    };
    for variant in variants {
        cx.update_window(handle, |_, window, cx| {
            preview.update(cx, |preview, cx| {
                preview.walkthrough_scene(variant, window, cx)
            });
            window.render_frame(cx);
        })?;
        cx.run_until_parked();
        cx.update_window(handle, |_, window, cx| {
            match variant {
                "shapes" => window.click("annotation-shapes-overflow", cx),
                "exit" => preview.update(cx, |preview, cx| preview.confirm_exit(window, cx)),
                _ => {}
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

/// Opt-in native frame submission measurements. This does not measure display
/// presentation or count dropped frames on a physical screen.
#[cfg(target_os = "macos")]
fn audit_rooftops(output: &std::path::Path) -> Result<()> {
    use std::time::Instant;
    let readback = std::env::args().any(|arg| arg == "--audit-rooftops-gpu");
    let frames = if readback { 90 } else { 330 };
    let mut reports = Vec::new();
    for (width, height) in [(960., 600.), (1440., 800.)] {
        for (scene, level, results) in [
            ("level", 0, false),
            ("level", 1, false),
            ("level", 2, false),
            ("results", 2, true),
            ("slide-1", 0, false),
            ("slide-2", 1, false),
            ("arrival", 0, false),
            ("options", 0, false),
        ] {
            let platform = gpui_kit::platform::current_platform(true);
            let mut cx = HeadlessAppContext::with_platform(
                platform.text_system(),
                Arc::new(gpui_kit::assets::AllAssets),
                gpui_kit::platform::current_headless_renderer,
            );
            let (handle, game) = cx.update(|cx| {
                gpui_kit::init(cx);
                crate::theme::open_window(
                    WindowOptions {
                        window_bounds: Some(WindowBounds::Windowed(Bounds {
                            origin: Default::default(),
                            size: size(px(width), px(height)),
                        })),
                        focus: false,
                        show: false,
                        ..Default::default()
                    },
                    cx,
                    |window, cx| {
                        cx.new(|cx| match scene {
                            "slide-1" | "slide-2" => rooftops::Rooftops::for_checkpoint_preview(
                                window,
                                cx,
                                level,
                                Some(0.575),
                            ),
                            "arrival" => rooftops::Rooftops::for_arrival_preview(window, cx, 0.6),
                            "options" => rooftops::Rooftops::for_options_preview(window, cx),
                            _ => rooftops::Rooftops::for_preview(window, cx, level, results),
                        })
                    },
                )
            })?;
            let mut samples = Vec::with_capacity(300);
            for frame in 0..frames {
                let started = Instant::now();
                cx.update_window(handle, |_, window, cx| {
                    game.update(cx, |game, cx| game.audit_frame(frame, cx));
                    window.render_frame(cx);
                })?;
                cx.run_until_parked();
                if readback {
                    std::hint::black_box(cx.capture_screenshot(handle)?);
                }
                if frame >= 30 {
                    samples.push(started.elapsed().as_secs_f64() * 1000.);
                }
            }
            samples.sort_by(f64::total_cmp);
            let report = serde_json::json!({
                "viewport": [width, height], "scene": scene, "level": level + 1, "results": results,
                "gpu_readback": readback, "frames": samples.len(), "p50_ms": samples[samples.len() / 2], "p95_ms": samples[samples.len() * 95 / 100],
                "p99_ms": samples[samples.len() * 99 / 100], "max_ms": samples[samples.len() - 1],
                "over_16_67_ms": samples.iter().filter(|&&ms| ms > 1000. / 60.).count(),
            });
            println!("{report}");
            reports.push(report);
        }
    }
    if !readback {
        rooftops::audio::audit(output)?;
    }
    std::fs::write(
        output.join(if readback {
            "rooftops-metal-frames.json"
        } else {
            "rooftops-frames.json"
        }),
        serde_json::to_vec_pretty(&reports)?,
    )?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn render_respawn_probe(output: &std::path::Path) -> Result<()> {
    struct Probe;
    impl gpui_kit::Render for Probe {
        fn render(
            &mut self,
            _: &mut gpui_kit::Window,
            _: &mut gpui_kit::Context<Self>,
        ) -> impl gpui_kit::IntoElement {
            rooftops::respawn_probe()
        }
    }
    let platform = gpui_kit::platform::current_platform(true);
    let mut cx = HeadlessAppContext::with_platform(
        platform.text_system(),
        Arc::new(gpui_kit::assets::AllAssets),
        gpui_kit::platform::current_headless_renderer,
    );
    let (handle, _) = cx.update(|cx| {
        gpui_kit::init(cx);
        crate::theme::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds {
                    origin: Default::default(),
                    size: size(px(600.), px(480.)),
                })),
                focus: false,
                show: false,
                ..Default::default()
            },
            cx,
            |_, cx| cx.new(|_| Probe),
        )
    })?;
    cx.update_window(handle, |_, window, cx| window.render_frame(cx))?;
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| window.render_frame(cx))?;
    let image = cx
        .capture_screenshot(handle)
        .context("native probe unavailable")?;
    image.save(output)?;
    let ratio = image.width() as f32 / 600.;
    let at = |x: f32, y: f32| image.get_pixel((x * ratio) as u32, (y * ratio) as u32);
    anyhow::ensure!(
        at(295., 345.) == at(330., 345.),
        "Respawn body overlap is darker than the single-layer body"
    );
    println!("Native respawn alpha and slider endpoint checks passed.");
    Ok(())
}

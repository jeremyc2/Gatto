#![allow(dead_code)]

mod app;
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

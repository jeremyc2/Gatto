//! Deterministic native interaction frames, authored without an on-screen window.
use super::*;

const FPS: usize = 30;
const FRAMES: usize = 14 * FPS;
const CIRCLE_START: [f32; 2] = [60., 88.];
const CIRCLE_END: [f32; 2] = [128., 162.];
const ARROW_START: [f32; 2] = [470., 245.];
const ARROW_END: [f32; 2] = [133., 125.];
const TEXT_ANCHOR: [f32; 2] = [390., 115.];
const TEXT_CHUNKS: [&str; 13] = [
    "Str", "ong", "er ", "out", "lin", "e f", "or\n", "the", " ac", "tiv", "e c", "olo", "r",
];

fn lerp(a: [f32; 2], b: [f32; 2], t: f32) -> [f32; 2] {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
}

fn ease(t: f32) -> f32 {
    let t = t.clamp(0., 1.);
    t * t * (3. - 2. * t)
}

pub(super) fn render(output: &std::path::Path) -> Result<()> {
    let platform = gpui_kit::platform::current_platform(true);
    let mut cx = HeadlessAppContext::with_platform(
        platform.text_system(),
        Arc::new(gpui_kit::assets::AllAssets),
        gpui_kit::platform::current_headless_renderer,
    );
    cx.update(|cx| {
        gpui_kit::init(cx);
        cx.set_reduce_motion(true);
        image_preview::init_keybindings(cx);
        theme::apply_dark_theme(cx);
        theme::set_ui_font_size(CAPTURE_UI_FONT_SIZE, cx);
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
            |_, cx| cx.new(image_preview::ImagePreview::for_annotation_demo),
        )
    })?;
    cx.update_window(handle, |_, window, cx| window.render_frame(cx))?;
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| window.render_frame(cx))?;

    let (origin, scale, backing, circle_tool, arrow_tool, text_tool, save_button) = cx
        .update_window(handle, |_, window, cx| {
            let (origin, scale) = preview.read(cx).annotation_demo_transform(window);
            let center = |selector: (&'static str, usize)| {
                let point = window.find(selector).bounds().center();
                [point.x.as_f32(), point.y.as_f32()]
            };
            let save = window.find("annotation-apply").bounds().center();
            (
                origin,
                scale,
                window.scale_factor(),
                center(("annotation-tool", 4usize)),
                center(("annotation-tool", 6usize)),
                center(("annotation-tool", 2usize)),
                [save.x.as_f32(), save.y.as_f32()],
            )
        })?;
    let native = |p: [f32; 2]| [p[0] * backing, p[1] * backing];
    let image_native = |p: [f32; 2]| native([origin[0] + p[0] * scale, origin[1] + p[1] * scale]);
    let circle_tool = native(circle_tool);
    let arrow_tool = native(arrow_tool);
    let text_tool = native(text_tool);
    let save_button = native(save_button);
    let circle_start = image_native(CIRCLE_START);
    let circle_end = image_native(CIRCLE_END);
    let arrow_start = image_native(ARROW_START);
    let arrow_end = image_native(ARROW_END);
    let text_anchor = image_native(TEXT_ANCHOR);
    let mut metadata = Vec::new();
    let mut last_state = None;
    let mut dimensions = (0, 0);
    let mut text_bounds = None;
    for frame in 0..FRAMES {
        let action = match frame {
            15 => Some(("ellipse", "tool", CIRCLE_START)),
            30 => Some(("ellipse", "down", CIRCLE_START)),
            31..=62 => Some((
                "ellipse",
                "move",
                lerp(CIRCLE_START, CIRCLE_END, ease((frame - 30) as f32 / 33.)),
            )),
            63 => Some(("ellipse", "up", CIRCLE_END)),
            75 => Some(("arrow", "tool", ARROW_START)),
            93 => Some(("arrow", "down", ARROW_START)),
            94..=137 => Some((
                "arrow",
                "move",
                lerp(ARROW_START, ARROW_END, ease((frame - 93) as f32 / 45.)),
            )),
            138 => Some(("arrow", "up", ARROW_END)),
            153 => Some(("text", "tool", TEXT_ANCHOR)),
            168 => Some(("text", "down", TEXT_ANCHOR)),
            169 => Some(("text", "up", TEXT_ANCHOR)),
            _ => None,
        };
        let chunk = if (174..=234).contains(&frame) && (frame - 174) % 5 == 0 {
            Some(TEXT_CHUNKS[(frame - 174) / 5])
        } else {
            None
        };
        let changed = action.is_some() || chunk.is_some() || frame == 0 || frame == 246;
        let path = output.join(format!("frame-{frame:04}.png"));
        // Holds share storage; unlink first so reruns cannot rewrite an earlier hold.
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        if changed {
            cx.update_window(handle, |_, window, cx| {
                preview.update(cx, |preview, cx| {
                    if let Some((tool, phase, at)) = action {
                        if phase == "tool" {
                            preview.annotation_demo_tool(tool, window, cx);
                        } else {
                            preview.annotation_demo_pointer(phase, at, window, cx);
                        }
                    }
                    if let Some(text) = chunk {
                        preview.annotation_demo_type(text, window, cx);
                    }
                    if frame == 246 {
                        preview.annotation_demo_commit(window, cx);
                    }
                });
                window.render_frame(cx);
            })?;
            cx.run_until_parked();
            cx.update_window(handle, |_, window, cx| window.render_frame(cx))?;
            let screenshot = cx
                .capture_screenshot(handle)
                .context("GPUI Metal frame readback unavailable")?;
            dimensions = screenshot.dimensions();
            screenshot.save(&path)?;
            last_state = Some(path.clone());
            if frame == 246 {
                let (png, bounds) = cx.update_window(handle, |_, _, cx| {
                    (
                        preview.read(cx).annotation_demo_png(),
                        preview.read(cx).annotation_demo_text_bounds(),
                    )
                })?;
                std::fs::write(output.join("final-annotated.png"), png?)?;
                text_bounds = bounds.map(|b| {
                    [
                        image_native([b[0], b[1]])[0],
                        image_native([b[0], b[1]])[1],
                        b[2] * scale * backing,
                        b[3] * scale * backing,
                    ]
                });
            }
        } else {
            std::fs::hard_link(last_state.as_ref().unwrap(), &path)?;
        }
        let cursor = match frame {
            0..=14 => lerp([1800., 1430.], circle_tool, ease(frame as f32 / 15.)),
            15..=20 => circle_tool,
            21..=29 => lerp(circle_tool, circle_start, ease((frame - 21) as f32 / 9.)),
            30..=63 => lerp(circle_start, circle_end, ease((frame - 30) as f32 / 33.)),
            64..=92 => lerp(circle_end, arrow_start, ease((frame - 64) as f32 / 29.)),
            93..=138 => lerp(arrow_start, arrow_end, ease((frame - 93) as f32 / 45.)),
            139..=167 => lerp(arrow_end, text_anchor, ease((frame - 139) as f32 / 29.)),
            168..=245 => text_anchor,
            246..=305 => lerp(
                text_anchor,
                image_native([790., 300.]),
                ease((frame - 246) as f32 / 24.),
            ),
            306..=335 => lerp(
                image_native([790., 300.]),
                save_button,
                ease((frame - 306) as f32 / 30.),
            ),
            _ => save_button,
        };
        let kind = if (21..=63).contains(&frame) || (75..=138).contains(&frame) {
            "crosshair"
        } else if (153..=245).contains(&frame) {
            "ibeam"
        } else {
            "pointer"
        };
        let event = match frame {
            15 => "tool",
            75 | 153 => "shortcut",
            30 | 93 => "draw-down",
            63 | 138 => "place",
            168 => "text-down",
            169 => "text-up",
            246 => "commit",
            345 => "save",
            _ if chunk.is_some() => "type",
            _ => "",
        };
        metadata.push(serde_json::json!({
            "frame": frame, "time": frame as f32 / FPS as f32,
            "file": format!("frame-{frame:04}.png"), "cursor": cursor, "cursorKind": kind,
            "pressed": (30..63).contains(&frame) || (93..138).contains(&frame) || frame == 168,
            "event": event, "text": chunk,
        }));
        if frame % FPS == 0 {
            eprintln!("annotation demo {frame}/{FRAMES}");
        }
    }
    std::fs::write(
        output.join("metadata.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "fps": FPS, "frames": FRAMES, "width": dimensions.0, "height": dimensions.1,
            "logicalWidth": 1000, "logicalHeight": 760, "imageOrigin": native(origin), "imageScale": scale * backing,
            "circleTool": circle_tool, "arrowTool": arrow_tool, "textTool": text_tool, "saveButton": save_button,
            "circleStart": circle_start, "circleEnd": circle_end, "arrowStart": arrow_start, "arrowEnd": arrow_end,
            "textAnchor": text_anchor, "textBounds": text_bounds, "feedback": TEXT_CHUNKS.join(""),
            "finalAnnotated": "final-annotated.png", "timeline": metadata,
        }))?,
    )?;
    Ok(())
}

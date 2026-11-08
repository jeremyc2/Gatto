//! Sparse native project-workflow states for the headless product film.
use super::*;

type MainHandle = gpui_kit::AnyWindowHandle;

fn redraw(cx: &mut HeadlessAppContext, handle: MainHandle) -> Result<()> {
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| window.render_frame(cx))?;
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| window.render_frame(cx))?;
    Ok(())
}

fn bounds(cx: &mut HeadlessAppContext, handle: MainHandle, selector: &str) -> Result<[f32; 4]> {
    cx.update_window(handle, |_, window, _| {
        let b = if selector == "project-name-field" {
            window
                .within("project-name-field")
                .find("input-state")
                .bounds()
        } else if selector == "notification" {
            // The preceding upload toast can still have an invisible exit node.
            // Observe the active toast, rather than the retiring animation scope.
            window
                .within(("slide-down", 0_u64))
                .find("notification")
                .bounds()
        } else {
            window.find(selector.to_owned()).bounds()
        };
        let scale = window.scale_factor();
        [
            b.left().as_f32() * scale,
            b.top().as_f32() * scale,
            b.size.width.as_f32() * scale,
            b.size.height.as_f32() * scale,
        ]
    })
}

fn center(b: [f32; 4]) -> [f32; 2] {
    [b[0] + b[2] / 2., b[1] + b[3] / 2.]
}

fn hover(cx: &mut HeadlessAppContext, handle: MainHandle, native: [f32; 2]) -> Result<()> {
    cx.update_window(handle, |_, window, cx| {
        let scale = window.scale_factor();
        window.dispatch_event(
            gpui_kit::PlatformInput::MouseMove(gpui_kit::MouseMoveEvent {
                position: gpui_kit::point(px(native[0] / scale), px(native[1] / scale)),
                ..Default::default()
            }),
            cx,
        );
    })?;
    redraw(cx, handle)
}

fn capture(
    cx: &mut HeadlessAppContext,
    handle: MainHandle,
    output: &std::path::Path,
    id: &str,
    duration: f32,
    cursor: [f32; 2],
    click: bool,
) -> Result<serde_json::Value> {
    redraw(cx, handle)?;
    let file = format!("{id}.png");
    cx.capture_screenshot(handle)
        .context("GPUI Metal frame readback unavailable")?
        .save(output.join(&file))?;
    eprintln!("product demo: {id}");
    let focus = if id == "project-menu" || id == "attachment-menu" || id == "annotate-choice" {
        cx.update_window(handle, |_, window, _| {
            let b = window.within("popup-menu").find(2usize).bounds();
            [
                b.center().x.as_f32() * window.scale_factor(),
                b.center().y.as_f32() * window.scale_factor(),
            ]
        })?
    } else if id == "create-dialog" || id.starts_with("name-") || id == "confirm-create" {
        [1000., 760.]
    } else if id == "project-annotated" || id == "final-clean" || id.starts_with("upload-progress-")
    {
        [1180., 555.]
    } else if id == "upload-complete" || id == "copy-confirmed" || id == "copy-ready" {
        [1530., 350.]
    } else {
        cursor
    };
    Ok(
        serde_json::json!({"id":id,"file":file,"duration":duration,"cursor":cursor,"click":click,"focus":focus}),
    )
}

fn argument_path(arguments: &[std::ffi::OsString], flag: &str) -> Option<PathBuf> {
    arguments
        .iter()
        .position(|value| value == flag)
        .and_then(|index| arguments.get(index + 1))
        .map(PathBuf::from)
}

fn capture_outro(
    cx: &mut HeadlessAppContext,
    handle: MainHandle,
    view: &gpui_kit::Entity<UploaderApp>,
    output: &std::path::Path,
    annotated: PathBuf,
) -> Result<(Vec<serde_json::Value>, serde_json::Value)> {
    anyhow::ensure!(
        annotated.exists(),
        "Generate the annotated image before capturing the workflow, or pass --demo-annotation <image.png>"
    );
    let attachment_id = cx.update_window(handle, |_, window, cx| {
        view.update(cx, |view, cx| {
            view.product_demo_stage_image(annotated, window, cx)
        })
    })?;
    redraw(cx, handle)?;
    let upload = center(bounds(cx, handle, "send-image")?);
    let mut outro = vec![capture(
        cx,
        handle,
        output,
        "project-annotated",
        0.6,
        [1230., 1180.],
        false,
    )?];
    hover(cx, handle, upload)?;
    outro.push(capture(
        cx,
        handle,
        output,
        "upload-ready",
        0.65,
        upload,
        false,
    )?);
    for (percent, duration) in [(0, 0.35), (35, 0.45), (70, 0.45)] {
        cx.update_window(handle, |_, _, cx| {
            view.update(cx, |view, cx| {
                view.product_demo_upload_progress(percent, cx)
            })
        })?;
        outro.push(capture(
            cx,
            handle,
            output,
            &format!("upload-progress-{percent:02}"),
            duration,
            upload,
            percent == 0,
        )?);
    }
    cx.update_window(handle, |_, window, cx| {
        view.update(cx, |view, cx| view.product_demo_upload_complete(window, cx))
    })?;
    outro.push(capture(
        cx,
        handle,
        output,
        "upload-complete",
        0.85,
        upload,
        false,
    )?);
    let copy_selector = format!("copy-staged-image-markdown-{attachment_id}");
    let copy = center(bounds(cx, handle, &copy_selector)?);
    hover(cx, handle, copy)?;
    outro.push(capture(
        cx,
        handle,
        output,
        "copy-ready",
        0.55,
        copy,
        false,
    )?);
    cx.update_window(handle, |_, window, cx| {
        view.update(cx, |view, cx| view.product_demo_copy_feedback(window, cx))
    })?;
    outro.push(capture(
        cx,
        handle,
        output,
        "copy-confirmed",
        1.4,
        copy,
        true,
    )?);
    let toast_bounds = bounds(cx, handle, "notification")?;
    let toast = center(toast_bounds);
    cx.update_window(handle, |_, window, cx| {
        use gpui_kit::component::WindowExt as _;
        window.clear_notifications(cx);
    })?;
    outro.push(capture(
        cx,
        handle,
        output,
        "final-clean",
        2.,
        [1740., 1320.],
        false,
    )?);
    Ok((
        outro,
        serde_json::json!({"upload":upload,"copy":copy,"toastBounds":toast_bounds,"toast":toast,"attachment":[1180.,555.]}),
    ))
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
        theme::apply_dark_theme(cx);
        theme::set_ui_font_size(CAPTURE_UI_FONT_SIZE, cx);
        app::init_keybindings(cx);
        image_preview::init_keybindings(cx);
    });
    let (handle, view) = cx.update(|cx| {
        crate::theme::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds {
                    origin: Default::default(),
                    size: size(px(1000.), px(760.)),
                })),
                focus: false,
                show: false,
                ..app::uploader_window_options()
            },
            cx,
            |window, cx| cx.new(|cx| UploaderApp::for_product_demo(window, cx)),
        )
    })?;
    redraw(&mut cx, handle)?;
    let arguments: Vec<_> = std::env::args_os().collect();
    let annotated = argument_path(&arguments, "--demo-annotation")
        .unwrap_or_else(|| PathBuf::from("/tmp/gatto-annotation-v2/final-annotated.png"));
    if arguments
        .iter()
        .any(|arg| arg == "--product-demo-outro-only")
    {
        let source = argument_path(&arguments, "--demo-intro").unwrap_or_else(|| output.to_owned());
        let mut manifest: serde_json::Value =
            serde_json::from_slice(&std::fs::read(source.join("workflow.json"))?)?;
        for state in manifest["states"]
            .as_array()
            .context("Workflow is missing intro states")?
        {
            let file = state["file"]
                .as_str()
                .context("Intro state is missing file")?;
            if source != output {
                std::fs::copy(source.join(file), output.join(file))?;
            }
        }
        cx.update_window(handle, |_, window, cx| {
            view.update(cx, |view, cx| view.product_demo_create_project(window, cx))
        })?;
        let (outro, controls) = capture_outro(&mut cx, handle, &view, output, annotated)?;
        manifest["outro"] = serde_json::Value::Array(outro);
        manifest["controls"]
            .as_object_mut()
            .context("Workflow is missing controls")?
            .extend(controls.as_object().unwrap().clone());
        std::fs::write(
            output.join("workflow.json"),
            serde_json::to_vec_pretty(&manifest)?,
        )?;
        return Ok(());
    }
    let create = center(bounds(&mut cx, handle, "create-project")?);
    let mut states = vec![capture(
        &mut cx, handle, output, "empty", 0.9, create, false,
    )?];
    cx.update_window(handle, |_, window, cx| window.click("create-project", cx))?;
    redraw(&mut cx, handle)?;
    let field = center(bounds(&mut cx, handle, "project-name-field")?);
    states.push(capture(
        &mut cx,
        handle,
        output,
        "create-dialog",
        0.5,
        create,
        true,
    )?);
    for (index, character) in "Palette review".chars().enumerate() {
        cx.update_window(handle, |_, window, cx| {
            window.input(&character.to_string(), cx)
        })?;
        states.push(capture(
            &mut cx,
            handle,
            output,
            &format!("name-{:02}", index + 1),
            0.075,
            field,
            false,
        )?);
    }
    let confirm = center(bounds(&mut cx, handle, "confirm-project-name")?);
    states.push(capture(
        &mut cx,
        handle,
        output,
        "name-ready",
        0.45,
        field,
        false,
    )?);
    states.push(capture(
        &mut cx,
        handle,
        output,
        "confirm-create",
        0.13,
        confirm,
        true,
    )?);
    // The dialog and text input are real. Its persistent commit is represented by
    // the same state in memory, leaving all personal settings/project files alone.
    cx.update_window(handle, |_, window, cx| {
        view.update(cx, |view, cx| view.product_demo_create_project(window, cx))
    })?;
    redraw(&mut cx, handle)?;
    let project_menu = center(bounds(&mut cx, handle, "project-menu-0")?);
    states.push(capture(
        &mut cx,
        handle,
        output,
        "project-created",
        0.65,
        confirm,
        false,
    )?);
    cx.update_window(handle, |_, window, cx| window.click("project-menu-0", cx))?;
    states.push(capture(
        &mut cx,
        handle,
        output,
        "project-menu",
        1.15,
        project_menu,
        true,
    )?);
    cx.update_window(handle, |_, window, cx| window.press("escape", cx))?;
    redraw(&mut cx, handle)?;
    let drop = [1216., 345.];
    hover(&mut cx, handle, drop)?;
    states.push(capture(
        &mut cx,
        handle,
        output,
        "project-menu-dismissed",
        0.45,
        drop,
        false,
    )?);
    let source =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("docs/images/walkthrough/source/canvas.png");
    let attachment_id = cx.update_window(handle, |_, window, cx| {
        view.update(cx, |view, cx| {
            view.product_demo_stage_image(source, window, cx)
        })
    })?;
    redraw(&mut cx, handle)?;
    let attachment_menu_selector = format!("attachment-options-{attachment_id}");
    let attachment_menu = center(bounds(&mut cx, handle, &attachment_menu_selector)?);
    states.push(capture(
        &mut cx,
        handle,
        output,
        "image-staged",
        0.9,
        drop,
        true,
    )?);
    cx.update_window(handle, |_, window, cx| {
        window.click(attachment_menu_selector.clone(), cx)
    })?;
    states.push(capture(
        &mut cx,
        handle,
        output,
        "attachment-menu",
        1.15,
        attachment_menu,
        true,
    )?);
    let annotate = cx.update_window(handle, |_, window, _| {
        let b = window.within("popup-menu").find(2usize).bounds();
        [
            b.center().x.as_f32() * window.scale_factor(),
            b.center().y.as_f32() * window.scale_factor(),
        ]
    })?;
    hover(&mut cx, handle, annotate)?;
    states.push(capture(
        &mut cx,
        handle,
        output,
        "annotate-choice",
        0.4,
        annotate,
        true,
    )?);
    cx.update_window(handle, |_, window, cx| window.press("escape", cx))?;
    let (outro, outro_controls) = capture_outro(&mut cx, handle, &view, output, annotated)?;
    let mut controls = serde_json::json!({"create":create,"field":field,"confirm":confirm,"projectMenu":project_menu,"attachmentMenu":attachment_menu,"annotate":annotate});
    controls
        .as_object_mut()
        .unwrap()
        .extend(outro_controls.as_object().unwrap().clone());
    std::fs::write(
        output.join("workflow.json"),
        serde_json::to_vec_pretty(
            &serde_json::json!({"width":2000,"height":1520,"states":states,"outro":outro,"controls":controls}),
        )?,
    )?;
    Ok(())
}

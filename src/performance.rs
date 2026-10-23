//! Opt-in release measurements of production app paths. No network, preferences or UI side effects.
use std::{hint::black_box, process::Command, time::Instant};

use crate::{
    annotation::{Document, Fill, Mark, Pos, PreviewCache, Style, Tool},
    model::StagedAttachment,
};

const CASES: &[&str] = &[
    "import",
    "open_editor",
    "reopen_editor",
    "export",
    "pixelation",
    "preview",
    "comparison",
];

fn fixture() -> image::RgbaImage {
    // 4K screenshot-like content, with flat panels, fine detail and alpha edges.
    image::RgbaImage::from_fn(3840, 2160, |x, y| {
        let panel = ((x / 160 + y / 90) % 6) as u8;
        image::Rgba([
            30 + panel * 25,
            (x % 251) as u8,
            (y % 241) as u8,
            if x < 16 { (x * 16) as u8 } else { 255 },
        ])
    })
}
fn fingerprint(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    })
}
fn annotated() -> Document {
    let mut doc = Document::from_image(fixture());
    for i in 0..24 {
        let mut mark = Mark::new(
            Tool::Rectangle,
            Pos::new(80. + (i % 6) as f32 * 590., 80. + (i / 6) as f32 * 480.),
            Style {
                opacity: if i % 2 == 0 { 1. } else { 0.6 },
                fill: Fill::Soft,
                color: i % 12,
                ..Style::default()
            },
        );
        mark.points[1] = Pos::new(mark.points[0].x + 180., mark.points[0].y + 120.);
        mark.rotation = i as f32 * 0.07;
        doc.marks.push(mark);
    }
    doc
}

#[test]
#[ignore = "opt-in: cargo test --release --bin gatto performance::measure_workflows -- --ignored --exact --nocapture"]
fn measure_workflows() {
    if cfg!(debug_assertions) {
        panic!("measure the optimized app with --release");
    }
    let Ok(case) = std::env::var("GATTO_PERF_CASE") else {
        let mut reports = Vec::new();
        for case in CASES {
            let output = Command::new("/usr/bin/time")
                .arg("-l")
                .arg(std::env::current_exe().unwrap())
                .args([
                    "--ignored",
                    "--exact",
                    "performance::measure_workflows",
                    "--nocapture",
                    "--test-threads=1",
                ])
                .env("GATTO_PERF_CASE", case)
                .env("LC_ALL", "C")
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{case}: {}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            let stdout = String::from_utf8(output.stdout).unwrap();
            let line = stdout
                .lines()
                .find_map(|line| line.split_once("GATTO_PERF ").map(|(_, json)| json))
                .unwrap();
            let mut report: serde_json::Value = serde_json::from_str(line).unwrap();
            let stderr = String::from_utf8(output.stderr).unwrap();
            let timing = stderr
                .lines()
                .find(|line| line.contains(" real ") && line.contains(" user "))
                .unwrap();
            let fields: Vec<_> = timing.split_whitespace().collect();
            report["process_cpu_ms"] =
                ((fields[2].parse::<f64>().unwrap() + fields[4].parse::<f64>().unwrap()) * 1000.)
                    .into();
            let rss = stderr
                .lines()
                .find(|line| line.contains("maximum resident set size"))
                .unwrap();
            report["process_peak_rss_bytes"] = rss
                .split_whitespace()
                .next()
                .unwrap()
                .parse::<u64>()
                .unwrap()
                .into();
            println!("{report}");
            reports.push(report);
        }
        let report = serde_json::json!({"schema": 1, "cases": reports});
        if let Ok(path) = std::env::var("GATTO_PERF_OUTPUT") {
            std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
        }
        if let Ok(path) = std::env::var("GATTO_PERF_BASELINE") {
            let baseline: serde_json::Value =
                serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
            for current in report["cases"].as_array().unwrap() {
                let previous = baseline["cases"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|p| p["case"] == current["case"])
                    .unwrap();
                assert_eq!(
                    current["fingerprint"], previous["fingerprint"],
                    "output changed: {}",
                    current["case"]
                );
                for metric in ["wall_ms", "process_cpu_ms", "process_peak_rss_bytes"] {
                    // Sub-millisecond timings are below a useful regression threshold.
                    let limit = (previous[metric].as_f64().unwrap() * 1.25).max(1.);
                    assert!(
                        current[metric].as_f64().unwrap() <= limit,
                        "{} regressed: {metric}: {} > {limit}",
                        current["case"],
                        current[metric]
                    );
                }
            }
        }
        return;
    };
    assert!(CASES.contains(&case.as_str()));
    let mut encoded = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(fixture())
        .write_to(&mut encoded, image::ImageFormat::Png)
        .unwrap();
    let encoded = encoded.into_inner();
    let attachment =
        StagedAttachment::from_bytes("fixture.png".into(), encoded.clone(), None).unwrap();
    let doc = matches!(case.as_str(), "export" | "preview").then(annotated);
    let session = crate::annotation::Session::default();
    let preview = attachment.image().unwrap().preview.clone();
    let mut cache = PreviewCache::default();
    let mut brush_doc = (case == "pixelation").then(|| Document::from_image(fixture()));
    let mut wall = Vec::new();
    let mut result = 0;
    for sample in 0..8 {
        // Reset user state outside the timed region. First sample warms code/caches.
        cache.retired_images(true);
        if let Some(doc) = &mut brush_doc {
            doc.marks.clear();
        }
        let start = Instant::now();
        let output = match case.as_str() {
            "import" => {
                let imported =
                    StagedAttachment::from_bytes("fixture.png".into(), encoded.clone(), None)
                        .unwrap();
                assert_eq!(imported.image().unwrap().pixel_size, (3840, 2160));
                black_box(imported);
                Vec::new()
            }
            "open_editor" => {
                black_box(Document::decode(&encoded).unwrap());
                Vec::new()
            }
            "reopen_editor" => {
                for _ in 0..100 {
                    let retained = session.open(&encoded, preview.clone()).unwrap();
                    let saved = retained.document.edit_state();
                    session.retain(retained.document, retained.image, saved);
                }
                Vec::new()
            }
            "export" => doc.as_ref().unwrap().png(None).unwrap().0,
            "preview" => {
                cache.update(doc.as_ref().unwrap());
                Vec::new()
            }
            "pixelation" => {
                let brush_doc = brush_doc.as_mut().unwrap();
                let mut mark = Mark::new(
                    Tool::Redact,
                    Pos::new(120., 240.),
                    Style {
                        width: 70.,
                        ..Style::default()
                    },
                );
                brush_doc.marks.push(mark.clone());
                for i in 0..100 {
                    mark.points
                        .push(Pos::new(120. + i as f32 * 25., 240. + i as f32 * 10.));
                    brush_doc.marks[0] = mark.clone();
                    cache.update(brush_doc);
                    black_box(cache.retired_images(false));
                }
                Vec::new()
            }
            "comparison" => {
                let combined =
                    crate::image_composer::combine([attachment.clone(), attachment.clone()])
                        .unwrap();
                combined.image().unwrap().bytes.to_vec()
            }
            _ => unreachable!(),
        };
        let elapsed = start.elapsed().as_secs_f64() * 1000.;
        let hash = if output.is_empty() {
            0
        } else {
            fingerprint(
                image::load_from_memory(&output)
                    .unwrap()
                    .to_rgba8()
                    .as_raw(),
            )
        };
        if sample > 0 {
            assert_eq!(hash, result, "scenario must produce deterministic pixels");
            wall.push(elapsed);
        }
        result = hash;
    }
    wall.sort_by(f64::total_cmp);
    println!(
        "GATTO_PERF {}",
        serde_json::json!({"case": case, "samples": 7, "wall_ms": wall[3], "fingerprint": result})
    );
}

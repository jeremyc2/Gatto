//! File-only software encoding with the bundled, statically linked helpers.
use super::*;
use serde::Deserialize;
use std::{
    path::PathBuf,
    process::Command,
    time::{Duration, Instant},
};

#[derive(Deserialize)]
struct Probe {
    streams: Vec<Stream>,
    format: Format,
}

#[derive(Deserialize)]
struct Format {
    duration: String,
}

#[derive(Deserialize)]
struct Stream {
    index: u32,
    codec_type: String,
    codec_name: Option<String>,
    width: Option<usize>,
    height: Option<usize>,
    color_transfer: Option<String>,
    start_time: Option<String>,
    #[serde(default)]
    side_data_list: Vec<serde_json::Value>,
}

impl Stream {
    fn start_time(&self) -> Option<f64> {
        self.start_time
            .as_deref()?
            .parse::<f64>()
            .ok()
            .filter(|time| time.is_finite())
    }
    fn rotation(&self) -> i64 {
        self.side_data_list
            .iter()
            .find_map(|value| value.get("rotation").and_then(serde_json::Value::as_f64))
            .unwrap_or(0.)
            .round() as i64
    }

    fn hdr(&self) -> bool {
        matches!(
            self.color_transfer.as_deref(),
            Some("smpte2084" | "arib-std-b67")
        ) || self.side_data_list.iter().any(|value| {
            value
                .get("side_data_type")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|kind| {
                    kind.contains("Mastering display")
                        || kind.contains("Content light")
                        || kind.contains("DOVI")
                })
        })
    }
}

struct Tools {
    encoder: PathBuf,
}

impl Tools {
    fn locate() -> Result<Self> {
        let executable = std::env::current_exe().context("Could not locate the video encoder")?;
        let directory = executable.parent().context("Could not locate Gatto")?;
        let bundled = Self {
            encoder: directory.join("gatto-video-ffmpeg"),
        };
        if bundled.encoder.is_file() {
            return Ok(bundled);
        }
        #[cfg(any(debug_assertions, test))]
        {
            let arch = if cfg!(target_arch = "aarch64") {
                "arm64"
            } else {
                "x86_64"
            };
            let directory = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(".tools/video-encoders")
                .join(arch)
                .join("bin");
            let development = Self {
                encoder: directory.join("ffmpeg"),
            };
            if development.encoder.is_file() {
                return Ok(development);
            }
        }
        bail!("The bundled video encoder is missing. Reinstall Gatto, or choose Fast compression.")
    }

    fn inspect(
        &self,
        path: &Path,
        control: &CompressionControl,
        deadline: Instant,
    ) -> Result<Probe> {
        let bytes = process::run(
            Command::new(&self.encoder).arg("--gatto-inspect").arg(path),
            control,
            deadline,
            true,
            |_| {},
        )?;
        serde_json::from_slice(&bytes)
            .context("Could not read this video's format. Export an SDR MP4 and try again.")
    }
}

pub(super) fn transcode(
    source: &Path,
    destination: &Path,
    control: &CompressionControl,
    attempt: u8,
    progress: &impl Fn(Progress),
    limit: VideoUploadLimit,
    mode: CompressionMode,
) -> Result<()> {
    let tools = Tools::locate()?;
    let deadline = Instant::now() + Duration::from_secs(600);
    let source = source
        .canonicalize()
        .context("The original video is unavailable. Add it again.")?;
    let source_info = tools.inspect(&source, control, deadline)?;
    let videos: Vec<_> = source_info
        .streams
        .iter()
        .filter(|stream| stream.codec_type == "video")
        .collect();
    let audios: Vec<_> = source_info
        .streams
        .iter()
        .filter(|stream| stream.codec_type == "audio")
        .collect();
    ensure!(
        videos.len() == 1 && audios.len() <= 1,
        "Compression requires one video track and at most one audio track. Export a flattened screen recording first."
    );
    let video = videos[0];
    ensure!(
        !video.hdr(),
        "HDR recordings cannot be compressed by this option. Export an SDR MP4 first."
    );
    let (width, height) = (video.width.unwrap_or(0), video.height.unwrap_or(0));
    ensure!(
        width > 0 && height > 0 && width.is_multiple_of(2) && height.is_multiple_of(2),
        "This video has unsupported pixel dimensions. Export an even-sized H.264 MP4 first."
    );
    ensure!(
        mode != CompressionMode::SmallerFiles || (width >= 64 && height >= 64),
        "AV1 compression needs at least 64 × 64 pixels. Choose Quality or Fast compression for this video."
    );
    let duration: f64 = source_info
        .format
        .duration
        .parse()
        .context("This video has no usable duration. Export it again as MP4.")?;
    let rate = video_bitrate(duration, !audios.is_empty(), attempt, limit)?;
    let directory = destination
        .parent()
        .context("The temporary video path is invalid")?;
    let pass_log = directory.join("encoding-pass");
    for pass in 1..=2 {
        control.check()?;
        let mut command = Command::new(&tools.encoder);
        command.args([
            "-hide_banner",
            "-v",
            "error",
            "-nostdin",
            "-nostats",
            "-y",
            "-protocol_whitelist",
            "file,pipe",
            "-noautorotate",
            "-copyts",
            "-start_at_zero",
        ]);
        if video.codec_name.as_deref() == Some("av1") {
            command.args(["-c:v", "libaom-av1"]);
        }
        command
            .arg("-i")
            .arg(&source)
            .arg("-map")
            .arg(format!("0:{}", video.index));
        command
            .args([
                "-map_metadata",
                "-1",
                "-vf",
                "fps=30",
                "-fps_mode:v",
                "passthrough",
                "-threads",
                "4",
                "-b:v",
            ])
            .arg(rate.to_string())
            .args(["-g", "300"]);
        // MOV edit lists can shorten a decoded final frame's nominal duration.
        // Keep fps duplication inside the container's declared playback interval.
        command.arg("-t").arg(duration.to_string());
        match mode {
            CompressionMode::Quality => {
                command.args(["-c:v", "libx264", "-preset", "slow", "-pix_fmt", "yuv420p"]);
            }
            CompressionMode::SmallerFiles => {
                command.args([
                    "-c:v",
                    "libsvtav1",
                    "-preset",
                    "6",
                    "-svtav1-params",
                    "tune=0:scd=1:scm=3:lp=4",
                    "-pix_fmt",
                    "yuv420p10le",
                ]);
            }
            CompressionMode::Fast => unreachable!("Fast uses the native pipeline"),
        }
        command
            .arg("-pass")
            .arg(pass.to_string())
            .arg("-passlogfile")
            .arg(&pass_log)
            .args(["-progress", "pipe:1"]);
        if pass == 1 {
            command.args(["-an", "-f", "null", "/dev/null"]);
        } else {
            if let Some(audio) = audios.first() {
                command
                    .arg("-map")
                    .arg(format!("0:{}", audio.index))
                    .args(["-c:a", "aac", "-b:a", "64000", "-ac", "1", "-ar", "48000"]);
            } else {
                command.arg("-an");
            }
            command.args(["-movflags", "+faststart"]).arg(destination);
        }
        let reported = std::cell::Cell::new(0);
        process::run(&mut command, control, deadline, false, |microseconds| {
            let within_pass = (microseconds as f64 / 1_000_000. / duration).clamp(0., 1.);
            let percent = (((pass - 1) as f64 + within_pass) * 50.).min(99.) as u8;
            if reported.replace(percent) != percent {
                progress(Progress { attempt, percent });
            }
        })?;
    }
    let output = tools.inspect(destination, control, deadline)?;
    let output_videos: Vec<_> = output
        .streams
        .iter()
        .filter(|stream| stream.codec_type == "video")
        .collect();
    let output_audios: Vec<_> = output
        .streams
        .iter()
        .filter(|stream| stream.codec_type == "audio")
        .collect();
    ensure!(
        output_videos.len() == 1 && output_audios.len() == audios.len(),
        "The finished MP4 is missing a video or audio track."
    );
    let encoded = output_videos[0];
    let codec = if mode == CompressionMode::SmallerFiles {
        "av1"
    } else {
        "h264"
    };
    ensure!(
        encoded.codec_name.as_deref() == Some(codec)
            && encoded.width == Some(width)
            && encoded.height == Some(height),
        "The video encoder could not preserve this video's format or original pixel dimensions."
    );
    ensure!(
        encoded.rotation().rem_euclid(360) == video.rotation().rem_euclid(360),
        "The video encoder could not preserve this video's orientation."
    );
    ensure!(
        output_audios
            .iter()
            .all(|audio| audio.codec_name.as_deref() == Some("aac")),
        "The finished MP4 has an unreadable audio format."
    );
    if let (Some(audio), Some(encoded_audio)) = (audios.first(), output_audios.first())
        && let (Some(source_video), Some(source_audio), Some(output_video), Some(output_audio)) = (
            video.start_time(),
            audio.start_time(),
            encoded.start_time(),
            encoded_audio.start_time(),
        )
    {
        ensure!(
            ((source_audio - source_video) - (output_audio - output_video)).abs() < 0.06,
            "The video encoder could not preserve audio timing. Try Fast compression."
        );
    }
    let output_duration: f64 = output
        .format
        .duration
        .parse()
        .context("The finished MP4 has no usable duration")?;
    ensure!(
        output_duration.is_finite() && (output_duration - duration).abs() < 0.15,
        "The video encoder could not preserve this video's duration."
    );
    // Verify a decoded picture, including AV1 on Macs without an AV1 media engine.
    let mut verify = Command::new(&tools.encoder);
    verify.args([
        "-v",
        "error",
        "-nostdin",
        "-protocol_whitelist",
        "file,pipe",
        "-noautorotate",
    ]);
    if mode == CompressionMode::SmallerFiles {
        verify.args(["-c:v", "libaom-av1"]);
    }
    verify.arg("-i").arg(destination).args([
        "-map",
        "0:v:0",
        "-an",
        "-frames:v",
        "1",
        "-f",
        "null",
        "-",
    ]);
    process::run(&mut verify, control, deadline, false, |_| {})?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample-audio.mp4")
    }

    #[test]
    fn integrated_inspection_rejects_invalid_files_and_preserves_hdr_detection() {
        let tools = Tools::locate().unwrap();
        let control = CompressionControl::default();
        let deadline = Instant::now() + Duration::from_secs(30);
        let temporary = tempfile::tempdir().unwrap();
        let broken = temporary.path().join("broken.mp4");
        std::fs::write(&broken, b"not a video").unwrap();
        assert!(tools.inspect(&broken, &control, deadline).is_err());
        assert!(
            tools
                .inspect(
                    Path::new("https://example.invalid/video.mp4"),
                    &control,
                    deadline
                )
                .is_err()
        );
        let hdr = temporary.path().join("hdr.mp4");
        process::run(
            Command::new(&tools.encoder)
                .args(["-v", "error", "-i"])
                .arg(fixture())
                .args([
                    "-map",
                    "0:v:0",
                    "-an",
                    "-c:v",
                    "libx264",
                    "-x264-params",
                    "colorprim=bt2020:transfer=smpte2084:colormatrix=bt2020nc",
                ])
                .arg(&hdr),
            &control,
            deadline,
            false,
            |_| {},
        )
        .unwrap();
        let info = tools.inspect(&hdr, &control, deadline).unwrap();
        assert!(info.streams[0].hdr());
        let error = transcode(
            &hdr,
            &temporary.path().join("rejected.mp4"),
            &control,
            1,
            &|_| {},
            VideoUploadLimit::Standard,
            CompressionMode::Quality,
        )
        .unwrap_err();
        assert!(error.to_string().contains("HDR"));
    }

    #[test]
    fn software_modes_preserve_audio_dimensions_duration_and_original() {
        let source = fixture();
        let bytes = std::fs::read(&source).unwrap();
        let tools =
            Tools::locate().expect("Run bun scripts/prepare-video-encoders.ts before media tests");
        for mode in [CompressionMode::Quality, CompressionMode::SmallerFiles] {
            let temporary = tempfile::tempdir().unwrap();
            let output = temporary.path().join("output.mp4");
            let control = CompressionControl::default();
            let values = std::cell::RefCell::new(Vec::new());
            transcode(
                &source,
                &output,
                &control,
                1,
                &|value| values.borrow_mut().push(value.percent),
                VideoUploadLimit::Standard,
                mode,
            )
            .unwrap();
            let info = tools
                .inspect(&output, &control, Instant::now() + Duration::from_secs(10))
                .unwrap();
            assert_eq!(info.streams.len(), 2);
            let video = info
                .streams
                .iter()
                .find(|stream| stream.codec_type == "video")
                .unwrap();
            assert_eq!(
                video.codec_name.as_deref(),
                Some(if mode == CompressionMode::Quality {
                    "h264"
                } else {
                    "av1"
                })
            );
            assert_eq!((video.width, video.height), (Some(64), Some(64)));
            assert!(std::fs::metadata(&output).unwrap().len() < VideoUploadLimit::Standard.bytes());
            assert!(values.borrow().windows(2).all(|pair| pair[0] <= pair[1]));
            assert!(values.borrow().iter().all(|value| *value < 100));
            assert_eq!(std::fs::read(&source).unwrap(), bytes);
        }
    }

    #[test]
    fn software_preserves_rotation_and_rejects_multiple_video_tracks() {
        let tools = Tools::locate().unwrap();
        let control = CompressionControl::default();
        let deadline = Instant::now() + Duration::from_secs(30);
        let directory = tempfile::tempdir().unwrap();
        let rotated = directory.path().join("rotated.mp4");
        process::run(
            Command::new(&tools.encoder)
                .args(["-v", "error", "-display_rotation:v:0", "90", "-i"])
                .arg(fixture())
                .args(["-c", "copy"])
                .arg(&rotated),
            &control,
            deadline,
            false,
            |_| {},
        )
        .unwrap();
        assert_ne!(
            tools.inspect(&rotated, &control, deadline).unwrap().streams[0].rotation(),
            0
        );
        for mode in [CompressionMode::Quality, CompressionMode::SmallerFiles] {
            transcode(
                &rotated,
                &directory.path().join("output.mp4"),
                &control,
                1,
                &|_| {},
                VideoUploadLimit::Standard,
                mode,
            )
            .unwrap();
        }
        let multiple = directory.path().join("multiple.mp4");
        process::run(
            Command::new(&tools.encoder)
                .args(["-v", "error", "-i"])
                .arg(fixture())
                .args(["-map", "0:v", "-map", "0:v", "-c", "copy"])
                .arg(&multiple),
            &control,
            deadline,
            false,
            |_| {},
        )
        .unwrap();
        let error = transcode(
            &multiple,
            &directory.path().join("rejected.mp4"),
            &control,
            1,
            &|_| {},
            VideoUploadLimit::Standard,
            CompressionMode::Quality,
        )
        .unwrap_err();
        assert!(error.to_string().contains("one video track"), "{error}");
    }

    #[test]
    fn software_preserves_delayed_audio_timing() {
        let tools = Tools::locate().unwrap();
        let control = CompressionControl::default();
        let deadline = Instant::now() + Duration::from_secs(30);
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("offset.mp4");
        process::run(
            Command::new(&tools.encoder)
                .args(["-v", "error", "-i"])
                .arg(fixture())
                .args(["-itsoffset", "0.25", "-i"])
                .arg(fixture())
                .args(["-map", "0:v", "-map", "1:a", "-c", "copy"])
                .arg(&source),
            &control,
            deadline,
            false,
            |_| {},
        )
        .unwrap();
        let input = tools.inspect(&source, &control, deadline).unwrap();
        let audio = input
            .streams
            .iter()
            .find(|stream| stream.codec_type == "audio")
            .unwrap();
        assert!(audio.start_time().unwrap() > 0.2);
        for mode in [CompressionMode::Quality, CompressionMode::SmallerFiles] {
            transcode(
                &source,
                &directory.path().join("output.mp4"),
                &control,
                1,
                &|_| {},
                VideoUploadLimit::Standard,
                mode,
            )
            .unwrap();
        }
    }

    #[test]
    fn software_cancellation_never_stages_an_attachment() {
        let control = CompressionControl::default();
        let error = compress(
            &fixture(),
            VideoUploadLimit::Standard,
            CompressionMode::Quality,
            &control,
            |value| {
                if value.percent > 0 {
                    control.cancel();
                }
            },
        )
        .err()
        .unwrap();
        assert!(error.downcast_ref::<Canceled>().is_some(), "{error}");
    }

    #[test]
    fn hdr_probe_rejects_transfer_functions_and_mastering_metadata() {
        for extra in [
            serde_json::json!({"color_transfer":"smpte2084"}),
            serde_json::json!({"color_transfer":"arib-std-b67"}),
            serde_json::json!({"side_data_list":[{"side_data_type":"Mastering display metadata"}]}),
        ] {
            let mut value = serde_json::json!({"index":0,"codec_type":"video"});
            value
                .as_object_mut()
                .unwrap()
                .extend(extra.as_object().unwrap().clone());
            assert!(serde_json::from_value::<Stream>(value).unwrap().hdr());
        }
    }
}

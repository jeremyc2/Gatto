//! Offline import compression. Native objects stay on one background worker;
//! the UI exchanges only cancellation flags, progress, and a staged snapshot.
use crate::model::{StagedAttachment, VideoUploadLimit};
use anyhow::{Context as _, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CompressionMode {
    SmallerFiles,
    Fast,
    #[default]
    #[serde(other)]
    Quality,
}

impl CompressionMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Quality => "Quality — H.264",
            Self::SmallerFiles => "Smaller files — AV1",
            Self::Fast => "Fast — H.264",
        }
    }

    #[cfg(target_os = "macos")]
    pub fn provenance(self) -> crate::video::VideoCompression {
        match self {
            Self::Quality => crate::video::VideoCompression::H264Quality,
            Self::Fast => crate::video::VideoCompression::H264Fast,
            Self::SmallerFiles => crate::video::VideoCompression::Av1,
        }
    }
}

#[derive(Clone, Default)]
pub struct CompressionControl(Arc<AtomicBool>);

impl CompressionControl {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }
    pub fn is_canceled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
    fn check(&self) -> Result<()> {
        if self.is_canceled() {
            bail!(Canceled);
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Video compression canceled.")]
pub struct Canceled;

#[derive(Clone, Copy)]
pub struct Progress {
    pub attempt: u8,
    pub percent: u8,
}

impl Progress {
    /// Reserve half the bar for each possible size attempt. Software encoding
    /// already combines its two internal encoder passes into one attempt.
    fn overall(self) -> Self {
        Self {
            percent: ((self.attempt - 1) * 50 + self.percent / 2).min(99),
            ..self
        }
    }
}

/// ABR is a target; acceptance always uses the finalized MP4's actual byte count.
fn video_bitrate(
    seconds: f64,
    has_audio: bool,
    attempt: u8,
    limit: VideoUploadLimit,
) -> Result<u32> {
    ensure!(
        seconds.is_finite() && seconds > 0.,
        "This video has no usable duration. Export it again as MP4."
    );
    // Media Foundation's lowest AAC setting is 96 kb/s. The macOS encoders
    // both use 64 kb/s. Account for the platform's actual audio allocation.
    let audio = if has_audio {
        if cfg!(windows) { 96_000. } else { 64_000. }
    } else {
        0.
    };
    let payload_bytes = limit.bytes() as f64 * 0.92;
    let rate = (payload_bytes * 8. / seconds - audio).min(12_000_000.)
        * if attempt == 1 { 1. } else { 0.7 };
    ensure!(
        rate >= 100_000.,
        "This recording is too long to fit below {} MB at its original resolution. Trim it and try again.",
        limit.megabytes()
    );
    Ok(rate.floor() as u32)
}

pub fn compress(
    path: &Path,
    limit: VideoUploadLimit,
    mode: CompressionMode,
    control: &CompressionControl,
    progress: impl Fn(Progress),
) -> Result<StagedAttachment> {
    control.check()?;
    #[cfg(any(target_os = "macos", windows))]
    {
        #[cfg(windows)]
        let _ = mode;
        let original =
            std::fs::metadata(path).context("The original video is unavailable. Add it again.")?;
        ensure!(original.is_file(), "Choose a regular video file.");
        let report = |value: Progress| progress(value.overall());
        for attempt in 1..=2 {
            control.check()?;
            let directory = tempfile::Builder::new()
                .prefix("gatto-compress-")
                .tempdir()
                .context("Could not create temporary storage. Check available disk space.")?;
            let output = directory.path().join("compressed.mp4");
            report(Progress {
                attempt,
                percent: 0,
            });
            #[cfg(target_os = "macos")]
            match mode {
                CompressionMode::Fast => {
                    native::transcode(path, &output, control, attempt, &report, limit)?
                }
                _ => software::transcode(path, &output, control, attempt, &report, limit, mode)?,
            }
            #[cfg(windows)]
            windows::transcode(path, &output, control, attempt, &report, limit)?;
            control.check()?;
            let current = std::fs::metadata(path)
                .context("The source video was removed during compression. Add it again.")?;
            ensure!(
                current.len() == original.len()
                    && current.modified().ok() == original.modified().ok(),
                "The source video changed during compression. Add it again."
            );
            let len = std::fs::metadata(&output)
                .context("Could not inspect the compressed video")?
                .len();
            if len > 0 && len < limit.bytes() {
                let mut attachment = StagedAttachment::from_path(output, limit)?;
                let stem = path
                    .file_stem()
                    .and_then(|name| name.to_str())
                    .unwrap_or("video");
                attachment.name = format!("{stem}.mp4");
                attachment.title = attachment.name.clone();
                #[cfg(target_os = "macos")]
                let provenance = mode.provenance();
                #[cfg(windows)]
                let provenance = crate::video::VideoCompression::H264;
                attachment.compressed_with = Some(provenance);
                control.check()?;
                progress(Progress {
                    attempt,
                    percent: 100,
                });
                return Ok(attachment);
            }
            // The attempt's private directory is removed before retrying from
            // the original. Never repeatedly transcode a lossy intermediate.
        }
        bail!(
            "Compression could not fit this video below {} MB at its original resolution. Trim the recording and try again.",
            limit.megabytes()
        )
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        let _ = (path, progress, limit, mode);
        bail!("Video compression is unavailable on this platform.")
    }
}

#[cfg(target_os = "macos")]
mod native;
#[cfg(target_os = "macos")]
mod process;
#[cfg(target_os = "macos")]
mod software;

#[cfg(windows)]
mod windows;

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(target_os = "macos")]
    use crate::model::MAX_ATTACHMENT_BYTES;

    #[test]
    fn progress_reserves_both_attempts_without_moving_backward() {
        let percentages = [1, 2]
            .into_iter()
            .flat_map(|attempt| {
                (0..=100).map(move |percent| Progress { attempt, percent }.overall().percent)
            })
            .collect::<Vec<_>>();
        assert_eq!(percentages[0], 0);
        assert_eq!(percentages[100], 50);
        assert_eq!(percentages[101], 50);
        assert_eq!(*percentages.last().unwrap(), 99);
        assert!(percentages.windows(2).all(|pair| pair[0] <= pair[1]));
    }

    #[test]
    fn duration_budget_reserves_audio_and_rejects_unusable_durations() {
        let rate = video_bitrate(60., true, 1, VideoUploadLimit::Standard).unwrap();
        assert!(rate <= 1_200_000);
        assert!((rate as f64 + 64_000.) * 60. / 8. <= 9_200_000.);
        assert!(video_bitrate(60., true, 2, VideoUploadLimit::Standard).unwrap() < rate);
        assert!(video_bitrate(15., true, 1, VideoUploadLimit::Standard).unwrap() > rate);
        for duration in [0., -1., f64::NAN, f64::INFINITY, 86_400.] {
            assert!(video_bitrate(duration, true, 1, VideoUploadLimit::Standard).is_err());
        }
    }

    #[test]
    fn paid_compression_budget_uses_selected_limit_and_reserves_overhead() {
        let rate = video_bitrate(120., true, 1, VideoUploadLimit::Paid).unwrap();
        assert!(rate > video_bitrate(120., true, 1, VideoUploadLimit::Standard).unwrap());
        assert!((rate as f64 + 64_000.) * 120. / 8. <= 92_000_000.);
        assert!(video_bitrate(120., true, 2, VideoUploadLimit::Paid).unwrap() < rate);
        let error = video_bitrate(86_400., true, 1, VideoUploadLimit::Paid).unwrap_err();
        assert!(error.to_string().contains("100 MB"));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn native_transcode_produces_a_readable_bounded_mp4_without_modifying_source() {
        let source =
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample.mp4");
        let original = std::fs::read(&source).unwrap();
        let result = compress(
            &source,
            VideoUploadLimit::Standard,
            CompressionMode::Fast,
            &CompressionControl::default(),
            |_| {},
        )
        .unwrap();
        assert!(result.len() < MAX_ATTACHMENT_BYTES);
        assert_eq!(result.mime_type, "video/mp4");
        assert_eq!(result.name, "sample.mp4");
        assert_eq!(
            result.compressed_with,
            Some(crate::video::VideoCompression::H264Fast)
        );
        assert_eq!(std::fs::read(source).unwrap(), original);
    }

    #[test]
    fn canceled_job_never_opens_the_source() {
        let control = CompressionControl::default();
        control.cancel();
        let error = compress(
            Path::new("/does-not-exist.mp4"),
            VideoUploadLimit::Standard,
            CompressionMode::Fast,
            &control,
            |_| {},
        )
        .err()
        .unwrap();
        assert!(error.downcast_ref::<Canceled>().is_some());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn cancellation_during_native_encoding_produces_no_staged_attachment() {
        let source = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/sample-audio.mp4");
        let control = CompressionControl::default();
        let error = compress(
            &source,
            VideoUploadLimit::Standard,
            CompressionMode::Fast,
            &control,
            |progress| {
                if progress.percent > 0 {
                    control.cancel();
                }
            },
        )
        .err()
        .unwrap();
        assert!(error.downcast_ref::<Canceled>().is_some(), "{error}");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn output_contains_h264_and_aac_and_preserves_pixel_dimensions_and_duration() {
        use objc2::{AnyThread as _, rc::autoreleasepool};
        use objc2_av_foundation::*;
        use objc2_foundation::{NSString, NSURL};
        let source = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/sample-audio.mp4");
        let compressed = compress(
            &source,
            VideoUploadLimit::Standard,
            CompressionMode::Fast,
            &CompressionControl::default(),
            |_| {},
        )
        .unwrap();
        let mut output = tempfile::Builder::new().suffix(".mp4").tempfile().unwrap();
        std::io::copy(&mut compressed.reader().unwrap(), output.as_file_mut()).unwrap();
        autoreleasepool(|_| unsafe {
            let url = NSURL::fileURLWithPath(&NSString::from_str(output.path().to_str().unwrap()));
            let asset = AVURLAsset::initWithURL_options(AVURLAsset::alloc(), &url, None);
            assert!((asset.duration().seconds() - 2.).abs() < 0.05);
            let reader =
                AVAssetReader::initWithAsset_error(AVAssetReader::alloc(), &asset).unwrap();
            let tracks = asset.tracks();
            let mut expected = Vec::new();
            for track in tracks.iter() {
                let video = &*track.mediaType() == AVMediaTypeVideo.unwrap();
                if video {
                    assert_eq!(track.naturalSize().width, 64.);
                    assert_eq!(track.naturalSize().height, 64.);
                }
                let output = AVAssetReaderTrackOutput::initWithTrack_outputSettings(
                    AVAssetReaderTrackOutput::alloc(),
                    &track,
                    None,
                );
                assert!(reader.canAddOutput(&output));
                reader.addOutput(&output);
                expected.push((output, if video { *b"avc1" } else { *b"aac " }));
            }
            assert_eq!(expected.len(), 2, "audio must survive compression");
            assert!(reader.startReading());
            for (output, codec) in expected {
                let sample = loop {
                    let sample = output.copyNextSampleBuffer().unwrap();
                    if sample.format_description().is_some() {
                        break sample;
                    }
                };
                assert_eq!(
                    sample.format_description().unwrap().media_sub_type(),
                    u32::from_be_bytes(codec)
                );
            }
            reader.cancelReading();
        });
    }
}

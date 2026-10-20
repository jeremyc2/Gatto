//! Offline import compression. Native objects stay on one background worker;
//! the UI exchanges only cancellation flags, progress, and a staged snapshot.
use crate::model::{StagedAttachment, VideoUploadLimit};
use anyhow::{Context as _, Result, bail, ensure};
use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

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
    let audio = if has_audio { 64_000. } else { 0. };
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
    control: &CompressionControl,
    progress: impl Fn(Progress),
) -> Result<StagedAttachment> {
    control.check()?;
    #[cfg(target_os = "macos")]
    {
        let original =
            std::fs::metadata(path).context("The original video is unavailable. Add it again.")?;
        ensure!(original.is_file(), "Choose a regular video file.");
        for attempt in 1..=2 {
            control.check()?;
            let directory = tempfile::Builder::new()
                .prefix("gatto-compress-")
                .tempdir()
                .context("Could not create temporary storage. Check available disk space.")?;
            let output = directory.path().join("compressed.mp4");
            progress(Progress {
                attempt,
                percent: 0,
            });
            native::transcode(path, &output, control, attempt, &progress, limit)?;
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
                attachment.name = format!("{stem}-compressed.mp4");
                attachment.description = attachment.name.clone();
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
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (path, progress, limit);
        bail!("Video compression requires macOS.")
    }
}

#[cfg(target_os = "macos")]
mod native {
    use super::*;
    use block2::RcBlock;
    use objc2::{
        AnyThread as _,
        rc::{Retained, autoreleasepool},
        runtime::AnyObject,
    };
    use objc2_av_foundation::*;
    use objc2_avf_audio::*;
    use objc2_core_media::{CMSampleBuffer, CMTime};
    use objc2_core_video::{
        CVPixelBufferGetHeight, CVPixelBufferGetWidth, kCVPixelBufferPixelFormatTypeKey,
        kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange,
    };
    use objc2_foundation::{NSArray, NSDictionary, NSError, NSNumber, NSString, NSURL};
    use std::{
        sync::mpsc,
        thread,
        time::{Duration, Instant},
    };

    fn dictionary(
        values: &[(&NSString, &AnyObject)],
    ) -> Retained<NSDictionary<NSString, AnyObject>> {
        NSDictionary::from_slices(
            &values.iter().map(|(key, _)| *key).collect::<Vec<_>>(),
            &values.iter().map(|(_, value)| *value).collect::<Vec<_>>(),
        )
    }

    fn native_error(message: &str, error: Option<Retained<NSError>>) -> anyhow::Error {
        anyhow::anyhow!(
            "{message}: {}",
            error
                .map(|error| error.to_string())
                .unwrap_or_else(|| "macOS did not provide a reason".into())
        )
    }

    fn wait(
        receiver: &mpsc::Receiver<()>,
        control: &CompressionControl,
        deadline: Instant,
    ) -> Result<()> {
        loop {
            control.check()?;
            ensure!(
                Instant::now() < deadline,
                "macOS took too long to process this video. Try a shorter clip."
            );
            match receiver.recv_timeout(Duration::from_millis(50)) {
                Ok(()) => return Ok(()),
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(_) => bail!("The macOS media operation ended unexpectedly."),
            }
        }
    }

    struct Session {
        reader: Retained<AVAssetReader>,
        writer: Retained<AVAssetWriter>,
    }

    impl Drop for Session {
        fn drop(&mut self) {
            // Only the owning worker reads/appends/cancels. No native cancel
            // operation races copyNextSampleBuffer or appendSampleBuffer.
            unsafe {
                if self.reader.status() == AVAssetReaderStatus::Reading {
                    self.reader.cancelReading();
                }
                if matches!(
                    self.writer.status(),
                    AVAssetWriterStatus::Unknown | AVAssetWriterStatus::Writing
                ) {
                    self.writer.cancelWriting();
                }
            }
        }
    }

    fn next_video_frame(
        output: &AVAssetReaderTrackOutput,
        control: &CompressionControl,
    ) -> Result<Option<Retained<CMSampleBuffer>>> {
        loop {
            control.check()?;
            // Zero-sample edit/marker buffers are not decoded pictures.
            let Some(sample) = (unsafe { output.copyNextSampleBuffer() }) else {
                return Ok(None);
            };
            if unsafe { sample.num_samples() } > 0 {
                return Ok(Some(sample));
            }
        }
    }

    fn verify_dimensions(url: &NSURL, width: usize, height: usize, audio: bool) -> Result<()> {
        // Verify a real decoded output frame so an unsupported encoder size
        // cannot silently produce a smaller-resolution accepted attachment.
        unsafe {
            let asset = AVURLAsset::initWithURL_options(AVURLAsset::alloc(), url, None);
            let tracks = asset.tracks();
            let videos = tracks
                .iter()
                .filter(|track| &*track.mediaType() == AVMediaTypeVideo.unwrap())
                .collect::<Vec<_>>();
            let audios = tracks
                .iter()
                .filter(|track| &*track.mediaType() == AVMediaTypeAudio.unwrap())
                .count();
            ensure!(
                videos.len() == 1 && audios == usize::from(audio),
                "The finished MP4 is missing a video or audio track."
            );
            let reader = AVAssetReader::initWithAsset_error(AVAssetReader::alloc(), &asset)
                .map_err(|error| native_error("Could not verify the finished MP4", Some(error)))?;
            let key = NSString::from_str(&kCVPixelBufferPixelFormatTypeKey.to_string());
            let settings = dictionary(&[(
                &key,
                &*NSNumber::new_u32(kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange),
            )]);
            let output = AVAssetReaderTrackOutput::initWithTrack_outputSettings(
                AVAssetReaderTrackOutput::alloc(),
                &videos[0],
                Some(&settings),
            );
            ensure!(
                reader.canAddOutput(&output),
                "The compressed MP4 cannot be decoded."
            );
            reader.addOutput(&output);
            ensure!(reader.startReading(), "The compressed MP4 cannot be read.");
            let result = (|| {
                let sample = next_video_frame(&output, &CompressionControl::default())?
                    .context("The compressed MP4 has no readable picture")?;
                let pixels = sample
                    .image_buffer()
                    .context("The compressed MP4 has no readable pixels")?;
                ensure!(
                    CVPixelBufferGetWidth(&pixels) == width
                        && CVPixelBufferGetHeight(&pixels) == height,
                    "The hardware encoder could not preserve this video's original pixel dimensions."
                );
                Ok(())
            })();
            reader.cancelReading();
            result
        }
    }

    pub(super) fn transcode(
        source: &Path,
        destination: &Path,
        control: &CompressionControl,
        attempt: u8,
        progress: &impl Fn(Progress),
        limit: VideoUploadLimit,
    ) -> Result<()> {
        let source = source
            .to_str()
            .context("The video path cannot be represented by macOS")?;
        let destination = destination
            .to_str()
            .context("The temporary video path is invalid")?;
        let deadline = Instant::now() + Duration::from_secs(600);
        autoreleasepool(|_| {
            // All objects and retained samples stay on this worker, dictionaries
            // contain the documented native value types, and session ordering
            // is checked before reading/appending/finalizing.
            unsafe {
                let source_url = NSURL::fileURLWithPath(&NSString::from_str(source));
                let destination_url = NSURL::fileURLWithPath(&NSString::from_str(destination));
                let options = dictionary(&[(
                    AVURLAssetPreferPreciseDurationAndTimingKey,
                    &*NSNumber::new_bool(true),
                )]);
                let asset = AVURLAsset::initWithURL_options(
                    AVURLAsset::alloc(),
                    &source_url,
                    Some(&options),
                );
                let (sender, receiver) = mpsc::channel();
                let loaded = RcBlock::new(move || {
                    let _ = sender.send(());
                });
                let keys = NSArray::from_slice(&[
                    &*NSString::from_str("tracks"),
                    &*NSString::from_str("duration"),
                    &*NSString::from_str("readable"),
                ]);
                asset.loadValuesAsynchronouslyForKeys_completionHandler(&keys, Some(&loaded));
                if let Err(error) = wait(&receiver, control, deadline) {
                    asset.cancelLoading();
                    return Err(error);
                }
                ensure!(
                    asset.isReadable() && !asset.hasProtectedContent(),
                    "macOS cannot decode this video for compression. Export it as an unprotected H.264 MP4 and try again."
                );
                let duration = asset.duration().seconds();
                let tracks = asset.tracks();
                let videos = tracks
                    .iter()
                    .filter(|track| &*track.mediaType() == AVMediaTypeVideo.unwrap())
                    .collect::<Vec<_>>();
                let audios = tracks
                    .iter()
                    .filter(|track| &*track.mediaType() == AVMediaTypeAudio.unwrap())
                    .collect::<Vec<_>>();
                ensure!(
                    videos.len() == 1 && audios.len() <= 1,
                    "Compression requires one video track and at most one audio track. Export a flattened screen recording first."
                );
                let bitrate = video_bitrate(duration, audios.len() == 1, attempt, limit)?;
                let track = videos[0].clone();
                ensure!(
                    !track.hasMediaCharacteristic(AVMediaCharacteristicContainsHDRVideo.unwrap()),
                    "HDR recordings cannot be compressed by this option. Export an SDR MP4 first."
                );
                let reader = AVAssetReader::initWithAsset_error(AVAssetReader::alloc(), &asset)
                    .map_err(|error| {
                        native_error("Could not open the video decoder", Some(error))
                    })?;
                let writer = AVAssetWriter::initWithURL_fileType_error(
                    AVAssetWriter::alloc(),
                    &destination_url,
                    AVFileTypeMPEG4.unwrap(),
                )
                .map_err(|error| {
                    native_error(
                        "Could not create the MP4. Check available disk space",
                        Some(error),
                    )
                })?;
                let session = Session { reader, writer };
                let pixel_key = NSString::from_str(&kCVPixelBufferPixelFormatTypeKey.to_string());
                let video_decode = dictionary(&[(
                    &pixel_key,
                    &*NSNumber::new_u32(kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange),
                )]);
                let video_output = AVAssetReaderTrackOutput::initWithTrack_outputSettings(
                    AVAssetReaderTrackOutput::alloc(),
                    &track,
                    Some(&video_decode),
                );
                video_output.setAlwaysCopiesSampleData(false);
                ensure!(
                    session.reader.canAddOutput(&video_output),
                    "macOS cannot decode this video format. Export H.264 MP4 first."
                );
                session.reader.addOutput(&video_output);
                let audio_output = if audios.len() == 1 {
                    let settings = dictionary(&[
                        (
                            AVFormatIDKey.unwrap(),
                            &*NSNumber::new_u32(u32::from_be_bytes(*b"lpcm")),
                        ),
                        (AVSampleRateKey.unwrap(), &*NSNumber::new_u32(48_000)),
                        (AVNumberOfChannelsKey.unwrap(), &*NSNumber::new_u32(1)),
                        (AVLinearPCMBitDepthKey.unwrap(), &*NSNumber::new_u32(16)),
                        (AVLinearPCMIsFloatKey.unwrap(), &*NSNumber::new_bool(false)),
                        (
                            AVLinearPCMIsBigEndianKey.unwrap(),
                            &*NSNumber::new_bool(false),
                        ),
                        (
                            AVLinearPCMIsNonInterleaved.unwrap(),
                            &*NSNumber::new_bool(false),
                        ),
                    ]);
                    let output = AVAssetReaderTrackOutput::initWithTrack_outputSettings(
                        AVAssetReaderTrackOutput::alloc(),
                        &audios[0],
                        Some(&settings),
                    );
                    output.setAlwaysCopiesSampleData(false);
                    ensure!(
                        session.reader.canAddOutput(&output),
                        "macOS cannot decode this audio track. Export the recording again with AAC audio."
                    );
                    session.reader.addOutput(&output);
                    Some(output)
                } else {
                    None
                };
                ensure!(
                    session.reader.startReading(),
                    "{}",
                    native_error("Could not start decoding", session.reader.error())
                );
                let mut first_frame = Some(
                    next_video_frame(&video_output, control)?
                        .context("No decodable video frames were found")?,
                );
                let buffer = first_frame
                    .as_ref()
                    .unwrap()
                    .image_buffer()
                    .context("macOS returned a video frame without pixels")?;
                let width = CVPixelBufferGetWidth(&buffer);
                let height = CVPixelBufferGetHeight(&buffer);
                ensure!(
                    width > 0 && height > 0 && width.is_multiple_of(2) && height.is_multiple_of(2),
                    "This video has unsupported pixel dimensions. Export an even-sized H.264 MP4 first."
                );

                // AVAssetWriter passes these documented settings to native
                // VideoToolbox: ABR targets bits/sec, FPS is a source hint, and
                // GOP caps both frame count and elapsed time. ABR is not a cap.
                let compression = dictionary(&[
                    (
                        AVVideoAverageBitRateKey.unwrap(),
                        &*NSNumber::new_u32(bitrate),
                    ),
                    (
                        AVVideoExpectedSourceFrameRateKey.unwrap(),
                        &*NSNumber::new_u32(30),
                    ),
                    (
                        AVVideoMaxKeyFrameIntervalKey.unwrap(),
                        &*NSNumber::new_u32(300),
                    ),
                    (
                        AVVideoMaxKeyFrameIntervalDurationKey.unwrap(),
                        &*NSNumber::new_u32(10),
                    ),
                    (
                        AVVideoProfileLevelKey.unwrap(),
                        AVVideoProfileLevelH264HighAutoLevel.unwrap(),
                    ),
                    (
                        AVVideoAllowFrameReorderingKey.unwrap(),
                        &*NSNumber::new_bool(false),
                    ),
                ]);
                // The encoder specification is TOP LEVEL, not a compression
                // property. Requiring hardware forbids a silent CPU fallback.
                let hardware_key = NSString::from_str(&objc2_video_toolbox::kVTVideoEncoderSpecification_RequireHardwareAcceleratedVideoEncoder.to_string());
                let hardware = dictionary(&[(&hardware_key, &*NSNumber::new_bool(true))]);
                let settings = dictionary(&[
                    (AVVideoCodecKey.unwrap(), AVVideoCodecTypeH264.unwrap()),
                    (AVVideoWidthKey.unwrap(), &*NSNumber::new_usize(width)),
                    (AVVideoHeightKey.unwrap(), &*NSNumber::new_usize(height)),
                    (AVVideoCompressionPropertiesKey.unwrap(), &*compression),
                    (AVVideoEncoderSpecificationKey.unwrap(), &*hardware),
                ]);
                ensure!(
                    session.writer.canApplyOutputSettings_forMediaType(
                        Some(&settings),
                        AVMediaTypeVideo.unwrap()
                    ),
                    "Hardware H.264 encoding is unavailable for this video at its original resolution."
                );
                let video_input = AVAssetWriterInput::assetWriterInputWithMediaType_outputSettings(
                    AVMediaTypeVideo.unwrap(),
                    Some(&settings),
                );
                video_input.setExpectsMediaDataInRealTime(false);
                video_input.setTransform(track.preferredTransform());
                ensure!(
                    session.writer.canAddInput(&video_input),
                    "The MP4 writer cannot accept this video format."
                );
                session.writer.addInput(&video_input);
                let audio_input = if audio_output.is_some() {
                    let settings = dictionary(&[
                        (
                            AVFormatIDKey.unwrap(),
                            &*NSNumber::new_u32(u32::from_be_bytes(*b"aac ")),
                        ),
                        (AVEncoderBitRateKey.unwrap(), &*NSNumber::new_u32(64_000)),
                        (AVSampleRateKey.unwrap(), &*NSNumber::new_u32(48_000)),
                        (AVNumberOfChannelsKey.unwrap(), &*NSNumber::new_u32(1)),
                    ]);
                    ensure!(
                        session.writer.canApplyOutputSettings_forMediaType(
                            Some(&settings),
                            AVMediaTypeAudio.unwrap()
                        ),
                        "The native AAC audio encoder is unavailable."
                    );
                    let input = AVAssetWriterInput::assetWriterInputWithMediaType_outputSettings(
                        AVMediaTypeAudio.unwrap(),
                        Some(&settings),
                    );
                    input.setExpectsMediaDataInRealTime(false);
                    ensure!(
                        session.writer.canAddInput(&input),
                        "The MP4 writer cannot accept the audio track."
                    );
                    session.writer.addInput(&input);
                    Some(input)
                } else {
                    None
                };
                session.writer.setShouldOptimizeForNetworkUse(true);
                ensure!(
                    session.writer.startWriting(),
                    "{}",
                    native_error(
                        "Could not start hardware compression",
                        session.writer.error()
                    )
                );
                session.writer.startSessionAtSourceTime(CMTime::new(0, 1));
                let mut video_done = false;
                let mut audio_done = audio_input.is_none();
                let mut last_video = f64::NEG_INFINITY;
                let mut furthest = 0_f64;
                let mut reported = 0;
                let mut audio_samples = 0;
                while !video_done || !audio_done {
                    control.check()?;
                    ensure!(
                        Instant::now() < deadline,
                        "Compression took too long. Try a shorter recording."
                    );
                    ensure!(
                        session.writer.status() == AVAssetWriterStatus::Writing,
                        "{}",
                        native_error(
                            "MP4 encoding failed. Check available disk space",
                            session.writer.error()
                        )
                    );
                    let advanced = autoreleasepool(|_| -> Result<bool> {
                        let mut advanced = false;
                        if !video_done && video_input.isReadyForMoreMediaData() {
                            let sample = match first_frame.take() {
                                Some(sample) => Some(sample),
                                None => next_video_frame(&video_output, control)?,
                            };
                            if let Some(sample) = sample {
                                let time = sample.presentation_time_stamp().seconds();
                                ensure!(
                                    time.is_finite(),
                                    "A video frame has an invalid timestamp. Export the recording again."
                                );
                                let pixels = sample
                                    .image_buffer()
                                    .context("A decoded video frame has no pixels")?;
                                ensure!(
                                    CVPixelBufferGetWidth(&pixels) == width
                                        && CVPixelBufferGetHeight(&pixels) == height,
                                    "Video dimensions changed during compression. Export a fixed-size recording first."
                                );
                                // Explicitly cap input frames to about 30 fps, keeping
                                // original PTS so audio and video never drift.
                                if time - last_video >= 1. / 30. - 0.000_001 {
                                    ensure!(
                                        video_input.appendSampleBuffer(&sample),
                                        "{}",
                                        native_error(
                                            "Hardware video encoding failed",
                                            session.writer.error()
                                        )
                                    );
                                    last_video = time;
                                }
                                furthest = furthest.max(time);
                            } else {
                                ensure!(
                                    session.reader.status() != AVAssetReaderStatus::Failed,
                                    "{}",
                                    native_error("Video decoding failed", session.reader.error())
                                );
                                video_done = true;
                                video_input.markAsFinished();
                            }
                            advanced = true;
                        }
                        if !audio_done
                            && let (Some(input), Some(output)) = (&audio_input, &audio_output)
                            && input.isReadyForMoreMediaData()
                        {
                            if let Some(sample) = output.copyNextSampleBuffer() {
                                if sample.num_samples() == 0 {
                                    return Ok(true);
                                }
                                audio_samples += 1;
                                furthest = furthest.max(sample.presentation_time_stamp().seconds());
                                ensure!(
                                    input.appendSampleBuffer(&sample),
                                    "{}",
                                    native_error(
                                        "AAC audio encoding failed",
                                        session.writer.error()
                                    )
                                );
                            } else {
                                ensure!(
                                    session.reader.status() != AVAssetReaderStatus::Failed,
                                    "{}",
                                    native_error("Audio decoding failed", session.reader.error())
                                );
                                audio_done = true;
                                input.markAsFinished();
                            }
                            advanced = true;
                        }
                        Ok(advanced)
                    })?;
                    let percent = (furthest / duration * 100.).clamp(0., 99.) as u8;
                    if percent != reported {
                        progress(Progress { attempt, percent });
                        reported = percent;
                    }
                    if !advanced {
                        thread::sleep(Duration::from_millis(2));
                    }
                }
                ensure!(
                    audio_input.is_none() || audio_samples > 0,
                    "No readable audio samples were found. Export the recording again with AAC audio."
                );
                session.writer.endSessionAtSourceTime(asset.duration());
                let (sender, receiver) = mpsc::channel();
                let finished = RcBlock::new(move || {
                    let _ = sender.send(());
                });
                session.writer.finishWritingWithCompletionHandler(&finished);
                wait(&receiver, control, deadline)?;
                ensure!(
                    session.writer.status() == AVAssetWriterStatus::Completed,
                    "{}",
                    native_error(
                        "Could not finalize the MP4. Check available disk space",
                        session.writer.error()
                    )
                );
                verify_dimensions(&destination_url, width, height, audio_input.is_some())?;
                Ok(())
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::MAX_ATTACHMENT_BYTES;

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
            &CompressionControl::default(),
            |_| {},
        )
        .unwrap();
        assert!(result.len() < MAX_ATTACHMENT_BYTES);
        assert_eq!(result.mime_type, "video/mp4");
        assert!(result.name.ends_with("-compressed.mp4"));
        assert_eq!(std::fs::read(source).unwrap(), original);
    }

    #[test]
    fn canceled_job_never_opens_the_source() {
        let control = CompressionControl::default();
        control.cancel();
        let error = compress(
            Path::new("/does-not-exist.mp4"),
            VideoUploadLimit::Standard,
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
        let error = compress(&source, VideoUploadLimit::Standard, &control, |progress| {
            if progress.percent > 0 {
                control.cancel();
            }
        })
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

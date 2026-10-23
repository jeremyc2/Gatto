#![deny(unsafe_code)]
//! The orchestration is safe Rust; bindings isolate the worker-owned codec calls.
use super::*;
use block2::RcBlock;
use objc2::{
    AnyThread as _,
    rc::{Retained, autoreleasepool},
    runtime::AnyObject,
};
use objc2_av_foundation::*;
use objc2_avf_audio::*;
use objc2_core_media::CMTime;
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
#[allow(unsafe_code)]
#[path = "bindings.rs"]
mod bindings;
#[allow(unsafe_code)]
#[path = "settings.rs"]
mod settings;
use bindings::{Asset, Frame, Media, Output, Reader, Session, is_hdr, seconds};
use settings::*;

fn dictionary(values: &[(&NSString, &AnyObject)]) -> Retained<NSDictionary<NSString, AnyObject>> {
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
fn next_video_frame(output: &Output, control: &CompressionControl) -> Result<Option<Frame>> {
    loop {
        control.check()?;
        let Some(sample) = output.next() else {
            return Ok(None);
        };
        if sample.count() > 0 {
            return Ok(Some(sample));
        }
    }
}
fn verify_dimensions(url: &NSURL, width: usize, height: usize, audio: bool) -> Result<()> {
    // Verify a decoded frame; an encoder must not silently lower the resolution.
    let asset = Asset::open(url, false);
    let (videos, audios) = asset.tracks();
    ensure!(
        videos.len() == 1 && audios.len() == usize::from(audio),
        "The finished MP4 is missing a video or audio track."
    );
    let reader = Reader::new(&asset, "Could not verify the finished MP4")?;
    let output = reader.output(
        &videos[0],
        &video_decode_settings(),
        false,
        "The compressed MP4 cannot be decoded.",
    )?;
    reader.start("The compressed MP4 cannot be read.", false)?;
    let sample = next_video_frame(&output, &CompressionControl::default())?
        .context("The compressed MP4 has no readable picture")?;
    let dimensions = sample
        .dimensions()
        .context("The compressed MP4 has no readable pixels")?;
    ensure!(
        dimensions == (width, height),
        "The hardware encoder could not preserve this video's original pixel dimensions."
    );
    Ok(())
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
        let source_url = NSURL::fileURLWithPath(&NSString::from_str(source));
        let destination_url = NSURL::fileURLWithPath(&NSString::from_str(destination));
        let asset = Asset::open(&source_url, true);
        asset.load(control, deadline)?;
        let duration = seconds(asset.duration());
        let (videos, audios) = asset.tracks();
        ensure!(
            videos.len() == 1 && audios.len() <= 1,
            "Compression requires one video track and at most one audio track. Export a flattened screen recording first."
        );
        let bitrate = video_bitrate(duration, audios.len() == 1, attempt, limit)?;
        let track = &videos[0];
        ensure!(
            !is_hdr(track),
            "HDR recordings cannot be compressed by this option. Export an SDR MP4 first."
        );
        let session = Session::new(&asset, &destination_url)?;
        let video_output = session.reader.output(
            track,
            &video_decode_settings(),
            true,
            "macOS cannot decode this video format. Export H.264 MP4 first.",
        )?;
        let audio_output = audios
            .first()
            .map(|track| {
                session.reader.output(track, &audio_decode_settings(), true,
            "macOS cannot decode this audio track. Export the recording again with AAC audio.")
            })
            .transpose()?;
        session.reader.start("Could not start decoding", true)?;
        let mut first_frame = Some(
            next_video_frame(&video_output, control)?
                .context("No decodable video frames were found")?,
        );
        let (width, height) = first_frame
            .as_ref()
            .unwrap()
            .dimensions()
            .context("macOS returned a video frame without pixels")?;
        ensure!(
            width > 0 && height > 0 && width.is_multiple_of(2) && height.is_multiple_of(2),
            "This video has unsupported pixel dimensions. Export an even-sized H.264 MP4 first."
        );
        let video_input = session.writer.input(
            Media::Video,
            &video_encode_settings(bitrate, width, height),
            Some(track),
            "Hardware H.264 encoding is unavailable for this video at its original resolution.",
            "The MP4 writer cannot accept this video format.",
        )?;
        let audio_input = audio_output
            .as_ref()
            .map(|_| {
                session.writer.input(
                    Media::Audio,
                    &audio_encode_settings(),
                    None,
                    "The native AAC audio encoder is unavailable.",
                    "The MP4 writer cannot accept the audio track.",
                )
            })
            .transpose()?;
        session.writer.start()?;
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
            session.writer.check()?;
            let advanced = autoreleasepool(|_| -> Result<bool> {
                let mut advanced = false;
                if !video_done && video_input.ready() {
                    let sample = match first_frame.take() {
                        Some(sample) => Some(sample),
                        None => next_video_frame(&video_output, control)?,
                    };
                    if let Some(sample) = sample {
                        let time = sample.time();
                        ensure!(
                            time.is_finite(),
                            "A video frame has an invalid timestamp. Export the recording again."
                        );
                        let dimensions = sample
                            .dimensions()
                            .context("A decoded video frame has no pixels")?;
                        ensure!(
                            dimensions == (width, height),
                            "Video dimensions changed during compression. Export a fixed-size recording first."
                        );
                        // Keep original timestamps while capping input to about 30 fps.
                        if time - last_video >= 1. / 30. - 0.000_001 {
                            session.writer.append(
                                &video_input,
                                &sample,
                                "Hardware video encoding failed",
                            )?;
                            last_video = time;
                        }
                        furthest = furthest.max(time);
                    } else {
                        session.reader.check("Video decoding failed")?;
                        video_done = true;
                        video_input.finish();
                    }
                    advanced = true;
                }
                if !audio_done
                    && let (Some(input), Some(output)) = (&audio_input, &audio_output)
                    && input.ready()
                {
                    if let Some(sample) = output.next() {
                        if sample.count() == 0 {
                            return Ok(true);
                        }
                        audio_samples += 1;
                        furthest = furthest.max(sample.time());
                        session
                            .writer
                            .append(input, &sample, "AAC audio encoding failed")?;
                    } else {
                        session.reader.check("Audio decoding failed")?;
                        audio_done = true;
                        input.finish();
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
        session.writer.finish(asset.duration(), control, deadline)?;
        verify_dimensions(&destination_url, width, height, audio_input.is_some())?;
        Ok(())
    })
}

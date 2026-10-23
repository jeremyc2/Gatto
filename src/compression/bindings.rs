//! Worker-local, owning media operations. Native objects never leave this module
//! or its calling worker; only paths, progress and cancellation cross to the UI.
use super::*;
use objc2_core_media::CMSampleBuffer;
use std::{marker::PhantomData, rc::Rc};

type Settings = NSDictionary<NSString, AnyObject>;

/// A retained framework object whose access and destruction stay on the creating
/// worker. This remains !Send/!Sync even if the generated framework type is Send.
struct WorkerObject<T: objc2::Message> {
    object: Retained<T>,
    _worker: PhantomData<Rc<()>>,
}
impl<T: objc2::Message> From<Retained<T>> for WorkerObject<T> {
    fn from(object: Retained<T>) -> Self {
        Self {
            object,
            _worker: PhantomData,
        }
    }
}
impl<T: objc2::Message> std::ops::Deref for WorkerObject<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.object
    }
}

pub(super) struct Asset(WorkerObject<AVURLAsset>);
impl Asset {
    pub(super) fn open(url: &NSURL, precise: bool) -> Self {
        // SAFETY: URL is retained by AVURLAsset; the documented option is an
        // NSNumber boolean. Constants exist on the supported macOS versions.
        let asset = unsafe {
            let options = dictionary(&[(
                AVURLAssetPreferPreciseDurationAndTimingKey,
                &*NSNumber::new_bool(true),
            )]);
            AVURLAsset::initWithURL_options(AVURLAsset::alloc(), url, precise.then_some(&options))
        };
        Self(asset.into())
    }
    pub(super) fn load(&self, control: &CompressionControl, deadline: Instant) -> Result<()> {
        let (sender, receiver) = mpsc::channel();
        let loaded = RcBlock::new(move || {
            let _ = sender.send(());
        });
        let keys = NSArray::from_slice(&[
            &*NSString::from_str("tracks"),
            &*NSString::from_str("duration"),
            &*NSString::from_str("readable"),
        ]);
        // SAFETY: The block captures its sender by value, so it remains valid
        // even when cancellation/deadline returns before native loading ends.
        unsafe {
            self.0
                .loadValuesAsynchronouslyForKeys_completionHandler(&keys, Some(&loaded));
        }
        if let Err(error) = wait(&receiver, control, deadline) {
            // SAFETY: Loading and cancellation are controlled by this worker.
            unsafe {
                self.0.cancelLoading();
            }
            return Err(error);
        }
        // SAFETY: Read only after the native load completion; no concurrent mutation.
        let readable = unsafe { self.0.isReadable() && !self.0.hasProtectedContent() };
        ensure!(
            readable,
            "macOS cannot decode this video for compression. Export it as an unprotected H.264 MP4 and try again."
        );
        Ok(())
    }
    pub(super) fn tracks(&self) -> (Vec<Retained<AVAssetTrack>>, Vec<Retained<AVAssetTrack>>) {
        // SAFETY: Synchronous queries on our immutable asset. Every returned
        // track is retained independently of the temporary native array.
        unsafe {
            let tracks = self.0.tracks();
            let videos = tracks
                .iter()
                .filter(|track| &*track.mediaType() == AVMediaTypeVideo.unwrap())
                .collect();
            let audios = tracks
                .iter()
                .filter(|track| &*track.mediaType() == AVMediaTypeAudio.unwrap())
                .collect();
            (videos, audios)
        }
    }
    pub(super) fn duration(&self) -> CMTime {
        // SAFETY: Immutable value query on the worker-owned, loaded asset.
        unsafe { self.0.duration() }
    }
}
pub(super) fn seconds(time: CMTime) -> f64 {
    // SAFETY: CMTime is passed by value; native conversion has no pointer lifetime.
    unsafe { time.seconds() }
}
pub(super) fn is_hdr(track: &AVAssetTrack) -> bool {
    // SAFETY: Immutable track query and a system constant on supported macOS.
    unsafe { track.hasMediaCharacteristic(AVMediaCharacteristicContainsHDRVideo.unwrap()) }
}

pub(super) struct Reader(WorkerObject<AVAssetReader>);
impl Reader {
    pub(super) fn new(asset: &Asset, message: &str) -> Result<Self> {
        // SAFETY: The reader retains its immutable asset. Only this worker uses it.
        unsafe { AVAssetReader::initWithAsset_error(AVAssetReader::alloc(), &asset.0) }
            .map(|object| Self(object.into()))
            .map_err(|error| native_error(message, Some(error)))
    }
    pub(super) fn output(
        &self,
        track: &AVAssetTrack,
        settings: &Settings,
        zero_copy: bool,
        message: &str,
    ) -> Result<Output> {
        // SAFETY: Settings contain the documented PCM or pixel-buffer schema.
        let output = unsafe {
            AVAssetReaderTrackOutput::initWithTrack_outputSettings(
                AVAssetReaderTrackOutput::alloc(),
                track,
                Some(settings),
            )
        };
        // SAFETY: Configured before reading. Owned sample wrappers retain each
        // native buffer while it is inspected/appended, including zero-copy data.
        let accepted = unsafe {
            if zero_copy {
                output.setAlwaysCopiesSampleData(false);
            }
            self.0.canAddOutput(&output)
        };
        ensure!(accepted, "{message}");
        // SAFETY: canAddOutput succeeded; add exactly once before startReading.
        unsafe {
            self.0.addOutput(&output);
        }
        Ok(Output(output.into()))
    }
    pub(super) fn start(&self, message: &str, details: bool) -> Result<()> {
        // SAFETY: All outputs are installed, and this worker starts reading once.
        if !unsafe { self.0.startReading() } {
            if details {
                // SAFETY: Immutable error query on this owned reader.
                return Err(native_error(message, unsafe { self.0.error() }));
            }
            bail!("{message}");
        }
        Ok(())
    }
    pub(super) fn check(&self, message: &str) -> Result<()> {
        // SAFETY: Only this worker reads, cancels or queries the reader.
        if unsafe { self.0.status() } == AVAssetReaderStatus::Failed {
            // SAFETY: Immutable error query on this owned reader.
            return Err(native_error(message, unsafe { self.0.error() }));
        }
        Ok(())
    }
}
impl Drop for Reader {
    fn drop(&mut self) {
        // SAFETY: No read operation can outlive this worker-local owner.
        unsafe {
            if self.0.status() == AVAssetReaderStatus::Reading {
                self.0.cancelReading();
            }
        }
    }
}
pub(super) struct Output(WorkerObject<AVAssetReaderTrackOutput>);
impl Output {
    pub(super) fn next(&self) -> Option<Frame> {
        // SAFETY: Called serially after startReading, never during cancellation.
        unsafe { self.0.copyNextSampleBuffer() }.map(|frame| Frame(frame.into()))
    }
}
pub(super) struct Frame(WorkerObject<CMSampleBuffer>);
impl Frame {
    pub(super) fn count(&self) -> isize {
        // SAFETY: The owning sample is never invalidated or mutated by Rust.
        unsafe { self.0.num_samples() }
    }
    pub(super) fn time(&self) -> f64 {
        // SAFETY: The retained sample owns its timestamp; conversion is by value.
        unsafe { self.0.presentation_time_stamp().seconds() }
    }
    pub(super) fn dimensions(&self) -> Option<(usize, usize)> {
        // SAFETY: Retain the image buffer for both immutable dimension queries.
        let pixels = unsafe { self.0.image_buffer() }?;
        Some((
            CVPixelBufferGetWidth(&pixels),
            CVPixelBufferGetHeight(&pixels),
        ))
    }
}

pub(super) enum Media {
    Video,
    Audio,
}
impl Media {
    fn native(&self) -> &'static NSString {
        // SAFETY: Framework string constants exist on all supported OS versions.
        unsafe {
            match self {
                Self::Video => AVMediaTypeVideo.unwrap(),
                Self::Audio => AVMediaTypeAudio.unwrap(),
            }
        }
    }
}
pub(super) struct Writer(WorkerObject<AVAssetWriter>);
impl Writer {
    pub(super) fn new(url: &NSURL) -> Result<Self> {
        // SAFETY: A fresh private destination and the documented MP4 file type.
        unsafe {
            AVAssetWriter::initWithURL_fileType_error(
                AVAssetWriter::alloc(),
                url,
                AVFileTypeMPEG4.unwrap(),
            )
        }
        .map(|object| Self(object.into()))
        .map_err(|error| {
            native_error(
                "Could not create the MP4. Check available disk space",
                Some(error),
            )
        })
    }
    pub(super) fn input(
        &self,
        media: Media,
        settings: &Settings,
        track: Option<&AVAssetTrack>,
        unavailable: &str,
        rejected: &str,
    ) -> Result<Input> {
        // SAFETY: Dictionary types match the documented H.264 or AAC schema.
        let accepted = unsafe {
            self.0
                .canApplyOutputSettings_forMediaType(Some(settings), media.native())
        };
        ensure!(accepted, "{unavailable}");
        // SAFETY: Writer accepted these settings. Configure before writing and
        // preserve the source track's by-value transform, when present.
        let input = unsafe {
            let input = AVAssetWriterInput::assetWriterInputWithMediaType_outputSettings(
                media.native(),
                Some(settings),
            );
            input.setExpectsMediaDataInRealTime(false);
            if let Some(track) = track {
                input.setTransform(track.preferredTransform());
            }
            input
        };
        // SAFETY: Only this worker configures the still-unstarted writer.
        ensure!(unsafe { self.0.canAddInput(&input) }, "{rejected}");
        // SAFETY: canAddInput succeeded; add exactly once before startWriting.
        unsafe {
            self.0.addInput(&input);
        }
        Ok(Input(input.into()))
    }
    pub(super) fn start(&self) -> Result<()> {
        // SAFETY: All inputs are configured. No append occurs before this start.
        let started = unsafe {
            self.0.setShouldOptimizeForNetworkUse(true);
            self.0.startWriting()
        };
        ensure!(
            started,
            "{}",
            self.error("Could not start hardware compression")
        );
        // SAFETY: Valid zero origin; matches the source timestamp-preserving loop.
        unsafe {
            self.0.startSessionAtSourceTime(CMTime::new(0, 1));
        }
        Ok(())
    }
    fn error(&self, message: &str) -> anyhow::Error {
        // SAFETY: Immutable query on our worker-owned writer.
        native_error(message, unsafe { self.0.error() })
    }
    pub(super) fn check(&self) -> Result<()> {
        // SAFETY: Queries and writes run serially on the owning worker.
        ensure!(
            unsafe { self.0.status() } == AVAssetWriterStatus::Writing,
            "{}",
            self.error("MP4 encoding failed. Check available disk space")
        );
        Ok(())
    }
    pub(super) fn append(&self, input: &Input, frame: &Frame, message: &str) -> Result<()> {
        // SAFETY: Caller checks readiness; the frame is retained for the entire
        // append. Only this worker appends and it never mutates sample storage.
        ensure!(
            unsafe { input.0.appendSampleBuffer(&frame.0) },
            "{}",
            self.error(message)
        );
        Ok(())
    }
    pub(super) fn finish(
        &self,
        end: CMTime,
        control: &CompressionControl,
        deadline: Instant,
    ) -> Result<()> {
        // SAFETY: All inputs are marked finished and appends have stopped.
        unsafe {
            self.0.endSessionAtSourceTime(end);
        }
        let (sender, receiver) = mpsc::channel();
        let finished = RcBlock::new(move || {
            let _ = sender.send(());
        });
        // SAFETY: The escaping block owns its sender even if wait is canceled.
        unsafe {
            self.0.finishWritingWithCompletionHandler(&finished);
        }
        wait(&receiver, control, deadline)?;
        // SAFETY: Completion has run; immutable status/error queries are serial.
        ensure!(
            unsafe { self.0.status() } == AVAssetWriterStatus::Completed,
            "{}",
            self.error("Could not finalize the MP4. Check available disk space")
        );
        Ok(())
    }
}
impl Drop for Writer {
    fn drop(&mut self) {
        // SAFETY: No append can outlive this worker-local owner. Completion blocks
        // own their channel senders, independent of an early return here.
        unsafe {
            if matches!(
                self.0.status(),
                AVAssetWriterStatus::Unknown | AVAssetWriterStatus::Writing
            ) {
                self.0.cancelWriting();
            }
        }
    }
}
pub(super) struct Input(WorkerObject<AVAssetWriterInput>);
impl Input {
    pub(super) fn ready(&self) -> bool {
        // SAFETY: Queried only on the owning worker, between serial appends.
        unsafe { self.0.isReadyForMoreMediaData() }
    }
    pub(super) fn finish(&self) {
        // SAFETY: Caller reached EOF and will not append to this input again.
        unsafe {
            self.0.markAsFinished();
        }
    }
}

/// Construction and use stay on one worker, enforced even when framework types
/// themselves permit Send/Sync. Field Drop cancels every partially built session.
pub(super) struct Session {
    pub(super) reader: Reader,
    pub(super) writer: Writer,
}
impl Session {
    pub(super) fn new(asset: &Asset, url: &NSURL) -> Result<Self> {
        let reader = Reader::new(asset, "Could not open the video decoder")?;
        let writer = Writer::new(url)?;
        Ok(Self { reader, writer })
    }
}

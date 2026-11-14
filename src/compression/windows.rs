//! The Windows preset supports H.264 MP4 with optional mono or stereo AAC.
//! The source reader and sink writer run synchronously on one worker and
//! exchange decoded samples without loading the entire recording into memory.
use super::*;
use ::windows::{
    Win32::{
        Foundation::{FreeLibrary, HMODULE},
        Media::MediaFoundation::*,
        System::{
            Com::{
                COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize,
                StructuredStorage::PropVariantToUInt64,
            },
            LibraryLoader::{GetProcAddress, LOAD_LIBRARY_SEARCH_SYSTEM32, LoadLibraryExW},
        },
    },
    core::{GUID, HRESULT, Interface as _, PCSTR, PCWSTR, s, w},
};
use std::os::windows::ffi::OsStrExt as _;

const VIDEO: u32 = MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32;
const AUDIO: u32 = MF_SOURCE_READER_FIRST_AUDIO_STREAM.0 as u32;
const ALL: u32 = MF_SOURCE_READER_ALL_STREAMS.0 as u32;
const ANY: u32 = MF_SOURCE_READER_ANY_STREAM.0 as u32;
const SOURCE: u32 = MF_SOURCE_READER_MEDIASOURCE.0 as u32;
const TICKS_PER_SECOND: f64 = 10_000_000.;
const AAC_BYTES_PER_SECOND: u32 = 12_000;
const UNSUPPORTED: &str = "Windows compression requires an H.264 MP4, with either no audio or mono/stereo AAC audio at 44.1 or 48 kHz.";

// Load these entry points only for compression, so Windows N can still use
// the rest of the app even without the optional Media Feature Pack.
type Raw = *mut std::ffi::c_void;

struct SystemLibrary(HMODULE);
impl SystemLibrary {
    fn open(name: PCWSTR) -> Result<Self> {
        // SAFETY: The file name is static; restrict lookup to trusted OS DLLs.
        unsafe { LoadLibraryExW(name, None, LOAD_LIBRARY_SEARCH_SYSTEM32) }
            .map(Self)
            .context("Windows Media Foundation is unavailable. On Windows N, install the Media Feature Pack in Settings → Optional features.")
    }
}
impl Drop for SystemLibrary {
    fn drop(&mut self) {
        // SAFETY: This object owns the LoadLibraryExW reference.
        let _ = unsafe { FreeLibrary(self.0) };
    }
}

struct Com;
impl Drop for Com {
    fn drop(&mut self) {
        // SAFETY: Balance CoInitializeEx on the same worker.
        unsafe { CoUninitialize() };
    }
}

struct MediaFoundation {
    create_attributes: unsafe extern "system" fn(*mut Raw, u32) -> HRESULT,
    create_type: unsafe extern "system" fn(*mut Raw) -> HRESULT,
    create_reader: unsafe extern "system" fn(PCWSTR, Raw, *mut Raw) -> HRESULT,
    create_writer: unsafe extern "system" fn(PCWSTR, Raw, Raw, *mut Raw) -> HRESULT,
    shutdown: unsafe extern "system" fn() -> HRESULT,
    _platform: SystemLibrary,
    _read_write: SystemLibrary,
    _com: Com,
}

impl MediaFoundation {
    fn start() -> Result<Self> {
        let platform = SystemLibrary::open(w!("mfplat.dll"))?;
        let read_write = SystemLibrary::open(w!("mfreadwrite.dll"))?;
        // SAFETY: CoInitializeEx has no reserved parameter. The guard balances
        // every successful call, including S_FALSE for an initialized thread.
        unsafe { CoInitializeEx(None, COINIT_MULTITHREADED).ok() }?;
        let com = Com;
        fn symbol(
            library: &SystemLibrary,
            name: PCSTR,
        ) -> Result<unsafe extern "system" fn() -> isize> {
            // SAFETY: This library is live and each name is a static export.
            unsafe { GetProcAddress(library.0, name) }
                .context("A required Windows Media Foundation component is unavailable")
        }
        // SAFETY: Each type matches its SDK export. Retain both DLLs until all
        // reader, writer and media-type objects have been dropped.
        unsafe {
            macro_rules! load {
                ($dll:expr, $name:literal, $ty:ty) => {
                    std::mem::transmute::<unsafe extern "system" fn() -> isize, $ty>(symbol(
                        $dll,
                        s!($name),
                    )?)
                };
            }
            let start = load!(
                &platform,
                "MFStartup",
                unsafe extern "system" fn(u32, u32) -> HRESULT
            );
            let create_attributes = load!(
                &platform,
                "MFCreateAttributes",
                unsafe extern "system" fn(*mut Raw, u32) -> HRESULT
            );
            let create_type = load!(
                &platform,
                "MFCreateMediaType",
                unsafe extern "system" fn(*mut Raw) -> HRESULT
            );
            let create_reader = load!(
                &read_write,
                "MFCreateSourceReaderFromURL",
                unsafe extern "system" fn(PCWSTR, Raw, *mut Raw) -> HRESULT
            );
            let create_writer = load!(
                &read_write,
                "MFCreateSinkWriterFromURL",
                unsafe extern "system" fn(PCWSTR, Raw, Raw, *mut Raw) -> HRESULT
            );
            let shutdown = load!(
                &platform,
                "MFShutdown",
                unsafe extern "system" fn() -> HRESULT
            );
            start(MF_VERSION, MFSTARTUP_FULL)
                .ok()
                .context("Could not start Windows Media Foundation")?;
            Ok(Self {
                create_attributes,
                create_type,
                create_reader,
                create_writer,
                shutdown,
                _com: com,
                _platform: platform,
                _read_write: read_write,
            })
        }
    }

    fn attributes(&self, size: u32) -> Result<IMFAttributes> {
        output(|ptr| {
            // SAFETY: The SDK function writes an owned IMFAttributes through ptr.
            unsafe { (self.create_attributes)(ptr, size) }
        })
    }

    fn media_type(&self, major: &GUID, subtype: &GUID) -> Result<IMFMediaType> {
        let format: IMFMediaType = output(|ptr| {
            // SAFETY: The SDK function writes an owned IMFMediaType through ptr.
            unsafe { (self.create_type)(ptr) }
        })?;
        // SAFETY: The media type copies both GUID values.
        unsafe {
            format.SetGUID(&MF_MT_MAJOR_TYPE, major)?;
            format.SetGUID(&MF_MT_SUBTYPE, subtype)?;
        }
        Ok(format)
    }

    fn reader(&self, path: &Path) -> Result<IMFSourceReader> {
        let path: Vec<_> = path.as_os_str().encode_wide().chain([0]).collect();
        let attrs = self.attributes(2)?;
        // SAFETY: Filesystem path and attributes remain live through construction.
        unsafe {
            attrs.SetUINT32(&MF_READWRITE_ENABLE_HARDWARE_TRANSFORMS, 0)?;
            attrs.SetUINT32(&MF_SOURCE_READER_DISABLE_DXVA, 1)?;
            output(|ptr| (self.create_reader)(PCWSTR(path.as_ptr()), attrs.as_raw(), ptr))
                .context(UNSUPPORTED)
        }
    }

    fn writer(&self, path: &Path) -> Result<IMFSinkWriter> {
        let path: Vec<_> = path.as_os_str().encode_wide().chain([0]).collect();
        let attrs = self.attributes(2)?;
        // SAFETY: Filesystem path and attributes remain live through construction.
        unsafe {
            attrs.SetUINT32(&MF_READWRITE_ENABLE_HARDWARE_TRANSFORMS, 0)?;
            attrs.SetGUID(&MF_TRANSCODE_CONTAINERTYPE, &MFTranscodeContainerType_MPEG4)?;
            output(|ptr| {
                (self.create_writer)(
                    PCWSTR(path.as_ptr()),
                    std::ptr::null_mut(),
                    attrs.as_raw(),
                    ptr,
                )
            })
            .context("Could not open the compressed MP4")
        }
    }
}

impl Drop for MediaFoundation {
    fn drop(&mut self) {
        // SAFETY: All reader/writer objects are dropped before this scope ends.
        let _ = unsafe { (self.shutdown)() };
    }
}

fn output<T: ::windows::core::Interface>(write: impl FnOnce(*mut Raw) -> HRESULT) -> Result<T> {
    let mut raw = std::ptr::null_mut();
    write(&mut raw).ok()?;
    ensure!(!raw.is_null(), "Windows did not return a media object");
    // SAFETY: The caller uses an SDK function returning an owned interface of T.
    Ok(unsafe { T::from_raw(raw) })
}

fn duration(reader: &IMFSourceReader) -> Result<u64> {
    // SAFETY: Media Foundation owns these attributes and returns an owned value.
    unsafe {
        let value = reader.GetPresentationAttribute(SOURCE, &MF_PD_DURATION)?;
        Ok(PropVariantToUInt64(&value)?)
    }
}

pub(super) fn transcode(
    source: &Path,
    output: &Path,
    control: &CompressionControl,
    attempt: u8,
    progress: &impl Fn(Progress),
    limit: VideoUploadLimit,
) -> Result<()> {
    ensure!(
        source
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("mp4")),
        "{UNSUPPORTED}"
    );
    let media = MediaFoundation::start()?;
    let reader = media.reader(source)?;
    let ticks = duration(&reader)?;

    // SAFETY: COM is initialized on this worker, all interfaces are owned, and
    // every out parameter is valid until its call returns.
    unsafe {
        let video = reader.GetNativeMediaType(VIDEO, 0).context(UNSUPPORTED)?;
        ensure!(
            video.GetGUID(&MF_MT_SUBTYPE)? == MFVideoFormat_H264,
            "{UNSUPPORTED}"
        );
        ensure!(
            video.GetUINT32(&MF_MT_VIDEO_ROTATION).unwrap_or(0) == 0,
            "Windows compression does not support rotation metadata. Export the video upright and try again."
        );
        let frame = video.GetUINT64(&MF_MT_FRAME_SIZE)?;
        ensure!(
            (frame >> 32).is_multiple_of(2) && (frame as u32).is_multiple_of(2),
            "Windows H.264 compression requires even video dimensions."
        );
        let audio = match reader.GetNativeMediaType(AUDIO, 0) {
            Ok(format) => {
                let rate = format.GetUINT32(&MF_MT_AUDIO_SAMPLES_PER_SECOND)?;
                let channels = format.GetUINT32(&MF_MT_AUDIO_NUM_CHANNELS)?;
                ensure!(
                    format.GetGUID(&MF_MT_SUBTYPE)? == MFAudioFormat_AAC
                        && matches!(rate, 44_100 | 48_000)
                        && matches!(channels, 1 | 2),
                    "{UNSUPPORTED}"
                );
                Some((rate, channels))
            }
            Err(error) if error.code() == MF_E_INVALIDSTREAMNUMBER => None,
            Err(error) => return Err(error).context("Could not inspect the original audio stream"),
        };
        let bitrate = video_bitrate(
            ticks as f64 / TICKS_PER_SECOND,
            audio.is_some(),
            attempt,
            limit,
        )?;

        reader.SetStreamSelection(ALL, false)?;
        reader.SetStreamSelection(VIDEO, true)?;
        reader
            .SetCurrentMediaType(
                VIDEO,
                None,
                &media.media_type(&MFMediaType_Video, &MFVideoFormat_NV12)?,
            )
            .context("Could not decode the original H.264 video")?;
        let writer = media.writer(output)?;
        let encoded_video = media.media_type(&MFMediaType_Video, &MFVideoFormat_H264)?;
        encoded_video.SetUINT32(&MF_MT_AVG_BITRATE, bitrate)?;
        encoded_video.SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32)?;
        encoded_video.SetUINT64(&MF_MT_FRAME_SIZE, frame)?;
        encoded_video.SetUINT64(&MF_MT_FRAME_RATE, video.GetUINT64(&MF_MT_FRAME_RATE)?)?;
        encoded_video.SetUINT64(
            &MF_MT_PIXEL_ASPECT_RATIO,
            video
                .GetUINT64(&MF_MT_PIXEL_ASPECT_RATIO)
                .unwrap_or((1 << 32) | 1),
        )?;
        let video_output = writer.AddStream(&encoded_video)?;
        writer
            .SetInputMediaType(video_output, &reader.GetCurrentMediaType(VIDEO)?, None)
            .context("Could not configure the software H.264 encoder")?;

        let audio_output = if let Some((rate, channels)) = audio {
            let pcm = media.media_type(&MFMediaType_Audio, &MFAudioFormat_PCM)?;
            let aac = media.media_type(&MFMediaType_Audio, &MFAudioFormat_AAC)?;
            for format in [&pcm, &aac] {
                format.SetUINT32(&MF_MT_AUDIO_BITS_PER_SAMPLE, 16)?;
                format.SetUINT32(&MF_MT_AUDIO_NUM_CHANNELS, channels)?;
                format.SetUINT32(&MF_MT_AUDIO_SAMPLES_PER_SECOND, rate)?;
            }
            pcm.SetUINT32(&MF_MT_AUDIO_BLOCK_ALIGNMENT, channels * 2)?;
            pcm.SetUINT32(&MF_MT_AUDIO_AVG_BYTES_PER_SECOND, rate * channels * 2)?;
            aac.SetUINT32(&MF_MT_AUDIO_AVG_BYTES_PER_SECOND, AAC_BYTES_PER_SECOND)?;
            reader.SetStreamSelection(AUDIO, true)?;
            reader
                .SetCurrentMediaType(AUDIO, None, &pcm)
                .context("Could not decode the original AAC audio")?;
            let index = writer.AddStream(&aac)?;
            writer
                .SetInputMediaType(index, &reader.GetCurrentMediaType(AUDIO)?, None)
                .context("Could not configure the AAC encoder")?;
            Some(index)
        } else {
            None
        };
        writer
            .BeginWriting()
            .context("Could not start Windows MP4 compression")?;

        let mut video_stream = Some(stream_index(&reader, VIDEO)?);
        let mut audio_stream = audio_output
            .map(|_| stream_index(&reader, AUDIO))
            .transpose()?;
        let mut last_percent = 0;
        while video_stream.is_some() || audio_stream.is_some() {
            control.check()?;
            let (mut index, mut flags, mut time, mut sample) = (0, 0, 0, None);
            reader
                .ReadSample(
                    ANY,
                    0,
                    Some(&mut index),
                    Some(&mut flags),
                    Some(&mut time),
                    Some(&mut sample),
                )
                .context("Could not decode the next MP4 sample")?;
            let has = |flag: MF_SOURCE_READER_FLAG| flags & flag.0 as u32 != 0;
            ensure!(
                !has(MF_SOURCE_READERF_ERROR),
                "Windows could not read this MP4."
            );
            let (stream, output) = if video_stream == Some(index) {
                (&mut video_stream, video_output)
            } else if audio_stream == Some(index) {
                (
                    &mut audio_stream,
                    audio_output.expect("audio stream requires an output"),
                )
            } else {
                bail!("Windows returned an unexpected MP4 stream.");
            };
            // A current type notification alone can also accompany the first
            // decoded samples. The sink verifies that subsequent samples fit.
            if let Some(sample) = sample {
                writer
                    .WriteSample(output, &sample)
                    .context("Could not encode an MP4 sample")?;
            } else if has(MF_SOURCE_READERF_STREAMTICK) {
                writer.SendStreamTick(output, time)?;
            }
            if has(MF_SOURCE_READERF_ENDOFSTREAM) {
                *stream = None;
                // Do not ask the source reader for this stream again.
                reader.SetStreamSelection(index, false)?;
            }
            let percent = (time.max(0) as f64 / ticks as f64 * 100.).min(99.) as u8;
            if percent > last_percent {
                progress(Progress { attempt, percent });
                last_percent = percent;
            }
        }
        control.check()?;
        writer
            .Finalize()
            .context("Could not finish the compressed MP4")?;
    }
    Ok(())
}

/// Source Reader pseudo-indices select a stream by type, while ReadSample
/// returns numeric indices. Read the native type of each numbered stream.
fn stream_index(reader: &IMFSourceReader, stream: u32) -> Result<u32> {
    let major = if stream == VIDEO {
        MFMediaType_Video
    } else {
        MFMediaType_Audio
    };
    // SAFETY: The reader and returned media types live on the initialized worker.
    unsafe {
        for index in 0.. {
            let media = match reader.GetNativeMediaType(index, 0) {
                Err(error) if error.code() == MF_E_INVALIDSTREAMNUMBER => break,
                result => result?,
            };
            if media.GetGUID(&MF_MT_MAJOR_TYPE)? == major {
                return Ok(index);
            }
        }
    }
    bail!("Could not find the selected MP4 stream")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compression_preserves_dimensions_audio_and_the_original_mp4() -> Result<()> {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample-audio.mp4");
        let original = std::fs::read(&fixture)?;
        let directory = tempfile::tempdir()?;
        let input = directory.path().join("original.mp4");
        let output = directory.path().join("compressed.mp4");
        std::fs::write(&input, &original)?;
        let compressed = compress(
            &input,
            VideoUploadLimit::Standard,
            CompressionMode::Quality,
            &CompressionControl::default(),
            |_| {},
        )?;
        assert_eq!(compressed.mime_type, "video/mp4");
        assert!(compressed.len() < VideoUploadLimit::Standard.bytes());
        std::io::copy(
            &mut compressed.reader()?,
            &mut std::fs::File::create(&output)?,
        )?;
        let media = MediaFoundation::start()?;
        let file = media.reader(&output)?;
        assert!((duration(&file)? as f64 / TICKS_PER_SECOND - 2.).abs() < 0.15);
        // SAFETY: Tests inspect the finalized file with the worker initialized.
        unsafe {
            let video = file.GetNativeMediaType(VIDEO, 0)?;
            let audio = file.GetNativeMediaType(AUDIO, 0)?;
            assert_eq!(video.GetGUID(&MF_MT_SUBTYPE)?, MFVideoFormat_H264);
            assert_eq!(video.GetUINT64(&MF_MT_FRAME_SIZE)?, (64 << 32) | 64);
            assert_eq!(audio.GetGUID(&MF_MT_SUBTYPE)?, MFAudioFormat_AAC);
        }
        assert_eq!(std::fs::read(input)?, original);
        Ok(())
    }

    #[test]
    fn canceling_during_media_foundation_compression_does_not_stage_output() {
        let input = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample.mp4");
        let control = CompressionControl::default();
        let error = compress(
            &input,
            VideoUploadLimit::Standard,
            CompressionMode::Quality,
            &control,
            |report| {
                if report.percent > 0 {
                    control.cancel();
                }
            },
        )
        .err()
        .expect("compression should be canceled");
        assert!(error.downcast_ref::<Canceled>().is_some(), "{error:#}");
    }
}

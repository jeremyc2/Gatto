# Native compression of imported videos

Researched 2026-10-19. Scope: offer **Cancel / Compress** for an imported video
that exceeds Gatto's 10 MB upload limit. Recording the screen or microphone is
outside this scope. Findings describe an implementation architecture, not a
tested encoder integration.

The implementation is now in [compression.rs](../../src/compression.rs); see
[current usage](../WALKTHROUGH.md#compress-a-larger-video) and
[architecture](../ARCHITECTURE.md) for the shipped policy and verification.
The findings below preserve the investigation that preceded implementation.

## Recommended native pipeline

Use `AVURLAsset` → `AVAssetReader` → decoded video / PCM samples →
`AVAssetWriter` → MP4 containing H.264 and AAC. AVFoundation performs encoding
through native frameworks; require hardware video encoding explicitly. This
uses macOS frameworks without an FFmpeg subprocess or a bundled software
encoder. Apple's reader/writer guide demonstrates this decode / reencode
pattern and separate audio/video pumps. [Apple reader/writer guide](https://developer.apple.com/library/archive/documentation/AudioVideo/Conceptual/AVFoundationPG/Articles/05_Export.html),
[AVAssetWriter](https://developer.apple.com/documentation/avfoundation/avassetwriter)

For this existing-file workflow, `screencapturekit` adds no capability. The
current `videotoolbox` wrapper is actively developed, rather than an abandoned
binding: 0.21.0 exposes a native encoder with bitrate, frame rate, GOP and
hardware requirements. Its documentation nevertheless labels it experimental.
The sibling `avassetwriter` 0.13.0 explicitly does **not** wrap `AVAssetReader`.
Using `objc2-av-foundation` 0.3.2 fits Gatto's existing `objc2` dependencies and
avoids introducing a second family of Core Media ownership wrappers. These are
thin framework bindings, not codec implementations.
[VideoToolbox wrapper](https://docs.rs/videotoolbox/0.21.0/videotoolbox/),
[writer wrapper](https://docs.rs/avassetwriter/0.13.0/avassetwriter/),
[typed AVFoundation bindings](https://docs.rs/objc2-av-foundation/0.3.2/objc2_av_foundation/)

Use narrowly selected AVFoundation features: `AVAsset`, `AVAssetTrack`,
`AVAssetReader`, `AVAssetReaderOutput`, `AVAssetWriter`, `AVAssetWriterInput`,
`AVAsynchronousKeyValueLoading`, `AVAudioSettings`, `AVVideoSettings`,
`AVMediaFormat`, `objc2-core-media`, `objc2-core-video`, `block2`, and `dispatch2`.
Additional directly used Core Media / Video types should use the same `objc2`
0.3 framework family. Enable the relevant features with default features off;
verify the final set by compiling against the locked crate release.
[AVFoundation feature definitions](https://docs.rs/crate/objc2-av-foundation/0.3.2/features)

## Encoding settings

The video input's **top-level** output dictionary contains:

- `AVVideoCodecKey`: `AVVideoCodecTypeH264`.
- `AVVideoWidthKey`, `AVVideoHeightKey`: the source's encoded pixel dimensions.
- `AVVideoEncoderSpecificationKey`: a dictionary containing
  `kVTVideoEncoderSpecification_RequireHardwareAcceleratedVideoEncoder: true`.
- `AVVideoCompressionPropertiesKey`: a dictionary containing the average
  bitrate, source frame rate hint, maximum GOP of 300 frames, maximum GOP
  duration of 10 seconds, and an auto H.264 profile level appropriate for the
  actual dimensions / frame rate.

Hardware-required creation fails when the device / configuration is unsupported
or hardware resources are unavailable. Treat this as a recoverable compression
error instead of silently falling back to CPU encoding.
[Encoder specification placement](https://developer.apple.com/documentation/avfoundation/avvideoencoderspecificationkey),
[hardware requirement](https://developer.apple.com/documentation/videotoolbox/kvtvideoencoderspecification_requirehardwareacceleratedvideoencoder)

The nested compression keys are `AVVideoAverageBitRateKey`,
`AVVideoExpectedSourceFrameRateKey`, `AVVideoMaxKeyFrameIntervalKey`,
`AVVideoMaxKeyFrameIntervalDurationKey`, `AVVideoProfileLevelKey`, and optionally
`AVVideoAllowFrameReorderingKey`. The frame rate is a hint, not a frame rate
conversion. Preserve source timestamps and durations. A 300-frame GOP does not
by itself mean ten seconds for variable-rate material, so set the duration key
too. Encoders may still emit additional keyframes.
[Compression properties](https://developer.apple.com/documentation/avfoundation/avvideocompressionpropertieskey),
[frame-rate hint](https://developer.apple.com/documentation/avfoundation/avvideoexpectedsourceframeratekey),
[keyframe semantics](https://developer.apple.com/documentation/videotoolbox/kvtcompressionpropertykey_maxkeyframeinterval)

Decode audio as linear PCM, then encode AAC using `AVFormatIDKey` =
`kAudioFormatMPEG4AAC`, `AVEncoderBitRateKey` = 64,000 bits/s,
`AVSampleRateKey` = 48,000 and `AVNumberOfChannelsKey` = 1 for spoken clips.
Explicitly choosing mono changes stereo content; the UI should communicate the
compression tradeoff. Preserve audio rather than dropping it on conversion
failure. No audio input is needed for a silent source.
[AAC writer settings](https://developer.apple.com/documentation/avfoundation/avassetwriterinput/assetwriterinputwithmediatype:outputsettings:),
[reader audio format](https://developer.apple.com/documentation/avfoundation/avassetreadertrackoutput)

## File size policy

Use decimal bytes consistently: success means a finished file is **strictly
less than 10,000,000 bytes**. At 60 seconds, a 1.2 Mbps video target consumes
9,000,000 bytes; 64 kbps audio consumes 480,000 bytes, leaving 520,000 bytes for
container overhead and rate-control variation. At 128 kbps, only 40,000 bytes
remain. This arithmetic is a budget, not an encoder guarantee.

Calculate the video bitrate from the actual duration, reserving container
headroom and audio first; for example, target 9,200,000 bytes total and compute
`video_bps = floor(8 * 9_200_000 / duration_seconds) - audio_bps`. At 60 seconds
with 64 kbps audio, that is about 1,162,666 video bits/s. Reject non-finite,
zero, or negative duration. Long clips can yield impractically low bitrates;
report that the clip cannot fit with the supported policy instead of claiming
crisp text is guaranteed.

Average bitrate is a target. Apple's `DataRateLimits` supplies sliding-window
hard limits for direct VideoToolbox sessions, with codec-dependent support;
it does not account for AAC or MP4 overhead. Do not invent an
`AVVideoDataRateLimitsKey`: explicit `VTSessionSetProperty` needs a direct VT
session rather than a documented AVAssetWriter setting.
[Apple rate-control explanation](https://developer.apple.com/library/archive/qa/qa1958/_index.html),
[DataRateLimits](https://developer.apple.com/documentation/videotoolbox/kvtcompressionpropertykey_dataratelimits)

After successful writer finalization, inspect the actual file length. Publish
the temporary MP4 to Gatto's upload queue only if it passes the byte gate.
If it overshoots, restart from the original at a conservatively reduced
bitrate, with a bounded retry count. Never repeatedly transcode the already
compressed attempt. Keep the original untouched and remove failed temporary
files. This acceptance gate guarantees that accepted outputs satisfy the
limit; it does not guarantee every source can be made acceptable. Native
resolution preserves pixel dimensions, not lossless text: complex movement,
long clips, 4:2:0 chroma and lossy H.264 can still reduce readability.

`AVAssetExportSession.fileLengthLimit` exists, but an export preset does not
provide the same direct control of hardware requirements, exact source
resolution and custom bitrate. Estimates are estimates; validate actual output
with either pipeline. [Export file limit](https://developer.apple.com/documentation/avfoundation/avassetexportsession/filelengthlimit),
[export estimates](<https://developer.apple.com/documentation/avfoundation/avassetexportsession/estimateoutputfilelength(completionhandler:)>)

## Reading, ownership, progress and cancellation

Concrete 0.3.2 Rust method names include:

- `AVAssetReader::initWithAsset_error`.
- `AVAssetReaderTrackOutput::initWithTrack_outputSettings`.
- `AVAssetReaderOutput::setAlwaysCopiesSampleData(false)` and
  `copyNextSampleBuffer() -> Option<Retained<CMSampleBuffer>>`.
- `AVAssetReader::{canAddOutput, addOutput, startReading, status, error,
cancelReading}`.
- `AVAssetWriterInput::{requestMediaDataWhenReadyOnQueue_usingBlock,
isReadyForMoreMediaData, appendSampleBuffer, markAsFinished}`.

The returned retained sample owns its native reference. Keep all native objects
on an owning worker / serial dispatch queue, use an autorelease pool around
sample processing, and avoid transferring raw borrowed pointers to the UI.
Do not mark arbitrary Objective-C objects `Send` / `Sync` merely to satisfy a
callback. [Reader bindings](https://docs.rs/objc2-av-foundation/0.3.2/src/objc2_av_foundation/generated/AVAssetReader.rs.html),
[output bindings](https://docs.rs/objc2-av-foundation/0.3.2/src/objc2_av_foundation/generated/AVAssetReaderOutput.rs.html),
[writer-input bindings](https://docs.rs/objc2-av-foundation/0.3.2/src/objc2_av_foundation/generated/AVAssetWriterInput.rs.html)

Set `expectsMediaDataInRealTime` to false. Pump each track only while its writer
input is ready; otherwise yield. At EOF inspect reader status to distinguish
completion from failure, finish that input, and finalize the writer after
both inputs finish. Preserve the shared source timeline, including gaps and
offsets, rather than synthesizing timestamps from frame counts. Copy the
source `preferredTransform` to the video writer input to preserve orientation
without rotating / scaling pixel buffers.
[Writer backpressure](https://developer.apple.com/documentation/avfoundation/avassetwriterinput),
[session timeline](<https://developer.apple.com/documentation/avfoundation/avassetwriter/startsession(atsourcetime:)>),
[orientation](https://developer.apple.com/library/archive/qa/qa1744/)

UI cancellation should set a shared cancellation flag and schedule teardown on
the owner queue. `cancelReading` must **not** run concurrently with
`copyNextSampleBuffer`. Cancel the writer and clear callbacks / temporary
outputs. Use an exactly-once terminal state so cancellation, native failure and
finalization cannot each enqueue the same result. Progress is the furthest
fully appended source presentation time divided by duration, clamped below
100% until finalization and byte validation; retries should display their
attempt rather than pretending progress never moves backward. These state and
UI policies are implementation recommendations; the native cancellation
constraint is documented in the reader bindings above.

## Input and error policy

Probe the asset and actual tracks before creating the writer. Validate a
readable video track, finite duration, supported dimensions / codec, and a
usable audio track when present. A filename extension does not establish
decoder support. In particular, GitHub accepting WebM does not establish
AVFoundation support on the user's macOS release; test readable tracks and
reader initialization and return a clear unsupported-input error.
[Runtime supported types](https://developer.apple.com/documentation/avfoundation/avurlasset/audiovisualcontenttypes),
[reader validation](https://developer.apple.com/documentation/avfoundation/avassetreader)

Reject or explicitly handle multiple video tracks, HDR / wide-gamut source
conversion, alpha, unsupported odd dimensions and track format changes.
Preserving pixel dimensions alone is insufficient to handle those cases
correctly. Distinguish unsupported media, decode failure, encoder unavailable,
disk full / write failure, cancellation, finalization failure and target size
failure. Compression reads a user-selected file; it needs no screen-capture
or microphone authorization.

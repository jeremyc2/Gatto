# Screen-recording codec compatibility

Investigated 2026-11-07 for Gatto's compression investigation. This is a
source-based compatibility assessment, not a claim that every listed device has
been tested. It complements [the native compression investigation](native-video-compression.md)
and [GitHub attachment research](github-video-attachments.md). Encoding benchmarks
must establish the quality, size, and processing-time tradeoffs separately.
The [benchmark record](screen-recording-codec-benchmarks.md) now includes those
measurements. The following app changes implement software Quality H.264 and
optional AV1, retaining the native encoder for Fast mode. Descriptions of the
original native-only implementation below refer to the investigation baseline.

## Findings relevant to a default

**AV1 is a credible choice for a Chrome-desktop audience, including M1/M2 Macs.**
Chrome can decode AV1 in software; AV1 hardware is an efficiency advantage rather
than a prerequisite for playback. **HEVC is the smaller change to Gatto's native
encoding pipeline, but its Chrome playback support remains conditional on the
platform decoder.** Recent hardware is encouraging, not a complete compatibility
rule. These conclusions are inferences from the first-party implementation and
platform evidence below, rather than population measurements.

GitHub accepts MP4, MOV, and WebM attachments, recommends H.264 for greatest
compatibility, and explicitly makes codec compatibility browser dependent. Its
container allowance does not certify every codec/profile combination inside
those containers. The documentation does not establish that GitHub produces
fallback renditions for uploaded AV1 or HEVC files. No attachment was uploaded as
part of this source research. [GitHub attachment documentation](https://docs.github.com/en/get-started/writing-on-github/working-with-advanced-formatting/attaching-files)

## Four different questions

1. Can Gatto **encode** the codec on the sender's Mac?
2. Will GitHub accept and serve the chosen **container and payload**?
3. Can the recipient's browser **decode and play** that payload?
4. Does that device have an efficient **hardware decoder**, rather than relying
   on software?

A positive result for one does not answer the others. In particular, the sender's
Mac cannot determine the hardware available to a future GitHub reader. A single
attachment URL supplies one media file; this is different from a publisher
providing several alternative `<source>` files for browser selection. WebKit's
examples explain how alternate sources and their codec identifiers work.
[WebKit source selection explanation](https://webkit.org/blog/14445/webkit-features-in-safari-17-0/)

## Expected playback matrix

Scope: current Google Chrome desktop, ordinary SDR screen recordings near 1080p
at 30 fps, and these distribution profiles: H.264 High 8-bit 4:2:0 in MP4/AAC;
HEVC Main/Main10 4:2:0 in MP4/AAC using `hvc1`; VP9 Profile 0 8-bit 4:2:0 in
WebM/Opus; AV1 Main 8/10-bit 4:2:0 in MP4/AAC or WebM/Opus. Different dimensions,
levels, chroma formats, audio tracks, driver blocklists, disabled acceleration,
and managed-browser policies can change the result. **Expected** means supported
by the cited architecture/platform evidence, not exercised on a device lab.

| Recipient | H.264 | HEVC | VP9 | AV1 | Hardware qualification |
| --- | --- | --- | --- | --- | --- |
| Chrome on Apple M1/M2 Mac | Expected | Expected through platform decoder | Expected | Expected, including software decode | Apple lists native H.264/HEVC media engines; do not require AV1 hardware for Chrome playback. |
| Chrome on Apple M3 Mac and later AV1-equipped models | Expected | Expected through platform decoder | Expected | Expected | M3 explicitly adds AV1 decode. Check the exact later model rather than extrapolating from its purchase date. |
| Chrome on supported Intel Mac | Expected | Conditional on platform/decoder support | Expected | Expected through software fallback | AV1 playback is independent of a hardware AV1 decoder; hardware acceleration remains model dependent. |
| Chrome on Windows with compatible recent Intel graphics | Expected | Conditional on usable GPU/driver decoder | Expected | Expected | Intel documents AV1 decode in specified 11th-generation integrated graphics and later listed parts. A CPU without active integrated graphics is a different case. |
| Chrome on Windows with compatible NVIDIA RTX 30/40/50 GPU | Expected | Conditional on usable GPU/driver decoder | Expected | Expected | Listed Ampere/Ada/Blackwell models have AV1 decoding; exact board, active GPU, and driver matter. |
| Chrome on Windows with compatible AMD graphics | Expected | Conditional on usable GPU/driver decoder | Expected | Expected | RX 6600/6700/6800 and listed newer parts decode AV1; RX 6500 is a counterexample to a blanket recent-GPU rule. |
| Chrome on Linux | Expected in Google Chrome | Conditional on browser build, video acceleration, and driver stack | Expected | Expected through built-in software fallback | Do not promise HEVC merely because the GPU specification lists it. |
| Chrome on recent Android | Expected | Device-dependent platform decode | Expected | Expected on current capable configurations | Android lists HEVC since 5.0, VP9 since 4.4, AV1 since 10; AV1 encoder/decoder become mandatory at Android 14. Performance still varies. |
| Safari / WebKit on current macOS and iOS | Expected | Expected for supported platform profiles | Expected for ordinary WebM profiles on current releases | Requires AV1-capable Apple hardware | Chrome on iOS using WebKit belongs to this platform category, not the desktop-Chrome software-decoder category. |

The desktop Chrome entries follow Chromium's official codec inventory and its
built-in decoder paths: H.264 with proprietary codecs enabled, VP9 through
libvpx, and AV1 through its enabled AV1 decoder. Chromium's configuration enables
dav1d on Blink builds. HEVC is an optional platform profile, rather than one of
those built-in software codecs. These source links are pinned to Chromium
commit `d42ef1f738aaf51d1b9e7b1925423c2d5358a6fc` from 2026-11-07; development
source corroborates architecture, not every stable binary's enabled features.
[Chromium codec inventory](https://www.chromium.org/audio-video/),
[built-in and optional decoder support](https://chromium.googlesource.com/chromium/src/+/d42ef1f738aaf51d1b9e7b1925423c2d5358a6fc/media/base/supported_types.cc),
[decoder build configuration](https://chromium.googlesource.com/chromium/src/+/d42ef1f738aaf51d1b9e7b1925423c2d5358a6fc/media/media_options.gni)

The HEVC Windows path queries GPU decoder GUIDs, supported resolutions, and
workarounds. Its direct D3D implementation means a generic statement that all
Chrome HEVC playback requires buying a Microsoft Store codec extension is not
established by this research. Check the running Chrome/GPU configuration.
[Chromium Windows decoder capability checks](https://chromium.googlesource.com/chromium/src/+/d42ef1f738aaf51d1b9e7b1925423c2d5358a6fc/media/gpu/windows/supported_profile_helpers.cc)

Apple's M2 specification lists hardware H.264/HEVC encoding and decoding; M3
additionally lists AV1 **decode**, without advertising AV1 encoding. Chromium's
Mac decoder checks VideoToolbox hardware capabilities for AV1 and VP9.
[Apple M2 specification](https://support.apple.com/en-us/111867),
[Apple M3 specification](https://support.apple.com/en-us/117735),
[Chromium Mac decoder](https://chromium.googlesource.com/chromium/src/+/d42ef1f738aaf51d1b9e7b1925423c2d5358a6fc/media/gpu/mac/video_toolbox_video_decoder.cc)

The Windows hardware examples are device specifications, not guarantees that a
browser selects those devices. Intel's documented 11th-generation UHD support
includes AVC, HEVC, VP9, and AV1. NVIDIA's decoder table identifies codec support
by board and distinguishes decode from encode. AMD's table explicitly excludes
AV1 on RX 6500 while listing it on RX 6600/6700/6800 and specified later GPUs.
[Intel support article](https://www.intel.com/content/www/us/en/support/articles/000092831/graphics.html),
[NVIDIA encode/decode matrix](https://developer.nvidia.com/video-encode-decode-support-matrix),
[AMD media engine tables](https://www.amd.com/en/products/graphics/radeon-for-creators/video-editing.html)

Android's table states decoder/API requirements, not a claim that all formats
have hardware acceleration. Safari's WebM support expanded to iOS/iPadOS 17.4;
WebKit had already supported WebM on macOS. Apple explicitly requires hardware
support for AV1 playback in its players. These Safari statements should not be
used to exclude AV1 software playback in Chrome desktop.
[Android media formats](https://developer.android.com/media/platform/supported-formats),
[WebKit Safari 17.4 formats](https://webkit.org/blog/15063/webkit-features-in-safari-17-4/),
[Apple AV1 playback requirement](https://developer.apple.com/streaming/examples/)

## Encoding and screen-content qualifications

The baseline (now Fast mode) uses AVAssetReader/AVAssetWriter with hardware-required H.264,
original decoded dimensions, 4:2:0 buffers, an approximately 30 fps input cap,
a 300-frame / ten-second maximum keyframe interval, and frame reordering
disabled. It budgets a selected upload size rather than promising constant
quality. These are implementation observations, not codec limitations.
[Current settings](../../src/compression/settings.rs),
[current orchestration](../../src/compression/native.rs),
[bitrate budget](../../src/compression.rs)

Apple exposes H.264 and HEVC as native AVFoundation codec constants.
`VTCopyVideoEncoderList` lists available encoders, including hardware status and
multipass support. Probe the actual machine before promising a native codec;
AV1 hardware **decoding** is not evidence of a native AV1 **encoder**. AV1/VP9
software results from a developer-installed FFmpeg are useful benchmarks but do
not establish a production integration. The app changes following these tests
add a separate, source-built software helper with static x264, SVT-AV1 and
libaom libraries; packaging and verification are recorded in the benchmark note.
[AVFoundation codec types](https://developer.apple.com/documentation/avfoundation/avvideocodectype),
[VideoToolbox encoder enumeration](https://developer.apple.com/documentation/videotoolbox/vtcopyvideoencoderlist(_:_:)),
[encoder list capabilities](https://developer.apple.com/documentation/videotoolbox/video-encoder-list-keys)

AV1 includes palette mode for blocks with few colors and intra-block copying for
repeated patterns in screen content. This makes it a motivated test candidate;
it does not prove that every AV1 encoder/preset uses those tools or wins on this
corpus. [AOMedia AV1 tool description, section 3.8](https://aomedia.org/docs/AV1_ToolDescription_v11-clean.pdf)

4:2:0 stores less color detail than brightness detail, so switching among common
4:2:0 codec profiles alone cannot preserve all thin colored edges. A 4:4:4
comparison can measure the difference, but it is a different compatibility
profile and must be tested independently. Neither a decoder's codec-name match
nor a GPU's 4:2:0 capability certifies 4:4:4. [Apple chroma subsampling explanation](https://developer.apple.com/documentation/accelerate/converting-chroma-subsampled-images)

H.264 also has quality experiments worth testing. Native multipass requires
replaying the nominated source segments, checking `canPerformMultiplePasses`
after writing starts, and respecting multipass backpressure; setting a Boolean
alone does not implement the workflow. The default is no multipass. Native
settings also expose frame reordering, expected frame rate, bitrate, and
keyframe interval. `AVVideoQualityKey` is documented for JPEG, so it should not
be presented as a supported H.264/HEVC quality slider.
[Apple multipass contract](https://developer.apple.com/documentation/avfoundation/avassetwriterinput/performsmultipassencodingifsupported),
[Apple video settings](https://developer.apple.com/documentation/avfoundation/video-settings)

## Evidence needed before changing the default

A default change should combine matched-size quality tests, first-frame and
scene-cut inspection, encoding time, native encoder availability, and playback
of the **actual files**. Use full codec strings when querying capability. Test
file playback, seek to later keyframes, confirm dimensions/duration/audio, and
record decoded/dropped frames. API capability reporting and hardware capability
are different signals. Chrome's MediaCapabilities explanation distinguishes
supported, smooth, and power-efficient results; optimistic results can precede
real playback history. [Chrome MediaCapabilities explanation](https://developer.chrome.com/blog/media-updates-in-chrome-63-64/)

Source research makes AV1 plausible for a Chrome-desktop-focused option and HEVC
plausible for an inexpensive native option. It does **not** establish a universal
winner or justify a device-specific support percentage. A short user-facing
comparison should say “current Chrome desktop,” “requires compatible platform
decoder,” or “older Apple players need H.264/HEVC,” rather than publish an exact
chip-year promise. Keep the full dated matrix in a help view or documentation;
a tooltip can link to that table instead of holding an unreadable hardware list.
This section records the UI recommendation from the investigation. The
[benchmark record](screen-recording-codec-benchmarks.md#preferences-and-implementation-boundary)
describes the implemented choices.

Before a new default is described as GitHub-tested, verify a real attachment in
an authorized repository and inspect its returned media type and playback in
Chrome on representative Mac/Windows devices. Neither GitHub's extension list
nor local Chrome playback alone establishes that end-to-end result.

## Additional native settings to measure

The macOS SDK headers explicitly permit VideoToolbox compression-property keys
inside `AVVideoCompressionPropertiesKey`; they also warn that individual keys
are encoder specific. Thus testing these settings through AVAssetWriter is
supported API usage, provided the selected encoder/mode accepts the property.
[Apple compression-property dictionary](https://developer.apple.com/documentation/avfoundation/avvideocompressionpropertieskey)

| Property | SDK availability | Contract and test implication |
| --- | --- | --- |
| `PrioritizeEncodingSpeedOverQuality = false` | macOS 11+ | Requests quality over speed at a given bitrate. Optional; current SDK documentation says H.264/HEVC hardware encoders already prioritize quality by default. A no-op is a plausible outcome. |
| `MaxAllowedFrameQP` | macOS 12+ | Optional, mode dependent. Unsupported modes return `kVTPropertyNotSupportedErr` (`-12900`). The encoder may discard frames to meet both quality and bitrate goals; inspect frame count and timestamp gaps. |
| VideoToolbox `Quality` | macOS 10.8+ | An optional 0–1 quality control, with format-dependent behavior that may use fixed QP. This is distinct from AVFoundation `AVVideoQualityKey`; it does not guarantee a byte limit. |
| `SuggestedLookAheadFrameCount` | macOS 15+, unavailable on iOS | Requests analysis lookahead. Ignored with low-latency rate control or `Quality = 1`; incompatible with multipass storage. The encoder may select a nearby count. |
| `SpatialAdaptiveQPLevel` | macOS 15+, unavailable on iOS | Codec dependent. Allows per-block QP adaptation from spatial statistics; must be disabled with low-latency rate control. |
| `ConstantQualityFactor` | macOS 27+ | New optional 0–1 consistent-quality request, with or without bitrate constraints. Requires runtime availability and encoder-support checks; it cannot be an unconditional setting for older supported macOS versions. |

Availability and the detailed defaults above were checked in the installed
Xcode macOS SDK's `VideoToolbox.framework/Headers/VTCompressionProperties.h`
(quality: lines 252–265; speed priority: 312–328; QP cap: 1215–1224;
lookahead/spatial QP: 1563–1588; constant quality: 1595–1602) and
`AVFoundation.framework/Headers/AVVideoSettings.h` (lines 205–215). Locate the
active SDK reproducibly with `xcrun --show-sdk-path`. These are SDK interface
contracts; available constants do not imply support from every runtime encoder.
[Apple speed/quality hint](https://developer.apple.com/documentation/videotoolbox/kvtcompressionpropertykey_prioritizeencodingspeedoverquality),
[Apple QP cap](https://developer.apple.com/documentation/videotoolbox/kvtcompressionpropertykey_maxallowedframeqp),
[Apple VideoToolbox quality](https://developer.apple.com/documentation/videotoolbox/kvtcompressionpropertykey_quality),
[Apple lookahead](https://developer.apple.com/documentation/videotoolbox/kvtcompressionpropertykey_suggestedlookaheadframecount),
[Apple spatial QP](https://developer.apple.com/documentation/videotoolbox/kvtcompressionpropertykey_spatialadaptiveqplevel),
[Apple constant quality](https://developer.apple.com/documentation/videotoolbox/kvtcompressionpropertykey_constantqualityfactor)

Apple's low-latency H.264 explanation specifically describes lowering maximum
frame QP as a way to keep screen content sharp by sacrificing frame rate once
the bitrate budget is exhausted. It gives the H.264 QP range as 1–51. This is a
mode-specific explanation, not proof that every offline HEVC encoder accepts or
interprets the property identically. A sharper output with missing frames must
be reported as a quality/motion tradeoff, not an unqualified improvement.
[Apple WWDC21 max-frame-QP discussion, 12:56–14:29](https://developer.apple.com/videos/play/wwdc2021/10158/)

## Local native capability observations

The separate local probes reported on 2026-11-07 used Apple M4 Pro, macOS
27.0.1, arm64. Hardware-required session creation succeeded for H.264 and HEVC
(status `0`), and failed for AV1 and VP9 (`kVTCouldNotFindVideoEncoderErr`,
`-12908`). The enumerated encoders included Apple H.264 and HEVC hardware/software
implementations; no native AV1/VP9 encoder was listed. HEVC profiles included
Main, Main10, and Main42210. These are observations about that machine and OS,
not a prohibition on future native encoders or proof of browser compatibility.
The benchmark's capability probe/output are the owning empirical evidence.

A separate AVAssetWriter capability probe at 3456×2234, requiring hardware and
enabling `performsMultiPassEncodingIfSupported`, reported
`canPerformMultiplePasses = true` for both H.264 and HEVC on that system. This
establishes a native multipass candidate. It does not establish a quality gain,
that more than one pass will be requested for every clip, or equal speed to
single-pass encoding.

## Minimal native multipass flow

The following is a proposed worker-serial flow for a silent-video prototype,
derived from the SDK contracts. It has not by itself been validated as shipping
Gatto behavior. Apple demonstrates the reader/writer handoff in its multipass
session. [Apple multipass walkthrough, around 41:29–45:20](https://developer.apple.com/videos/play/wwdc2014/513/)

```text
Before starting:
  videoReaderOutput.supportsRandomAccess = true
  videoWriterInput.performsMultiPassEncodingIfSupported = true
  videoWriterInput.expectsMediaDataInRealTime = false
  attach reader output and writer input

Start writer and one output timeline; start reader.
After startWriting, inspect canPerformMultiplePasses.
Register respondToEachPassDescriptionOnQueue exactly once.
Use a serial notification queue; keep all media mutations on one owner.

For each pass notification:
  description = input.currentPassDescription
  if description is nil:
    output.markConfigurationAsFinal()
    input.markAsFinished()
    finish remaining tracks, end session, finish writer
    return

  if this is a replay:
    output.resetForReadingTimeRanges(description.sourceTimeRanges)

  register requestMediaDataWhenReadyOnQueue for this pass
  while input.isReadyForMoreMediaData:
    sample = output.copyNextSampleBuffer()
    if sample is nil:
      check reader failure/cancellation separately from normal range EOF
      input.markCurrentPassAsFinished()
      return immediately; await the next pass notification
    skip marker-only samples
    append the same selected source frames at their original timestamps
```

`currentPassDescription` becomes nonnil during `startWriting`; the initial
request covers `[0, +infinity)`. Repeated pass descriptions can request disjoint
subsets. `markCurrentPassAsFinished` ends the current readiness callback
registration, allowing a new `requestMediaDataWhenReadyOnQueue` registration for
the next pass. `respondToEachPassDescriptionOnQueue` itself must be registered
only once. Its final notification has a nil pass description.
[Apple pass descriptions](https://developer.apple.com/documentation/avfoundation/avassetwriterinput/currentpassdescription),
[Apple pass completion](https://developer.apple.com/documentation/avfoundation/avassetwriterinput/markcurrentpassasfinished()),
[Apple pass callbacks](https://developer.apple.com/documentation/avfoundation/avassetwriterinput/respondtoeachpassdescription(on:using:))

Random-access output must be enabled before `startReading`. Reuse it directly;
reader recreation is unnecessary. Reset only after the preceding read reaches
nil without failure. The supplied ranges must be ordered, nonoverlapping, and
nonnegative in duration, with numeric starts; positive-infinite duration is
allowed. They are intersected with the asset duration. Do not call reset after
`markConfigurationAsFinal`; that finalization is appropriate when the writer
reports no more passes. Until then, the random-access reader can remain Reading
at range EOF. [Apple random access](https://developer.apple.com/documentation/avfoundation/avassetreaderoutput/supportsrandomaccess),
[Apple reset contract](https://developer.apple.com/documentation/avfoundation/avassetreaderoutput/resetforreading(timeranges:)),
[Apple reader finalization](https://developer.apple.com/documentation/avfoundation/avassetreaderoutput/markconfigurationasfinal())

Production implications inferred from these contracts and Gatto's code:

- Retain a stable first-pass selection when capping frame rate. Restarting the
  existing `last_video` filter at a partial replay range can select a 60 fps
  source frame that the first pass skipped. Store accepted timestamps or use an
  equivalent deterministic selection. Compare rational CMTime values correctly;
  different value/timescale pairs can denote the same instant.
- Treat a requested range's end as exclusive. Do not append the boundary sample
  twice when moving among ranges.
- Audio is not replayed with each video pass. Multipass video may keep the audio
  writer input unready until video analysis finishes. A separate audio reader
  and a serial pump after video multipass completes is a simple candidate to
  test; preserve the shared source timeline.
- Keep cancellation/deadlines active while waiting for analysis notifications.
  A Rust block can capture only a channel sender and notify the owning worker;
  it need not transfer worker-owned Objective-C media objects across threads.
- Callbacks may arrive asynchronously. Do not assume a nil reader sample means
  the entire writer is ready for finalization; it may only end the current pass.
- If multipass is unsupported, a sequential pass followed by `markAsFinished`
  remains valid. A pass-driven implementation may also use the documented
  single initial description, with terminal completion after marking that pass.

The Objective-C methods above are present in the locked
`objc2-av-foundation` 0.3.2 bindings. The pass-notification callback additionally
requires that crate's `dispatch2` feature; random-access methods are exposed in
`AVAssetReaderOutput`. APIs remain available on macOS 10.10+, while the macOS 27
Swift SDK advertises newer controller alternatives. SDK header evidence:
`AVAssetWriterInput.h` lines 447–537 and `AVAssetReaderOutput.h` lines 107–179;
inspect these under the path returned by `xcrun --show-sdk-path`.

# Screen-recording compression benchmarks

Measured 2026-11-07 against Gatto commit `3c4236c` (0.23.3), on an Apple M4 Pro
running macOS 27.0.1. The baseline is the published native path. The uncommitted app changes following
this investigation implement Quality H.264, optional AV1, and Fast native H.264;
see the production verification below.

## Decision

**Keep H.264 as the quality default, with a better two-pass software
encoder. Offer AV1 as an optional smaller-file mode.** The tests support improving
the encoding strategy more strongly than changing the default codec name.

On the original recording, software H.264 two-pass had the clearest first frame.
At nearly equal sizes, two-pass H.264 and two-pass AV1 had very similar sampled
whole-recording quality, with H.264 encoding faster here. AV1 also produced a
useful smaller-file result: 7.95 MB with nearly the quality of the 9.43 MB H.264
file. These are observations on this corpus, not a universal ranking.

Current desktop Chrome is a credible AV1 target, including software decoding on
M1/M2 Macs. HEVC has more conditional platform/GPU support. The dated
[compatibility matrix and primary sources](screen-recording-codec-compatibility.md)
separate receiver playback, hardware decoding, sender encoding, and GitHub
acceptance. No GitHub attachment was published during this investigation.

## Full recording results

The source is the supplied 333,013,277-byte, 90.093333-second screen recording:
3456 × 2234, H.264 Main, 8-bit 4:2:0, BT.709, 4,328 variable-rate frames
(about 48.03 fps average), with no audio. Tests preserve the full pixel dimensions.
The accepted full outputs below are strictly below 10,000,000 bytes.

PSNR and SSIM are RGB pixel comparisons against the decoded source: higher is
better. PSNR is logarithmic; these numbers are not percentages of perceived
quality. Full-recording scores use 31 samples at 0, 3, …, 90 seconds, including
the first frame and the final keyframe. The scores were checked alongside matching visual crops.

| Encoder / settings | Size (MB) | Encode time (s) | First RGB PSNR (dB) | Sampled RGB PSNR (dB) | Sampled RGB SSIM |
| --- | ---: | ---: | ---: | ---: | ---: |
| Current hardware H.264 | 9.77 | 32.9 | 31.22 | 35.68 | 0.9629 |
| Native HEVC Main10 | 9.63 | 34.8 | 32.14 | 38.76 | 0.9780 |
| Software H.264, two passes | 9.43 | 58.7 | 44.96 | 42.51 | 0.9872 |
| AV1 fast, similar size | 9.24 | 25.2 | 40.18 | 40.14 | 0.9824 |
| AV1, two passes, similar size | 9.27 | 76.1 | 40.72 | 42.39 | 0.9884 |
| AV1, two passes, smaller file | 7.95 | 69.2 | 40.59 | 42.05 | 0.9873 |
| Native H.264 multipass prototype | 9.05 | 66.2 | 34.27 | 32.85 | 0.9299 |
| Native HEVC multipass prototype | 9.36 | 69.2 | 34.51 | 34.76 | 0.9508 |

The first-frame RGB error of software two-pass H.264 was about 96% lower than
the current path's error on this recording (44.96 versus 31.22 dB). This refers
to mean squared pixel error, not a subjective quality percentage. The near-size
AV1 comparison is 9.27 MB versus H.264's 9.43 MB; the 0.11 dB whole-video difference
is too small to call a general codec winner on one recording.

Visual inspection compared matching first-frame crops at half resolution, near
the original logical display scale. The reference already contains the original recording's H.264/4:2:0 losses; recompression cannot
recover color detail that was absent from it.

## Settings that mattered

- **Two-pass software H.264:** `libx264`, `slow`, 8-bit 4:2:0, about 817 kbps video,
  300-frame maximum keyframe interval. It used the available size budget well,
  removed the unusually weak opening, and had the strongest first-frame score.
- **Two-pass AV1:** SVT-AV1 4.2.0, preset 6, perceptual tune 0, anti-alias-aware
  screen-content detection `scm=3`, 10-bit 4:2:0, 300-frame keyframe interval.
  At about 817 kbps it produced 7.95 MB; at 970 kbps it produced 9.27 MB. It is a
  useful optional size/quality tradeoff, with longer encoding on this machine.
- **Single-pass AV1:** presets 8 and 6 were efficient and often excellent on the
  12-second excerpts. The complete recording exposed a weak last keyframe at
  90 seconds: the first frame of that new group scored about 30.5 dB even when
  the nominal rate increased. Its packet was about 56 KB versus about 505 KB for
  the previous keyframe at 80 seconds in the 1.25 Mbps preset-8 output. This is
  consistent with poor allocation to the very short final group; that explanation
  is an inference. Two-pass AV1 removed this observed end-of-clip quality dip.
  A fast one-pass AV1 default needs further end-of-group validation.
- **Native HEVC Main10:** much better average quality than current hardware
  H.264 at similar speed and size, but the opening remained weak. It does not
  solve the reported first-frame problem by itself.
- **Native frame reordering:** small or mixed average-quality changes on the
  controlled clips; the first-frame score was unchanged. It is not an opening
  quality fix.
- **15 fps H.264:** improved still-frame quality but reduced motion fidelity.
  The moving UI control's RGB PSNR fell from 39.91 to 34.43 dB when held back to
  30 fps for comparison. It is a motion/size choice, not a free improvement.
- **`PrioritizeEncodingSpeedOverQuality=false`:** identical bytes and metrics to
  the baseline on both controls. Apple documents quality priority as the normal
  hardware default; see the compatibility note's SDK contracts.
- **Maximum frame QP 28:** sharper in some clips but failed the size gate after
  both retries on the original opening and full recording. It is not a reliable
  bounded-size default. Accepted 30 fps excerpts retained 360 frames; Apple also
  permits frame dropping in supported operating modes, so count validation is
  still necessary.
- **VP9 screen tuning:** useful quality, but one original excerpt took 64.6 s
  to encode for 12 s of video, versus 8.8 s for AV1 preset 6. The other VP9 excerpt
  overshot its byte budget. VP9 4:4:4 preserved more thin color detail on the
  controls, but is a separate profile with a different acceleration path.

The tested SVT options follow the
[SVT-AV1 parameter documentation](https://gitlab.com/AOMediaCodec/SVT-AV1/-/blob/v4.2.0/Docs/Parameters.md)
and [screen-content tools](https://gitlab.com/AOMediaCodec/SVT-AV1/-/blob/master/Docs/Appendix-Intra-Block-Copy.md).
FFmpeg's [codec documentation](https://ffmpeg.org/ffmpeg-codecs.html) describes
its software encoder wrappers. The initial experiments used the installed tools. The app now bundles a minimal,
source-built FFmpeg with x264 and SVT-AV1 rather than that Homebrew build.

## Native multipass experiment

The hardware-required AVAssetWriter inputs report multipass support for both
H.264 and HEVC on this M4 Pro. A video-only Swift prototype implemented the
actual callback/replay workflow, not just its Boolean setting. It uses one serial
queue, a random-access reader, explicit pass completion, final-reader completion,
and stable accepted presentation timestamps across replayed ranges. The full
recording selected exactly 2,181 frames in each of its two passes.

The prototype's **single-pass** control reproduced the current pipeline's byte
count, PSNR and SSIM exactly. Its multipass outputs improved the opening but
regressed sampled overall quality, as the table shows. Do not enable native
multipass globally based on capability alone. These results characterize this
prototype and source, not every possible Apple encoder configuration. General
production support would also need audio, cancellation, partial replay ranges,
source edits, errors, and fallback tests. The
[API-flow audit](screen-recording-codec-compatibility.md#minimal-native-multipass-flow)
records the native contracts and implementation risks.

## Playback tests and compatibility

**35 of 35 local Chrome checks passed**, with zero reported dropped frames during
the short checks. The test opened actual generated files, verified dimensions,
played the opening, sought beyond a later keyframe, and played near the end.
It exercised every successful full-recording output and one file for each
other fixture/codec/profile combination. Chrome was 156.0.8078.80 on this M4 Pro.
These are spot checks, not a claim of uninterrupted playback across every frame
on every platform.

Chrome's MediaCapabilities reported supported/smooth/power-efficient for the
ordinary profiles. VP9 4:4:4 played, but reported **not power efficient**. That
illustrates why a codec-name support claim is insufficient for arbitrary profiles.
The browser's reduced user agent reports an old Intel/macOS identifier; actual
OS/hardware were measured separately. Do not infer hardware support from that
user-agent string or from a device purchase year.

| Receiver class | H.264 High 4:2:0 | AV1 Main 4:2:0 | HEVC Main/Main10 4:2:0 |
| --- | --- | --- | --- |
| Current desktop Google Chrome on Mac/Windows/Linux | Expected; broad playback | Expected; software fallback available | Platform/GPU/driver dependent |
| Current Safari / Apple players on M1/M2 Mac | Expected | No AV1 hardware path advertised; use another format | Expected for supported platform profiles |
| Current Safari / Apple players on AV1-equipped M3 and later models | Expected | Expected on AV1-capable models | Expected for supported platform profiles |
| Chrome on iOS using WebKit | Apple platform requirements | Requires AV1-capable Apple hardware | Apple platform requirements |

Only the local M4/Chrome row was exercised here. The cross-device rows are
source-based expectations, with detailed Windows GPU, Android, Linux, and
Apple qualifications in the [full compatibility matrix](screen-recording-codec-compatibility.md#expected-playback-matrix).
GitHub accepts MP4/MOV/WebM containers and recommends H.264; it does not guarantee
every codec/profile inside them. Local browser success does not establish
GitHub upload acceptance or a fallback rendition.
[GitHub attachment policy](https://docs.github.com/en/get-started/writing-on-github/working-with-advanced-formatting/attaching-files),
[Chromium codec inventory](https://www.chromium.org/audio-video/)

## Preferences and implementation boundary

The preferences expose the user's goal along with the codec:

| Mode | Encoding strategy | Help text |
| --- | --- | --- |
| Quality — H.264 (recommended) | Quality-oriented software two-pass H.264 | Sharp text and the broadest playback compatibility; longer compression than the current hardware path. |
| Smaller files — AV1 | Software two-pass AV1 | Good fit for current desktop Chrome; older Safari/Apple devices may require H.264. |
| Fast — H.264 | Existing native hardware path | Faster compression with the current quality tradeoff. |

The initial UI used a recipient playback table. Following review, the app now
uses a click-only info icon beside Video uploads and a compact matrix for screen
detail, encoding time and final size relative to Fast. Software H.264 is shown
as approximately 1.8× the encoding time and 4% smaller; two-pass AV1 as 2.1× and
19% smaller, rounded from the production results versus the native baseline.
These are screen-recording estimates rather than universal rankings. Detailed
compatibility stays in the source-based matrix instead of the app popover.
A sender's Mac cannot establish which devices future GitHub readers will use.

The implementation uses bundled, separately signed software helpers and keeps
the native path for Fast mode. Neither software encoder is available through
AVAssetWriter on this Mac. Packaging includes the GPL helper licenses and exact
corresponding source with its build recipe. No GitHub attachment was uploaded;
local playback does not establish GitHub validation.

## Method and reproducibility

There are **55 benchmark cases**, including two failures of the native QP cap and
four oversized software outputs, all recorded in the appendix. The corpus consists
of the single real recording, 0–12 s and 30–42 s excerpts from it, a still based
on the supplied screenshot, and a moving native Gatto UI sequence. This is not
55 independent videos or a large representative population.

The four short fixtures are 12 seconds at 30 fps. Original excerpts retain
3456 × 2234 and use a ProRes 4444/10-bit mezzanine after frame-rate normalization;
the still is 1672 × 948 and the UI control is 1280 × 972. All comparisons for an
excerpt use the same mezzanine. ProRes is not assumed mathematically lossless.
The whole-recording comparisons use the original directly. No source was modified.

Short tests targeted 800,000 video bits/s with a 1,304,347-byte acceptance budget.
Native tests use a copy of the real four-file compression pipeline, its 92%
budget policy and 70% retry. Attempts are recorded; some native excerpts needed
both. Software results record overshoot explicitly and oversized outputs are
excluded from accepted-output recommendations. Later AV1 targets were adjusted
while keeping a fixed 10,000,000-byte full-file gate to compare actual sizes,
rather than assume equal nominal bitrates produce equal files.

The full native path retains 2,181 selected VFR frames (about 24.2 fps), while the
software experiments normalize to 30 fps and output 2,703 frames. Metrics align
decoded pictures on a 30 fps timeline. Thus the full experiment compares resulting
pipelines, including their motion sampling; the controlled 30 fps excerpts help
isolate codec/encoder changes. Intentional input frame-rate reduction is different
from the Chrome player's dropped-frame counter.

Short-fixture quality covers all 360 timeline frames. Whole-recording quality
samples every 90th 30 fps timeline frame: 31 samples, including both endpoints.
Decoded outputs and references are converted to planar RGB for PSNR/SSIM so color
sampling losses participate. PSNR aggregates mean squared error before conversion
to dB. Timings are measured encoding wall time, including both software passes
and native retry attempts, excluding quality analysis and fixture preparation.
Software uses four worker threads; native concurrency is managed by macOS.
Timings are single-machine observations with warm caches and ordinary desktop
activity, not controlled thermal or population measurements.

The original SHA-256 was `b4d7fdfd4f246884bfc9ba40558c72f82a6231f4787ab29a078ef3e89bea514c`. The private source, generated
media, crop PNGs, JSON exports and one-off benchmark/probe scripts were removed
after recording their findings here. The user's Desktop recording was untouched.

To reproduce the software settings with FFmpeg 9.0.2, run two encodes of the
original with `-threads 4 -b:v RATE -g 300 -vf fps=30 -pass N -passlogfile LOG`.
Pass 1 uses `-an -f null /dev/null`; pass 2 uses `-movflags +faststart OUTPUT.mp4`.
Use the codec/preset/pixel-format/SVT parameters listed above. Preserve a source
with audio using mono AAC 64 kbps at 48 kHz and reserve that rate from the total
budget. Production additionally caps the output to the container duration,
preserves rotation and shared audio/video timestamps, and validates the result.

Measure matching frames with FFmpeg's `psnr` and `ssim` filters after applying
`fps=30,select=not(mod(n\,90)),settb=AVTB,setpts=PTS-STARTPTS,format=gbrp`
to both full-recording inputs (31 frames). For excerpts omit `select` and compare
all 360 frames. Aggregate PSNR from mean squared error, not an arithmetic mean
of per-frame dB. Use only the same normalized reference for each comparison.

## Complete measured case log

MB uses decimal bytes. Budget failures remain visible rather than being counted
as accepted output. Native attempt counts include a 70% bitrate retry when used.
The two native multipass rows use the real callback/replay prototype.

| Fixture | Configuration | Target bits/s | Budget bytes | Output bytes | Seconds | First RGB PSNR | RGB PSNR | RGB SSIM | Accepted |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| desktop | native-h264 | 800000 | 1304347 | 1105408 | 1.64 | 33.01 | 36.71 | 0.9740 | Yes |
| desktop | native-h264-reorder | 800000 | 1304347 | 1040998 | 1.44 | 33.01 | 36.90 | 0.9753 | Yes |
| desktop | native-h264-15fps | 800000 | 1304347 | 1017301 | 0.80 | 34.31 | 37.20 | 0.9770 | Yes |
| desktop | native-h264-qp28 | 800000 | 1304347 | 1090809 | 1.40 | 35.64 | 37.24 | 0.9770 | Yes |
| desktop | native-h264-quality | 800000 | 1304347 | 1105408 | 1.42 | 33.01 | 36.71 | 0.9740 | Yes |
| desktop | native-hevc | 800000 | 1304347 | 921474 | 1.52 | 33.16 | 37.40 | 0.9759 | Yes |
| desktop | native-hevc-reorder | 800000 | 1304347 | 942501 | 1.49 | 33.16 | 37.27 | 0.9751 | Yes |
| desktop | native-hevc-main10 | 800000 | 1304347 | 501199 | 1.50 | 33.37 | 40.07 | 0.9843 | Yes |
| ui-motion | native-h264 | 800000 | 1304347 | 1001071 | 1.18 | 37.71 | 39.91 | 0.9929 | Yes |
| ui-motion | native-h264-reorder | 800000 | 1304347 | 974643 | 1.18 | 37.71 | 39.95 | 0.9931 | Yes |
| ui-motion | native-h264-15fps | 800000 | 1304347 | 886288 | 0.69 | 39.15 | 34.43 | 0.9907 | Yes |
| ui-motion | native-h264-qp28 | 800000 | 1304347 | 1003301 | 1.18 | 40.41 | 40.18 | 0.9933 | Yes |
| ui-motion | native-h264-quality | 800000 | 1304347 | 1001071 | 1.18 | 37.71 | 39.91 | 0.9929 | Yes |
| ui-motion | native-hevc | 800000 | 1304347 | 618065 | 1.24 | 39.59 | 40.23 | 0.9920 | Yes |
| ui-motion | native-hevc-reorder | 800000 | 1304347 | 563251 | 1.17 | 39.59 | 40.21 | 0.9922 | Yes |
| ui-motion | native-hevc-main10 | 800000 | 1304347 | 338262 | 1.27 | 40.01 | 41.86 | 0.9934 | Yes |
| desktop | x264-slow-2pass | 800000 | 1304347 | 1353307 | 3.45 | 39.04 | 39.23 | 0.9839 | Oversize |
| desktop | vp9-screen | 800000 | 1304347 | 1331176 | 3.93 | 39.42 | 39.41 | 0.9849 | Oversize |
| desktop | vp9-screen-444 | 800000 | 1304347 | 1605630 | 4.17 | 48.41 | 48.41 | 0.9912 | Oversize |
| desktop | av1-preset8 | 800000 | 1304347 | 378050 | 1.31 | 44.37 | 44.48 | 0.9927 | Yes |
| desktop | av1-preset6 | 800000 | 1304347 | 374903 | 2.12 | 44.42 | 44.53 | 0.9928 | Yes |
| ui-motion | x264-slow-2pass | 800000 | 1304347 | 1153665 | 2.50 | 44.66 | 42.69 | 0.9888 | Yes |
| ui-motion | vp9-screen | 800000 | 1304347 | 1080869 | 4.23 | 44.61 | 42.64 | 0.9884 | Yes |
| ui-motion | vp9-screen-444 | 800000 | 1304347 | 965246 | 9.32 | 48.27 | 48.52 | 0.9889 | Yes |
| ui-motion | av1-preset8 | 800000 | 1304347 | 542932 | 1.03 | 48.33 | 45.76 | 0.9971 | Yes |
| ui-motion | av1-preset6 | 800000 | 1304347 | 515468 | 1.57 | 48.32 | 45.75 | 0.9970 | Yes |
| original-opening | native-h264 | 800000 | 1304347 | 1099357 | 11.53 | 31.15 | 32.72 | 0.9307 | Yes |
| original-opening | native-h264-qp28 | 800000 | 1304347 | — | — | — | — | — | Failed size gate |
| original-opening | native-hevc | 800000 | 1304347 | 1144673 | 11.74 | 31.75 | 33.89 | 0.9394 | Yes |
| original-opening | native-hevc-main10 | 800000 | 1304347 | 1068079 | 11.78 | 31.26 | 34.41 | 0.9465 | Yes |
| original-opening | x264-slow-2pass | 800000 | 1304347 | 1227235 | 10.14 | 38.04 | 36.79 | 0.9659 | Yes |
| original-opening | vp9-screen | 800000 | 1304347 | 1297954 | 64.61 | 41.49 | 38.80 | 0.9749 | Yes |
| original-opening | av1-preset8 | 800000 | 1304347 | 1068841 | 5.79 | 39.37 | 39.76 | 0.9793 | Yes |
| original-opening | av1-preset6 | 800000 | 1304347 | 1049473 | 8.83 | 39.32 | 40.14 | 0.9806 | Yes |
| original-motion | native-h264 | 800000 | 1304347 | 1273353 | 5.63 | 31.16 | 32.90 | 0.9273 | Yes |
| original-motion | native-h264-qp28 | 800000 | 1304347 | 1180698 | 11.11 | 37.68 | 39.25 | 0.9810 | Yes |
| original-motion | native-hevc | 800000 | 1304347 | 1015395 | 11.75 | 31.90 | 35.27 | 0.9572 | Yes |
| original-motion | native-hevc-main10 | 800000 | 1304347 | 1038415 | 11.77 | 31.51 | 37.24 | 0.9694 | Yes |
| original-motion | x264-slow-2pass | 800000 | 1304347 | 1301166 | 8.59 | 39.83 | 40.54 | 0.9849 | Yes |
| original-motion | vp9-screen | 800000 | 1304347 | 1525965 | 23.72 | 42.17 | 42.36 | 0.9890 | Oversize |
| original-motion | av1-preset8 | 800000 | 1304347 | 850612 | 5.12 | 40.22 | 42.92 | 0.9876 | Yes |
| original-motion | av1-preset6 | 800000 | 1304347 | 884993 | 7.91 | 40.56 | 43.24 | 0.9884 | Yes |
| original-full | native-h264 | 816930 | 9999992 | 9767629 | 32.93 | 31.22 | 35.68 | 0.9629 | Yes |
| original-full | native-h264-qp28 | 816930 | 9999992 | — | — | — | — | — | Failed size gate |
| original-full | native-hevc-main10 | 816930 | 9999992 | 9626583 | 34.76 | 32.14 | 38.76 | 0.9780 | Yes |
| original-full | x264-slow-2pass | 816930 | 9999992 | 9425270 | 58.68 | 44.96 | 42.51 | 0.9872 | Yes |
| original-full | av1-preset8 | 816930 | 9999992 | 6133668 | 25.08 | 38.63 | 39.23 | 0.9783 | Yes |
| original-full | av1-screen-preset6 | 816930 | 9999992 | 6100494 | 45.20 | 38.58 | 39.40 | 0.9791 | Yes |
| original-full | av1-preset8 | 1250000 | 10000000 | 9243298 | 25.18 | 40.18 | 40.14 | 0.9824 | Yes |
| original-full | av1-screen-preset6 | 1250000 | 10000000 | 9830944 | 44.76 | 39.63 | 40.33 | 0.9832 | Yes |
| original-full | swift-native-h264-single | 816930 | 10000000 | 9767629 | 32.83 | 31.22 | 35.68 | 0.9629 | Yes |
| original-full | swift-native-h264-multi | 816930 | 10000000 | 9045491 | 66.22 | 34.27 | 32.85 | 0.9299 | Yes |
| original-full | swift-native-hevc-multi | 816930 | 10000000 | 9356800 | 69.24 | 34.51 | 34.76 | 0.9508 | Yes |
| original-full | av1-screen-2pass | 816930 | 10000000 | 7951590 | 69.20 | 40.59 | 42.05 | 0.9873 | Yes |
| original-full | av1-screen-2pass | 970000 | 10000000 | 9273864 | 76.06 | 40.72 | 42.39 | 0.9884 | Yes |

## Recorded Chrome playback observations

Chrome 156.0.8078.80, macOS 27.0.1, Apple M4 Pro. All checks verified dimensions,
opening playback, a seek beyond a later keyframe and playback near the end.
These are short spot checks. Power efficiency is the API report, not measured
battery consumption. The full codec string avoids confusing profiles.

| Fixture / configuration | Codec string | Frames decoded | Dropped | Power efficient | Playback |
| --- | --- | ---: | ---: | --- | --- |
| desktop / native-h264 | video/mp4; codecs="avc1.640028" | 123 | 0 | true | Passed |
| desktop / native-hevc | video/mp4; codecs="hvc1.1.6.L120.B0" | 122 | 0 | true | Passed |
| desktop / native-hevc-main10 | video/mp4; codecs="hvc1.2.4.L120.B0" | 123 | 0 | true | Passed |
| ui-motion / native-h264 | video/mp4; codecs="avc1.640020" | 123 | 0 | true | Passed |
| ui-motion / native-hevc | video/mp4; codecs="hvc1.1.6.L120.B0" | 123 | 0 | true | Passed |
| ui-motion / native-hevc-main10 | video/mp4; codecs="hvc1.2.4.L120.B0" | 123 | 0 | true | Passed |
| desktop / vp9-screen | video/webm; codecs="vp09.00.40.08" | 121 | 0 | true | Passed |
| desktop / vp9-screen-444 | video/webm; codecs="vp09.01.40.08" | 121 | 0 | false | Passed |
| desktop / av1-preset8 | video/mp4; codecs="av01.0.08M.10.0.110.12.13.01.0" | 122 | 0 | true | Passed |
| ui-motion / x264-slow-2pass | video/mp4; codecs="avc1.640028" | 181 | 0 | true | Passed |
| ui-motion / vp9-screen | video/webm; codecs="vp09.00.40.08" | 179 | 0 | true | Passed |
| ui-motion / vp9-screen-444 | video/webm; codecs="vp09.01.40.08" | 179 | 0 | false | Passed |
| ui-motion / av1-preset8 | video/mp4; codecs="av01.0.08M.10.0.110.02.02.01.0" | 123 | 0 | true | Passed |
| original-opening / native-h264 | video/mp4; codecs="avc1.640033" | 123 | 0 | true | Passed |
| original-opening / native-hevc | video/mp4; codecs="hvc1.1.6.L150.B0" | 123 | 0 | true | Passed |
| original-opening / native-hevc-main10 | video/mp4; codecs="hvc1.2.4.L150.B0" | 123 | 0 | true | Passed |
| original-opening / vp9-screen | video/webm; codecs="vp09.00.50.08" | 122 | 0 | true | Passed |
| original-opening / av1-preset8 | video/mp4; codecs="av01.0.12M.10.0.110.01.01.01.0" | 123 | 0 | true | Passed |
| original-motion / native-h264 | video/mp4; codecs="avc1.640033" | 123 | 0 | true | Passed |
| original-motion / native-hevc | video/mp4; codecs="hvc1.1.6.L150.B0" | 123 | 0 | true | Passed |
| original-motion / native-hevc-main10 | video/mp4; codecs="hvc1.2.4.L150.B0" | 123 | 0 | true | Passed |
| original-motion / vp9-screen | video/webm; codecs="vp09.00.50.08" | 121 | 0 | true | Passed |
| original-motion / av1-preset8 | video/mp4; codecs="av01.0.12M.10.0.110.01.01.01.0" | 123 | 0 | true | Passed |
| original-full / native-h264 | video/mp4; codecs="avc1.640033" | 450 | 0 | true | Passed |
| original-full / native-hevc-main10 | video/mp4; codecs="hvc1.2.4.L150.B0" | 450 | 0 | true | Passed |
| original-full / x264-slow-2pass | video/mp4; codecs="avc1.640033" | 530 | 0 | true | Passed |
| original-full / av1-preset8 | video/mp4; codecs="av01.0.12M.10.0.110.01.01.01.0" | 578 | 0 | true | Passed |
| original-full / av1-screen-preset6 | video/mp4; codecs="av01.0.12M.10.0.110.01.01.01.0" | 578 | 0 | true | Passed |
| original-full / av1-preset8 | video/mp4; codecs="av01.0.12M.10.0.110.01.01.01.0" | 578 | 0 | true | Passed |
| original-full / av1-screen-preset6 | video/mp4; codecs="av01.0.12M.10.0.110.01.01.01.0" | 578 | 0 | true | Passed |
| original-full / swift-native-h264-single | video/mp4; codecs="avc1.640033" | 450 | 0 | true | Passed |
| original-full / swift-native-h264-multi | video/mp4; codecs="avc1.640033" | 106 | 0 | true | Passed |
| original-full / swift-native-hevc-multi | video/mp4; codecs="hvc1.2.4.L150.B0" | 106 | 0 | true | Passed |
| original-full / av1-screen-2pass | video/mp4; codecs="av01.0.12M.10.0.110.01.01.01.0" | 578 | 0 | true | Passed |
| original-full / av1-screen-2pass | video/mp4; codecs="av01.0.12M.10.0.110.01.01.01.0" | 578 | 0 | true | Passed |

## Native probe record

The M4 Pro enumeration advertised Apple hardware and software H.264 and HEVC.
No native AV1/VP9 encoder appeared. Both hardware-required AVAssetWriter inputs
reported multipass support after writing began. A single-pass H.264 prototype
selected 2,181 frames; both H.264 and HEVC multipass runs selected 2,181 in each
of two passes. Actual quality regressed overall, as recorded above.

## Production verification — 2026-11-07

The actual Rust compression entry point, rather than a research copy, was run
on the original recording using the self-contained helpers. Both files passed
the strict 10 MB gate, retained 3456 × 2234 pixels, and decoded successfully.
Quality produced **9,423,773 bytes in 60.26 s**; Smaller files produced
**7,949,076 bytes in 69.94 s**. These wall times include both passes, probing,
first-picture validation and staging. The original was checked for changes.

| Production mode | First RGB PSNR (dB) | Sampled RGB PSNR (dB) | Sampled RGB SSIM |
| --- | ---: | ---: | ---: |
| Quality — H.264 | 44.94 | 42.50 | 0.9872 |
| Smaller files — AV1 | 40.59 | 42.05 | 0.9873 |

The production measurements agree closely with the research outputs. The
production AV1 file was about 15.6% smaller than the production H.264 file on
this one recording, at similar sampled quality. This is not a general size guarantee.
No extra live GitHub upload was performed. Browser playback observations above
refer to the benchmark files; production verification includes local decoding.

Real media tests cover both software modes with AAC audio, rotated sources,
delayed audio, multiple-track rejection, cancellation, source preservation and
monotonic progress. A fixture exposed a MOV edit-list tail longer than its
declared playback interval; output is now capped to that interval to avoid
lengthening a clip during frame duplication. HDR transfer/metadata rejection is
also tested. The existing native compression and import tests still pass.

The native preferences and its open comparison table were rendered offscreen
and checked for wrapping. The revised comparison fits a compact popover. The
local app bundle was packaged and ad hoc signed, with both helpers signed before
the outer bundle. Each helper is approximately 11 MB; both link only system
dynamic libraries. The corresponding-source archive contains exact pinned inputs
and the Bun recipe. The release command was not run; nothing was published.

## Removing the separate ffprobe executable

The runtime now uses one FFmpeg executable. A small Gatto-specific
`--gatto-inspect` entry point reads only the duration, stream properties,
start times, rotation and HDR metadata required by the existing checks. The
reader shares the encoder's already linked libavformat/libavcodec libraries.
The build disables ffprobe and includes the GPL adapter source and recipe in
the corresponding-source archive. Developer artwork tools can still use a
separately installed ffprobe.

The ad hoc signed helper measures 11,479,040 bytes, versus
22,782,272 bytes for the previous two signed helpers: a
11.30 MB reduction in the installed codec payload. This isolates
helper size from concurrent app/UI changes; final release DMG size is unmeasured.

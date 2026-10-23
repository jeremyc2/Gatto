# Side-by-side comparison profile

**Historical measurements:** the split-editor implementation profiled below was
superseded by the attachment picker and single editor on 2026-10-23. The old
Preview/Save buttons, per-side caching, and menu interaction harness no longer
exist. These tables preserve their original evidence; they do not measure the
current workflow. Current behavior is documented in [the architecture guide](../ARCHITECTURE.md).
Sizing research and primary tldraw sources are in [image sizing](comparison-image-sizing.md).


Measured October 22, 2026 on an Apple M4 Pro, 48 GiB RAM, macOS 27.0.1,
Rust 1.99.0. These are local examples, not measurements of the user's attachments.

## Initial baseline

The development build makes this CPU-heavy operation dramatically slower than
the optimized build. The initial composition pipeline also did unnecessary image work
in both builds: each annotated source was encoded to PNG and immediately decoded,
then the stitched canvas passed through another full-document render and PNG
encode. No network request is involved in Preview.

| Input size, per image | Development median | Optimized median |
| --- | ---: | ---: |
| 1920 × 1080 | 7,249.5 ms | 266.3 ms |
| 3840 × 2160 | 27,847.2 ms | 934.7 ms |

Three runs per size, with fixture preparation excluded from each composition
timer. Both sides use the repository's `image-editor.png`, resized to the input
size, with a rotated rectangle extending outside the source bounds. Result sizes
were 387,180 and 1,063,874 bytes respectively. Timings include attachment
validation but exclude UI rendering, uploads, and disk writes.

Temporary stage timers in the actual production pipeline gave this typical
optimized 4K breakdown. The timers have been removed from normal code.

| Work | Approximate time |
| --- | ---: |
| Rasterize two annotated sources | 151 ms |
| Encode their two intermediate PNGs | 253 ms |
| Decode those PNGs again | 55 ms |
| Stitch source pixels and paint tabs | 22 ms |
| Rasterize the final canvas and label layers | 220 ms |
| Encode the final PNG | 180 ms |
| Decode for attachment validation | 52 ms |

Allocation, background fill, label layout, and timing overhead account for the
remaining time. An independent optimized three-run repeat gave medians of 253.3
and 894.9 ms; the table above retains the first complete paired baseline.

`/usr/bin/time -l` reported 794,460,160 bytes (about 758 MiB) maximum resident
memory over the entire repeated benchmark. This includes fixture preparation and
process/runtime memory; it is not a per-export allocation count.

## Actual button path

A separate harness clicks the production buttons using the two existing
1240 × 1560 walkthrough images. It loads the sources before starting the timer.

| Measurement | Development | Optimized |
| --- | ---: | ---: |
| Preview click through first captured GPU frame | 5,622.8 ms | 167.4 ms |
| Save click through staged-attachment event, before reuse | 5,057.4 ms | 140.4 ms |

These are individual samples, not medians. The preview timer includes renderer
settling, image decoding, and screenshot readback. The Save timer ends at the
composer's attachment event; it does not open the main-window draft dialog or
upload anything. Source contents and annotations differ from the synthetic-size
benchmark, so its numbers should not be compared directly with that table.

## Changes made

- The image-menu button now supplies opaque black idle, hover, and pressed fills.
  Before the fix, a spacebar pan over the button changed a black-fill sample to
  `[30, 32, 35, 255]`. Afterward, all 12 sampled crossing frames stay
  `[0, 0, 0, 255]`.
- An explicit Preview/Save operation state drives the appropriate label and
  spinner. Preview says **Preparing preview…**; Save only says **Saving…** during
  a save operation.
- Save reuses the encoded attachment while the composed preview is displayed.
  The optimized button harness measured 0.8 ms to the staged-attachment event
  afterward, with Preview still taking 168.9 ms in that run. A test checks that
  Save shares the same attachment ID and encoded byte allocation. Leaving
  preview mode or changing its inputs clears the displayed result.

A final optimized UI run after removing the temporary timers measured 168.3 ms
for Preview and 0.8 ms for the reused Save event.

## Follow-up image options border check

A later hover report exposed a gap in the original background-only check: the
outlined custom button replaced its border color while retaining the black fill.
The native harness now compares the whole 48 × 64 device-pixel button against
its idle capture, including its border and icon, in ordinary hover, pressed,
returned-to-idle, and twelve spacebar-pan crossing frames. Before the fix, hover
changed 432 pixels, including in six pan-crossing frames. After using a
non-outline custom variant with constant border and explicit black fill, all
fifteen samples have zero changed pixels. The idle capture itself is
byte-for-byte unchanged. The enabled button now explicitly uses the pointer
cursor and retains its **Image options** tooltip.

## Follow-up optimization

The three recommendations from the initial profile were implemented on the same
branch on October 22, 2026:

1. The canonical annotation renderer now returns pixels independently of PNG
   encoding. Comparison sources pass directly to stitching, with the same
   demultiplication as PNG export. Intermediate PNG encoding and decoding are
   gone; pixelation and expanded bounds still use the existing renderer.
2. Native-size rendering borrows the source pixels instead of cloning them and
   reuses the premultiplied source allocation when the bounds fit. Stitching
   transfers ownership of one opaque output canvas into the annotation renderer.
   Label layers are only as large as their tabs; the final canvas no longer
   becomes a second document. Internally rendered pixels supply staging dimensions
   without decoding the final PNG again. External attachments retain their full
   decoding/validation path. Only the finished comparison is PNG-encoded.
3. The composer retains one rendering per side. The cache checks shared immutable
   source identity and exact annotation content, including pixelation and
   undo/redo changes. It does not rely on attachment IDs or history length.
   Labels are rendered separately and do not invalidate source pixels. Pan and
   zoom also leave these pixels unchanged. Swapping moves both caches; removing,
   replacing, or resetting releases them. Cache work stays on the background
   worker and returns only to the matching window generation.

### Optimized-build results

Three-run medians with the same fixtures and timing boundaries as the baseline:

| Input size per side | Initial pipeline | New, cold | Both sides unchanged | Right side edited |
| --- | ---: | ---: | ---: | ---: |
| 1920 × 1080 | 266.3 ms | 94.0 ms | 54.3 ms | 74.8 ms |
| 3840 × 2160 | 934.7 ms | 337.5 ms | 182.5 ms | 259.6 ms |

The cold comparison is about 64% faster at both sizes. Warm runs still stitch,
render labels, and encode the final PNG; they skip only the unchanged source
renderings. Each right-edit run moves the right rectangle one pixel further,
retaining the left rendering. Warm-up composition is outside those timers.
Cold runs discard the cache before each measurement.

The cold 1080p and 4K outputs are **byte-for-byte identical** to the saved baseline
PNGs (387,180 and 1,063,874 bytes). Regression coverage also checks transparency,
translucent rotated annotations, emoji labels, pixelation, expanded bounds,
cache reuse, undo/redo, replacement, and swapping. Bounded label layers are
compared against a full-canvas reference render.

The same `/usr/bin/time -l` process measurement now reports 394,805,248 bytes
(about 377 MiB) maximum resident memory, down from 794,460,160 bytes (758 MiB),
a reduction of about 50%. The extended harness now includes cold, unchanged, and
one-side-edited scenarios; both measurements include fixture preparation,
repeated runs, allocator retention, and runtime memory rather than measuring
per-export allocations.

The optimized actual-button harness measured **56.9 ms** from Preview click to
first captured frame, versus the original 167.4 ms sample. Saving the displayed
preview still emits its cached staged attachment in **0.8 ms**, without an upload.
These are individual samples, not medians.

The development build's cold medians also improved from 7,249.5 to 3,254.7 ms
at 1080p and from 27,847.2 to 12,377.9 ms at 4K. Unchanged-side medians were
1,841.0 and 6,729.6 ms; right-side-edit medians were 2,567.7 and 9,624.1 ms.
Its actual-button samples were 1,968.3 ms for Preview and 3.1 ms for the reused
Save event. The regular development build remains much slower for pixel and PNG
work; use an optimized build when evaluating the app's expected latency.

## Current workflow reproduction

```sh
DEVELOPER_DIR=/Library/Developer/CommandLineTools cargo run --release --features walkthrough-screenshots --bin walkthrough-screenshots -- /tmp/gatto-comparison --comparison-only
```

The current native harness imports the two walkthrough screenshots, transfers the
completed picker request to the main window, waits for background combination,
and captures the staged result and regular editor. It reports elapsed time from
adding the second source to completed staging, output dimensions, and PNG bytes.
It also verifies that entering inline caption editing preserves the column bounds.
It does not upload or change preferences. Its source sizes, output policy, and
measured boundary differ from the historical split-editor benchmarks above;
direct speedup ratios would be misleading.

### Picker and single-editor sample, 2026-10-23 (before footer captions)

On the local Apple M4 Pro Mac, the optimized native harness produced a
2536 × 1676 PNG of 181,409 bytes from the two checked-in walkthrough screenshots.
The second-attachment-to-staging boundary took 74.6 ms in one run (not a median).
This includes reading/validating the second source and background combination;
it excludes the first source import, opening the main window, annotation saving,
and upload. No byte-budget retry was needed for this compressible screenshot
pair. The complete screenshot harness also paints multiple windows, so its
process memory must not be presented as the isolated combine peak.

### Borderless caption revision, 2026-10-23

The subsequent caption revision moves labels beneath the image row, removes tab
chrome, and uses the ordinary freeform text input. Native captures include
multiline text and a 44 × 44 image centered above a 400-pixel-wide caption
column, alongside a 512 × 512 image. Both captions share a baseline. The input
keeps its column width and fitted extent while typing, refitting the expanded
footer on completion. Regression tests reproduce and prevent the former
content-dependent field width and blocked space-pan gesture. These are event
and layout regressions; they do not establish a universal frame-time guarantee.

One optimized native walkthrough run produced a 2536 × 1684 PNG of 183,490 bytes
in 114.2 ms from second-source import through staging on the same M4 Pro Mac.
This is one sample with the revised layout and renderer, not a median or a
direct comparison to the historical split-editor data.

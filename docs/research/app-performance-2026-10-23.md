# Rust app performance measurements, October 23, 2026

Measured on an Apple M4 Pro, arm64 macOS, with rustc 1.99.0
(`b940084d7`, September 28, 2026). The baseline app source is
`d692a4d0d7c955ad800b2dd0c6bcbe6b70b6870e` (0.17.7). The final measurements
use the local changes accompanying this record. No dependency or dependency
feature changed. All new app and benchmark code is safe Rust.

## Method

The ignored `performance::measure_workflows` test calls production app paths with
generated 3840 × 2160 RGBA screenshots, including alpha edges. Each case runs in
a fresh child process under macOS `/usr/bin/time -l`. One warm-up operation is
followed by seven measured samples; `wall_ms` is their median. The parent runs
cases sequentially and can save JSON or check a previous report.

`process_cpu_ms` is the child's total user plus system CPU time, at the system
tool's 10 ms reporting precision. `process_peak_rss_bytes` is its maximum resident
set size in bytes. Both include fixture preparation, warm-up, all eight samples,
runtime overhead, allocator retention, and output verification. They are not
per-operation allocation counts, GPU memory, or the idle app's footprint. The
benchmarks do not launch the application, open windows, access preferences or
clipboard contents, authenticate, upload, or contact a network service.

Cases cover full-validation import, initial editor decoding, 100 retained-session
reopens per sample, PNG export with 24 rotated opaque/translucent rectangles,
100 incremental pixelation pointer updates, cold preparation of annotation
preview geometry/textures, and combining two screenshots into an editable
comparison. Reopening includes one initial decode during warm-up; its reported
operation time measures cached session access rather than window presentation.
Pixelation samples retain the source mosaic cache after warm-up, as the app does.
These inputs do not establish bounds for every supported image/video format,
network conditions, or button-to-frame latency.

Export and comparison reports fingerprint decoded RGBA pixels, independently of
PNG packaging. The other cases report zero in that field; existing behavior tests
cover their results. Differential rendering tests additionally compare the new
annotation composition against a full-canvas reference byte-for-byte across all
shapes, text/emoji, rotations, scale, transparency, clipping and odd row widths.
A PNG regression test requires identical encoded bytes for all 256 alpha values
and downscaled, expanded-bound exports.

## Results

Times are milliseconds; memory is decimal MB. Raw values are preserved in
[baseline.json](app-performance-2026-10-23/baseline.json) and
[final.json](app-performance-2026-10-23/final.json).

| Scenario                   | Baseline operation | Final operation | Baseline process CPU | Final process CPU | Baseline peak RSS | Final peak RSS |
| -------------------------- | -----------------: | --------------: | -------------------: | ----------------: | ----------------: | -------------: |
| Import                     |              32.53 |           30.24 |                  330 |               310 |             47.01 |          46.86 |
| Open editor                |              33.03 |           30.20 |                  320 |               290 |             80.23 |          46.87 |
| Reopen session × 100       |             0.0044 |          0.0035 |                   90 |                80 |             80.20 |          46.89 |
| Export 24 annotations      |             904.26 |          158.49 |                7,810 |             1,830 |            120.68 |         119.62 |
| Pixelation × 100 updates   |               7.36 |            7.35 |                  120 |               120 |             59.08 |          58.80 |
| Prepare annotation preview |              10.79 |            9.84 |                  150 |               140 |             65.11 |          64.11 |
| Create comparison          |             234.06 |          195.38 |                2,210 |             1,890 |            187.76 |         179.54 |

The substantial changes are about 82% lower export latency, 77% less CPU time
in its repeated process scenario, and 42% less peak memory in initial editor
decoding. Comparison creation is about 17% faster with 4% lower peak RSS. Small
differences in unchanged cases and sub-millisecond session timings are noise, not
evidence of additional improvements. Export peak RSS is effectively unchanged:
the raster and reusable layer still coexist during painting.

Both export fingerprints remain identical to the baseline:
`7179087487929262514` for annotations and `3025692317843924600` for comparison.

## Iterations retained

1. Consume decoded/resized RGBA images rather than copying their pixel buffers;
   convert completed preview bitmaps in place. This removes a 33,177,600-byte
   copy from opening the 4K RGBA fixture and reduces comparison/preview work.
   [Measurements](app-performance-2026-10-23/after-copy-removal.json).
2. Consume the finished export raster for PNG alpha conversion, retaining the
   original compression, filtering and rounding. This removes another full-image
   copy, although overall process RSS remains dominated by painting.
   [Measurements](app-performance-2026-10-23/after-owned-png.json).
3. Reuse one transparent layer for all export annotations and captions. Paint in
   unchanged full-canvas coordinates, find the occupied rows, composite those
   rows once at whole-object opacity, and clear them before the next mark. This
   removes repeated full-image allocations and most transparent-pixel blending.
   It avoids estimated drawing bounds and preserves bitmap glyph coverage.
   [Measurements](app-performance-2026-10-23/after-layer-reuse.json).
4. Compile the app package with one code generation unit and explicit symbol
   stripping. Keep dependency compilation, thin LTO, and unwind handling intact.
   The 159 ms export result persists with this setting. An app-package size
   optimization level of `s` was rejected: export rose to about 247 ms and preview
   preparation to 12.6 ms. No unsafe allocator or native measurement binding is
   included in the finished benchmark.

Compiler choices were tested rather than assumed; the
[Cargo profile reference](https://doc.rust-lang.org/cargo/reference/profiles.html)
describes code generation units, symbol stripping, and optimization tradeoffs.

## Executable size

`cargo build --release --locked --bin gatto` on the same machine/toolchain:

| Artifact                   | File bytes |
| -------------------------- | ---------: |
| Baseline arm64 executable  | 28,444,256 |
| Optimized arm64 executable | 26,947,888 |

Reduction: 1,496,368 bytes, about 5.3%. These are local stripped executables,
not downloaded DMGs or published bundle measurements. Package resources,
dependencies, icons, and artwork are retained. README figures remain tied to the
published release; refresh them through the normal release-size procedure after
publication. The app's dependencies, lockfile and security/privacy boundaries
are unchanged.

## Reproduction and regression gates

Run on an otherwise idle Mac with the same toolchain and fixtures:

```sh
GATTO_PERF_OUTPUT=target/performance.json cargo test --release --locked --bin gatto performance::measure_workflows -- --ignored --exact --nocapture
GATTO_PERF_BASELINE=target/performance.json cargo test --release --locked --bin gatto performance::measure_workflows -- --ignored --exact --nocapture
```

The second command fails if exported pixels change or any measured scenario's
median latency, process CPU, or peak RSS exceeds the saved baseline by 25%.
Latency below 1 ms has a 1 ms threshold to avoid gating on microsecond noise.
Keep individual metrics visible; a better aggregate must not conceal a regression.
Rebaseline only after investigating differences in the machine, compiler, inputs,
or app. Timing is noisy: a worthwhile code change need not improve every sample.
The gate was also run against the saved original test executable with the optimized
report as its limit. It correctly failed editor-open memory: 80,183,296 bytes
exceeded the 58,593,280-byte threshold. The optimized test executable passes the same
gate, so the measurement loop detects the resource regression it is meant to prevent.

Verification includes the standard debug Rust suite (264 passed, one opt-in test
ignored), the release benchmark, formatting, all-target checks and all-feature
Clippy. Native renderer checks compare translucent shapes, rotated text and emoji
with saved PNG output. Seven existing app UI tests fail in release mode because
their debug UI markers are absent; the saved baseline test executable reproduces
the same seven failures. The standard debug suite passes, and all new differential
rendering tests pass in release mode.

## Safety migration follow-up

The later [Rust migration](safe-rust-migration-2026-10-23.md) keeps these
optimizations and passes all seven regression gates against `final.json`.
[The measured follow-up](app-performance-2026-10-23/after-rust-migration.json)
includes unchanged export/comparison fingerprints and no gate failures for wall
time, process CPU or peak resident memory. Small timing differences across runs
are not evidence that the safety migration itself speeds up the image workflows.

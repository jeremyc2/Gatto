# Rust migration and remaining native exceptions

Implemented October 23, 2026, for the local Gatto 0.17.7 working tree, following
[the first-party safety audit](native-safety-audit-2026-10-23.md). This record
supersedes that audit's implementation inventory, preserving its dated findings.
Scope is the client app; the website and surrounding Rust tooling are unchanged.

## Applied changes

The upload implementation, status menu and Finder service now use the already
installed Rust framework bindings. The app's Objective-C implementation falls
from **564 lines in two files to 147 lines in `native/Hotkey.m`**. The handwritten
shared C header shrinks from 33 to nine lines. There is no standalone app-owned C
or C++ implementation. Swift/Sparkle remains 394 lines.

- **Uploads:** Rust owns the reader, bounded producer buffer, response `Vec`,
  synchronized state, monotonic deadlines and cleanup guard. Typed Objective-C
  delegates own their state. The old three function-pointer callbacks, borrowed
  stack context, C response layout, `malloc`/`strdup` and matching `free` interface
  are removed. The worker waits for session invalidation before
  returning, including during Rust unwinding. Reader errors/panics become source
  errors, and progress callback panics cannot unwind through Objective-C. Polling
  drains temporary Foundation objects every turn instead of accumulating them
  throughout a slow upload.
- **Menus and Finder:** typed retained AppKit objects replace opaque menu handles.
  The menu action target owns its channel sender; destruction disconnects all
  unretained targets, including items native menu tracking might still retain.
  A process-lifetime Rust service provider resolves file references and forwards
  whole selections to the existing GPUI event queue. Its sole raw out-parameter
  returns an autoreleased error string with the required Services ownership.
- **Thread requirements:** desktop owners check `MainThreadMarker` at construction
  and retain it. The concrete updater class is `MainThreadOnly`; its typed methods
  match the Swift selectors and block argument types. The library remains loaded
  for process lifetime because registered classes/callbacks cannot survive unload.
- **Video compression:** safe Rust orchestrates policy, progress, cancellation,
  channels and deadlines. Private owning reader/writer/sample operations isolate
  native calls, and a session cannot move across threads. Reader/writer Drop
  cancels partial operations. Four dictionary constructors hold the SDK's typed
  codec schemas. The former 364-line unsafe region is gone: native operation
  blocks are at most 12 lines, and the largest schema block is 42 lines.
- **Bitmap glyphs:** Core Graphics owns and initializes the drawing allocation;
  a bounded copy supplies Rust-owned pixels. The context no longer borrows a Rust
  allocation through an untracked raw pointer. A small fallible constructor checks
  null and balances native create/retain references; it avoids the locked wrapper's
  allocation-failure assertion. Empty/overflowing dimensions return `None`.
- **Small cleanup:** one checked GPUI-to-AppKit conversion replaces four raw-view
  conversions; deferred close work retains its window. A Drop guard restores the
  score export's graphics state on every return. Model, media-validation and
  diagnostics modules forbid unsafe code. Media orchestration denies unsafe code,
  with explicit exceptions only for its private binding/schema modules. Every
  app entry point denies implicit unsafe operations inside unsafe functions.

No dependency was added or removed. Existing `objc2` framework features were
expanded. `Cargo.lock` is byte-identical to the pre-migration copy; complete
locked Cargo metadata contains the same **851 package identities** before and
after, with no additions or removals. This number includes all platforms and
build/development packages; it is not a count of code executed by the macOS app.

The Rust upload implementation is longer than the previous combined transport
and FFI shim, largely because typed protocol methods and safety contracts are
explicit. This is the intended tradeoff: Rust ownership and synchronization
replace raw callbacks and allocator conventions. It is not a claim of zero unsafe
Rust or a claim that changing a filename alone makes native calls safe.

## Exceptions retained, and why

| Location                                                                                                                                        | Contract and reason                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| ----------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [`native/Hotkey.m`](../../native/Hotkey.m), [`desktop.rs`](../../src/desktop.rs)                                                                | Carbon registration and media-key event taps need C callbacks, SDK key constants and native cleanup. A Rust port without another binding dependency would duplicate these raw declarations and SDK mappings. ARC plus 147 focused lines remains the smaller maintainable exception. Rust owns a boxed sender until unregister succeeds; failed OS cleanup detaches callbacks before freeing Rust state. A detached native owner may deliberately leak rather than become a dangling OS callback. Repeated failures have no proven aggregate leak bound. |
| [`github/native.rs`](../../src/github/native.rs), private `InputOnce`                                                                           | One `unsafe impl Send` transfers an **unopened, unscheduled** bound input stream exclusively to Foundation, once under a mutex. No clone or borrowed getter exists. Rust never opens/reads/schedules that input. The worker owns only the paired output. This does **not** mark Foundation streams generally Send/Sync. The old Objective-C transport already performed this unchecked transfer; the new wrapper documents and restricts it.                                                                                                            |
| Upload delegates and output stream                                                                                                              | Protocol declarations must match native selector/block signatures. Each borrowed completion is invoked synchronously once. Output writes borrow a checked slice only for the synchronous call; scheduling and teardown use the same worker run loop. Native response bytes are capped before copying. These operations still trust Foundation's contracts.                                                                                                                                                                                              |
| [`updater.rs`](../../src/updater.rs), [`native/Updater.swift`](../../native/Updater.swift)                                                      | Bundle-relative library loading, typed Objective-C selectors, and one borrowed callback string conversion remain. Swift's actor isolation, ARC, errors and Sparkle integration are appropriate here. Replacing Sparkle's security-sensitive updater/installer for language uniformity would add substantially more code and risk without a dependency-free equivalent.                                                                                                                                                                                  |
| [`compression/bindings.rs`](../../src/compression/bindings.rs), [`settings.rs`](../../src/compression/settings.rs)                              | Apple owns decoding, hardware H.264/AAC encoding and native samples. Rust retains each sample while inspecting/appending it; one worker sequences reads/writes/cancellation. Native dictionary keys require the documented value classes. Hardware and codec implementations remain trusted.                                                                                                                                                                                                                                                            |
| [`annotation/text/layout/macos.rs`](../../src/annotation/text/layout/macos.rs), [`bitmap.rs`](../../src/annotation/text/layout/macos/bitmap.rs) | The locked Core Text wrapper lacks two typesetter functions. A checked owning CFType fills that gap without changing line breaking. Three small Core Graphics declarations provide fallible, native-owned RGBA allocation where the existing wrapper asserts; create references are balanced explicitly. Native glyph drawing and font attributes remain system integration.                                                                                                                                                                            |
| [`window_limits.rs`](../../src/window_limits.rs)                                                                                                | GPUI supplies the borrowed NSView; one main-thread-checked conversion retains its NSWindow before deferred work. The existing hide/reuse workaround remains as documented in its ADR.                                                                                                                                                                                                                                                                                                                                                                   |
| [`installer_cleanup.rs`](../../src/installer_cleanup.rs), [`rooftops_score.rs`](../../src/rooftops_score.rs), [`theme.rs`](../../src/theme.rs)  | UID lookup, system property lists, native font/bitmap export and the native appearance constant are small justified platform operations. Validation stays in Rust, native objects have owning wrappers, and graphics state has Drop cleanup.                                                                                                                                                                                                                                                                                                            |

The operating-system and dependency code still includes Objective-C, C/C++, Swift
and unsafe Rust internally. Safe Rust interfaces do not turn those implementations
into safe Rust or sandbox them. This work reviews first-party contracts and tests
the changed paths; it is not a complete transitive-source audit or a memory-safety
proof of Apple, GPUI, renderer libraries or Sparkle.

## Behavior and verification

Transport policy is preserved: ephemeral sessions; no cookie, credential or cache
storage; default OS server-trust handling only; no redirects or request replay;
64 KiB producer/response limits; unchanged progress cadence, total/idle deadlines,
error messages and received-success-versus-cancel rule. The app still uses the
same native renderer, line breaker and hardware codec settings.

Verification commands (macOS main-thread checks use the standalone harness):

```sh
cargo test --locked --bin gatto
cargo clippy --locked --all-targets --all-features -- -D warnings
./scripts/test-platform.sh
./scripts/test-updater.sh
xcrun clang --analyze -fobjc-arc -fblocks -Wall -Wextra -Werror \
  -Wno-unused-parameter -Wno-deprecated-declarations -mmacosx-version-min=13.0 \
  native/Hotkey.m -o target/hotkey-analysis.plist
xcrun clang -fobjc-arc -fblocks -fsanitize=address,undefined -g \
  -Wall -Wextra -Werror -Wno-unused-parameter -Wno-deprecated-declarations \
  -mmacosx-version-min=13.0 native/HotkeyTests.m \
  -framework AppKit -framework Carbon -framework CoreGraphics \
  -o target/hotkey-sanitized
target/hotkey-sanitized
```

The platform tests exercise the production Rust menu's action dispatch, shortcuts,
visibility, update states, and target disconnection. Service tests cover modern and
legacy pasteboards, mixed selections, special characters, live/stale file references
and non-file rejection. The disposable Finder app runs the Rust provider for both
cold and warm service delivery, as well as Open With. Native hotkey tests inject
unregister and handler-removal failures to check detached-owner lifetime; these
passed ASan/UBSan, and Clang analysis reported no diagnostics. Swift's isolated
fake-client/channel/consent checks and signed/tampered feed checks also pass.

The full debug app suite passed **268 tests**, with the performance test explicitly
ignored in ordinary runs. This includes real local HTTP privacy/failure tests,
native H.264/AAC output and cancellation, source-reader panic/error handling,
background-thread rejection, bitmap dimension validation and pixel-for-pixel color
glyph agreement with the previous renderer. The existing rapid-typing timing check
failed once while a release build competed for CPU; it passed on the quiet rerun.
No threshold or test expectation was relaxed.

The seven opt-in release performance cases passed against the previous optimized
baseline, including wall time, process CPU, peak resident memory and output
fingerprints. [Measured results](app-performance-2026-10-23/after-rust-migration.json)
record import 30.1 ms, editor opening 30.0 ms, export 158.0 ms and comparison
195.1 ms. These are within the existing 25% regression gates; small run-to-run
variations are not evidence of a speedup from the language conversion. Export and
comparison fingerprints are unchanged. The earlier optimization gains remain;
see the [measurement method and original baseline](app-performance-2026-10-23.md).

```sh
GATTO_PERF_OUTPUT=target/rust-migration/performance-after.json \
GATTO_PERF_BASELINE=docs/research/app-performance-2026-10-23/final.json \
cargo test --locked --release --bin gatto performance::measure_workflows -- \
  --ignored --exact --nocapture
```

Remaining coverage limits: tests do not simulate every live menu-tracking event,
Touch Bar hardware, every media-key permission state, network/proxy behavior,
codec/driver fault, or OS/framework failure. Sanitizers here cover the retained
Objective-C hotkey adapter, not every Rust or system-library operation.

The final arm64 local release app builds and passes ad-hoc signature verification
(`scripts/build-app.sh --release`), at `dist/local/Gatto.app`. The stripped
executable is **26,775,568 bytes (26.78 MB)**, compared with
26,947,888 bytes before this migration: about 172.3 kB smaller.
The local bundle's summed file sizes are 29,191,468 bytes; it excludes Sparkle,
so this is not a published download or installed release size. Both executable
measurements use the existing optimized release profile on this arm64 host.
The pre-migration copy used Cargo's short commit metadata; the final packaged
build embeds the same commit's full hash. Published README figures remain tied
to their measured release version.

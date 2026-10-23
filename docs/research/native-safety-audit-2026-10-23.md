# Native code and unsafe Rust audit

Reviewed October 23, 2026, against app version 0.17.7, commit
`d692a4d0d7c955ad800b2dd0c6bcbe6b70b6870e`, including the local performance
changes. This is an audit and recommendation record; no application code,
dependencies, or behavior were changed for the audit.

Implementation follow-up: [applied Rust conversions and current exceptions](safe-rust-migration-2026-10-23.md).
The inventory below describes the pre-migration code reviewed on this date.

## Assessment

The macOS integrations generally justify a small native implementation. The
ownership discipline is substantially better than an unstructured collection of
raw pointers: native references have owning wrappers, callback storage has a
defined lifetime, cancellation is coordinated, and failed hotkey cleanup detaches
callbacks before freeing Rust state. I found no confirmed memory-safety defect
in the reviewed first-party paths. That is a scoped review result, not proof that
the app or its dependencies cannot contain one.

There are worthwhile changes to make before accepting these interfaces as the
long-term design. In particular, dependency reduction alone does not establish
that maintaining a handwritten native implementation is the safer choice. The
best next step is to strengthen thread/ownership types and reduce duplicated
interfaces, then consider moving status-menu and Finder-service ownership into
the existing Rust bindings. A wholesale rewrite of the uploader or updater is
not needed to make progress.

## What Rust can and cannot guarantee here

Safe Rust checks borrowing and ownership in Rust code. At a foreign call, someone
must additionally establish the native argument types, object lifetime, buffer
extent, threading rules, and callback behavior. An `unsafe` block marks that
responsibility; it does not disable normal Rust checks throughout the program.
The foreign implementation runs in the same process, so a native memory error
would not be contained by the safe Rust around it. Problems can include crashes,
leaks, races, hangs, or corrupted output as well as security vulnerabilities.
[Rust's FFI guidance](https://doc.rust-lang.org/nomicon/ffi.html).

The goal should be a safe Rust core with small reviewed native interfaces.
Removing a `.m` file by moving identical raw pointers into a large Rust `unsafe`
block is not sufficient. Conversely, calls through a safe wrapper can still
depend on a lifetime that the wrapper does not encode; the bitmap example below
is one such case.

The surrounding Rust already uses the advantages requested here: shared immutable
image buffers use `Arc`, native references use `Retained`/Core Foundation owners,
temporary video files use RAII cleanup, and normal file/decode/transport failures
use `Result`. Those patterns should remain the default. Raw handles and callback
contexts should be the exceptions hidden behind those safe interfaces.

## Complete first-party exception inventory

A scan of `src/**/*.rs` found **52 explicit unsafe blocks across nine files**:
51 in shipping paths and one in a compression test. There are also five
`unsafe extern` declaration blocks. There are no app-owned `unsafe fn` definitions,
unsafe trait implementations, `static mut`, or `transmute` calls in these sources.
These counts describe syntax, not the amount of behavior under review: one
compression block encloses hundreds of lines.

| Rust file | Blocks, including tests | Native responsibility | Disposition |
| --- | ---: | --- | --- |
| [`github/native.rs`](../../src/github/native.rs) | 8 | Upload callbacks, response buffers, native result destruction | Keep a small transport interface; retain current lifetime guarantees in any port. |
| [`desktop.rs`](../../src/desktop.rs) | 10 | Own menu/hotkey handles and Rust callback senders | Strengthen main-thread typing; status menus/services are a reasonable migration candidate. |
| [`updater.rs`](../../src/updater.rs) | 9 | Load the packaged updater, discover its class, call selectors, receive a block callback | Keep Sparkle/Swift; replace untyped object messaging with one typed bridge declaration. |
| [`compression.rs`](../../src/compression.rs) | 6, one test-only | Configure AVFoundation, retain samples, sequence decode/encode/cancel/finalize | Keep system codecs; reduce the scope of unsafe code and encode session ownership more explicitly. |
| [`annotation/text/layout/macos.rs`](../../src/annotation/text/layout/macos.rs) | 6 | Core Text line breaking, native font attributes and Core Foundation ownership | Keep the small binding gap; improve bitmap-buffer ownership described below. |
| [`window_limits.rs`](../../src/window_limits.rs) | 5 | Borrow NSView from GPUI, obtain retained NSWindow, window/Dock integration | Keep the necessary GPUI interop; centralize the repeated handle conversion. |
| [`installer_cleanup.rs`](../../src/installer_cleanup.rs) | 4 | Real UID, system property-list parsing and JSON conversion | Small justified OS interface; keep parsing and ejection checks fail-closed. |
| [`rooftops_score.rs`](../../src/rooftops_score.rs) | 3 | Offscreen bitmap allocation, text attributes and PNG export | Native font/rendering exception is reasonable; make graphics-state restoration an owning guard if refactored. |
| [`theme.rs`](../../src/theme.rs) | 1 | Read the native dark-appearance constant | Reasonable isolated binding operation; no meaningful gain from replacing it with a copied string. |

The app-owned foreign implementations are:

| File | Lines at review | Role | Memory model |
| --- | ---: | --- | --- |
| [`native/Upload.m`](https://github.com/jeremyc2/Gatto/blob/d692a4d0d7c955ad800b2dd0c6bcbe6b70b6870e/native/Upload.m) | 268 | Foundation streaming HTTP transport | Objective-C ARC for objects; manual C buffers and callback contexts. |
| [`native/Desktop.m`](https://github.com/jeremyc2/Gatto/blob/d692a4d0d7c955ad800b2dd0c6bcbe6b70b6870e/native/Desktop.m) | 296 | Finder service, status menu, Carbon shortcuts and media-key event tap | Objective-C ARC plus explicit Core Foundation/Carbon resource cleanup. |
| [`native/Updater.swift`](../../native/Updater.swift) | 394 | Sparkle integration, consent, channels and update status | Swift reference counting, `@MainActor`, throwing errors and weak captures. No app-authored unsafe-pointer operations found in this file. |
| [`native/Platform.h`](../../native/Platform.h) | 33 | Shared C-callable interface for the two Objective-C files | Pointer and scalar layout contract, duplicated by handwritten Rust declarations. |

There are no standalone app-owned `.c` or `.cpp` implementations. The `.m` files
are Objective-C, with some C APIs. ARC manages object references but not every C
allocation or callback lifetime. [Clang's ARC specification](https://clang.llvm.org/docs/AutomaticReferenceCounting.html).

The ordinary application menu and custom cursors already call native facilities
through safe Rust interfaces in [`application_menu.rs`](../../src/application_menu.rs)
and [`image_preview/editor/cursor.rs`](../../src/image_preview/editor/cursor.rs).
Other platform calls similarly need no app-written unsafe block when the existing
wrapper can express the contract. These still ultimately execute Apple code.

The separate Swift/Objective-C test harnesses, the DMG-artwork Swift script, and
the feed-merging Swift script are development/release tools, not additional
implementations loaded by the app. They were inventoried to distinguish them from
shipping code; the website and surrounding tooling were not the subject of this
audit. [`build.rs`](../../build.rs) compiles the two `.m` files with ARC and strict
warnings; packaging builds the Swift updater separately with Sparkle.

## Findings and recommended order

### 1. Encode main-thread requirements instead of relying on caller discipline

[`desktop.rs:41`](../../src/desktop.rs), `Menu::new`, and `Hotkey::register`
promise main-thread operation. The owners' `PhantomData<Rc<()>>` prevents sending
them to another thread, but does not establish their thread of construction.
Native constructors reject a background-thread call, and service installation
asserts the requirement. Current application callers run these on the main thread.

The updater has a similar implicit contract: [`updater.rs:185`](../../src/updater.rs)
loads and invokes an `@MainActor` Swift object through `Retained<AnyObject>` and
`msg_send!`, without a main-thread marker in its Rust interface. Swift actor
annotations are not a Rust compile-time guarantee for these manually declared calls.

Use the already available `objc2::MainThreadMarker` at construction and retain it
in main-thread owners. Declare the updater's concrete class and selectors once,
with the correct integer, string and block types. This makes future misuse harder
without rewriting platform behavior or adding a dependency. It is a preventative
interface improvement, not evidence of a current off-thread call.
[`MainThreadMarker` contract](https://docs.rs/objc2/0.6.5/objc2/struct.MainThreadMarker.html).

### 2. Remove the implicit borrowed-buffer lifetime in bitmap glyph drawing

[`annotation/text/layout/macos.rs:192–204`](../../src/annotation/text/layout/macos.rs)
passes `Pixmap`'s mutable data pointer into `CGContext::create_bitmap_context`.
The locked wrapper exposes that operation as a safe function, but the context's
type carries no borrow of the pixel allocation. Correctness still requires that
the allocation stay alive, fixed in place, and unused by Rust while Core Graphics
is writing it.

The current order satisfies those requirements: the local pixmap remains alive,
no competing reference accesses its bytes, and `CTFont::draw_glyphs` consumes the
context before the image is put into an `Arc`. I did not find a dangling pointer
in that sequence. However, moving the context into a cache or returning it would
silently break the unexpressed relationship.

Prefer a Core Graphics-owned bitmap followed by a bounded copy into Rust-owned
pixels, or a small abstraction that cannot let the context outlive the borrowed
pixels. Benchmark the glyph-cache path if choosing the extra copy. Also note that
the existing context constructor asserts on a null native allocation; that failure
becomes a panic rather than the function's `Option` fallback. Improving this error
path may need a fallible wrapper/upstream change and should not introduce more
unreviewed raw code simply to lower a block count.
Verified against the locked `core-graphics 0.25.0` `context.rs` constructor and
`core-text 22.0.0` `font.rs::draw_glyphs` implementations; this is not an allegation
that every safe Core Graphics call is unsound.

### 3. Shrink the video compression unsafe scope

[`compression.rs:300–663`](../../src/compression.rs) encloses settings construction,
validation, channels, loops, progress reporting, native reads/writes and finishing
in one unsafe block. Its documented worker ownership is sound in the paths
reviewed, but a future unsafe call added anywhere in that region receives no new
syntactic review checkpoint.

Keep Apple decoding and hardware encoding. Extract a few small operations around
native session construction, sample reading/appending, and finalization, leaving
policy, budget calculations, progress and errors in safe Rust. Maintain `Retained`
ownership, single-worker ordering, cancellation in Drop, and completion blocks
that capture owned channel senders rather than borrowed stack state. Do not
replace the native codecs with custom parsing/encoding to remove an exception.

### 4. Reduce duplicated native ownership where it produces a simpler interface

`Platform.h` and the Rust declarations separately describe the same scalar
layouts, raw handles, callback signatures and destructor obligations. They agree
at review, including `Response`'s `#[repr(C)]` layout and allocator pairing. That
agreement is nevertheless maintained by hand.

The status-menu and Finder-service portion is the best substantial migration
candidate. Existing `objc2` packages contain the native class/protocol bindings.
Typed retained objects and Rust-owned delegate fields can remove opaque owners
and raw sender pointers. Some framework features need enabling; verify the
resolved graph rather than assuming this cannot activate another package.

Keep the Carbon/media-key portion small if a port merely duplicates the same
low-level declarations. A full Foundation upload port is feasible but more
involved: streamed-body readiness, watchdogs, delegate threading and invalidation
are real implementation responsibilities, not incidental syntax. Preserve all
the controls below and test them before replacing the current adapter.
In particular, adding a timeout that simply returns while native callbacks can
still access borrowed Rust state would introduce a lifetime bug. A nonblocking
shutdown design would first need independently owned callback state and a defined
point at which native access ends.

## Exceptions that currently earn their keep

- **Upload lifetime and privacy controls:** Rust's reader is mutex-owned; shared
  control/progress state has appropriate thread bounds; read/progress panics are
  caught before returning through C. The worker waits for session invalidation
  before stack callback state is released. Response allocations are returned to
  `free` through the native destructor and Rust Drop, rather than being adopted
  by a Rust allocator. Foundation uses an ephemeral session, default OS trust,
  no redirects or POST replay, and a 64 KiB response cap. These contracts matter
  more than whether the implementation file ends in `.m` or `.rs`.
- **Hotkey cleanup failure:** native code deliberately clears callback pointers
  and leaks a detached owner if the OS refuses handler cleanup. Rust then can
  release its sender without leaving a dangling callback. Keeping that native
  allocation is a justified exceptional safety tradeoff. There is no demonstrated
  bound on repeated failures; it should not be described as an always-bounded
  leak. Failure-injection tests would strengthen confidence in these rare paths.
- **Updater loading:** the library path is bundle-relative, loader errors become
  `Result`, and [release packaging](../../scripts/release.sh) uses hardened-runtime
  signing without disabling
  library validation. The library remains loaded for process lifetime because
  registered classes and callbacks cannot survive unloading. A source build is
  not equivalent to a signed release's loader protections. Keeping Swift/Sparkle
  is reasonable: this is already memory-managed native integration, and replacing
  the security-sensitive installer is not justified for language uniformity.
- **Core Text typesetter:** the two direct typesetter declarations fill a real
  gap in the locked `core-text` wrapper. The created object is checked for null
  and released by an owning `CFType`; UTF-16 break positions are mapped to bounded
  Rust grapheme ranges. Keep this small exception unless an equivalent existing
  binding covers it. Do not replace contextual shaping with a different wrapping
  algorithm just to remove native calls.
- **GPUI/AppKit window conversion:** raw NSView pointers come from a borrowed GPUI
  window handle, references stay within that borrow, and deferred close requests
  capture a retained NSWindow rather than a borrowed raw view. Centralizing that
  conversion would reduce repetition. The hide-and-reuse close workaround has its
  own [accepted ADR](../adr/0001-retain-macos-windows-to-avoid-touch-bar-crashes.md);
  removing it solely to eliminate native calls would risk restoring a known crash.
- **Small Foundation/AppKit calls:** UID lookup, property-list parsing, dark
  appearance, bitmap PNG allocation and native font attributes are ordinary
  platform work. Data is copied or retained before temporary native objects end.
  Most surrounding validation/error handling already remains in safe Rust.

## History and tradeoff

The Objective-C upload and desktop files were introduced for dependency reduction,
replacing reqwest/Tokio/futures-util and tray-icon/global-hotkey. I found no
measurement establishing a performance reason for choosing Objective-C. The Swift
updater predates that change and was introduced to integrate Sparkle.
The [primary-source alternatives investigation](native-boundary-alternatives-2026-10-23.md)
records commits, framework sources, feature constraints and migration options.

A sensible policy is to prefer generated/typed Rust wrappers already in the graph,
keep unavoidable foreign operations in small platform modules, and document why
each pointer/thread/callback contract holds. Safe-core lint enforcement can help
prevent new unsafe operations from spreading. Do not replace native ownership
helpers with handwritten bindings solely to save a manifest entry; assess the
resulting maintenance and safety obligations, not just dependency count.

## Verification and limits

During this audit:

- Clang static analysis of both shipping Objective-C files reported no diagnostics.
- `native/DesktopTests.m` passed with AddressSanitizer and UndefinedBehaviorSanitizer,
  exercising menu/service lifecycle and event dispatch. This does not cover every
  OS hotkey failure or a live menu tracking session.
- Six Rust GitHub tests passed, including real local HTTP streaming, cancellation,
  timeout/idle classification, redirect/authentication refusal, response caps and
  replay prevention. No real credentials or GitHub upload were used.
- Six compression tests passed, including native H.264/AAC output, decoded
  dimensions, timing, cancellation and source preservation.
- The platform harness passed native menu, Finder cold/warm delivery and shortcut
  registration/replacement/stale-event checks.
- The Swift updater harness compiled with warnings as errors and passed using its
  fake client and isolated preferences, including channels, consent and retry.

The source review covers every explicit first-party unsafe site, both C-callable
interfaces and all three shipping foreign implementations. Safe Rust was also
searched for raw-pointer/ownership escape routes, subprocess calls and relevant
error handling. It is not a line-by-line correctness proof of every UI state,
a formal memory-safety proof, a dependency-source audit, or exhaustive testing of
Apple frameworks, GPU/codec implementations and Sparkle. SDK calls, third-party
wrappers and the Rust standard library remain trusted implementation layers.

Recommended implementation sequence: main-thread/typed-updater interfaces;
bitmap ownership and smaller media operations; then status-menu/Finder migration.
Keep current behavior and the performance/rendering baselines as acceptance tests
for each separate change.

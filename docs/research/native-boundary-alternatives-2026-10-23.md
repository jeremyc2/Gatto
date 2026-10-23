# Native boundary history and safer alternatives

Researched 2026-10-23 against Gatto `d692a4d0d7c955ad800b2dd0c6bcbe6b70b6870e`
and the current working tree. Scope: the shipping app's Objective-C/Swift adapters
and opportunities in the already installed Rust framework bindings. This note
does not audit every line or every transitive dependency and changes no app code,
dependencies, or behavior. The concurrent performance work is outside this review.

Implementation follow-up: [applied Rust conversions and current exceptions](safe-rust-migration-2026-10-23.md).
The inventory below describes the pre-migration code reviewed on this date.

## What changed, and why

The app-owned native files are **Objective-C**, not standalone C implementations:
[`Upload.m`](https://github.com/jeremyc2/Gatto/blob/d692a4d0d7c955ad800b2dd0c6bcbe6b70b6870e/native/Upload.m) and [`Desktop.m`](https://github.com/jeremyc2/Gatto/blob/d692a4d0d7c955ad800b2dd0c6bcbe6b70b6870e/native/Desktop.m).
They use Apple frameworks and some C APIs/raw buffers. [`build.rs`](../../build.rs)
compiles both with automatic reference counting (`-fobjc-arc`), blocks, and strict
warnings. ARC manages Objective-C object references, but C allocations, raw callback
contexts, and Core Foundation references still require explicit contracts.
[Clang ARC specification](https://clang.llvm.org/docs/AutomaticReferenceCounting.html)

Both files were introduced in
[`1623641`](https://github.com/jeremyc2/Gatto/commit/162364161193c19819eb0c9ef9e09e51f7fbe3fd)
on October 22. The accompanying manifest diff and
[dependency-review implementation record](direct-dependency-review.md#implementation-follow-up-2026-10-22)
establish the purpose: replace the app's reqwest/Tokio/futures-util transport,
tray-icon/global-hotkey desktop adapters, and libloading loader abstraction without
adding replacement third-party packages. The record reports **35 fewer reachable
macOS Rust normal/build package versions** across the combined changes. This is a
supply-chain/code-graph tradeoff, not proof of better memory safety. I found no
benchmark in that change establishing that Objective-C itself was chosen for speed.

[`Updater.swift`](../../native/Updater.swift) predates that reduction: it was added
in [`682178b`](https://github.com/jeremyc2/Gatto/commit/682178bd3d12963aaa5712c03b0101897dfb9226)
on October 19 to integrate Sparkle. It delegates archive verification, replacement,
and relaunch to Sparkle and implements Gatto's consent/status policy. The later
dependency reduction changed the Rust loader; it did not introduce Swift to replace
a Rust updater. Swift is a memory-managed language, and this file already uses
`@MainActor`, typed protocols, throwing errors, and weak captures. Its remaining
foreign boundary is the dynamically discovered Objective-C class/selectors and
block ABI in [`updater.rs`](../../src/updater.rs).
[Sparkle API reference](https://sparkle-project.org/documentation/api-reference/),
[Sparkle security settings](https://sparkle-project.org/documentation/customization/)

## What can become safer without replacing dependencies

| Boundary | Existing option | Judgment |
| --- | --- | --- |
| Main-thread desktop ownership | `objc2::MainThreadMarker::new()` validates thread origin; keeping the marker in an owner prevents sending it across threads. | Best small improvement. Current `PhantomData<Rc<()>>` prevents transfer but does not itself establish that construction happened on the main thread. Native constructors also check the thread; `install_file_service` instead relies on an assertion and caller discipline. |
| Status menu and Finder service | The already declared `objc2-app-kit`/`objc2-foundation` packages contain NSStatusBar/NSStatusItem/NSMenu/NSMenuItem/NSPasteboard and application-delegate bindings; `objc2::define_class!` supports Rust-owned Objective-C classes with typed Rust ivars. | Feasible migration target. Enable the necessary existing crate features, use `Retained<T>` and a main-thread owner, and retain a small reviewed target/action or delegate declaration. This can eliminate opaque `void *` menu owners and the duplicated handwritten C ABI, but is not a zero-unsafe rewrite. |
| HTTP uploads | `objc2-foundation` 0.3.2 contains NSURLSession/NSURLSessionConfiguration/NSStream and delegate protocols. | Feasible but substantial. The session-with-delegate constructor, stream scheduling/buffer operations, and delegate implementations still require unsafe declarations or calls. Porting the current transport mechanically could increase Rust unsafe blocks while improving ownership typing. Preserve streaming, byte caps, cancellation, no replay, redirect refusal, OS trust validation, and session invalidation before releasing callback state. |
| Global hotkeys/media keys | Ordinary shortcuts use Carbon registration; media shortcuts use a Core Graphics event tap and AppKit event interpretation. | A platform exception remains. Merely copying the Carbon declarations into Rust preserves the same C ABI/lifetime obligations and gains little. Keep this boundary narrow; move the pure key-name/modifier mapping to safe Rust if later refactoring materially improves clarity. |
| Sparkle updater | A typed `extern_class!`/`extern_methods!` declaration for GattoUpdaterBridge could replace repeated `msg_send!` calls in Rust. | Reasonable local cleanup, but selectors/ABI still need one audited declaration. Keeping the Swift adapter is justified: it fits Sparkle's native APIs, actor isolation, and errors. Reimplementing the updater or security-sensitive installer in Rust is not justified merely to remove a language. |

These are available APIs in the **locked source**, not promises about hypothetical
future versions. Verified source: objc2 0.6.5
[`MainThreadMarker`](https://github.com/madsmtm/objc2/blob/d7d2fa23ceaa5e6096c923b081040e5d81b3b9df/crates/objc2/src/main_thread_marker.rs),
objc2 framework 0.3.2
[`NSURLSession`](https://github.com/madsmtm/objc2/blob/7b1abfd750a2cacaea71d6a56ecfb83cb7de560b/framework-crates/objc2-foundation/src/generated/NSURLSession.rs),
[`NSStream`](https://github.com/madsmtm/objc2/blob/7b1abfd750a2cacaea71d6a56ecfb83cb7de560b/framework-crates/objc2-foundation/src/generated/NSStream.rs),
and [`NSStatusBar`](https://github.com/madsmtm/objc2/blob/7b1abfd750a2cacaea71d6a56ecfb83cb7de560b/framework-crates/objc2-app-kit/src/generated/NSStatusBar.rs).
The [objc2 example](https://docs.rs/objc2/0.6.5/objc2/) demonstrates main-thread
classes with Rust ivars; [`Retained`](https://docs.rs/objc2/0.6.5/objc2/rc/struct.Retained.html)
owns native references through Drop. Feature changes must still be checked with
Cargo metadata and the lockfile: using an existing package does not automatically
guarantee that no additional transitive package or framework feature is activated.

Core Text also already has safe call surfaces for lines, glyph runs, positions,
paths, glyph bounds, and glyph drawing; Gatto uses these in
[`annotation/text/layout/macos.rs`](../../src/annotation/text/layout/macos.rs).
The locked core-text 22.0.0 source has no typesetter module or wrapper for
`CTTypesetterCreateWithAttributedString`/`CTTypesetterSuggestLineBreak`, so its
small raw declaration is a real binding gap. Replacing it with a different wrapping
algorithm risks changing contextual line breaks. Keeping the small declaration and
owning its result with `CFType` is a reasonable exception until an existing binding
covers it. [Locked Core Text source](https://github.com/servo/core-foundation-rs/tree/d96fc4a040efc665969601b29805e31ecabf4ef7/core-text/src)

## Contracts that matter more than counting unsafe blocks

- **Thread origin and reentrancy:** keep AppKit mutation on its main thread.
  Native callbacks should enqueue identifiers/status and avoid borrowing live GPUI
  views. Gatto currently does that. A main-thread marker strengthens enforcement;
  a `!Send` owner alone only prevents subsequent transfer.
  [Apple thread-safety guide](https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/Multithreading/ThreadSafetySummary/ThreadSafetySummary.html)
- **Asynchronous lifetime:** stopping an upload task is not enough to free Rust
  callback state. The current upload implementation waits for delegate invalidation;
  preserve this join in any rewrite. Desktop teardown clears callbacks before
  releasing Rust senders; on failed unregister it deliberately detaches/leaks the
  native owner rather than leaving a dangling callback. This exceptional
  cleanup policy is preferable to a use-after-free.
  [Rust FFI callback guidance](https://doc.rust-lang.org/nomicon/ffi.html#asynchronous-callbacks),
  [Apple session invalidation](https://developer.apple.com/documentation/foundation/urlsession/invalidateandcancel())
- **Allocation ownership:** upload response buffers come from `malloc`/`strdup`
  and return to the same native allocator through `GattoUploadResultFree`, wrapped
  in Rust Drop. Do not reconstruct a Rust Vec/String directly from those pointers.
  A typed Rust transport could remove this extra allocation boundary entirely.
  [`github/native.rs`](../../src/github/native.rs),
  [`Platform.h`](../../native/Platform.h),
  [Rust FFI destructor guidance](https://doc.rust-lang.org/nomicon/ffi.html#destructors)

No concrete exploitable defect was established by this high-level alternatives
investigation. That is not a memory-safety proof. The main improvement is to encode
thread and ownership invariants in types and reduce duplicated ABI definitions;
moving native calls between languages does not remove the operating-system trust
boundary. Prioritize small enforceable changes, then migrate desktop ownership if
its resulting implementation is simpler. Treat a full upload port as a separately
validated change, not as a prerequisite for accepting this audit.

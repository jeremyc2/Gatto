# Dependency reduction and supply-chain review

Reviewed **2026-10-21**, against Gatto commit
`b8ec31770406813dffccbe18ccccf08b20c4bd45` and live primary sources.

Scope: every direct package in `Cargo.toml`, `package.json`, and
`web/package.json`, excluding GPUI and GPUI-Kit: **29 Rust packages and 15
JavaScript packages**. Sparkle is reviewed separately because it is downloaded
and bundled outside either package manager. GitHub CLI is also noted as an
external runtime requirement. Operating-system frameworks and the complete
transitive graph are not individually audited here.

The objective is to reduce independently supplied code and publishing authority,
while keeping Gatto's behavior. This is a review and recommendation document;
no dependencies, implementation, or security-policy exceptions were changed.
Current architecture and security instructions remain in
[Architecture](../ARCHITECTURE.md) and [Security](../../SECURITY.md).

## Implementation follow-up (2026-10-22)

The original inventory and assessments below describe the reviewed source at the
date above. The subsequent implementation removes **six direct Rust dependencies**
(`reqwest`, `tokio`, `futures-util`, `tray-icon`, `global-hotkey`, `libloading`) and
**three JavaScript dependencies** (`toastify-js`, its types, and `typescript`).
No replacement third-party package was added. Foundation/AppKit/Carbon adapters
are tracked source compiled by Apple's installed tools; Core Text, image/Unicode
handling, serialization and Sparkle remain.

Measured against implementation starting commit `7372b41`:

| Measurement | Before | After | Removed |
| --- | ---: | ---: | ---: |
| Reachable macOS Apple-silicon normal/build Rust package versions, using the metadata traversal described below | 533 | 498 | **35** |
| Bun lock entries, including all platform binaries and the workspace entry | 221 | 198 | **23** |

The combined changes remove additional Tokio-related macOS nodes that were
retained when considering each root independently. A Tokio lock entry can still
exist for other target configurations; this is not a claim about every target's
graph. The later implementation starting commit had 221 Bun entries rather than
the original review snapshot's 214. Comparing the actual starting/final lockfiles
confirms 23 removed entries and no added entries. Release DMG/bundle size figures
remain tied to their existing measured release until a new release is published.

The website uses its existing notification styling and a manual-popover/fixed
fallback; both TypeScript projects use Bun's built-in checker. Builds require
installed local executables, explicit script-disabled installation, and a
JavaScript audit gate. The one lint advisory remains visible under the expiring
[Security policy exception](../../SECURITY.md); it has not been fixed or globally
ignored. Native upload, main-thread shortcut/menu checks and release workflow
guards are documented in [Contributing](../../CONTRIBUTING.md).

Release compilation/check subprocesses strip inherited release-token variables;
Bun runtime/check/test commands disable dotenv loading. Bun 1.4.3's built-in
package-manager commands still load dotenv files despite the flag, so they are
not a credential boundary; installation scripts are disabled. Signing credential preflight follows the final
unsigned build. This is environment/phase separation, **not** isolation from the
same user's Keychain, files or CLI credential stores. Stronger isolation still
requires a separate credentialless build environment; no infrastructure or
credential access was provisioned by this implementation.

## Original recommended order

| Priority | Proposed change | Prospective graph reduction | Cost and judgment |
| --- | --- | --- | --- |
| 1 | Replace the website's single Toastify notification using the existing component; remove `toastify-js` and its types | **2 JavaScript entries** | Small scope and a real browser-library removal. Preserve announcement, focus, timer and dismissal behavior. |
| 2 | Use Bun's built-in checker for both TypeScript projects; remove the standalone `typescript` package | **21 JavaScript entries**, mostly mutually exclusive platform binaries | Both current configurations already pass with installed Bun 1.4.3. Validate a clean install without TypeScript and editor behavior before removing it. Keep typed lint checks. |
| 3 | Stop package executables from auto-installing missing tools; explicitly constrain install scripts; add JavaScript audit/update coverage | No promised package-count change | Directly addresses executing a newly fetched or unexpectedly trusted package. More immediate supply-chain value than deleting harmless manifest entries. |
| 4 | Implement the narrow GitHub upload adapter with macOS URLSession; remove our reqwest integration | **23 Rust package versions** in the current fixed graph | Largest meaningful app opportunity. Moderate, security-sensitive integration: streaming, progress, cancellation, redirects, response limits, deadlines and POST replay behavior must match. Tokio/futures-util can cease being direct dependencies if unused afterward, but GPUI retains them. |
| 5 | Evaluate native menu-bar and hotkey adapters together | **7 Rust package versions**, including two packages with build scripts | Feasible for a macOS-only app, but new native ownership/event/accessibility code becomes our responsibility. Prototype and compare complexity before committing to a rewrite. |
| 6 | Simplify updater bridge loading/linking and remove `libloading` | **1 Rust package version** | Bounded possible removal, but retain signed bundle-relative loading, bridge registration, Sparkle discovery and normal source-build behavior. Direct `dlopen` still loads privileged app code. |
| Defer | Replace serialization, URL parsing, Unicode segmentation, image codecs, or the rasterizer just to lower direct counts | **0 packages individually** for these roots in the current graph | Existing consumers, principally GPUI, retain them. Reimplementing mature algorithms would add our own correctness/security burden with no measured graph reduction. |
| Defer | Migrate Servo Core Foundation/Graphics/Text bindings to the already-used objc2 family | **0 existing packages removed** while GPUI retains Servo | Can improve native API consistency, but currently does not remove those publishers. Do it for maintainability, rather than claim supply-chain elimination. |

The two small website removals together could remove **three direct declarations
and 23 lock entries**. The native changes are separate projects, not approvals to
rewrite existing functionality. These estimates are not reductions already made.

### What the counts mean

Rust estimates prune root dependency edges from the resolved **Apple silicon
macOS normal/build graph**, containing 533 external package versions. Metadata
was obtained with `cargo metadata --locked --offline --filter-platform
aarch64-apple-darwin --format-version 1`; traversal excludes edges whose only
kind is `dev`. Existing feature resolution is retained. A real change must
re-resolve features and review its lockfile and compiled target graph before
claiming the final count. Intel macOS needs its own validation.

The JavaScript analysis walks `bun.lock` from the 15 manifest roots through
dependencies, optional dependencies and present peer dependencies. It finds 210
reachable entries out of 214 total lock entries, including packages for other
operating systems. Removing a root and repeating traversal gives the estimates.
These are **not 210 packages all executing on a Mac**. Twenty of the 21 potential
TypeScript removals are platform binaries. Package counts also do not equal
independent publisher counts: many packages share one maintainer family.

Counts within one ecosystem are not generally additive because branches share
packages. Removing tray-icon alone prunes four packages, global-hotkey alone
two, and both seven. Removing a direct dependency can still reduce local API
coupling or enabled features even if no package disappears, but does not remove
its publisher from the current build's trust boundary.

## How author trust was assessed

For each package, this review checks its actual use, the locked release, official
repository, registry publishing owners, current maintenance, known advisories,
and exposure to credentials, external input or build-time execution. Registry
owner records are stronger evidence of who can publish than an `authors` field.
GitHub/npm provenance is useful evidence connecting an artifact to a repository
and workflow, but does not prove that workflow or its source is benign.

This review does not assign people personal trust scores, verify their account
security, enumerate every publishing team's members, or establish a formal code
audit. A solo maintainer is a concentration/continuity consideration, not evidence
of malicious intent. A large organization is not an exemption from review.

Important concentrations include David Tolnay's authority over error-handling
and serialization packages, the small objc2 publishing group behind native
bindings, Oxc's publishing account, and Solid's publishing maintainers. Splitting
each family's work across more packages does not provide independent assurance.
Conversely, hand-copying native bindings would move ABI/lifetime maintenance into
our code while leaving the same underlying OS APIs and many transitive packages.

## Fresh checks and current controls

- **Rust:** `cargo-deny 0.20.2` passed advisories, bans, licenses and sources for
  the configured macOS targets, using RustSec database commit
  `4b587f00996ebe5c05a73eaef105ef241e876414` (2026-10-21).
  `unmaintained = "workspace"` means transitive maintenance notices still exist;
  a passing policy check is not a statement that every indirect package is
  maintained. No ignores were added. `cargo-audit` is not installed, so this is
  not a new independent cargo-audit or OSV scan of the Rust graph.
- **JavaScript:** `bun audit --json` exited 1 with the high-severity
  [braces advisory GHSA-vfj7-8cjw-p6xm](https://github.com/advisories/GHSA-vfj7-8cjw-p6xm).
  The installed route is StyleX lint plugin → micromatch → braces, not the
  production browser code. Inspected StyleX calls use a micromatch path that
  bypasses the affected recursive walkers; this is a scoped static inspection,
  not a full exploitability proof. The advisory lists no patched version. Do not
  describe the entire JavaScript graph as clean because the 15 direct versions
  have no indexed OSV findings.
- **Existing protections:** both lockfiles are committed; Cargo registry sources
  and licenses are constrained; JS direct versions are exact; release scripts
  use frozen Bun installs and run Rust policy checks. Sparkle has a pinned SDK
  archive checksum, and distributed code is signed and notarized.
- **Gaps worth addressing:** `.github/dependabot.yml` covers Cargo and GitHub
  Actions, not the JavaScript manifests. Security workflows are retained under
  `.github/workflows.disabled/`, so their presence does not provide ongoing CI
  enforcement. `scripts/release.sh` checks Rust advisories but does not currently
  run Bun's vulnerability audit. These are observed configuration gaps, not
  evidence that a malicious package is present.

### Hardening with greater value than cosmetic dependency removal

1. **Fail closed when a local tool is missing.** Current scripts use
   `bunx --bun` without `--no-install`. Bun documents that this can fall back to
   downloading a package, outside the intended frozen project install. Use the
   already installed binaries with `bunx --no-install --bun`; for a differently
   named binary such as `tsc`, be explicit about its package if it is retained.
   No need to permit a release job to discover a new executable from the registry.
   [Official bunx behavior and flags](https://bun.com/docs/pm/bunx).
2. **Make install-script policy explicit.** Neither manifest specifies
   `trustedDependencies`, so Bun's built-in popular-package allowlist applies.
   `bun pm untrusted` reported no blocked scripts, which does not establish that
   scripts are globally prohibited. Evaluate `trustedDependencies: []` or
   `--ignore-scripts` with a fresh install and all checks; allow only demonstrated
   requirements. A script-free install still executes dependency code during
   builds, so this is one layer rather than a sandbox.
   [Official lifecycle policy](https://bun.com/docs/pm/lifecycle).
3. **Review dependency changes as artifact changes.** Check newly added packages,
   registry owners/transfers, build scripts/macros, native binaries, install
   scripts and provenance. Keep frozen/locked resolution. Checksums establish
   artifact consistency; they faithfully preserve a compromised release too.
4. **Separate building from signing/publishing authority where practical.**
   Developer/build packages can execute with the user's permissions and alter
   outputs. A clean, minimally credentialed build followed by a distinct signing
   and publishing step limits exposure beyond what dependency counting achieves.
   A full sandbox/build-system redesign was not performed in this review.
5. **Add monitored JavaScript audit/update coverage** and preserve the scoped
   braces assessment until upstream fixes or removes the branch. Do not remove
   the useful StyleX rules merely to hide an advisory. Prerelease Solid upgrades
   should stay coordinated and deliberate; newer releases are not automatically
   safer than reviewed, pinned ones.

## Separately bundled and external dependencies

### Sparkle 2.10.0

`scripts/prepare-updater.sh` downloads the official Sparkle archive, verifies
SHA-256 `c2bf58aa8387266ac179357b1415d6f2635f044da8be41042af32425dae6da0c`,
and compiles `native/Updater.swift` against it. Sparkle supplies update scheduling,
signed feed/archive validation, installation, recovery and relaunch. It is a
high-consequence dependency because installing new app code is its purpose.

It is maintained by the Sparkle Project, with a public release history and
maintainer-published security fixes. The official release page identifies 2.10.0
as latest at review time. The security page lists five advisories: the two
August path/privileged cleanup issues are patched in 2.9.6; the incomplete delta
fix is patched in 2.9.5; the two May issues are patched in 2.9.2. Our 2.10.0 is
outside those published affected ranges. This is not an independent audit of the
SDK binary or a guarantee against undisclosed issues.
[Official project](https://sparkle-project.org/),
[2.10.0 release](https://github.com/sparkle-project/Sparkle/releases/tag/2.10.0),
[published advisories](https://github.com/sparkle-project/Sparkle/security),
[path fix](https://github.com/sparkle-project/Sparkle/security/advisories/GHSA-3x7w-j75x-ppq5),
[cleanup fix](https://github.com/sparkle-project/Sparkle/security/advisories/GHSA-4v99-qgq9-6pxp),
[delta follow-up](https://github.com/sparkle-project/Sparkle/security/advisories/GHSA-gmj2-gq3j-vqmj),
[original delta issue](https://github.com/sparkle-project/Sparkle/security/advisories/GHSA-hg88-v3cw-3qrh),
[XPC metadata issue](https://github.com/sparkle-project/Sparkle/security/advisories/GHSA-g3hp-f6mg-559v).

**Keep.** Replacing a mature updater with our own download/verification/privileged
replacement machinery would expand the code we must secure. Treat Sparkle updates
and the release signing keys as a separate security review surface; Cargo audits
do not cover this SDK. The pinned checksum checks the selected archive's identity,
not whether every line of that release is trustworthy.

### GitHub CLI and toolchains

Gatto invokes the user's GitHub CLI to obtain the token and query GitHub; its
Homebrew cask declares `gh` as a dependency. It is an official GitHub project with
private security reporting and public advisories. Keep using its existing
authentication integration rather than adding a second credential store.
The local observed CLI is 2.102.0; end-user installations are not universally
pinned to that version. We did not audit the CLI's Go dependency graph or every
published advisory in this review.
[GitHub CLI security process](https://github.com/cli/cli/security).

Bun, Rust/Cargo, Apple's compiler/SDK/signing tools, Git, and Cloudflare's deploy
CLI are build/runtime infrastructure, not additional package-manager declarations
in the 44-package inventory. They remain trusted execution surfaces. Reusing Bun's
available checker removes a separate distributed compiler package; it does not
remove the need to trust a type checker. No toolchain, CLI or platform migration
is proposed solely to reduce a count.


## Rust application libraries (excluding GPUI and GPUI-Kit)

This section checks the 13 general-purpose direct Rust dependencies against local call sites and first-party upstream/registry records on October 21, 2026. Versions below are **locked versions**, not just manifest ranges. “Publisher” means an account returned by the crates.io owners API; manifest `authors` fields are self-reported attribution and are not proof of publication authority. Team owners can have multiple members; this review did not enumerate those memberships, audit publisher accounts, or verify account security. Release dates and owner lists are point-in-time observations, not guarantees of safety.

### Reduction opportunities, in priority order

The most meaningful option in this group is **replacing our narrow reqwest upload adapter with macOS URLSession**. This hands networking/TLS implementation to the OS and can eliminate our direct reqwest/Tokio/futures-util coupling without writing a new HTTP client. The fixed-graph reachability check finds **23 packages reachable only through the reqwest root edge**, including its Hyper 1.x/TLS wrappers; that is the opportunity to validate after an actual Cargo feature re-resolution. Tokio and futures-util themselves remain elsewhere through GPUI. Removing any of the other 12 direct edges individually removes **zero** package nodes in that fixed graph. Those direct-entry cleanups can simplify application coupling but cannot honestly be claimed to remove their authors from the build's trust boundary.

URLSession is a realistic macOS-only replacement: Apple documents [file-backed uploads](https://developer.apple.com/documentation/foundation/urlsession/uploadtask(with:fromfile:)), [upload progress callbacks](https://developer.apple.com/documentation/foundation/urlsessiontaskdelegate/urlsession(_:task:didsendbodydata:totalbytessent:totalbytesexpectedtosend:)), [task cancellation](https://developer.apple.com/documentation/foundation/urlsessiontask/cancel()), and [redirect refusal in default/ephemeral sessions](https://developer.apple.com/documentation/foundation/urlsessiontaskdelegate/urlsession(_:task:willperformhttpredirection:newrequest:completionhandler:)). Gatto already uses Foundation/objc2 bindings and owns staged video files. Image bytes could use a data upload or a private staged file. This is **moderate integration work with sensitive behavioral requirements**, rather than a global networking rewrite: add a Foundation delegate adapter, map callbacks into existing upload control/progress, cap accumulated response data, preserve URL validation, disable redirects, preserve timeout/cancellation behavior, and verify non-idempotent POST replay/retry semantics. Apple notes that background sessions automatically follow redirects, so this adapter should use a default/ephemeral session. The current local HTTP tests provide useful behavior checks, but real GitHub/TLS and cancellation checks would also be needed. No claim is made that Apple's stack is vulnerability-free; the benefit is reducing independently shipped Rust networking code and publishers.

**Recommend a focused URLSession migration proposal as the next reduction project**, subject to proving the behavior above and measuring the final lockfile delta. If retaining reqwest, removing its unused `json` feature is a small low-risk cleanup, with little likely supply-chain benefit because serde_json is independently required.

Keep URL parsing, Unicode segmentation, image codecs, temporary-file handling, and the existing annotation rasterizer until there is a separate product or architectural reason to change them. Recreating those algorithms solely to remove direct manifest entries adds review burden while GPUI retains the dependencies. Native alternatives can be considered within real feature work, but this review does not recommend a broad rewrite.

### Error handling and serialization share maintenance concentration

These four dependencies are useful, but counting them as four independent sources of assurance would be misleading: David Tolnay (`dtolnay`) has publication access to all four. A compromised account could affect several of our foundations. This is a reason to review updates and retain lockfiles, rather than grounds for a personal judgment about a maintainer.

| Direct package | Why Gatto uses it / exposure | Verified maintenance and publication evidence | Judgment |
| --- | --- | --- | --- |
| **anyhow 1.0.104** | Context-rich application errors throughout media staging, settings, compression, shortcuts, and updater loading (`src/model.rs`, `src/settings.rs`, `src/compression.rs`, `src/updater.rs`). Handles errors from untrusted files and OS operations; it is not itself a media/network parser. Its build script executes during compilation. | Sole listed registry owner: **dtolnay / David Tolnay**. Locked version is current latest, released **2026-07-18**. The project lives under the same maintainer's GitHub account. [Registry](https://crates.io/api/v1/crates/anyhow), [publishers](https://crates.io/api/v1/crates/anyhow/owners), [source](https://github.com/dtolnay/anyhow). | **Keep.** Replacing it with boxed errors would add repetitive error plumbing for little practical reduction. Review its source/build script when upgrading because it participates in our build trust boundary. |
| **thiserror 2.0.21** | Derives `Error`/`Display` for structured `GithubError`, media/compression errors, and custom URL errors (`src/github.rs`, `src/model.rs`, `src/compression.rs`, `src/custom_url.rs`). This is build-time procedural-macro/code-generation trust; the generated errors classify runtime failures. | Sole listed owner: **dtolnay**. Latest release **2026-09-23**, matching lock. [Registry](https://crates.io/api/v1/crates/thiserror), [publishers](https://crates.io/api/v1/crates/thiserror/owners), [source](https://github.com/dtolnay/thiserror). | **Keep.** It has a different role from anyhow: typed errors where callers need categories, contextual errors for application orchestration. Removing it means handwritten trait implementations, not a new functional capability. |
| **serde 1.0.229** | Derives serialization for settings, annotation state, clipboard payloads, GitHub responses, and installer metadata. Gatto enables `derive`, so its transitive `serde_derive` procedural macro runs at compile time. Runtime code interprets local settings, pasted metadata, and server responses. | Owners: **dtolnay** and **github:serde-rs:publish**. Latest release **2026-07-18**, matching lock. Attribution also names Erick Tryzelaar, but that is not the current owners list. [Registry](https://crates.io/api/v1/crates/serde), [publishers](https://crates.io/api/v1/crates/serde/owners), [source](https://github.com/serde-rs/serde). | **Keep.** Strong fit for typed persistence and protocol data; replacing it with native Foundation serialization would still require handwritten schema conversion. |
| **serde_json 1.0.154** | Actual JSON parsing/encoding for settings, annotation clipboard metadata, GitHub responses, and installer metadata (`src/settings.rs`, `src/image_preview/editor/clipboard.rs`, `src/github.rs`, `src/installer_cleanup.rs`). This does parse external data. GitHub upload bodies are bounded to 64 KiB before parsing in `src/github.rs`. | Owners: **dtolnay** and **github:serde-rs:publish**. Latest release **2026-10-11**, matching lock. [Registry](https://crates.io/api/v1/crates/serde_json), [publishers](https://crates.io/api/v1/crates/serde_json/owners), [source](https://github.com/serde-rs/json). | **Keep.** Serde is the framework; serde_json supplies the JSON format. They are complementary dependencies. Continue applying limits at each external-input boundary rather than assuming a trusted parser makes arbitrary payload sizes safe. |

### Networking, asynchronous work, and URLs

| Direct package | Why Gatto uses it / exposure | Verified maintenance and publication evidence | Judgment |
| --- | --- | --- | --- |
| **reqwest 0.13.5** | Performs the streamed GitHub attachment POST (`src/github.rs`). Receives the **GitHub bearer token and attachment bytes**, so this is one of our most sensitive dependencies. Current code sets connect/total/idle timeouts, disables redirects and retries, limits response size, and validates returned attachment URLs. `native-tls` uses the macOS TLS framework. | Sole listed owner: **seanmonstar / Sean McArthur**. Latest release **2026-09-08**, matching lock. Upstream has source, CI, documented TLS behavior, and commercial support, but GitHub currently detects **no project SECURITY.md**; that does not establish absence of any reporting process. [Registry](https://crates.io/api/v1/crates/reqwest), [publishers](https://crates.io/api/v1/crates/reqwest/owners), [TLS/feature docs](https://docs.rs/reqwest/0.13.5/reqwest/), [security page](https://github.com/seanmonstar/reqwest/security/policy). | **Best meaningful removal candidate via URLSession; keep until migration is validated.** A macOS `URLSession` upload implementation could replace this networking stack, but preserving streaming, progress, cancellation, TLS, timeout, redirect, and retry behavior is a substantive project. **Small cleanup candidate:** `json` is enabled even though all Rust call sites parse response bytes with serde_json and none calls reqwest's `.json()` helpers. Validate removing that feature later; it does not remove serde_json. |
| **tokio 1.53.2** | Creates a single-thread async runtime for uploads, races completion/cancellation/stalls with `select!`, sleeps for idle monitoring, and uses `Notify` (`src/github.rs`). Its runtime indirectly executes the credential-bearing reqwest operation. | Owners: **carllerche / Carl Lerche**, **Darksonn / Alice Ryhl**, and **github:tokio-rs:core**. Active release and a published private reporting/coordinated disclosure policy. Locked version dates **2026-10-03**; **1.53.3** shipped **2026-10-13** and **1.54.1** **2026-10-21**. **1.54.0 is yanked**. [Registry](https://crates.io/api/v1/crates/tokio), [publishers](https://crates.io/api/v1/crates/tokio/owners), [changelog](https://github.com/tokio-rs/tokio/blob/master/tokio/CHANGELOG.md), [security policy](https://github.com/tokio-rs/tokio/security/policy). | **Keep; routine update review.** The changelog describes 1.53.3 bug fixes; this review did not identify a new advisory against 1.53.2. Do not label this an urgent vulnerability solely because a newer release exists. It remains necessary while using async reqwest; moving to blocking reqwest still retains Tokio internally. |
| **futures-util 0.3.34** | Only direct call is `stream::try_unfold` to read a staged attachment in 64 KiB chunks for reqwest, checking size and tracking upload progress (`src/github.rs:185`). It processes sensitive attachment bytes but does not interpret their format. Defaults are disabled and only `std` explicitly enabled. | Owners: **cramertj / Taylor Cramer**, **taiki-e / Taiki Endo**, **rust-lang-owner**. Latest release **2026-08-11**, matching lock; upstream is rust-lang/futures-rs. That organization name is provenance, not proof that every release has undergone a Rust Project security audit. [Registry](https://crates.io/api/v1/crates/futures-util), [publishers](https://crates.io/api/v1/crates/futures-util/owners), [source](https://github.com/rust-lang/futures-rs). | **Keep.** Reimplementing the stream state machine saves a direct manifest entry but adds cancellation/polling code to a sensitive upload path; dependency count alone is a poor reason to do that. |
| **async-channel 2.5.0** | Transfers events from platform callbacks/background work into the GUI: menu bar, global shortcuts, custom URL events, compression progress and updater events (`src/main.rs`, `src/menu_bar.rs`, `src/global_shortcut.rs`, `src/app.rs`, `src/updater.rs`). All observed channel constructors are unbounded; exposure is event-volume/resource usage rather than protocol parsing. | Registry publication belongs to **github:smol-rs:admins**. Manifest credits original author Stjepan Glavina, which is distinct from current team publication authority. Latest release **2025-07-06**, matching lock. [Registry](https://crates.io/api/v1/crates/async-channel), [publishers](https://crates.io/api/v1/crates/async-channel/owners), [source/API description](https://github.com/smol-rs/async-channel). | **Keep.** It is runtime-independent and fits native-to-GUI bridging; Tokio serves the upload runtime rather than the whole UI. Consolidation into Tokio channels is possible but must preserve callback and GUI-executor behavior. Separately inspect high-frequency progress producers before deciding whether they need bounded/coalesced delivery. |
| **url 2.5.8** | Parses arbitrary `gatto:` custom URL invocations and checks GitHub's returned attachment link (`src/custom_url.rs`, `src/github.rs`). This is an external-input/security boundary: we rely on parsed scheme/host/port/userinfo, not string-prefix guesses. | Owners: **SimonSapin / Simon Sapin**, **Hoverbear / Ana**, **seanmonstar / Sean McArthur**, **github:servo:cargo-publish**, **github:servo:rust-url**. Latest release **2026-01-05**, matching lock. Upstream follows the WHATWG URL Standard and documents its IDNA/ICU4X dependency. [Registry](https://crates.io/api/v1/crates/url), [publishers](https://crates.io/api/v1/crates/url/owners), [source](https://github.com/servo/rust-url). | **Keep.** Do not replace URL validation with ad hoc splitting. Native `NSURL` could support a future macOS-only rewrite but requires tests against normalization and component semantics. Reqwest also uses URL types, so removing our direct entry alone would not remove the package from the graph. |

Historical advisories demonstrate that maintained asynchronous libraries still require updates. Tokio's [RUSTSEC-2025-0023](https://rustsec.org/advisories/RUSTSEC-2025-0023.html) broadcast unsoundness is patched in >=1.44.2, which includes our 1.53.2; Gatto directly uses Notify, not broadcast. futures-util's [RUSTSEC-2020-0062](https://rustsec.org/advisories/RUSTSEC-2020-0062.html) affected FuturesUnordered and was patched in >=0.3.2; our 0.3.34 and direct try_unfold use are not the affected combination. These are historical context, not current findings against the locked versions. The full lockfile audit is the appropriate check for transitive advisories.

### Images, drawing, Unicode, and temporary storage

| Direct package | Why Gatto uses it / exposure | Verified maintenance and publication evidence | Judgment |
| --- | --- | --- | --- |
| **image 0.25.10** | Detects/decodes user-selected and clipboard PNG/JPEG/GIF/WebP (`src/model.rs`), supplies image buffers/frames, resizes artwork, and supports annotation/export operations (`src/annotation.rs`, `src/annotation/raster.rs`, `src/annotation/pixelation.rs`, `src/menu_bar.rs`). It parses **untrusted binary media**. Defaults are disabled; the four supported codecs are selected explicitly. | Owners: **theotherphil**, **fintelia**, **197g / Aurelia Molzer**, **github:image-rs:publish**. Upstream README names 197g/fintelia maintainers and links organization contribution guidelines. Latest release **2026-03-10**, matching lock. [Registry](https://crates.io/api/v1/crates/image), [publishers](https://crates.io/api/v1/crates/image/owners), [source](https://github.com/image-rs/image). | **Keep; prioritize input hardening.** Native ImageIO is an architectural alternative, but our pixel operations and GUI frame construction still use image types, so wholesale removal is more than swapping a decoder. Review strict image dimensions and total decoded-memory/animation budgets before decoding. Compressed upload size is not a decoded-memory bound. |
| **tiny-skia 0.11.4** | CPU rasterization for annotation paths, masks/compositing, text outlines supplied by Core Text, export, and cursor artwork (`src/annotation.rs`, `src/annotation/text/layout.rs`, `src/annotation/preview.rs`, `src/image_preview/editor/cursor.rs`). Geometry can derive from pasted annotation data and native glyph paths; it touches pixels but is not our font parser. | Manifest attributes **Yevhenii Reizner / RazrFalcon**. Current repository has transferred to **linebender/tiny-skia**, and registry owners are **waywardmonkeys / Bruce Mitchener**, **RazrFalcon**, **xStrom / Kaur Kuut**. Latest **0.12.0** shipped **2026-02-02**, versus locked **0.11.4 (2024-02-04)**. GitHub currently detects no project SECURITY.md. [Registry](https://crates.io/api/v1/crates/tiny-skia), [publishers](https://crates.io/api/v1/crates/tiny-skia/owners), [current source](https://github.com/linebender/tiny-skia), [changelog](https://github.com/linebender/tiny-skia/blob/master/CHANGELOG.md), [security page](https://github.com/linebender/tiny-skia/security/policy). | **Keep for now; evaluate 0.12 upgrade.** Its changelog fixes a panic drawing small anti-aliased rectangles, relevant enough to test against our annotation renderer. Reizner also originally authored ttf-parser, but present Linebender ownership/release continuity is direct evidence against assuming tiny-skia is abandoned. Native Core Graphics could replace rasterization on macOS, with visual parity/export work; it is not a drop-in dependency removal. |
| **unicode-segmentation 1.13.3** | Grapheme-safe text editing/cursor movement and mapping Core Text line breaks back to Rust byte indices (`src/image_preview/editor/text_input.rs`, `src/annotation/text/layout/macos.rs`). Handles arbitrary pasted/typed text, including combining characters and emoji. | Owners: **huonw / Huon Wilson**, **SimonSapin**, **Manishearth / Manish Goregaokar**, **kwantam / Riad S. Wahby**, **sujayakar / Sujay Jayakar**. Latest release **2026-06-01**, matching lock. Upstream implements UAX #29. Prior 1.13.0/1.13.1 were yanked for breakage/MSRV tagging; current lock is later. [Registry](https://crates.io/api/v1/crates/unicode-segmentation), [publishers](https://crates.io/api/v1/crates/unicode-segmentation/owners), [source/changelog](https://github.com/unicode-rs/unicode-segmentation). | **Keep.** Replacing graphemes with Rust chars would break editing. Foundation can enumerate composed character sequences, but bridging UTF-16 and matching behavior is new platform-specific work; this small focused library already fits both required sites. |
| **tempfile 3.27.0** | Production dependency, not just tests: `NamedTempFile` owns private staged video copies and securely reopens them; temporary directories hold native video compression outputs (`src/video.rs`, `src/compression.rs`). Handles potentially private attachments, disk permissions, filename races, and cleanup. | Sole owner: **Stebalien / Steven Allen**. Manifest additionally credits other historical contributors and “The Rust Project Developers”; the current publication authority is a single user account. Latest release **2026-03-11**, matching lock. Upstream explicitly documents cleanup and Unix/macOS temp directory security behavior. [Registry](https://crates.io/api/v1/crates/tempfile), [publishers](https://crates.io/api/v1/crates/tempfile/owners), [source/security documentation](https://github.com/Stebalien/tempfile/blob/master/src/lib.rs). | **Keep.** Handwritten random temporary paths are a worse maintenance/security tradeoff. Named-file/directory cleanup relies on destructors, so crash termination can leave staged media behind; this is a lifecycle/privacy property to document or handle, not evidence of malicious authorship. macOS's default private per-user temporary directory mitigates the shared-temp threats discussed upstream. |

`image::Limits` documents a **512 MiB default best-effort decoder allocation cap**, with **no default strict width/height limits**; some decoders may ignore the allocation cap. Gatto's `validate_image_bytes` uses the convenience decode function, with no application-specific dimension/memory budget at that call site. The recommendation to bound decoded work is an integration improvement, not a verified exploit or an accusation that image's maintainers are untrustworthy. [Version-specific limits documentation](https://docs.rs/image/0.25.10/image/struct.Limits.html). The historical [RUSTSEC-2020-0073](https://rustsec.org/advisories/RUSTSEC-2020-0073.html) image provenance issue is patched since 0.23.12 and therefore does not affect locked 0.25.10.

### What to do with these findings

1. **Prioritize replacing reqwest through a bounded URLSession adapter**, then remove the application's direct Tokio/futures-util entries if their only usage disappears. Verify the actual re-resolved dependency delta; fixed-graph analysis suggests 23 packages, while GPUI still retains Tokio/futures-util. The other dependencies have useful current call sites and no individually unique graph footprint.
2. **Review tiny-skia 0.12.0 compatibility and visual behavior**, especially the small-rectangle panic fix. This is the clearest stale-version finding in this group.
3. **Consider removing reqwest's unused `json` feature** as a small later cleanup. Continue using serde_json for the bounded body parsing we already implement.
4. **Give token-bearing upload dependencies and untrusted image decoding priority** in update review. Improve our decoded-image budgets before trying to remove a well-maintained decoding library simply to lower the package count.
5. **Treat publisher concentration explicitly.** anyhow/thiserror are solely dtolnay-published; serde/serde_json also grant that account access. reqwest and tempfile each list a sole user owner. Multiple team owners improve continuity evidence but do not prove that publication requires multiple approvals. No account-security, independent release-audit, or future-benignness guarantee was established here.

## Apple and desktop integration dependencies

Checked October 21, 2026. These 16 direct packages provide real functionality; none is an obvious unused dependency. Apple supplies the underlying system frameworks, but **the Rust bindings are community maintained, not Apple packages**. Being macOS-only makes consolidation possible; it does not eliminate the need for a Rust/native boundary.

### What each package does here

The uses below were traced in the local source rather than inferred from package descriptions. Version numbers are the actual lockfile versions.

| Direct dependency | Why Gatto uses it / concrete call sites | Boundary and recommendation |
| --- | --- | --- |
| `block2 0.6.2` | `src/updater.rs:114` turns a Rust closure into the callback accepted by the Swift updater bridge; `src/compression.rs:131` supplies AVFoundation completion handlers. | Native callback ABI, captured-data lifetime, and callback threads. Keep with `objc2`; hand-writing the Blocks ABI would increase risk. |
| `objc2 0.6.5` | Objective-C messages, retained objects, autorelease pools and main-thread checks throughout updater, video compression, window/cursor integration, theme, score export and installer cleanup. | Fundamental memory-management and ABI boundary. Keep. Already on the latest release, including the October 2026 autoreleased-return miscompilation fix. |
| `objc2-foundation 0.3.2` | NSString/NSURL/NSDictionary/NSArray/NSError/NSData and geometry/value types used by native integrations, including video settings and score PNG metadata (`src/compression.rs:144`, `src/rooftops_score.rs:241`, `src/installer_cleanup.rs:16`). | Rust/native ownership and collections. Keep; a shared base layer for the other bindings. |
| `objc2-app-kit 0.3.2` | Native windows and Dock visibility (`src/window_limits.rs`), dark appearance (`src/theme.rs:90`), custom cursor (`src/image_preview/editor/cursor.rs:72`), score image rendering (`src/rooftops_score.rs:260`), and installer volume ejection (`src/installer_cleanup.rs:15`). | Main-thread UI calls, NSView pointer casts, graphics and workspace actions. Keep. |
| `objc2-av-foundation 0.3.2` | Reads video tracks/samples and writes compressed movies through AVAssetReader/AVAssetWriter (`src/compression.rs:137`). | Decodes media and drives encoders using OS facilities; numerous unsafe native calls and asynchronous lifetimes. Keep. |
| `objc2-avf-audio 0.3.2` | Audio setting constants used when configuring the compression writer, including format, channels, sample rate and bitrate (`src/compression.rs:138`). | Small constants-only usage rather than recording/microphone access. Keep for authoritative SDK identifiers; removal would mostly mean copying constants. |
| `objc2-core-media 0.3.2` | `CMTime` and `CMSampleBuffer` for video timing, samples, validation and output (`src/compression.rs:139`). | Sample ownership and timing representation. Keep with AVFoundation. |
| `objc2-core-video 0.3.2` | Pixel-buffer dimensions and pixel-format settings for real-frame verification and encoding (`src/compression.rs:140`). | Native pixel buffers. Keep with AVFoundation. |
| `objc2-video-toolbox 0.3.2` | Only the `kVTVideoEncoderSpecification_RequireHardwareAcceleratedVideoEncoder` identifier is directly referenced (`src/compression.rs:468`). | Narrowest use of this family. Could technically be replaced by a hardcoded SDK key, but that saves a manifest entry at the expense of maintaining an Apple identifier ourselves. Keep unless measurement establishes a worthwhile cost. |
| `core-foundation 0.10.1` | Reference-counted attributed strings, strings, types and ranges used by annotation layout (`src/annotation/text/layout/macos.rs:3`). | Native reference counting and UTF-16 indices; candidate for consolidation into `objc2-core-foundation`. |
| `core-graphics 0.24.0` | Annotation glyph paths, transforms and bitmap contexts for color/emoji glyphs (`src/annotation/text/layout/macos.rs:74`, `:172`). | Native paths and a CGContext drawing into a Rust-owned pixel buffer; candidate for `objc2-core-graphics`. |
| `core-text 21.0.0` | Annotation line layout, font fallback, kerning, caret positions, glyph outlines and bitmap glyph rendering (`src/annotation/text/layout/macos.rs:21`, `:75`). | Complex OS text shaping. Keep Core Text functionality; consider moving the Rust binding to `objc2-core-text`, not restoring a separate font parser. |
| `global-hotkey 0.8.0` | Registers and replaces the system-wide Quick Copy shortcut, parses user shortcut strings and delivers pressed events (`src/global_shortcut.rs:4`). | OS event registration; media-key support in the package additionally contains an event-tap path. Keep: replacing it means owning native event handlers, key mapping and teardown. Consider an independent 0.9 update for broader key coverage. |
| `tray-icon 0.26.1` | Gatto's macOS menu-bar icon, native menu, menu shortcuts and action events (`src/menu_bar.rs:4`). | Native UI and callbacks, including its re-exported menu library. Keep: an NSStatusItem implementation is feasible on macOS but introduces native lifecycle/event code. |
| `raw-window-handle 0.6.2` | Retrieves GPUI's native NSView so Gatto can configure maximum size, close/hide behavior and cursors (`src/window_limits.rs:9`, `src/image_preview/editor/cursor.rs:75`). | Standard borrowed-handle contract; Gatto still owns correctness of the NSView cast and thread/lifetime usage. Keep; it is intentionally dependency-free, not an unnecessary cross-platform runtime. |
| `libloading 0.8.9` | Loads the bundled `Contents/Frameworks/libGattoUpdater.dylib` in packaged builds (`src/updater.rs:142`). | Executes native library initialization with app permissions. Keep unless the updater bridge's linking/loading architecture changes; a direct `dlopen` replacement removes a wrapper, not this trust boundary. |

The `objc2` project documents its generated framework interfaces and the relationship to Blocks callbacks. `block2` explicitly notes that thread-safe blocks cannot yet be represented in its types, so native APIs needing such callbacks remain unsafe. The macOS system frameworks do the actual media/text work; our code and bindings must uphold their contracts. [Framework binding design](https://docs.rs/objc2/latest/objc2/topics/about_generated/index.html), [Blocks lifetime and threading documentation](https://docs.rs/block2/0.6.2/block2/).

### Maintainers, publishing authority and trust assessment

**`objc2`, `block2` and the seven directly used `objc2-*` framework packages:** the source is the same [madsmtm/objc2 project](https://github.com/madsmtm/objc2). The core crate manifests identify **Mads Marquart (`madsmtm`)**; the generated package manifests generally omit authors. All nine packages currently have the same two registry owners, `madsmtm` and `simlay`; registry metadata identifies Marquart as the publisher of their current releases. These are separate facts from the Cargo `authors` field. Verify through the official owner endpoints, for example [objc2 owners](https://crates.io/api/v1/crates/objc2/owners), [block2 owners](https://crates.io/api/v1/crates/block2/owners), [AppKit owners](https://crates.io/api/v1/crates/objc2-app-kit/owners).

Assessment: **credible specialist maintenance with substantial concentration in one lead maintainer**. Generated bindings, type/ownership machinery, public issue handling and recent correctness fixes are positive evidence; they do not establish a formal audit or Apple endorsement. The project's [security policy](https://github.com/madsmtm/objc2/blob/main/.github/SECURITY.md) openly says the maintainer works in their free time and that responses can take up to a week. It distinguishes exploitable vulnerabilities from theoretical unsoundness and sends OS framework issues to Apple. Gatto's locked `objc2 0.6.5` is current and contains the October 5, 2026 fix for miscompilation of methods returning autoreleased objects. The seven framework crates are current at 0.3.2; `block2` is current at 0.6.2. [objc2 release history](https://github.com/madsmtm/objc2/blob/main/crates/objc2/CHANGELOG.md), [registry release metadata](https://crates.io/api/v1/crates/objc2).

**Servo's `core-foundation`, `core-graphics`, `core-text`:** manifests name **The Servo Project Developers**, with source under `servo/core-foundation-rs`. Each registry owner list includes `larsbergstrom` (Lars Bergstrom), `metajack` (Jack Moffitt), `mbrubeck` (Matt Brubeck), `pcwalton` (Patrick Walton), and the `github:servo:cargo-publish` team. Recent releases were published by `jdm` (Josh Bowman-Matthews) through project publishing rights. [Foundation owners](https://crates.io/api/v1/crates/core-foundation/owners), [Graphics owners](https://crates.io/api/v1/crates/core-graphics/owners), [Text owners](https://crates.io/api/v1/crates/core-text/owners). Servo is an independent project hosted at Linux Foundation Europe with a Technical Steering Committee; historical Mozilla origin should not be read as a present Mozilla support promise. [Servo's own organization description](https://servo.org/about/).

Assessment: **established project provenance, but handwritten Apple bindings carry an aging-maintenance and correctness burden**. The repo is not archived and still publishes releases. These three crates are **not explicitly deprecated** in their current manifests/README. The explicit deprecation was for the sibling `cocoa`, `cocoa-foundation`, and `io-surface` packages. [Deprecation change](https://github.com/servo/core-foundation-rs/pull/731), [current repository](https://github.com/servo/core-foundation-rs).

The project's [future-direction discussion](https://github.com/servo/core-foundation-rs/issues/729) has Servo members supporting a transition to `objc2`; it also candidly acknowledges fewer maintainers, migration churn and different ownership models. This is a good reason to plan consolidation, not evidence that existing Gatto code is exploitable. The alternative's Core Text convenience wrappers still have an [open improvement tracker](https://github.com/madsmtm/objc2/issues/739), so it is not a mechanical package rename.

**`global-hotkey`, `tray-icon`:** source is under the **Tauri** organization; the current registry publishing owner is the `tauri-bot` account, which published the locked releases. This represents project-controlled automation, not the individual author of every change. [Hotkey owners](https://crates.io/api/v1/crates/global-hotkey/owners), [Tray owners](https://crates.io/api/v1/crates/tray-icon/owners). Tauri has a working group, teams and a board under its Commons Conservancy programme. [Official governance](https://v2.tauri.app/about/governance/).

Assessment: **good institutional/project continuity**, with the normal caveat that a bot-owner listing does not reveal who can change its credentials or publishing workflow. Both repositories have current activity in October 2026. `tray-icon 0.26.1` is current. `global-hotkey 0.9.0` was released October 16; it upgrades keyboard-types and expands international/numpad/system key support. Its `META`/`SUPER` changes touch Gatto's shortcut parser and tests, so a deliberate update is preferable to assuming drop-in compatibility. [Hotkey changelog](https://github.com/tauri-apps/global-hotkey/blob/dev/CHANGELOG.md), [Tray releases](https://crates.io/api/v1/crates/tray-icon).

**`raw-window-handle`:** Cargo credits **Osspial**; the current registry owners are `Osspial`, `Lokathor`, and `kchibisov` (Kirill Chibisov). It lives in the [Rust Windowing community](https://github.com/rust-windowing), alongside Winit and Glutin; this is not a Rust core-team guarantee. [Registry owners](https://crates.io/api/v1/crates/raw-window-handle/owners). Assessment: **strong fit and limited scope**. The released 0.6.2 is old (May 2024) but remains the latest, the repository has later development, and the crate is intentionally dependency-free. An interoperability contract should not be judged stale purely by release frequency. [Design and safety documentation](https://docs.rs/raw-window-handle/0.6.2/raw_window_handle/), [current source](https://github.com/rust-windowing/raw-window-handle).

**`libloading`:** manifest author and sole registry owner are **Simonas Kazlauskas (`nagisa`)**. The source lives in their personal repository. [Registry ownership](https://crates.io/api/v1/crates/libloading/owners), [maintainer identity](https://github.com/nagisa). Assessment: **established, narrow library with single-maintainer continuity risk**. Its principal benefit is managing loaded-library/symbol lifetimes, although Gatto uses only library loading and deliberately keeps the bridge loaded until process exit. Current 0.9.0 (November 2025) raises the Rust minimum to 1.88 and changes some APIs; no observed macOS-specific security fix makes that update urgent for this usage. [Library purpose](https://github.com/nagisa/rust_libloading/blob/master/README.mkd), [official changelog](https://github.com/nagisa/rust_libloading/blob/master/src/changelog.rs).

### Concrete follow-ups and caveats

1. **Maintenance consolidation candidate: migrate the three Servo text/graphics bindings together to the `objc2` family.** That could replace the two handwritten CTTypesetter declarations in Gatto with generated bindings. This review's fixed macOS graph analysis finds that this removes **zero resolved packages** because GPUI retains the Servo family. It therefore improves our API consistency and local ownership handling, but does not remove those publishers from the shipped dependency graph. It would still require adding/reusing `objc2-core-foundation`, `objc2-core-graphics` and `objc2-core-text`; “consolidation” is the benefit, not zero native dependencies. Preserve annotation shaping, wrapping, fallback fonts, emoji rendering, caret positions and pixel-buffer ownership in any migration. This is an engineering recommendation inferred from Gatto's uses and upstream direction, not a claim of drop-in equivalence. [Interop guidance](https://docs.rs/objc2/latest/objc2/topics/crate_interop/index.html).
2. **If retaining Servo bindings, update the compatible pair together.** `core-text 21.0.0` uses `core-graphics 0.24`; `core-text 22.0.0` uses `0.25`. Version 21.1.0 was yanked after changing that public dependency in a minor release caused downstream compilation failures. Updating only the graphics crate would produce distinct Rust types in font APIs. `core-foundation 0.10.1` remains current. [Confirmed release mistake and yank](https://github.com/servo/core-foundation-rs/issues/750), [22.0 release](https://github.com/servo/core-foundation-rs/pull/751).
3. **Separate advisory checks from unsoundness reports.** No RustSec package entries were found for these 16 names in the locally updated official database. Nevertheless, Servo has open reports for [`CFData::from_arc` lifetime unsoundness](https://github.com/servo/core-foundation-rs/issues/759), [an incorrect Core Text vertical-translation signature](https://github.com/servo/core-foundation-rs/issues/749), and a contributor's [proposed fix for alleged Core Text use-after-free issues](https://github.com/servo/core-foundation-rs/pull/741). Gatto's direct text-layout path does not call the named APIs (`from_arc`, `get_vertical_translations_for_glyphs`, font-descriptor `traits`, or `create_font_descriptor`); `new_from_name` calls CTFontCreateWithName directly. That limits the observed direct exposure and is not a proof about transitive consumers or every possible lifetime bug. The proposed-fix PR is not equivalent to a maintainer-confirmed advisory.
4. **Consider the loader as a bounded dependency-removal experiment.** Gatto currently builds an explicit bundle-relative library path, signs the bridge and application with hardened runtime, and retains the library for the process lifetime (`src/updater.rs`, `scripts/release.sh:170`). Good properties to preserve. This review finds removing this root could remove one resolved package. A small macOS loader wrapper or linking the Swift bridge into release builds could achieve that, but it would replace libloading's tested lifecycle/error code or change packaging. Today `build.rs` does not build/link the bridge; `scripts/prepare-updater.sh` builds a Swift dylib only for packaging, and ordinary Cargo runs intentionally work without it. Link-time integration must preserve that development path, runtime class registration and Sparkle lookup. Loading still runs native code with app permissions; replacing libloading with direct platform calls does not reduce that privilege. [Loader safety explanation](https://docs.rs/libloading/0.8.9/libloading/struct.Library.html#method.new).

### Dependency-reduction priority for this group

For the user’s supply-chain objective, **package reachability outranks manifest entry count**. This review's fixed macOS normal/build graph gives these prospective reductions: replacing `tray-icon` removes four package nodes, replacing `global-hotkey` removes two, and replacing both removes seven because a shared channel package then becomes unreachable. This also removes two build-script-bearing packages through the tray dependency branch. Both native features could be implemented using Apple APIs, but neither replacement is a free cleanup: tray/menu behavior, accelerators, accessibility, native object ownership and hotkey registration/teardown need focused verification. Treat these as small design spikes with a clear acceptance checklist, not permission for a broad unsafe rewrite. These counts do not include a fresh Cargo feature re-resolution.

A practical order within this group is: (1) investigate one-package `libloading` removal while preserving the signed bundle loading boundary; (2) investigate `tray-icon`/`global-hotkey` together if maintaining those small native adapters is acceptable; (3) consolidate Servo text bindings for maintenance, with no immediate package-count or publisher reduction; (4) retain the generated AVFoundation/CoreMedia/CoreVideo framework interfaces rather than copying Apple declarations merely to lower Cargo's direct dependency count. The existing `objc2` family shares registry owners, so reducing one generated framework crate generally reduces package volume without removing a publisher identity. The precise exposure depends on which other framework crates remain.

No application code or dependency changes were made for this review.

## Website and JavaScript tooling: 15 direct packages

Reviewed the root and `web/` manifests, `bun.lock`, installed exact-version sources, the website README/design system, and live first-party registry metadata. Versions below are the exact direct declarations and lock resolutions. This code supports the public website and release/build scripts; it does not become a Rust dependency of the macOS app. Browser visitors are not restricted to macOS merely because the desktop app is.

### Reduction priorities after the user clarified the supply-chain goal

The strongest changes are eliminating small runtime utilities and consolidating duplicated build tools onto the Bun executable we already trust. Dropping a name from a manifest while it remains transitive is not the goal. No changes were made.

| Candidate | Actual lock entries made unreachable, all platforms | Benefit and cost |
| --- | ---: | --- |
| Replace `toastify-js` and remove `@types/toastify-js` | 2 | Low conceptual scope; removes one browser library and its separate type package. Preserve the existing accessible notification behavior. |
| Replace the two `tsc --noEmit` commands with built-in `bun check`; remove standalone `typescript` | 21 | Concrete promising consolidation. Installed Bun 1.4.3 successfully checked the current root config (8 files) and website config (29 files), with no writes. The 21 lock entries are one TypeScript wrapper plus 20 mutually platform-specific binaries, not 21 packages all executing on one Mac. Editor TypeScript-version selection is a separate concern. |
| Remove only the direct compiler declaration | 0 | Cosmetic dependency-count reduction: the official Solid plugin still requires and installs the compiler. Keep the explicit pin while on prereleases. |
| Remove StyleX-specific lint plugin | 8 | Would remove the flagged `braces` subtree, but also discards all three useful StyleX checks. Deliberate checking tradeoff rather than automatic recommendation. |
| Remove Oxfmt | 21 | Mostly mutually exclusive native platform artifacts plus tinypool; no verified available formatter replacement in the installed Bun. Eliminating formatting checks is a separate policy decision. |
| Remove Oxlint, tsgolint, and StyleX lint plugin together | 35 | Also removes lint/promise safety checks. There is no verified available built-in replacement in the installed Bun. |
| Rewrite the website without its four direct Solid packages | 33 | Feasible design investigation, but affects 18 TSX source files, hydration/prerendering and reactive interactions. This is a deliberate website rewrite, not a quick dependency deletion. StyleX and Vite would remain unless separately replaced. |
| Rewrite styling without the three direct StyleX packages | 21 | Removes a distinct compiler/lint/runtime family but changes the approved pervasive style system. Babel stays in the graph through Solid, so do not promise its complete elimination. |

Counts were calculated from the fixed `bun.lock` by traversing dependencies, optional dependencies and installed peer dependencies from both manifests, then recomputing reachability without each group. They include cross-platform native alternatives, are not additive, are not byte savings, and were not validated by actually rewriting manifests/reinstalling. The graph traversal was checked separately from the package-purpose review.

Bun's official [type-checking guide](https://bun.com/docs/runtime/check) confirms its built-in checker does not require the `typescript` package, ships the standard-library/DOM definitions, and aims at TypeScript 7 diagnostics. It also explains the editor uses its own TypeScript version. The read-only checks above passed while `typescript` was still installed; a clean install without it remains part of implementation validation.

Current official Bun documentation also describes built-in lint/format commands, but **the installed Bun 1.4.3 returns `Script not found` for `bun lint --help` and `bun fmt --help`**. Do not conflate those documentation claims with verified capabilities of our pinned executable. Validate the required runtime release and rule/format compatibility before proposing removal of Oxlint/Oxfmt.

### Findings that affect decisions

1. **Consider replacing Toastify and removing its separate types.** Our only notification is the fixed “Gattoverse mini game coming soon.” message in `web/src/App.tsx`. The wrapper in `SiteNotifications.tsx` already owns appearance, the live announcement region, browser popover placement, focus restoration, keyboard timer pauses, cleanup and single-notice behavior. Toastify adds DOM construction, timeout and close behavior. This makes a small existing-Solid notification implementation a plausible net simplification. It would remove a browser dependency and a types dependency, while transferring responsibility for timer/hover/keyboard behavior to us. This is a recommendation, not an implemented or tested replacement.
2. **Solid 2 and its build plugin are official but prerelease.** All three `2.0.0-rc.15` packages come from `solidjs/solid`; `3.0.0-next.50` comes from `solidjs/solid-vite-plugin`. These are authentic official releases, not an unofficial namesake. Both the runtime family and plugin are deliberately prerelease; the exact plugin release fixes interaction with RC.15's hydration markers and internal nonce exports. The risk is compatibility churn, separate from author trust. Keep the runtime/compiler/plugin versions coordinated and make retaining prereleases an explicit choice. [Solid releases](https://github.com/solidjs/solid/releases), [exact plugin release](https://github.com/solidjs/solid-vite-plugin/releases/tag/@solidjs%2Fvite-plugin@3.0.0-next.50).
3. **Keep `@stylexjs/eslint-plugin`: it is used by Oxlint.** `web/.oxlintrc.json` loads it under `jsPlugins` and enables `stylex/valid-styles`, `stylex/no-unused` and `stylex/valid-shorthands`. Its name does not imply that ESLint itself is needed. Oxlint officially supports ESLint-compatible JavaScript plugins; that integration is currently alpha and outside semver guarantees. [Oxlint plugin support](https://oxc.rs/docs/guide/usage/linter/js-plugins.html), [configuration stability note](https://oxc.rs/docs/guide/usage/linter/config).
4. **There is one flagged transitive lint dependency, with no identified reachable exploit in our current calls.** The fresh `bun audit --json` identifies `@stylexjs/eslint-plugin@0.19.1 → micromatch@4.0.8 → braces@3.0.3`, affected by GHSA-vfj7-8cjw-p6xm (stack-exhaustion DoS). The advisory currently lists no patched version. Inspection of the installed plugin finds only `micromatch.isMatch` calls; the installed micromatch implements this by calling `picomatch` directly. The vulnerable `braces` walkers are accessed through different APIs (`parse`, `braces`, `braceExpand`), not the inspected StyleX calls. It is nevertheless installed and imported as lint tooling. Track an upstream fix or removal of that unnecessary transitive import; do not describe it as an exposed remote vulnerability in the deployed website. This is static call-path analysis, not a full reachability proof or exploitation test. [Advisory](https://github.com/advisories/GHSA-vfj7-8cjw-p6xm), [micromatch exact source](https://github.com/micromatch/micromatch/blob/4.0.8/index.js).

### Package-by-package review

“Confidence” is a judgment about observable stewardship and provenance, not a character judgment or guarantee against future compromise. Build-time packages execute on the developer/release machine and can influence generated assets even when no library code ships to visitors.

| Package and exact version | Why Gatto has it; exposure | Stewardship / evidence / judgment | Recommendation |
| --- | --- | --- | --- |
| `@types/bun` **1.4.3** | Bun globals, `bun:test`, file/process APIs in website and release scripts; explicitly included by both TypeScript configs. Declarations only, no shipped browser or app runtime. The package's entry file references `bun-types`, so the actual definitions are a transitive package. | Official DefinitelyTyped package published by the `types` account; registry identifies Microsoft publication infrastructure. Contributors include Bun's Jarred Sumner and Bun automation. **High confidence in identity; low runtime exposure.** Registry has a signature, no provenance attestation for this version. [Registry](https://registry.npmjs.org/@types%2Fbun/1.4.3), [DT source](https://github.com/DefinitelyTyped/DefinitelyTyped/tree/master/types/bun). | Keep and match the Bun runtime version. Replacing the wrapper with `bun-types` directly saves a declaration wrapper but little practical risk or size; no compelling reason here. |
| `oxfmt` **0.73.0** | Website formatting and formatting checks in root scripts. Build/developer process only. Native platform packages plus a worker-pool dependency. | Oxc team, led by Boshen, with named core contributors; registry publishing authority is concentrated in one listed maintainer (`boshen`). Exact package was published by GitHub Actions trusted publishing with provenance to `oxc-project/oxc/.github/workflows/release_apps.yml`. **High identity confidence; pre-1.0 behavior-change risk.** Current registry latest is 0.74.0. [Registry](https://registry.npmjs.org/oxfmt/0.73.0), [Oxc team](https://oxc.rs/team). | Keep. Review formatting-only diffs on upgrades; no reason to add another formatter. |
| `oxlint` **1.88.0** | Lints TS/TSX, including async/promise safety; also executes the configured StyleX JS rules and orchestrates type checking. Build/developer process only. | Same Oxc stewardship and publishing concentration; official GitHub Actions trusted publishing/provenance. **High identity confidence.** Version is stable 1.x, but JS plugin compatibility remains alpha. Current registry latest is 1.89.0. [Registry](https://registry.npmjs.org/oxlint/1.88.0), [plugin limitations](https://oxc.rs/docs/guide/usage/linter/js-plugins.html). | Keep; upgrade with `oxlint-tsgolint` compatibility checks. |
| `oxlint-tsgolint` **7.0.2004** | Required backend for configured `--type-aware --type-check`; performs TypeScript semantic analysis in a Go native binary. Build/developer process only. | Official `oxc-project/tsgolint`; registry maintainer `boshen`, GitHub Actions trusted publishing and source-workflow provenance. The Oxc team page identifies tsgolint and typescript-eslint expertise. **High identity confidence; smaller subsystem with evolving rule coverage.** [Registry](https://registry.npmjs.org/oxlint-tsgolint/7.0.2004), [official requirements](https://oxc.rs/docs/guide/usage/linter/type-aware.html). | Keep: removing it breaks the intended typed lint checks. The unusual version number does not make it a third-party TypeScript fork of unknown origin. |
| `typescript` **7.0.2** | Separate `typecheck:web` and `typecheck:scripts` commands run `tsc --noEmit`. No TypeScript package runtime is shipped; Bun/Vite execute or compile the TS. This version dispatches to platform-native TypeScript packages. | Official Microsoft repository and multiple Microsoft/TypeScript publishing accounts, documented MSRC reporting process. **High confidence in established stewardship.** Registry signature present; no version provenance attestation advertised. Registry latest is 7.0.2. [Registry](https://registry.npmjs.org/typescript/7.0.2), [security process](https://github.com/microsoft/TypeScript/security/policy). | Candidate for removal via the already-installed `bun check`, which passed both existing configs in this review. Replace both commands and verify a clean dependency install plus editor expectations; see reduction priorities above. |
| `vite` **8.3.4** | Website dev/preview server, Solid/StyleX plugin host, and production client/server bundles used by prerendering. Runs with Bun. Build-time and development-server exposure; some generated Vite helper code may ship. Production deploy directory is static `web/dist/client/`. | Official Vite team, with public multi-person stewardship; npm lists `yyx990803`/`vitebot`. Exact artifact uses GitHub Actions trusted publishing and provenance from `vitejs/vite/.github/workflows/publish.yml`. Published security policy and maintained release lines. **High confidence; broad plugin/transitive execution surface.** Latest registry patch is 8.3.5; no direct exact-version OSV finding in this review. [Registry](https://registry.npmjs.org/vite/8.3.4), [team](https://vite.dev/team), [security policy](https://github.com/vitejs/vite/security/policy). | Keep; routine patch updates and existing loopback-only dev/preview binding are sensible. Bun's bundler alone is not a demonstrated replacement for this configured plugin/prerender workflow. |
| `solid-js` **2.0.0-rc.15** | Reactive signals/effects, lifecycle cleanup and `isStatic` in story/navigation/design components; browser runtime and build-time prerender use. | Official Solid project; registry lists Ryan Carniato (`ryansolid`) as publishing maintainer, so account authority is concentrated even though the project has contributors. Exact artifact uses GitHub Actions trusted publishing/provenance to Solid's `next` branch workflow. **Good identity confidence; materially higher release-stability risk because RC.** Stable `solid-js` dist-tag is still 1.9.17; `next` has later RCs. [Registry](https://registry.npmjs.org/solid-js/2.0.0-rc.15), [release history](https://github.com/solidjs/solid/releases). | Keep unless intentionally changing framework/version strategy. Do not casually downgrade to Solid 1: this code uses Solid 2 APIs and package layout. |
| `@solidjs/web` **2.0.0-rc.15** | JSX typing/runtime entry, DOM helpers and `HydrationScript`; both browser DOM/hydration and build-time HTML rendering. Separate renderer package in Solid 2. | Same official Solid monorepo and provenance; npm maintainers `ryansolid`, `davedbase`. **Good identity confidence; RC stability risk.** Serialization dependencies are pinned to `seroval`/`seroval-plugins` 1.6.7 in the lock. [Registry](https://registry.npmjs.org/@solidjs%2Fweb/2.0.0-rc.15). | Keep and coordinate with `solid-js` and compiler/plugin. |
| `@solidjs/compiler` **2.0.0-rc.15** | Native/WASM compiler used by the Vite plugin, including transforms required in every compiler mode. Developer/build executable, not browser runtime. No direct repo import was found. | Same Solid ownership and source provenance, maintainers `ryansolid`, `davedbase`. **Good identity confidence; RC/native-binary supply-chain exposure.** Exact installed plugin explicitly states the compiler is required even when JSX uses the Babel backend. [Registry](https://registry.npmjs.org/@solidjs%2Fcompiler/2.0.0-rc.15), [exact plugin source](https://github.com/solidjs/solid-vite-plugin/blob/a1c0656cd6825465c53847e37fb73109e801935a/src/index.ts). | Retain the deliberate direct exact pin if it is our version-alignment mechanism. It could be removed from the manifest because the plugin already depends on it, but that would not remove the compiler from the installed graph and could weaken version control. |
| `@solidjs/vite-plugin` **3.0.0-next.50** | Imported by `web/vite.config.ts` with `start: true, ssr: true`; compiles Solid, provides HMR and generates the server handler used in prerendering. Build/development exposure; determines emitted browser code. | Official renamed successor to `vite-plugin-solid`, under `solidjs/solid-vite-plugin`, not a lookalike. Maintainers `ryansolid`, `davedbase`; exact GitHub Actions source provenance. **Good identity confidence; explicitly prerelease and moving quickly.** Registry currently advertises next.51 as latest, but that tag does not imply stable semver. [Registry](https://registry.npmjs.org/@solidjs%2Fvite-plugin/3.0.0-next.50), [exact prerelease](https://github.com/solidjs/solid-vite-plugin/releases/tag/@solidjs%2Fvite-plugin@3.0.0-next.50). | Keep with coordinated Solid update checks. |
| `@stylexjs/stylex` **0.19.1** | Semantic tokens, style creation, atomic class composition and `attrs` throughout the approved design system. Compile-time extraction plus a small runtime for conditional composition; browser and prerender exposure. | Meta's `facebook/stylex` repository; five npm maintainers (`vincentriemer`, `anaybucket`, `naman34`, `necolas`, `mellyeliu`). Exact version published by `mellyeliu`, with registry signature but no npm provenance attestation. **Good stewardship confidence; weaker artifact-to-CI evidence than Solid/Oxc/Vite and pre-1.0 compatibility risk.** Latest registry version is 0.19.1. [Registry](https://registry.npmjs.org/@stylexjs%2Fstylex/0.19.1), [official project](https://stylexjs.com/). | Keep: this is a deliberate pervasive design-system choice, not an unused utility. |
| `@stylexjs/unplugin` **0.19.1** | Official Vite adapter compiles StyleX and extracts/minifies CSS; dev CSS virtual modules power HMR. Build/development exposure. | Same StyleX npm maintainers/publisher; no artifact provenance advertised. Package-version metadata lacks a repository field, but official StyleX docs and its own release announcement establish the integration's identity. **Good stewardship confidence; relatively new adapter and pre-1.0 package.** [Registry](https://registry.npmjs.org/@stylexjs%2Funplugin/0.19.1), [official Vite setup](https://stylexjs.com/docs/learn/installation/vite/), [official introduction](https://stylexjs.com/blog/v0.17.1/). | Keep. It is the CSS compilation step, not a redundant runtime styling library. |
| `@stylexjs/eslint-plugin` **0.19.1** | Loaded by Oxlint for StyleX correctness/unused/shorthand checks. Lint-only execution; no shipped runtime. | Same official Meta StyleX maintainers/publisher; registry signature, no advertised provenance. **Good identity confidence**, with the `braces` transitive alert described above and Oxlint's alpha JS integration as distinct risks. [Registry](https://registry.npmjs.org/@stylexjs%2Feslint-plugin/0.19.1), [Oxlint compatibility](https://oxc.rs/docs/guide/usage/linter/js-plugins.html). | Keep its useful checks. Track the transitive advisory and upstream package maintenance; removing the whole plugin solely because its name says ESLint would be a mistake. |
| `toastify-js` **1.12.0** | Builds/dismisses the single website toast and manages a timeout/hover pause. Browser runtime; imported in a component also processed during prerender. No dependencies or package lifecycle scripts. | Personal `apvarun/toastify-js` project; only npm maintainer/publisher `apvarun`. Latest published version is still July 21, 2022; GitHub API's last pushed timestamp is August 19, 2024; repository is not archived. Security tab has no policy and no published advisories. Registry signature but no provenance. **Lower maintenance/continuity confidence than the framework/tooling projects; no evidence of malicious authorship.** [Registry](https://registry.npmjs.org/toastify-js/1.12.0), [GitHub metadata](https://api.github.com/repos/apvarun/toastify-js), [security page](https://github.com/apvarun/toastify-js/security). | Best practical removal candidate. Current call passes a fixed text string and preserves the library's default escaping; no current unsafe HTML path was identified. |
| `@types/toastify-js` **1.12.4** | TS definitions for the Toastify import and returned toast type. Declarations only; no browser/app runtime, dependencies or lifecycle scripts. | DefinitelyTyped, published through `types`; contributor metadata names `adblanc` and `peterblazejewicz`. Distinct stewardship from Toastify's runtime author. Published May 6, 2025 and still current registry version. **High identity confidence; low direct execution exposure.** Signature present, no provenance. [Registry](https://registry.npmjs.org/@types%2Ftoastify-js/1.12.4), [source](https://github.com/DefinitelyTyped/DefinitelyTyped/tree/master/types/toastify-js). | Keep as long as Toastify is used; remove together if the runtime library is replaced. |

### Provenance and advisory-check limits

- All 15 direct package names/versions resolve consistently between manifests, `bun.lock` and live official npm registry metadata, and every direct lock SHA-512 equals the registry's `dist.integrity`. This verifies recorded artifact identity consistency, not a clean-room rebuild or independent rehash of every installed file.
- All 15 exact registry versions advertise registry signatures. Eight additionally advertise provenance attestations: Oxfmt, Oxlint, tsgolint, Vite, and the four Solid packages. The fetched statements identify the expected source repositories, GitHub Actions release workflows, and source commits. They were inspected, not cryptographically verified end to end; registry signatures alone do not prove the author or source code's safety. Trusted publishing reduces long-lived-token exposure but does not prevent compromised repository/workflow releases.
- An OSV `querybatch` against all 15 exact npm direct versions returned no indexed vulnerabilities. This is not a clean result for the full dependency graph: Bun's full audit identifies the transitive `braces` issue above. Advisory feeds have coverage/detection delays, and package-by-package source review was focused on use, identity and the identified alert, not exhaustive security auditing. [OSV API](https://google.github.io/osv.dev/api/).
- Neither `@types` package contributes a shipped runtime library. Absence of runtime code does not make all future package downloads inherently trustworthy; these exact metadata records have empty lifecycle scripts.
- No code, dependency, lockfile or configuration changes were made by this review. Registry latest tags are a snapshot and should be checked again when upgrading. License metadata is MIT for these packages except TypeScript (Apache-2.0); this is not a transitive license-compliance audit.

Read-only Bun checker verification, run from the repository root against the existing installed Bun 1.4.3:

```text
bun check -p tsconfig.json
✓ No type errors in 8 files [39.55ms]

bun check -p web/tsconfig.json
✓ No type errors in 29 files [58.18ms]
```

Oxlint's `oxlint-tsgolint` backend is a separate required executable for our typed lint rules. Its locked package does not depend on the standalone `typescript` npm package. Removing standalone `typescript` therefore does not remove or replace typed Oxlint linting.

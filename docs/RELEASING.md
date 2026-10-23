# Releasing Gatto

Releases are built locally on macOS with `scripts/release.sh`. The script produces
a Developer ID signed and notarized Apple Silicon app and disk image, then
publishes the DMG, signed Sparkle appcast, and checksums to GitHub Releases.
There is no active release workflow in GitHub Actions.

## Prerequisites

- The [development tools](../CONTRIBUTING.md#development-setup) and an authenticated `gh`
  account with permission to create releases in this repository.
- Apple signing/notarization tools available through `security`, `codesign`,
  `xcrun notarytool`, `xcrun stapler`, `spctl`, `plutil`, `ditto`, `hdiutil`, `diskutil`,
  `swift`, `tiffutil`, and `osascript`.
- Bun and `cargo-deny` 0.20.2 installed for release checks (`cargo install --locked
  cargo-deny --version 0.20.2` is one-time setup).
- A logged-in macOS desktop session. Finder saves the disk image's window layout;
  allow your terminal to control Finder if macOS asks on the first build.
- A **Developer ID Application** certificate and its private key in the login
  keychain. Check with `security find-identity -v -p codesigning`.
- Notarization credentials stored in the `gatto-notary` keychain profile. Follow
  the [one-time setup](#one-time-signing-setup); do not commit credentials.

| Environment variable | Purpose |
| --- | --- |
| `SIGN_IDENTITY` | Signing certificate name. Required if more than one Developer ID Application identity is present. |
| `SPARKLE_ACCOUNT` | Keychain account for the persistent Sparkle Ed25519 signing key; defaults to `gatto-updates`. |
| `NOTARY_PROFILE` | Existing notarization keychain profile; defaults to `gatto-notary`. |
| `GIT_COMMIT_HASH` | Build metadata override for ordinary Cargo builds. The release script sets this to the full release commit itself. |

## One-time signing setup

In your Apple Developer account, create a **Developer ID Application** certificate:
**Xcode → Settings → Accounts → Manage Certificates → + → Developer ID Application**.
Check it with `security find-identity -v -p codesigning`.

Create an app-specific password at [account.apple.com](https://account.apple.com/),
then store the notarization credentials:

```sh
xcrun notarytool store-credentials gatto-notary \
  --apple-id <apple-id> --team-id <team-id>
```

Enter the app-specific password when prompted. The certificate stays in your
login keychain and credentials in the keychain profile; neither belongs in the
repository or GitHub.

## One-time updater signing setup

Run on the release Mac:

```sh
./scripts/prepare-updater.sh
dist/sparkle/bin/generate_keys --account gatto-updates
```

Sparkle stores the private Ed25519 key in the login Keychain. Keep using this same
key for all releases and back it up securely using Sparkle's documented key
export/import procedure; never commit or log the private key. The release script
reads only its public key into the app's Info.plist. It fails after the unsigned
build and before signing if the key is absent. Changing `SPARKLE_ACCOUNT` must select the same signing key
used by installed clients; key rotation requires a separate migration plan.

`prepare-updater.sh` downloads Sparkle **2.10.0**, verifies its pinned SHA-256,
and builds the small Swift user driver. It does not alter Keychain items.
The SDK's license and notices are copied into the app's Resources directory.
Review upstream release notes and the checksum when upgrading it. Sparkle is a
permissively licensed native dependency used for signed feed/archive validation,
macOS authorization, app replacement, and relaunch; these operations are not
implemented by Gatto's upload adapter. `block2` provides the main-thread callback;
the system dynamic loader opens the signed bundle-relative Swift bridge and
keeps it loaded for the lifetime of the process, without a libloading dependency.
Normal Cargo builds remain independent of the SDK and signing credentials.

## Release with one command

After the one-time credential/tool setup, run from `main`:

```sh
./scripts/release.sh
```

Merge feature branches into `main` first. Your local main may contain uncommitted
work or commits ready to push, but must include `origin/main` without conflicts.
The script fetches the remote before proceeding.

The script guides the release in this order:

1. Check tool availability and compile the pinned native updater SDK without
   reading signing credentials.
2. Show a numbered release menu with the exact destination version and channel:
   the prepared version, next Beta, promotion to Stable, patch/minor/major Stable
   releases, and a custom version. Use a minor bump for new compatible
   capabilities and a major bump for user-facing compatibility breaks. Reuse
   prepared/Unreleased changelog notes, or ask for a short categorized summary.
   It updates Cargo, the lockfile, both bundle versions, and the dated changelog.
   The lockfile update changes only Gatto's package version, preserving dependency
   pins and checksums. Reusing a prepared version also repairs a stale lockfile.
3. Install locked Bun dependencies without lifecycle scripts, run the JavaScript
   dependency audit, and run Rust formatting, compilation, lint,
   tests, and dependency policy checks; script type checks/tests; native updater
   tests; native menu/hotkey checks; a disposable install/relaunch test; cask syntax; and website build,
   lint, type checks, and tests.
4. Regenerate walkthrough screenshots from the current app using the off-screen
   Metal renderer. A capture failure stops the release. Show the changed-file summary and ask before committing all pending code and
   release preparation changes and pushing `main`. A concurrent remote change
   makes a normal push fail; the script never force-pushes.
5. Build the unsigned binary, then verify GitHub authentication, signing identity,
   notarization profile and the persistent Sparkle key. Sign, notarize, and verify
   the app and DMG, then generate the signed
   update feed and checksums. Ask before uploading a draft, verify uploaded bytes
   against the local files, and publish it using the prepared changelog notes.
   Stable becomes GitHub's latest release; Beta is a prerelease with `latest=false`.
6. Refresh and verify the signed Beta discovery feed for either channel. For
   Stable, also measure/verify README figures and refresh/verify the Homebrew cask,
   then commit and push those metadata files. Beta leaves stable metadata intact.

The script is the recurring entry point. Code review, manual testing on affected
hardware, and one-time credentials still require human judgment; automated checks
do not verify Touch Bar hardware or production authorization prompts. Nothing is
committed or published merely by invoking the script: the commit/build and
publication confirmations remain explicit. Cancelling keeps prepared changes
and artifacts available for inspection.

## Build and publish

From the repository root:

```sh
./scripts/release.sh
```

The script installs the `aarch64-apple-darwin` Rust target, performs a locked release
build, and creates `dist/release/Gatto.app` with the plist, binary, icon assets,
and **Send to Gatto.workflow** in `Contents/Resources`. The workflow is sealed by
the outer app signature; it uses macOS's Run Shell Script action and has no nested
executable to sign. Both DMG and Homebrew users install it explicitly from the
Gatto menu using macOS's workflow installer. Updates leave any separately installed
per-user copy intact; [Setup](SETUP.md#install-the-finder-quick-action-optional)
documents replacement and removal.
It signs with hardened runtime and a timestamp, verifies the signature, submits
a temporary ZIP for notarization, staples and validates the ticket, and runs
Gatekeeper assessment. The ZIP is temporary and is not distributed. It then builds
the branded drag-to-Applications disk image, signs and notarizes it separately,
staples and validates its ticket, and assesses the disk image with Gatekeeper.
This preserves offline notarization tickets on both the copied app and the DMG.
Before signing, it prepares a writable installer template and records that
volume's UUID as `GattoInstallerVolumeUUID` in the app's Info.plist. After app
notarization, the signed/stapled bundle replaces the template's unsigned copy,
and the template is compressed to the final DMG without changing its volume UUID.
This pairing allows the installed app to eject its own disk using only system
metadata. The temporary template is removed before publication.
It creates:

- `dist/release/gatto-<version>-aarch64-apple-darwin.dmg`
- `dist/release/appcast.xml`
- `dist/release/SHA256SUMS`

Inspect and launch the resulting bundle to verify the release behavior before
answering the final **Publish** prompt. Confirm that App Preferences shows the
expected version and commit, and check custom URLs and window closing/reopening.
Open the DMG and check its background, icon alignment, and Applications shortcut.
Double-click the Applications shortcut to confirm it opens the correct folder.
For a full installation check, quit Gatto, drag the app into Applications, and
launch it from there. If you want to keep your existing installation, copy the
new app to a temporary folder and launch that copy instead. Automatic installer
ejection is deliberately skipped outside the two supported Applications locations;
use a disposable macOS user to verify the complete installation flow.
Answering `y` creates the GitHub release/tag at the exact commit and uploads the
DMG, signed appcast, and checksums, using the prepared changelog notes. After
stable publication, the script automatically refreshes and verifies the
[README size figures](#readme-size-figures) for that exact tag using the local DMG.

Nested Sparkle helpers, framework, and the Swift bridge are signed before the
outer bundle. Sparkle handles background scheduling every four hours and persists
its last check time. Automatic downloading and installation stay disabled; Gatto
provides the subtle indicator and explicit install click.
The signed feed at the latest release's `appcast.xml` points to that release's
immutable DMG URL. Do not edit the feed after signing it. `SHA256SUMS` covers the
DMG and feed. Keep the latest release stable and compatible with macOS 13; the
single-item feed does not preserve updates for older OS support branches.

### Stable and Beta publishing

The first beta is **0.17.0-beta.1**, including release channels and the dependency
reduction. Keep these changes together when preparing its release. Run the usual
release script and accept that prepared version. To continue the same beta,
choose **Advance to the next Beta** (for example, beta.9 becomes beta.10).
To promote it, choose **Promote this Beta to Stable** (0.17.0-beta.9 becomes
0.17.0). Every option shows the exact version and whether it is Beta or Stable
before selection. The prepared version remains the default until it is published.

Select a number or type an option name: `prepared`, `beta`, `stable` (or
`promote`), `patch`, `minor`, `major`, or `custom`. Explicit versions still work.
From a stable version, the Beta option starts a preview of the next minor
release. Patch/minor/major options produce Stable releases based on the newer of
the current prepared version and latest published version; their destinations
are displayed. Use Custom to start a beta of a different version. Invalid choices
explain the problem and repeat the prompt without changing files. At beta.255,
advancing the same beta is omitted; promotion and other version options remain.

Beta sequence numbers are 1–255, matching Apple's bundle suffix range.

Cargo, visible versions and tags use `0.17.0-beta.1` / `v0.17.0-beta.1`.
`CFBundleVersion` uses **0.17.0b1**; Stable uses **0.17.0**. Sparkle ignores hyphen
build suffixes, so putting the visible beta string in CFBundleVersion would make
successive betas and the final release compare equal. Preparation checks both
forms. Beta appcast items have `<sparkle:channel>beta</sparkle:channel>`; Stable
items have no channel. Do not edit signed appcasts after generation.

Stable subscribers use the existing `releases/latest/download/appcast.xml` URL.
Beta subscribers use `releases/download/beta/appcast.xml`. The fixed `beta` tag
belongs to a separate prerelease named **Beta channel feed**, containing discovery
metadata rather than an installer. `publish-beta-feed.sh` downloads and verifies
the previous Beta feed, the latest Stable feed and the published version's feed;
the tracked Swift XML merger retains the newest release from each channel.
It checks channel/version agreement and immutable Gatto DMG URLs, preserves
archive signatures, signs the combined feed again, uploads it, downloads it back,
and compares its exact bytes and signature. Beta users therefore also discover
newer stable releases. This process adds no package dependency.

Refresh that discovery feed after **either** channel publishes. A missing or
failed refresh exits with a `--resume` recovery path; the verified versioned
release stays published. Resume repeats the feed refresh safely and preserves
the version's prerelease/latest flags. It needs the existing Sparkle signing key
and SDK, including when recovering on a different machine. Never mark the fixed
`beta` feed release as latest, replace a stable feed with a beta-only feed, or
point Homebrew/the website's stable download at a beta. Resume has no new build
step before accessing signing credentials.

Channel changes invalidate cached offers and in-flight results. Switching to
Stable while running a newer beta keeps that beta until a stable version catches
up; it never performs a lower-version replacement. This follows Sparkle's
downgrade protection. This beta's settings remain additive; see
[Support](../SUPPORT.md#app-updates) for data compatibility and manual rollback.

The script then refreshes `Casks/gatto.rb` from that exact release's checksum,
verifies it with `--check`, and commits and pushes it with the README update so
users of the repository tap receive it on their next `brew update`. To repair a stale
cask independently, run `bun scripts/update-homebrew-cask.ts`. Starting with the
first updater-enabled release, the cask declares `auto_updates true`.

The script currently builds only Apple Silicon. Intel users need a local build;
changing the release targets requires a script change and separate verification.

## Interrupted or cancelled releases

Before the commit confirmation, cancellation leaves the prepared version and
notes in the working tree. Before publication, cancellation leaves signed files
in `dist/release`; rerunning normally reuses the prepared version/notes and
rebuilds those generated files. A failed check never commits or publishes.

Uploads are drafts until every asset has been downloaded and verified against
its local bytes and checksums. To finish an existing draft or published release:

```sh
./scripts/release.sh --resume
```

Resume checks that the checkout matches the release commit except for README/cask
metadata. A draft must retain its original local assets: the script repairs those
uploads, verifies them, and asks before publishing. For an already published
release it verifies the uploaded artifacts and only finishes metadata updates and
their commit/push. It never rebuilds, republishes, or replaces published assets.
If local artifacts are absent after publication, the size updater downloads the
release DMG. A failed metadata push can be retried with the same resume command.

Keep failed draft assets until recovery completes. Repairing a tag that points at
unrelated code, merging diverged branches, or changing lost signing credentials
requires inspecting the error; the script stops instead of rewriting history or
silently changing keys.

## Verify the download

Follow [Setup’s checksum instructions](SETUP.md#1-install-the-app) for the
published DMG and `SHA256SUMS`.

Open the DMG, drag Gatto onto Applications, then launch it from Applications and
verify its version and commit. The matching **Install Gatto** disk should eject
in the background; the downloaded DMG should remain. Also verify that a busy
disk or two mounted copies of the same installer stay mounted without a prompt,
and that launching the copy inside the DMG does not eject it. These checks use
disposable installations; do not overwrite an installation you need to retain.
Install before enabling Start at Login. Do not ask users to bypass a
failed signature or Gatekeeper assessment; investigate and replace the artifact.

## Verify updates and Homebrew

Run `./scripts/test-platform.sh` for native menu/hotkey checks and an Automator
workflow test against a disposable signed app. It verifies cold and existing-app
file delivery without changing the installed Gatto or installing a user service.
Before publishing, also use **Gatto → Install Finder Quick Action…** on a disposable
macOS user, enable it through Finder's Quick Actions customization if needed, and
send a single image, a mixed image/video batch, and a selection containing an
unsupported file. Verify the existing-draft choices and oversized-video prompt,
that nothing uploads automatically, and that removing the workflow from
`~/Library/Services` removes the action. Test the minimum supported macOS as well
as the release Mac; the automated harness cannot verify Finder's settings UI.

Run `./scripts/test-updater.sh` for native timing, consent, and signed-feed
verification/tamper tests, and
`bun test scripts` for cask, version/notes, asset-verification, and release-workflow
tests. The workflow tests use isolated stand-ins for signing, builds, GitHub, and
Git; they never publish or push to a real repository.
On a logged-in macOS desktop, run `bun scripts/test-update-install.ts` to test
actual replacement and relaunch using disposable ad-hoc signed app bundles and a
local feed. It uses Gatto's production native driver and real Sparkle scheduler,
verifies that recent checks suppress requests, and tests both an overdue check
and a cached badge clicked during startup. It simulates the explicit install
click and verifies that only then does the archive download and the new version
launch. These tests
do not modify the real app's preferences, Keychain, or installation.

Before publishing the first updater-enabled release, use disposable signed
bundles and a test feed to exercise an older-to-newer install and relaunch,
corrupt archive/feed rejection, permission failure, retry, and a Homebrew
installation. Verify no download occurs before the click, no check occurs inside
the saved four-hour interval, and an overdue running/background app checks again.
An initial DMG/Homebrew install is necessary for users of v0.14.0 and earlier.
A Developer ID signed two-version update remains a release acceptance check; the
local fixture verifies replacement and relaunch without testing notarization.

## Preview the installation window

See the [installation screenshot in Setup](SETUP.md#1-install-the-app) for the
expected window layout.

Build a local DMG from an existing app bundle without signing, notarizing, or publishing:

```sh
./scripts/build-dmg.sh dist/release/Gatto.app dist/gatto-preview.dmg
open dist/gatto-preview.dmg
```

The output path must not already exist. The preview preserves the input bundle's
signature, but the disk image itself is unsigned and is not a release artifact.
It creates a fresh volume UUID, so it does not receive the release's automatic
ejection pairing. Eject it in Finder when finished. The build uses a temporary
mount point and cleans it up on exit, without depending on or ejecting any
existing Gatto volume.
Mounting and cleanup use `diskutil image attach` and `diskutil eject`;
`hdiutil` creates, compresses, and verifies the image. Temporary mounts use the
`nobrowse` mount option so Finder opens only when the layout script requests it.

`packaging/render-dmg-background.swift` typesets one instruction over the original
dark anime-cat artwork in `packaging/dmg-artwork.png`. The generation prompts are
recorded in [the artwork notes](../packaging/dmg-artwork.md).
The Applications shortcut uses the system's actual folder icon.
`packaging/configure-dmg.applescript` positions the real Finder icons over the
artwork; keep those coordinates aligned if you change the design. The background
includes both standard and Retina resolutions. Finder's black unselected icon
names blend into the lower black band; native names remain accessible and appear
when an icon is selected.

## README size figures

The README reports the **latest published Apple Silicon release**, measured from
its DMG. On macOS with [Bun](https://bun.com/docs/installation) and GitHub CLI
available, run:

```sh
bun scripts/update-readme-size.ts
bun scripts/update-readme-size.ts --check
```

The TypeScript script needs no package installation.
It downloads the release’s DMG and `SHA256SUMS`, verifies its hash and
GitHub asset size, mounts it read-only at a temporary location, and sums regular
file bytes in `Gatto.app`. It checks the bundle version and architecture, ejects
only that temporary mount, and updates the marked README download button and size
paragraph. The button uses `docs/images/download-macos.svg` and links to the
measured release. It leaves your installed app and existing mounts alone.
`--check` reports stale figures
without editing the README. To reuse a downloaded release, add
`--dmg /absolute/path/to/gatto-<version>-aarch64-apple-darwin.dmg`; its bytes must
match the published asset. `--tag v<version>` selects a specific release.

Sizes use decimal MB (1 MB = 1,000,000 bytes), rounded to one decimal. The installed
figure is the sum of bundle file lengths, excluding installer artwork, filesystem
allocation overhead, downloaded DMGs, and the separately installed GitHub CLI.
Ejecting the installer unmounts it; it does not delete the download. Compression
means the DMG can be smaller than the app it contains.

The initial v0.12.0 measurement on 2026-10-14 was **16,598,419 bytes** downloaded
and **29,959,166 bytes** in the bundle. The local DMG matched the published asset’s
SHA-256. Future runs replace the README figures with measurements of the selected
release; this dated observation remains a baseline.

`release.sh` runs the updater and its `--check` verification after every successful
publication. It commits and pushes the README and cask updates after verifying both. The
commands above remain available for manual inspection; `--resume` finishes
interrupted post-publication steps. For changes
to code, embedded assets, dependencies, or packaging before publication, keep the
README’s released version label accurate and refresh the figures when that build
ships. See [documentation upkeep](agents/documentation.md) for ownership rules.

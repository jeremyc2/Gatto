# Releasing Gatto

Releases are built locally on macOS with `scripts/release.sh`. The script produces
a Developer ID signed and notarized Apple Silicon app and disk image, then
publishes the DMG and checksum to GitHub Releases. There is no active release
workflow in GitHub Actions.

## Prerequisites

- The [development tools](../CONTRIBUTING.md#development-setup) and an authenticated `gh`
  account with permission to create releases in this repository.
- Apple signing/notarization tools available through `security`, `codesign`,
  `xcrun notarytool`, `xcrun stapler`, `spctl`, `plutil`, `ditto`, `hdiutil`,
  `swift`, `tiffutil`, and `osascript`.
- A logged-in macOS desktop session. Finder saves the disk image's window layout;
  allow your terminal to control Finder if macOS asks on the first build.
- A **Developer ID Application** certificate and its private key in the login
  keychain. Check with `security find-identity -v -p codesigning`.
- Notarization credentials stored in the `gatto-notary` keychain profile. Follow
  the [one-time setup](#one-time-signing-setup); do not commit credentials.

| Environment variable | Purpose |
| --- | --- |
| `SIGN_IDENTITY` | Signing certificate name. Required if more than one Developer ID Application identity is present. |
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

## Prepare the release

1. Add release notes to [CHANGELOG.md](../CHANGELOG.md), moving the relevant
   Unreleased entries into the new version/date section.
2. Set the same version in `Cargo.toml` and
   `packaging/Info.plist` (`CFBundleShortVersionString`). Update `Cargo.lock` to
   reflect the package version with `cargo check`, and review the resulting diff.
   The release script copies the plist as-is; it does not synchronize these versions.
3. Run the [contributor checks](../CONTRIBUTING.md#code-quality). Manually verify
   changed workflows, regenerate affected screenshots, and review documentation.
4. Commit and push to `main`. The working tree must be clean and HEAD must equal
   `origin/main` after fetching. A release named `v<version>` must not already exist.

## Build and publish

From the repository root:

```sh
./scripts/release.sh
```

The script installs the `aarch64-apple-darwin` Rust target, performs a locked release
build, and creates `dist/release/Gatto.app` with the plist, binary, and icon assets.
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
DMG and checksum, with GitHub-generated release notes. After publication,
[refresh the README size figures](#readme-size-figures) and commit the documentation update.

The script currently builds only Apple Silicon. Intel users need a local build;
changing the release targets requires a script change and separate verification.

## Interrupted or cancelled releases

Cancelling the publication prompt leaves the artifacts in `dist/release` and does
not publish a release. A rerun deletes and rebuilds that directory, including any
previous artifacts; copy anything you need to retain elsewhere first.

If signing or notarization fails, fix the certificate/profile issue and rerun.
If publication fails, inspect `gh release view v<version>` and its assets before
retrying: GitHub may have created the release even if an asset upload failed.
The script refuses to rerun against an existing release. Repair missing assets
on that release after checking its tag/commit rather than deleting it blindly.

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
only that temporary mount, and updates the marked README paragraph. It leaves
your installed app and existing mounts alone. `--check` reports stale figures
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

After each publication, run the updater and commit its README change. For changes
to code, embedded assets, dependencies, or packaging before publication, keep the
README’s released version label accurate and refresh the figures when that build
ships. See [documentation upkeep](agents/documentation.md) for ownership rules.

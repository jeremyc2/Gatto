# Gatto

<p align="center">
  <img src="packaging/rocket-cat-transparent.png" alt="Gatto, a cat with rocket boosters" width="220">
</p>

A lightweight macOS menu bar app for uploading images to GitHub's native
user-attachment storage. Set an organization in App Preferences, paste, drop, or select an
image, then copy a ready-to-paste Markdown image snippet from the uploaded attachment.

The app is written in Rust with GPUI and GPUI Kit. GPUI Kit is pinned to `0.7.1`;
that release pins and re-exports its compatible GPUI `0.3.8` snapshot. The
repository's `rust-toolchain.toml` tracks the stable Rust channel and installs
the formatter and linter components.

> This project currently supports macOS only.

## Downloads

Releases are built, signed with a Developer ID certificate, and notarized on the
maintainer's Mac (see [Releasing](#releasing)), then published on this
repository's **Releases** page. Download
`gatto-<version>-aarch64-apple-darwin.zip` for Apple Silicon Macs and
unzip it to get `Gatto.app`. Move the app to `/Applications` before
enabling Start at Login, because the login item records the app's location.
`SHA256SUMS` contains the archive's SHA-256 checksum. GitHub CLI and
authentication are still required as described below. Intel Macs currently require
a local build.

## Features

- Lives in the macOS menu bar, and appears in the Dock while its window is open.
- Uses a dark theme only, with the same icon in the menu bar, Dock, and app window.
- Accepts clipboard images, drag-and-drop, and image files.
- Uploads through a repository from an organization you set in App Preferences, public or private, using the
  active GitHub CLI account.
- Uploads PNG, JPEG, GIF, and WebP images.
- Keeps previewed, uploaded, and failed images together as status-aware attachments.
- Adds a Markdown copy button to each attachment after it uploads.
- Can start automatically at macOS login.
- Shows the build's app version and Git commit hash in App Preferences.
- Preserves the staged image when GitHub returns an error so the upload can be retried.
- Includes an **Application logs** window with a copy button for troubleshooting.
- Provides menu bar actions for previewing a clipboard image or uploading and copying its Markdown
  in the background with a native macOS notification.

## Prerequisites

You will need:

- macOS 13 or newer.
- Xcode Command Line Tools.
- Rust installed through `rustup`.
- [GitHub CLI](https://cli.github.com/) authenticated with an account that can
  access the repository you want to upload through.

### 1. Install the macOS developer tools

```sh
xcode-select --install
```

If macOS reports that the tools are already installed, no further action is
needed. You can verify the selected developer directory with:

```sh
xcode-select -p
```

### 2. Install Rust

Install Rust with the official `rustup` installer:

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Choose the default installation when prompted, then restart your terminal. Verify
the installation:

```sh
rustc --version
cargo --version
rustup --version
```

Entering this repository makes `rustup` select the stable toolchain declared in
`rust-toolchain.toml`. To ensure the required tools are available:

```sh
rustup component add rustfmt clippy
```

### 3. Install and authenticate GitHub CLI

With [Homebrew](https://brew.sh/) installed:

```sh
brew install gh
gh auth login
gh auth status
```

Choose `GitHub.com` and HTTPS during login. The authenticated account must be able
to read the repositories of the organization you plan to configure in App Preferences.

The app does not need Accessibility, Automation, Full Disk Access, or other special macOS
permissions to run GitHub CLI. It launches `gh` as the signed-in user, and GitHub CLI uses that
user's existing authentication. A GUI app may not inherit your shell's Homebrew path, so the app
also checks `/opt/homebrew/bin/gh` (Apple Silicon) and `/usr/local/bin/gh` (Intel). If you use a
custom install location, launch the app with `GH_PATH=/absolute/path/to/gh` set in its environment.

## Project setup

Clone the repository, substituting its actual URL for the placeholder:

```sh
git clone <repository-url>
cd gatto
cargo fetch
```

Start a development build with:

```sh
cargo run
```

The app appears in the macOS menu bar and the Dock. Closing its window hides the
window and the Dock icon without quitting the app; use the menu bar item to show
it again or quit.

## Using the app

1. Open the app from the menu bar and, on first launch, set an organization in
   App Preferences. The app does nothing until one is set.
2. Paste an image, drag one into the window, or choose an image file.
3. Upload the image.
4. Wait for the attachment status to change to **Uploaded**.
5. Use the attachment's copy button, then paste the Markdown into a GitHub issue,
   pull request, discussion, or Markdown file.

For a faster path, pin a repository in App Preferences, copy an image, then choose **Quick Copy**
from the menu bar icon. Gatto uploads without opening its window, copies the resulting Markdown, and
sends a native macOS notification when it finishes. When multiple repositories are pinned, Quick
Copy uses the most recently selected pinned repository, falling back to the alphabetically first
one. Choose **Preview from Clipboard** when you want to stage the image in the app instead. These
menu items appear only while at least one repository is pinned.

The same actions are available to Shortcuts, shell scripts, launchers, and other macOS automation
through Gatto's custom URL scheme:

```text
gatto://open
gatto://preview
gatto://quick-copy
gatto://settings
```

For example, `open "gatto://quick-copy"` uploads the clipboard image through the preferred pinned
repository and copies its Markdown without leaving the app window open. As with the menu bar
action, Quick Copy requires an organization and at least one pinned repository.

Normal uploads do not change the clipboard. An uploaded attachment's copy button copies a value such as
`![screenshot.png](https://github.com/user-attachments/assets/…)`.

On the uploader screen, press **Command+Shift+C** to copy all uploaded URLs or
**Command+Shift+M** to copy all uploaded Markdown image snippets. These shortcuts do nothing
until an upload result is available.

## App Preferences

Open App Preferences from the app window or choose **App Preferences** from the menu bar icon.

- Enter your GitHub organization and save it. It is required, and it is stored in
  the current user's Application Support folder.
- Select a repository and pin it to place it ahead of unpinned repositories in the
  repository picker.
- Enable **Start at Login** to install a per-user macOS LaunchAgent. If the app is
  moved after enabling this setting, turn the setting off and on again so the
  saved executable path is refreshed.
- The About section shows the package version and the Git commit hash captured at
  build time. Builds made outside a Git checkout show `unknown` unless
  `GIT_COMMIT_HASH` is supplied to the build.
- **Application logs** opens a separate, in-memory event log. It records safe lifecycle,
  image-processing, GitHub CLI, and upload events; use **Copy all** when reporting an issue.
  Logs are discarded when the app quits and never include tokens or image bytes.

The app lists the organization's repositories through `gh api`, including private
repositories available to the active GitHub CLI account.

## Development commands

```sh
# Verify formatting without changing files
cargo fmt --all -- --check

# Check the project without producing a release binary
cargo check --all-targets

# Run the Rust linter
cargo clippy --all-targets --all-features -- -D warnings

# Run automated tests
cargo test --all-targets

# Audit advisories, licenses, and dependency sources
cargo install --locked cargo-deny --version 0.20.2
cargo deny --locked check --hide-inclusion-graph advisories licenses bans sources

# Create an optimized binary
cargo build --release
```

See [CONTRIBUTING.md](CONTRIBUTING.md) before opening a pull request.

## Releasing

`scripts/release.sh` builds, signs, notarizes, staples, and publishes a GitHub
release from a Mac. No signing secrets live in the repository or GitHub: the
certificate stays in the login keychain and the notarization credentials in a
keychain profile.

One-time setup:

1. In the Apple Developer account, create a **Developer ID Application**
   certificate (Xcode → Settings → Accounts → Manage Certificates → + → Developer
   ID Application). Check it with `security find-identity -v -p codesigning`.
2. Create an app-specific password at account.apple.com, then store it:
   ```sh
   xcrun notarytool store-credentials gatto-notary \
     --apple-id <apple-id> --team-id <team-id>
   ```
   Enter the app-specific password when prompted.

To release, bump the version in `Cargo.toml` and `packaging/Info.plist`, commit and
push to `main`, then run:

```sh
./scripts/release.sh
```

The script refuses to run unless the tree is clean and pushed, and it asks before
publishing `v<version>`. Set `SIGN_IDENTITY` if more than one Developer ID
certificate is installed.

## Project layout

```text
src/main.rs       Application entry point and GPUI startup
src/app.rs        Main window, interactions, and upload state
src/github.rs     GitHub CLI integration and attachment uploads
src/diagnostics.rs In-memory troubleshooting event log
src/log_viewer.rs Application logs window
src/menu_bar.rs   macOS menu bar behavior
src/model.rs      Shared application models
src/settings.rs   Repository setting and macOS login startup
build.rs          Build-time Git commit metadata
scripts/          Local signed-release script
packaging/        macOS application metadata
.github/          Continuous integration and contribution templates
```

## Project documentation

- [App walkthrough](docs/WALKTHROUGH.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Contributing guide](CONTRIBUTING.md)
- [Security policy and dependency audit](SECURITY.md)
- [Support guide](SUPPORT.md)
- [Code of Conduct](CODE_OF_CONDUCT.md)
- [Changelog](CHANGELOG.md)
- [License](LICENSE)

## Troubleshooting

### Rust linker or SDK errors

Install or update the Xcode Command Line Tools, then confirm `xcode-select -p`
prints a valid developer directory. After a macOS or Xcode upgrade, accepting the
Xcode license or reinstalling the tools may be necessary.

### The repository cannot be loaded

Run `gh auth status`, confirm the correct GitHub account is active, and verify that
it can access the organization set in App Preferences. For SAML-protected organizations,
authorize the token for single sign-on through GitHub. If it works in Terminal but not in the
app, open **Application logs**: it records the `gh` path the app found and the command's safe
error detail. Install Homebrew's `gh` in its normal location or set `GH_PATH` for custom installs.

### Uploads fail

Confirm the staged file is a supported image and that `gh auth token` succeeds.
GitHub's user-attachment upload endpoint is undocumented and may change without
notice; the app displays the returned error and keeps the image staged for retrying.

## Security and privacy

The app obtains the active token from GitHub CLI when needed and does not persist
it in project configuration. Image bytes are sent directly to GitHub's attachment
service. Do not upload secrets or images you are not permitted to share.

The committed `Cargo.lock` preserves the versions and registry checksums that were
audited. Dependency source and license policy lives in `deny.toml`, and automated
security scans run on dependency changes and every week. For the latest audit
results or to report a vulnerability, see [SECURITY.md](SECURITY.md).

## Community and license

Contributions are welcome. Please read [CONTRIBUTING.md](CONTRIBUTING.md) and
[CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md). Changes are documented in
[CHANGELOG.md](CHANGELOG.md).

This project is available under the [MIT License](LICENSE).

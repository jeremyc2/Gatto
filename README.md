# Gatto

<p align="center">
  <img src="packaging/rocket-cat-transparent.png" alt="Gatto, a cat with rocket boosters" width="220">
</p>

Lightweight app for adding images to GitHub Pull Requests and GitHub Issues. Configure
your GitHub organization in App Preferences, paste, drag-and-drop, or select an image,
then copy a Markdown image snippet from the uploaded attachment.

**Your images are stored by GitHub. Gatto runs locally on your Mac and has no
backend or server of its own.** It uses the same image attachment mechanism as pasting an image into
a GitHub issue description or pull request body on GitHub's website or in VS Code.
Gatto gives you a friendlier UI for uploads that can otherwise be clunky or unreliable.

## Where your images are stored

When you upload an image with Gatto, it goes directly from your Mac to GitHub's
image attachment storage. GitHub returns an image link that you can copy and use
in an issue, pull request, or anywhere else you use Markdown. The image stays
hosted by GitHub. Gatto has no backend or server: the app on your Mac communicates
directly with GitHub.

Behind the scenes, Gatto uses GitHub CLI to sign in with your existing GitHub
account and GitHub's API to upload the image through the same attachment mechanism
used when you paste images into issues and pull requests. Gatto makes that process
easier to use without running a separate image storage service.

## Platform and implementation

The app is written in Rust with GPUI and GPUI Kit. GPUI Kit is pinned to `0.7.1`;
that release pins and re-exports its compatible GPUI `0.3.8` snapshot. The
repository's `rust-toolchain.toml` tracks the stable Rust channel and installs
the formatter and linter components.

> This project currently supports macOS only.

## Downloads

Releases are built, signed with a Developer ID certificate, and notarized on the
maintainer's Mac (see [Releasing](#releasing)), then published on this
repository's **Releases** page. Download
`gatto-<version>-aarch64-apple-darwin.dmg` for Apple Silicon Macs, open it,
and drag Gatto onto the Applications folder in the installation window. Then open
Gatto from Applications and eject the **Install Gatto** disk. Install the app before
enabling Start at Login, because the login item records the app's location.
`SHA256SUMS` contains the disk image's SHA-256 checksum. GitHub CLI and
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
- Supports bulk uploads, per-image descriptions, partial-failure retries, and copying all results.
- Opens a separate image preview with zoom, pan, and previous/next navigation.
- Can start automatically at macOS login.
- Shows the build's app version and Git commit hash in App Preferences.
- Preserves the staged image when GitHub returns an error so the upload can be retried.
- Includes an **Application logs** window with a copy button for troubleshooting.
- Provides menu bar actions for previewing a clipboard image or uploading and copying its Markdown
  in the background with a native macOS notification.
- Can run Quick Copy from an optional, customizable global keyboard shortcut without requiring
  Accessibility permission.
- Opens the full walkthrough in the default browser from the main window's help icon.
- Supports `gatto://` automation actions and includes the Rooftops game behind the cat logo.

## Prerequisites

To use a downloaded release, you need macOS 13 or newer and an authenticated
GitHub CLI installation (step 3 below). Xcode and Rust are only needed to build
from source.

For a local build, you will need:

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

Clone the repository:

```sh
git clone https://github.com/jeremyc2/Gatto.git
cd Gatto
cargo fetch
```

Start a development build with:

```sh
cargo run
```

The app appears in the macOS menu bar and the Dock. Closing a window hides
and retains it for reuse without quitting the app. This avoids a native window
teardown crash on Touch Bar Macs. After the last window closes, the Dock icon
hides; use the menu bar item or click Gatto in the Dock to open the main page again.
Closing the main window discards staged images and upload results, so copy
anything you need first. Use **Quit** in the menu bar to exit.

## Using the app

1. Open the app from the menu bar and, on first launch, set an organization in
   App Preferences. Enter the organization login, such as `acme`, rather than a
   URL or `owner/repository`. Uploads require an organization.
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
repository and copies its Markdown entirely in the background, without creating a window. As with the menu bar
action, Quick Copy requires an organization and at least one pinned repository.

See the [walkthrough](docs/WALKTHROUGH.md#bulk-uploads-and-retries) for bulk uploads,
retry behavior, descriptions, and full-size preview controls. Quick Copy runs
independently of the app window and its staged images. On failure it leaves the
clipboard unchanged and sends a notification; normal uploads keep their
attachments available while the window remains open.

Normal uploads do not change the clipboard. An uploaded attachment's copy button copies a value such as
`![screenshot.png](https://github.com/user-attachments/assets/…)`.

On the uploader screen, press **Command+C** to copy all uploaded Markdown image snippets.
This shortcut does nothing until an upload result is available.
It also works for a single image, and a shortcut hint appears after any successful upload.

## App Preferences

Open App Preferences from the app window or choose **App Preferences** from the menu bar icon.

- Enter your GitHub organization and save it. It is required, and it is stored in
  the current user's Application Support folder.
- Select a repository and pin it to place it ahead of unpinned repositories in the
  repository picker.
- Enable **Start at Login** to install a per-user macOS LaunchAgent. If the app is
  moved after enabling this setting, turn the setting off and on again so the
  saved executable path is refreshed.
- Enable the global Quick Copy shortcut to upload the clipboard image from any app. The shortcut is
  off by default, starts with `Command+Shift+G`, and requires at least two modifier keys when
  customized. Gatto uses the macOS hotkey API, so this does not require Accessibility permission.
- Enable **Close the window after copy** to close Gatto’s window after copying uploaded Markdown
  from the main window. Closing clears the attachment list, including uncopied results.
- The About section shows the package version and the Git commit hash captured at
  build time. Builds made outside a Git checkout show `unknown` unless
  `GIT_COMMIT_HASH` is supplied to the build.
- **Application logs** opens a separate, in-memory event log. It records safe lifecycle,
  image-processing, GitHub CLI, and upload events; use **Copy all** when reporting an issue.
  Logs are discarded when the app quits and never include tokens or image bytes.

The app lists the organization's repositories through `gh api`, including private
repositories available to the active GitHub CLI account.

### Reset and uninstall

**App Preferences → Reset app** asks for confirmation, removes saved preferences
and the login item, disables the global shortcut, and clears the main upload
state. It does not sign you out of GitHub CLI or delete images already hosted by
GitHub.

To uninstall, reset the app first, choose **Quit** from the menu bar, and move
`Gatto.app` to the Trash. If the app cannot open, quit it and remove its settings
and login item manually using the exact paths in
[the support guide](SUPPORT.md#local-data-and-recovery). Rooftops scorecards saved
to Downloads are separate files; keep or remove them as you prefer.

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

These checks currently run locally: the workflow definitions are stored in
`.github/workflows.disabled/`, so GitHub Actions does not execute them. See
[CONTRIBUTING.md](CONTRIBUTING.md) before opening a pull request.

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
certificate is installed. For prerequisites, verification, artifact locations,
and recovery from an interrupted release, see the [release guide](docs/RELEASING.md).

## Project layout

The repository is organized into a few main areas:

- [`src/`](src/) — the Rust application, including its interface, GitHub uploads,
  and macOS integration.
- [`docs/`](docs/) — walkthroughs, architecture, and contributor guidance.
- [`scripts/`](scripts/) and [`packaging/`](packaging/) — tools and assets for
  building, packaging, and releasing the app.
- [`.github/`](.github/) — automated checks and contribution templates.

For how the app's parts work together, see the [architecture overview](docs/ARCHITECTURE.md).

## Project documentation

- [App walkthrough](docs/WALKTHROUGH.md)
- [Rooftops Easter egg](docs/EASTER_EGG.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Contributing guide](CONTRIBUTING.md)
- [Release guide](docs/RELEASING.md)
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

Confirm the staged file is a supported image and that
`gh auth status --hostname github.com` succeeds. Retry with **Upload**; successful attachments are skipped.
See [SUPPORT.md](SUPPORT.md) for account switching, clipboard, shortcut, and
settings recovery. Avoid sharing the output of `gh auth token`, which is a secret.

## Security and privacy

Images are hosted by GitHub using its existing issue and pull request attachment
mechanism, as described in [Where your images are stored](#where-your-images-are-stored).
Gatto runs locally on your Mac, has no backend or server of its own, and does not
host your images.

The app obtains the active token from GitHub CLI when needed, caches it in memory,
and does not persist it in project configuration. Restart Gatto after switching
GitHub CLI accounts. Image bytes are sent directly from your Mac to
GitHub's attachment service. Do not upload secrets or images you are not permitted
to share.

The committed `Cargo.lock` preserves the versions and registry checksums that were
audited. Dependency source and license policy lives in `deny.toml`, and scanner
workflow definitions are retained under
`.github/workflows.disabled/`. Run dependency scans locally while those workflows
are disabled; Dependabot is configured to check for updates weekly. For the
recorded audit results or to report a vulnerability, see [SECURITY.md](SECURITY.md).

## Community and license

Contributions are welcome. Please read [CONTRIBUTING.md](CONTRIBUTING.md) and
[CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md). Changes are documented in
[CHANGELOG.md](CHANGELOG.md).

This project is available under the [MIT License](LICENSE).

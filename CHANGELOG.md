# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0] - 2026-10-08

### Added

- An in-memory Application logs window with safe diagnostic events and copy support.
- GitHub CLI path diagnostics, including standard Homebrew locations and the optional
  `GH_PATH` override for custom installations.
- A Quick Paste menu bar action for staging a clipboard image against a pinned repository.
- Minimum and maximum window sizes for the main app and Application logs windows.
- A transparent rocket-cat illustration for the app header and project README.
- A close control on staged image previews for discarding an image draft.

### Changed

- Rebranded the app, package, bundle, release artifact, settings directory, and login item as
  Gatto.
- Reworked the Dock icon as a rounded blue-to-indigo tile with a dimensional rocket-cat mark.
- Reworked the main header into a responsive hero layout with an enlarged illustration.
- Closing the main window discards its staged image, and Quick Paste always opens the main page
  with a fresh image draft.

## [0.2.0] - 2026-10-07

### Added

- Required GitHub organization setting; the app asks for it on first launch and
  loads that organization's repositories.
- Dock icon while the window is open, alongside the menu bar item.
- Dark-only theme with an accent color palette.
- App icon built with Icon Composer, shared by the Dock, menu bar, and app window.
- Manually triggered release workflow publishing an ad-hoc signed Apple Silicon
  `Gatto.app` bundle archive and SHA-256 checksum to a
  commit-specific GitHub prerelease from `main`.
- Contributor, security, community, and project setup documentation.
- GitHub issue and pull request templates.
- Automated formatting, linting, checking, and test workflow definitions.
- Automated dependency update configuration.
- A committed Cargo lockfile, dependency source/license policy, and manually
  triggered RustSec/OSV supply-chain scanning.
- App Preferences page with a persistent organization, pinned repositories, and Start at Login.
- App version and build commit hash in App Preferences.
- Menu bar action for opening App Preferences.
- Explicit copy buttons for the uploaded URL and Markdown image snippet.
- Command+Shift+C and Command+Shift+M shortcuts for copying the uploaded URL and
  Markdown image snippet, with shortcut hints in the copy button tooltips.

### Changed

- Renamed Settings to App Preferences.
- The main screen scrolls when the window is small, and long text wraps.
- Status messages appear only for warnings, errors, and copy confirmations.

- Updated GPUI Kit from 0.7.0 to 0.7.1, including its compatible GPUI 0.3.8
  snapshot.
- Updated tray-icon from 0.26.0 to 0.26.1.
- Pinned GitHub Actions by immutable commit and removed third-party toolchain/cache
  actions from the primary CI workflow.
- CI checks formatting on Linux before running macOS lint and tests, caches Cargo
  output with `actions/cache`, and skips documentation-only changes.
- OSV scanning ignores informational advisories on transitive GPUI dependencies
  that have no patched release, with expiry dates for re-review.
- Upload results are displayed without automatically changing the clipboard.
- Documentation now describes settings persistence, login startup, and explicit
  clipboard behavior.

## [0.1.0] - 2026-10-07

### Added

- Initial macOS menu bar application built with Rust, GPUI, and GPUI Kit.
- Clipboard, drag-and-drop, and file-picker image staging.
- Repository loading through the authenticated GitHub CLI account.
- GitHub user-attachment uploads with automatic clipboard copying.
- Retry-friendly error handling that preserves the staged image.

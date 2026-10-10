# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.8.1] - 2026-10-10

### Changed

- Removed the Command+Shift+C URL-copy shortcut. When multiple uploads are available, the uploader
  now shows a Command+Shift+M hint for copying all uploaded Markdown snippets.

## [0.8.0] - 2026-10-10

### Added

- Added an optional global Quick Copy keyboard shortcut with a built-in shortcut recorder and
  macOS hotkey registration that does not require Accessibility permission.
- Added a preference to close the main window after copying uploaded URLs or Markdown snippets.
- Added a help icon that opens the repository walkthrough in the default browser.

### Changed

- `cargo run` now starts the Gatto app directly even though the repository also contains the
  walkthrough screenshot generator.
- Replaced wordy actions and supporting text throughout the main window, App Preferences, and
  Application Logs with compact icons, concise labels, and contextual tooltips.
- Simplified pinned-repository management and clarified the organization, Quick Copy, and general
  preference groups.
- Expanded the walkthrough to cover GitHub CLI setup, the shortcut recorder, the custom `gatto://`
  URL scheme, and every supported action, with updated screenshots.

### Fixed

- The repository picker now opens immediately when loading begins, shows its loading indicator
  inside the dropdown, and stays open when the repository list arrives.

## [0.7.0] - 2026-10-10

### Added

- Added `gatto://` URLs for opening the app, previewing the clipboard, running Quick Copy, and
  opening App Preferences from macOS automation.

## [0.6.2] - 2026-10-09

### Added

- Added an app walkthrough with regenerated screenshots and a deterministic macOS command for updating them.

## [0.6.1] - 2026-10-09

### Fixed

- Clicking the Dock icon reopens and activates the main screen after its window has been hidden.

## [0.6.0] - 2026-10-09

### Added

- Copy all uploaded Markdown snippets from the bulk attachment list, joined by newlines.

### Changed

- Uploaded results now remain in the attachment list with color-coded preview, uploading,
  uploaded, and failed states; each uploaded attachment owns its Markdown copy button instead of
  rendering a separate URL/Markdown success panel.
- Renamed Paste & Preview to Preview from Clipboard and Paste to URL to Quick Copy. Quick Copy now
  copies Markdown, runs in the background, and reports completion through a native macOS
  notification. Unsaved image drafts are labeled Draft, and the organization save button reflects
  whether the current value is already saved.

### Fixed

- Secondary windows are hidden and retained when closed, including the preview Escape shortcut,
  so closing a preview or Application logs window cannot tear down the main bundled app.
- Quick Copy discards its staged images and results after upload, so later clipboard previews do
  not prompt to replace an image from a completed Quick Copy.

## [0.5.0] - 2026-10-09

### Added

- Bulk upload mode for staging and uploading multiple images at once, including a prompt to switch
  modes when adding several images to an existing draft.
- Reset app option that clears local Gatto data and returns the app to first-run setup.

### Changed

- The previously selected repository is restored immediately at launch; GitHub authentication and
  the full repository list now load only when needed for an upload or repository selection.
- Staged image descriptions now open directly in an editor, and the replacement dialog supports
  keyboard shortcuts.

## [0.4.1] - 2026-10-09

### Fixed

- Restored GitHub attachment upload query parameters after the reqwest 0.13 dependency update.

## [0.4.0] - 2026-10-08

### Added

- Full-size image preview window for staged images, with previous and next navigation, an image
  position indicator, and left/right arrow key shortcuts.
- Multiple staged images, each tracked with its own name, description, and uploaded URL.
- Paste & Preview, Paste to Markdown, and Paste to URL actions in the menu bar, shown when a
  repository is pinned.
- Copy button in the Application logs window.
- The last used repository is remembered in settings.

### Changed

- Status messages are now shown as notifications instead of inline text.
- Main and Application logs windows can be hidden independently on macOS.

## [0.3.1] - 2026-10-08

### Fixed

- Image uploads failing with "error sending request" on networks that re-sign HTTPS traffic with
  a corporate certificate; the app now trusts the macOS system keychain.

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

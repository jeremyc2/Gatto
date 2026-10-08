# Architecture

GitHub Image Upload is a single-process macOS menu bar application. GPUI owns the
application and window lifecycle, while small modules isolate GitHub access,
platform integration, persistent settings, and shared state.

## Components

- `src/main.rs` starts GPUI and creates the application window.
- `src/app.rs` renders the interface and coordinates user actions, repository
  loading, image staging, uploads, explicit clipboard actions, settings, and
  status messages.
- `src/github.rs` calls GitHub CLI for authentication and repository lookup,
  then sends image bytes to GitHub's user-attachment service.
- `src/menu_bar.rs` owns the macOS status item and open/settings/quit actions.
- `src/model.rs` contains shared repository, image, and upload-state models.
- `src/settings.rs` persists the configured repository and manages the per-user macOS
  LaunchAgent used by Start at Login.
- `build.rs` records the current Git commit hash in the compiled application.
- `packaging/Info.plist` and `packaging/AppIcon.icns` provide the packaged application's metadata and icon.

## Data flow

```text
Paste, drop, or file selection
             |
             v
      Validate image data
             |
             v
      Stage image in memory
             |
             +------> Show preview and metadata
             |
             v
Request upload to the configured repository
             |
             v
Read active token from GitHub CLI
             |
             v
Send bytes to GitHub attachment service
             |
             v
Show returned URL and Markdown image snippet
             |
             v
User explicitly copies the preferred value
```

The staged image remains available after an upload error so the user can retry
without selecting it again.

## App Preferences and local state

The configured repository (`owner/name`) is saved as JSON under the current user's
`Library/Application Support/GitHub Image Upload` directory. The GitHub token and
image bytes are never written there. Until a repository is set, the main screen
only points the user to App Preferences.

Start at Login is represented by the exact file
`~/Library/LaunchAgents/com.jeremy-chandler.github-image-upload.plist`. Enabling the
setting writes a LaunchAgent containing the current executable's canonical path;
disabling it removes that one file. The agent runs in the user's Aqua session at
the next login. No privileged helper or system-wide service is installed.

The build script accepts an explicit `GIT_COMMIT_HASH` environment value or asks
the local Git executable for the current short commit. It falls back to `unknown`
when neither source is available.

## Dependency compatibility

GPUI Kit's published facade pins and re-exports the GPUI version it supports. The
application depends on GPUI Kit directly and imports GPUI through that facade to
avoid selecting an incompatible GPUI release independently.

Dependency upgrades should preserve this relationship and be reviewed alongside
the minimum supported Rust version in `Cargo.toml`.

## Security boundaries

- Authentication remains delegated to GitHub CLI.
- Tokens are requested only when needed and are not written to application config.
- Image content is sent to GitHub and otherwise remains local to the process.
- Upload results are copied only after the user presses a copy button.
- Persistent settings contain the repository name only; Start at Login stores the
  executable path in a per-user LaunchAgent.
- Logs and errors should never include tokens or raw image bytes.
- GitHub's attachment endpoint is undocumented, so failures must be handled as an
  expected condition and surfaced without losing user state.

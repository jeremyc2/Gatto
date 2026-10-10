# Architecture

Gatto is a single-process macOS menu bar application. GPUI owns the
application and window lifecycle, while small modules isolate GitHub access,
platform integration, persistent settings, and shared state.

## Components

- `src/main.rs` starts GPUI and creates the application window.
- `src/app.rs` renders the interface and coordinates user actions, repository
  loading, image staging, uploads, explicit clipboard actions, settings, and
  status messages.
- `src/github.rs` calls GitHub CLI for authentication and repository lookup,
  then sends image bytes to GitHub's user-attachment service.
- `src/diagnostics.rs` owns the bounded, in-memory troubleshooting event log.
- `src/log_viewer.rs` renders that event log in its own application window.
- `src/menu_bar.rs` owns the macOS status item and open/settings/quit actions.
- `src/global_shortcut.rs` validates and registers the optional system-wide Quick Copy shortcut.
- `src/custom_url.rs` validates `gatto://` action URLs before they are dispatched through the same
  action path as the menu bar.
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
      Stage attachment in memory
             |
             +------> Show preview state and metadata
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
Mark the attachment uploaded or failed
             |
             v
Add Markdown copy control to uploaded attachment
```

Attachments remain available after success or failure. Failed attachments can be retried without
selecting them again, and uploaded attachments retain their preview, description editor, and
Markdown copy control.

## App Preferences and local state

The organization, repository selections, shortcut configuration, and general preferences are
saved as JSON under the current user's `Library/Application Support/Gatto` directory. The GitHub
token and image bytes are never written there. Until an organization is set, the main screen only
points the user to App Preferences.

Start at Login is represented by the exact file
`~/Library/LaunchAgents/com.jeremy-chandler.gatto.plist`. Enabling the
setting writes a LaunchAgent containing the current executable's canonical path;
disabling it removes that one file. The agent runs in the user's Aqua session at
the next login. No privileged helper or system-wide service is installed.

The **Reset app** preference removes both of these per-user files and returns
the in-memory UI to first-run setup. It does not modify GitHub CLI
authentication, which is managed outside of Gatto.

The global Quick Copy shortcut uses macOS hotkey registration rather than global keyboard event
monitoring, so it does not require Accessibility permission. The shortcut is opt-in, must contain
at least two modifiers, and is unregistered when disabled or when the app resets.

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
- Custom action URLs do not accept parameters; repository selection remains controlled by saved
  app preferences.
- Upload results are copied after an explicit copy action or a user-configured Quick Copy action.
- Persistent settings contain repository names and app preferences, never tokens or image bytes;
  Start at Login stores the executable path in a per-user LaunchAgent.
- Logs and errors should never include tokens or raw image bytes.
- Diagnostic events are retained only in memory (up to 500 entries) and are lost on quit.
- GitHub's attachment endpoint is undocumented, so failures must be handled as an
  expected condition and surfaced without losing user state.

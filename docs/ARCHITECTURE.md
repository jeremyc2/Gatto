# Architecture

Gatto is a single-process macOS menu bar application. GPUI owns the
application and window lifecycle, while small modules isolate GitHub access,
platform integration, persistent settings, and shared state.

## Responsibilities

The application separates these responsibilities within a single local process:

- **Interface and upload state:** show repositories and image previews, coordinate
  uploads and retries, and make uploaded links available to copy.
- **GitHub access:** use GitHub CLI for authentication and repository lookup, then
  send image bytes directly to GitHub's attachment service. Gatto has no backend
  or server of its own.
- **macOS integration:** provide menu bar actions, the optional global shortcut,
  custom URL actions, and Start at Login. These entry points feed into the app's
  shared action handling.
- **Preferences and diagnostics:** persist local preferences and keep a bounded,
  in-memory event log available in the Application logs window.

Build and packaging tools supply version information, app metadata, and icons,
and produce the signed release. See [Releasing](../README.md#releasing) for the
release workflow.

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

## Loading, batches, and window lifecycle

Launch restores the remembered repository name and ID from settings without a
GitHub request. Opening the repository picker loads the organization's paginated
repository list and token through `gh`; uploading can instead resolve just the
remembered repository ID and token when needed. The token is cached in process
memory. Restart Gatto after changing the active GitHub CLI account to avoid using
a previously loaded session.

Image validation, CLI calls, and blocking HTTP uploads run on background tasks.
A batch uploads sequentially; each attachment transitions from preview or failed
to uploading, then uploaded or failed. Retrying skips uploaded attachments. The
copy result is rebuilt from successful attachments in their current order.
Generation counters prevent stale task completions from restoring state after
reset or draft replacement. Reset clears local state; it does not cancel an HTTP
request already sent to GitHub or remove a remote attachment.

The upload client uses a 15-second connection timeout and a 90-second request
timeout. It sends original image bytes to
`https://uploads.github.com/user-attachments/assets` with filename, content type,
and numeric repository ID query parameters. Returned URLs must use HTTPS,
`github.com`, and the `/user-attachments/assets/` path prefix. The endpoint is
hard-coded for GitHub.com; GitHub Enterprise Server hosts are not configurable.

Closing windows hides and retains them to avoid bundled AppKit/GPUI teardown
crashes. The main window also clears staged attachments and copy results and
hides the Dock icon. Dock reopening or a tray action activates the existing main
window. Closing Rooftops resets the run; preview and log windows can be hidden
independently. Quit ends the process.

Quick Copy shares the uploader pipeline, replaces existing drafts, writes Markdown
to the clipboard after successful uploads, sends a system notification, and clears
its attachments after completion or failure. Unlike normal uploads, its failed
images are not retained for retry.

## App Preferences and local state

The organization, repository selections, shortcut configuration, and general preferences are
saved as JSON in `~/Library/Application Support/Gatto/settings.json`. The GitHub
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

# Architecture

Gatto is a single-process macOS menu bar application. GPUI owns the
application and window lifecycle, while small modules isolate GitHub access,
platform integration, persistent settings, and shared state.

The public website is a separate static Solid project in `web/`. Vite runs under
Bun for development and builds; Bun prerenders its static HTML. It does not add
a backend to the app. See
[website development](../web/README.md) for its source and tooling boundaries.

## Responsibilities

The application separates these responsibilities within a single local process:

- **Interface and upload state:** show repositories and image previews, coordinate
  uploads and retries, and make uploaded links available to copy.
- **GitHub access:** use GitHub CLI for authentication and repository lookup, then
  send image bytes directly to GitHub's attachment service. Gatto has no backend
  or server of its own.
- **macOS integration:** provide menu bar actions, the optional global shortcut,
  custom URL actions, Start at Login, and silent installer ejection. Interactive
  entry points feed into the app's shared action handling.
- **Preferences and diagnostics:** persist local preferences and keep a bounded,
  in-memory event log available in the Application logs window.

Build and packaging tools supply version information, app metadata, and icons,
and produce the signed release. See [the release guide](RELEASING.md) for the
release workflow.

`installer_cleanup.rs` starts a background worker after the main window and menu
bar are ready. It acts only on a bundled executable in `/Applications/Gatto.app`
or the current user's `Applications/Gatto.app`. Release preparation creates the
installer volume and records its UUID in the app's Info.plist before signing.
The final DMG preserves that UUID through compression. Cleanup reads system disk
metadata through bounded `hdiutil info -plist` and `diskutil info -plist` calls;
it never reads mounted-volume files or opens the downloaded image. It requires
a single current-user, read-only disk image with the recorded UUID, rechecks its
identity, then requests ordinary AppKit ejection. Missing metadata, duplicate
mounts, development/image launches, and ejection failures are harmless. There
are no cleanup dialogs, download deletions, or additional persistent settings.
Each installed launch can retry cleanup; once ejected, later launches do nothing.

Rooftops uses native GPUI elements and the embedded gold-kitten coin sprite,
with the app’s existing dependencies.

## Release updates

`updater.rs` owns a process-lifetime GPUI entity observed by the uploader view.
It loads `Contents/Frameworks/libGattoUpdater.dylib` only in bundled builds and
retains the library for the process lifetime. The native updater reports status
changes back to Rust through a callback and channel; all native updater operations
run on the main thread. No network work depends on a window being
visible. No updater is loaded in an ordinary source build.

`native/Updater.swift` uses Sparkle 2.10.0 with a custom user driver. Sparkle owns
its four-hour check interval, persisted check time (`SULastCheckTime`), scheduling,
and retries against the public signed GitHub release feed. It checks when overdue
at launch and continues while Gatto runs, even with no windows. No extra Gatto
timer, updater cron job, or updater LaunchAgent is installed. Automatic downloads
and installation are disabled. Scheduled update notifications update the cached
badge and are dismissed in Sparkle without downloading the archive. The cache
keeps the badge visible across launches; an explicit click performs a fresh check
and authorizes installation. A click during a background check is carried into
the next update session before Sparkle schedules another check.

The header and menu bar menu show a compact Update control. Clicking initiates a
fresh Sparkle check, then authorizes download, verified installation, and relaunch
without another app confirmation. macOS can still request authorization. Uploads
already in progress prevent starting an update; new uploads and Quick Copy are
blocked during installation. The user driver never consents to install without
the click. Information-only releases and installation failures leave a retry
control and actionable tooltip; background-check errors only enter diagnostics.
The macOS **Gatto → Check for Updates…** action requests a fresh check without
granting installation consent. It queues behind an active background check and
deduplicates repeated requests. Only this manual check presents native alerts:
the available version with a release link and Update/Cancel choices, no update,
or a check error. Alerts run outside GPUI's current update to avoid reentrant
window borrows. Confirmation dismisses the checking session and sends an install
action back through Rust's existing upload guards and fresh installation check.
Cancellation dismisses without skipping future update reminders. Source builds
and updater startup failures report that checking is unavailable.
Sparkle verifies the signed feed and archive before extraction, handles app
replacement/recovery, and relaunches the app. Release packaging signs nested code
and embeds the release Mac's persistent Ed25519 public key.

The available version and Sparkle check time/state use the app's macOS
preferences domain (`~/Library/Preferences/com.jeremy-chandler.gatto.plist`);
Sparkle caches use `~/Library/Caches/com.jeremy-chandler.gatto`. Reset app clears
repository preferences and the login item while retaining updater state.
The Homebrew cask lives in `Casks/gatto.rb` in the same repository and installs
the published signed DMG, with GitHub CLI as a dependency.

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
GitHub request. Opening the repository picker loads the account or organization's paginated
repository list via GraphQL `repositoryOwner` (owned repositories only) and token through `gh`; uploading can instead resolve just the
remembered repository ID and token when needed. The token is cached in process
memory for the uploader window's lifetime. Reopen that window after changing the
active GitHub CLI account to avoid using a previously loaded session. Quick Copy
reads the current CLI token for each background job.

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

On macOS, closing a window orders it out and vetoes native destruction. GPUI's
window teardown can trigger an uncaught AppKit Touch Bar observer exception
([upstream report](https://github.com/longbridge/gpui-kit/issues/3192)). Each window
type retains and reuses one handle, so repeated closes do not accumulate windows.
Explicit quit mode keeps the menu bar process alive after all windows are hidden.
The main-window controller retains shared diagnostics and menu/shortcut services
and creates the uploader view on first use, loading saved preferences. Main-window
close cancels shortcut recording and discards staged attachments and copy results.
Utility windows close independently: preview close releases its image references,
logs refresh on reopening, and Rooftops resets its run on close. Other platforms
use native destruction.

The rationale, trade-offs, and conditions for removing this workaround are recorded
in [ADR 0001](adr/0001-retain-macos-windows-to-avoid-touch-bar-crashes.md).

Dock reopening activates existing presented windows without changing their pages.
If none are presented, it opens the main page. AppKit visibility includes occluded
and minimized windows; GPUI's drawing visibility is not used for this decision.
Native programmatic close requests run on the foreground executor outside a GPUI update,
because AppKit's synchronous close callback otherwise re-borrows the application.
The Dock icon hides when no windows are presented. Quit ends the process.

`application_menu` installs native App Preferences, Check for Updates, Close
Window and Quit Gatto menu items, with unscoped Command+, / Command+W /
Command+Q bindings for every app window. The cat icon's menu also shows the
preferences, quit and clipboard preview shortcuts. Successful global Quick Copy
registrations notify the menu controller, which refreshes its shortcut hint;
disabling or temporarily unregistering during recording clears that hint.
Close captures the
focused window, defers access until keyboard dispatch releases its borrow, then
uses the existing native close request and per-window cleanup callbacks.

Quick Copy runs through a process-lifetime background service with no window or
view dependency. Menu, shortcut, and URL actions all invoke that service. It
snapshots the clipboard, loads saved preferences, requires an account or organization and a
pinned repository, validates the clipboard image (or first copied image file),
and uses the same GitHub upload client and Markdown formatter as normal uploads.
It prefers the last selected repository when pinned, otherwise the first pinned
repository alphabetically. Success writes Markdown to the clipboard and sends a
macOS notification. Failure sends a notification and preserves the clipboard.
Concurrent Quick Copy requests are rejected while a job is running. Open windows
and staged attachments are untouched, and closing a window cannot stop its job.

## App Preferences and local state

The repository owner (personal account or organization), repository selections, shortcut configuration, and general preferences are
saved as JSON in `~/Library/Application Support/Gatto/settings.json`. The GitHub
token and image bytes are never written there. Until an account or organization is set, the main screen only
points the user to App Preferences. The `owner` setting accepts the former
`organization` key when reading existing preferences. Changing owners clears
repository selections, cached IDs, and pins after successfully saving settings.

Start at Login is represented by the exact file
`~/Library/LaunchAgents/com.jeremy-chandler.gatto.plist`. Enabling the
setting writes a LaunchAgent containing the current executable's canonical path
and a `--login` argument; disabling it removes that one file. The agent runs in
the user's Aqua session at the next login. A login launch hides the Dock icon and
creates no windows. Startup loads preferences and registers the global shortcut
without an uploader view; menu actions, custom URLs, and background update checks
remain available. Explicitly opening Gatto or choosing Open App shows the window.
On startup, an existing login item is atomically refreshed with the new argument;
this never enables Start at Login for users who have not selected it. No privileged
helper or system-wide login service is installed.

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

See [Contributing](../CONTRIBUTING.md#dependencies) for the pinned GPUI/GPUI Kit
pair and upgrade rules.

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

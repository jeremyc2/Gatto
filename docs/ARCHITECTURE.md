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

- **Interface and upload state:** show repositories and attachment previews, coordinate
  uploads and retries, and make uploaded links available to copy.
- **GitHub access:** use GitHub CLI for authentication and repository lookup, then
  stream image and video bytes directly to GitHub's attachment service. Gatto has no backend
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
      Validate media and selected upload limit
             |
             v
      Stage image in memory / video in a private temporary file
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
Report progress, then uploaded or failed
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

Media validation and CLI calls run on background tasks. Each HTTP upload uses
an async reqwest client in a background worker's current-thread Tokio runtime;
canceling drops the pending request. A batch uploads sequentially; each
attachment transitions from preview or failed to queued, then uploading with
byte progress, then uploaded or failed. Retrying skips uploaded attachments. The
copy result is rebuilt from successful attachments in their current order.
Image and video variants share staging, retry, upload, and copy interfaces.
The shared image formatter defaults to an HTML `<img>` element accepted in GitHub
Markdown, with `width` and `height` from the validated pixel dimensions and
escaped `alt` and `src` attributes. The saved `copy_images_as_markdown` preference
defaults to false, including for older settings. Enabling it selects
`ImageCopyFormat::Markdown`, producing escaped `![description](URL)` snippets
without dimensions. Individual copy, batch copy, and Quick Copy use the same
format selection; preference changes rebuild completed copy results without
uploading again. Video snippets remain bare URLs in both modes.
GPUI's clipboard image exposes encoded bytes without an
original display size or scale factor; copied dimensions therefore describe
pixels rather than the source app's logical size.
Video snippets are bare URLs, and bulk copy uses blank paragraphs between all
snippets so GitHub can promote each video URL to a player.
Generation counters prevent stale task completions from restoring state after
reset, close, or draft replacement. Reset and main-window close cancel active
uploads and clear local state; they cannot remove a remote attachment that
GitHub has already accepted.

Images are limited to 10,000,000 bytes. `allow_large_videos` defaults to false,
including for older saved settings; `video_upload_limit()` maps it to a typed
10,000,000- or 100,000,000-byte video policy. Selection, drop, Finder paste,
Quick Copy, bounded snapshots, and compression share that policy. Gatto does
not query billing eligibility. The checkbox is disabled while uploading or
importing; changes invalidate pending input jobs. Uploads recheck current limits
so disabling the option cannot send an already staged larger video.
Images retain shared immutable
encoded bytes; videos use a private `tempfile` snapshot, copied with a bounded
reader before container validation. Snapshotting preserves a stable retry source
if the original moves or changes. MP4/MOV validation checks box bounds, a video
handler, and media payload; WebM checks EBML/DocType, a video track, and a cluster.
These are container checks, not codec decoding or transcoding.

The upload client streams 64 KiB chunks without copying the full body. Progress
counts bytes read into the HTTP body, throttled to about ten updates per second
plus start/end. A separate waiting state lasts until the server returns a URL.
The client uses a 15-second connection timeout, a 90-second stream/response
inactivity limit, and a five-minute total timeout. Redirects and automatic POST
retries are disabled. Responses are capped at 64 KiB; errors distinguish auth,
permissions/SSO, size/format rejection, rate limits, and server/network failures.
It sends original image/video bytes to
`https://uploads.github.com/user-attachments/assets` with filename, content type,
and numeric repository ID query parameters. Returned URLs must use HTTPS,
`github.com`, the standard HTTPS port, no embedded credentials, and a nonempty
`/user-attachments/assets/` path. The endpoint is
hard-coded for GitHub.com; GitHub Enterprise Server hosts are not configurable.

Oversized video imports carry a typed `OversizedVideo` result into an ordered
import batch. The main window offers Cancel / Compress per file; Quick Copy
instead returns a main-window recovery hint. The batch waits for each choice
and compression result before staging the accepted items in their input order.
A separate cancellation flag and generation guard prevent stale native jobs
from restoring discarded imports. Uploads and updates are disabled during import.

`compression.rs` owns the offline AVURLAsset → AVAssetReader → AVAssetWriter
pipeline through narrowly enabled `objc2` framework bindings. It uses native
hardware-required H.264, original decoded pixel dimensions, the source transform,
a maximum input rate of about 30 fps, a 300-frame / ten-second GOP, and mono
48 kHz AAC at 64 kbps when audio exists. Audio and video retain their shared
source timestamps. Zero-sample edit markers are skipped, and video dimensions
are checked throughout decoding and again on a decoded finalized output frame.
HDR, multiple video/audio tracks, invalid durations and unsupported codecs or
sizes produce actionable errors rather than silent media loss or CPU fallback.

The ABR budget reserves audio first from 92% of the selected byte limit and scales
with source duration (capped at 12 Mbps for short clips). At 60 seconds with
audio under the default 10 MB policy it is about 1.16 Mbps for video. A second pass may retry from the original
at 70% of the first video bitrate. Only a finalized, nonempty MP4 strictly below
the selected 10 MB or 100 MB limit is copied into a staged video snapshot; byte estimates alone
never authorize upload. The original is read without modification and its size
and modification time are rechecked before accepting the result.

One background worker owns every native object, respects writer backpressure,
and uses per-iteration autorelease pools. UI cancellation only sets an atomic
flag; the owner cancels reader/writer operations serially. A ten-minute deadline
bounds each attempt. `gatto-compress-*` directories are private and removed on
success, failure, or cancellation; only the accepted upload snapshot survives.
Compression does not capture screens or microphones or invoke external encoders.

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
The custom image preview uses decoded GIF frame delays and requests redraws
while the active window animates; it pauses when inactive and respects Reduce
Motion. Image navigation excludes videos. Video previews run `/usr/bin/open -W`
on a background task to launch the default macOS player. A playback task retains
the private video snapshot until that application exits, independently of main
window/draft lifetime. Launch errors reach diagnostics and a notification;
codec/playback errors are handled by the external player.

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
pinned repository, validates the clipboard image (or first copied image/video file),
and uses the same streamed GitHub upload client and media snippet formatter as normal uploads.
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
- Image content remains in memory; videos use private per-user temporary files.
  Both are sent directly to GitHub, and video preview shares the local snapshot
  with the configured macOS player.
- Custom action URLs do not accept parameters; repository selection remains controlled by saved
  app preferences.
- Upload results are copied after an explicit copy action or a user-configured Quick Copy action.
- Persistent settings contain repository names and app preferences, never tokens or image bytes;
  Start at Login stores the executable path in a per-user LaunchAgent.
- Logs and errors should never include tokens or raw image bytes.
- Diagnostic events are retained only in memory (up to 500 entries) and are lost on quit.

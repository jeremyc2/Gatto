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

Rooftops uses native GPUI elements and the embedded gold-kitten coin sprite.
`rooftops/arrival.rs` draws its opening
flight, sputtering jetpack, and abandoned empty pack with the same native shapes.
Neighborhood facade geometry and the scorecard's static artwork are cached per
window/size; camera movement changes placement rather than rebuilding materials.
The player and ghost share a union silhouette path, preventing layered opacity
from exposing body overlaps during invulnerable respawn flashes.
The arrival has its own active-window clock, outside gameplay physics, scoring,
and replay recording. Movement skips it; Reduce Motion shows the landed state,
and replaying a completed run starts directly on the roof.
New runs hold gameplay physics and the shared score/replay clock until the first
movement or jump input; opening, landing, focusing, and restarting alone do not
start the clock. The first gameplay input establishes its time origin.

`rooftops/options.rs` holds play while a focused options card is open. Its music,
effects volume sliders, ghost and animation preferences share the existing
CBOR file with best times and replays. Debounced background preference writes
use atomic revisions and the same atomic replacement lock as completion writes;
ticket reservation never waits on disk I/O. `rooftops/audio/` synthesizes a
long-form stereo score and gameplay cues, feeding macOS AVAudioEngine through a
source-node render block. A dedicated owner thread handles native graph
startup, suspension and recovery. The UI communicates through atomic controls; the
audio callback uses fixed voice storage and preallocated delay/waveform storage.
Options can audition the music bus from its handle/track without advancing
gameplay. Effects audition once after a changed adjustment is released.
Closing clears transient option/audio state and restores game focus before the
retained window is hidden; reopening cannot retain focus on a hidden control. The cat's selected Visual Effect death patch is implemented in
native Rust; coin and shutter effects use independently authored transients. No music
recordings are shipped. See the [audio and options record](design/rooftops-audio.md).

## App buttons

The main-page header embeds `packaging/app-cat-pixel.png` at 72 points. Its
separate [Aseprite source and artwork record](design/explorations/art/app-cat/artwork.md)
retain the website Pocket Kitten's compact flying pose, dark purple coat,
continuous lavender body and tail outline and mostly obscured twin thrusters for the dark app
background. The Dock/menu-bar assets remain separate from this header image.

`theme.rs` owns `AppButtonStyle` and the shared confirmation footer. Ordinary
text actions use 32-point buttons with the same type size, corner radius, and
horizontal padding. Primary actions use the app's purple fill. Secondary actions
use the landing page's neutral background and subtle outline. Compact utility
buttons use the same outline at 20 points, including attachment description,
copy, and remove actions and log-window controls. Accessible names, keyboard
focus treatment, loading indicators, and disabled states stay on the underlying
GPUI Button.

Upload uses the standard text-button size and icon proportions, with enough width
for its progress label. Preview Save and Discard use the same styles and height
as their exit-dialog equivalents. The preview's Save button keeps a 120-point width
in both idle and loading states to fit its spinner and full “Saving…” label.
Confirmation footers constrain GPUI's full-size wrappers so their buttons stay
compact; the three-action exit dialog separates Discard from the Cancel/Save
group. The annotation tool palettes and illustrated Rooftops score card retain
their distinct controls.

This applies Refactoring UI's [hierarchy-first button guidance](https://medium.com/refactoring-ui/7-practical-tips-for-cheating-at-design-40c736799886):
make the primary action obvious, with quieter treatments for supporting actions.

## Release updates

`updater.rs` owns a process-lifetime GPUI entity observed by the uploader view.
It loads `Contents/Frameworks/libGattoUpdater.dylib` only in bundled builds and
retains the library for the process lifetime. The native updater reports status
changes back to Rust through a typed block and channel. A concrete
`MainThreadOnly` Rust bridge declaration matches the Swift selectors; all native updater operations
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
already in progress prevent starting an update; new uploads and Add from Clipboard are
blocked during installation. The user driver never consents to install without
the click. Information-only releases and installation failures leave a retry
control and actionable tooltip; background-check errors enter diagnostics and,
when no offer exists, the status in Preferences.
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

`AppSettings.release_channel` persists Stable/Beta in the existing JSON settings;
missing or unknown values default to Stable. Rust configures the bridge before
starting it and owns a channel generation. Both the bridge's active check and
queued Rust callbacks carry that generation. Switching clears the badge/menu
offer and consent, rejects results and prompt replies from the old generation,
and queues an information-only check after any active Sparkle session ends.
It does not cancel an authorized installation; channel changes are disabled then.
Preferences and the application menu use the same manual-check action.

Stable uses the latest stable release's signed appcast and permits only default
channel stable items. Beta uses the separately signed `beta` discovery feed and
allows tagged beta items as well as stable items. Both feeds point at immutable,
versioned DMGs. Publishing either channel refreshes the Beta feed with the newest
item from each channel; Stable publication alone refreshes stable download/cask
metadata. Beta versions use `x.y.zbN` internally and `x.y.z-beta.N` for display and
tags, preserving Sparkle's version ordering without a custom comparator.

Returning to Stable keeps a newer installed beta until Stable catches up. The
native no-update delegate reads Sparkle's latest signed appcast item and reports
both versions as a waiting status; it never offers a downgrade. Cached offers
are scoped to their channel and must still be newer than the installed build.
Release-channel selection does not migrate workspace data backward; manual
rollback limitations live in [Support](../SUPPORT.md#app-updates).

The available version and Sparkle check time/state use the app's macOS
preferences domain (`~/Library/Preferences/com.jeremy-chandler.gatto.plist`);
Sparkle caches use `~/Library/Caches/com.jeremy-chandler.gatto`. Reset app clears
repository preferences and the login item, resets the channel to Stable and
invalidates its offer while retaining Sparkle's check schedule.
The Homebrew cask lives in `Casks/gatto.rb` in the same repository and installs
the published signed DMG, with GitHub CLI as a dependency.

## Data flow

```text
Paste, drop, file selection, or Finder file opening
             |
             v
      Validate media and selected upload limit
             |
             v
      Stage attachments and save workspace copies
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

### Filesystem projects and attachment metadata

`projects.rs` owns portable `project.json` manifests and immutable copied
media in `media/`. Opening a project renames a legacy `gatto-media` directory
as a whole, including annotation sidecars. Conflicting old and new folders are
preserved and reported as an error. Legacy `gatto-project.json` files migrate on load after
version validation. Migration publishes the new name without replacing an existing
`project.json`; when both exist, the current name is authoritative and the old file
is preserved. Scratchpad uses the same storage and ordered writer as a
project, at `~/.gatto/projects/Scratchpad`; inactive workspaces are unloaded. Settings live
at `~/.gatto/settings.json`. New projects exclusively create named child folders
under `~/.gatto/projects`. Creation never reuses an existing folder. Initial-save
failures clean up only the new child folder.

Manifests retain UUIDs, attachment order, filenames, titles, descriptions, marker
notes, completed upload URLs, and workspace metadata: nullable description,
repository and owner, creation time, and update time. New projects with no
description store `null`; Scratchpad has a built-in description. Timestamps are
Unix milliseconds; updates advance monotonically. Project metadata is required
when reading the manifest. Foundation formats timestamps in the user's local
time zone.
The schema reference points to
`https://raw.githubusercontent.com/jeremyc2/Gatto/main/schemas/gatto-project.schema.json`.
Settings reference `schemas/gatto-settings.schema.json` at the same repository URL.
All three JSON formats (settings, projects, annotations) carry `version: 1`.
These are independent file-format versions, not application release numbers.
Readers reject unsupported explicit versions before changing files. Legacy settings
without a version use version 1 and are rewritten on load. Settings may omit
preferences and receive defaults; project and annotation schemas describe complete
current records. `$schema` identifies the editor schema and does not replace the
format version. The test-only `jsonschema` dependency validates actual saved
settings, manifests, and annotation sidecars using Draft 2020-12, with format
checks enabled and network reference resolution disabled.

Startup moves an existing `~/.gatto/scratchpad` into `projects/Scratchpad` as one
folder. If the destination exists, the legacy Scratchpad replaces it and its
previous contents are discarded; migration does not create recovery projects.
Legacy global repository fields only seed a missing Scratchpad selection; the
project's saved repository wins. The settings rewrite removes `last_repository`
and `last_repository_id`. New Scratchpads have no repository selected.

SHA-256 fingerprints bind URLs to saved bytes. Metadata edits and reordering
preserve URLs; pixel edits or external media changes invalidate only the affected
URL. Media filenames combine immutable UUIDs and hashes. Saving syncs media before
atomically replacing the manifest and removes retired files only after commit.
Images with annotations keep an immutable original PNG and current annotation
JSON beside the rendered media. `annotation/storage.rs` validates and restores
marks, style, scale, marker numbering, and comparison settings without serializing
undo/redo. Comparison validation also checks the derived caption footer rectangle;
invalid edited geometry returns a project-read error. The JSON references
`schemas/gatto-annotations.schema.json`. Sidecars use UUID/content-hash names,
are committed before the manifest, and are retired only after a successful
manifest replacement. External edits rerender the image and invalidate its URL
when the resulting bytes change.

`app/projects.rs` discovers projects only as direct folders in `~/.gatto/projects`;
its catalog is held in memory. It manages disk-backed workspace loading, ordered
background saves,
creation, transfer, metadata editing, removal, metadata dialogs, and sidebar search.
The active writer receives immutable snapshots of attachments and workspace
metadata. Revisions ignore stale completions; a flush supports shutdown and
switching after cancellation. Save failures retain UI state and the previous
manifest. Pending writes block closing and quitting; ordinary window closure
preserves work. The current workspace is never reloaded when selected again.
Switching or canceling invalidates pending token and repository-resolution results
before they can continue an upload in another workspace, and releases their busy
flags so the new workspace can start its own requests.

Creation offers a name, optional description, and an optional Scratchpad transfer.
The destination is committed before Scratchpad is cleared. Attachment moves use
the same destination-first transaction with rollback if the source save fails;
bulk moves commit the selected group together. UUIDs and cached URLs survive moves.
Scratchpad and each project restore their own repository selection only when its
saved owner matches the configured owner without regard to case, including after
repository loading. Repository name and GitHub database ID are one saved record.
Repository-list refresh matches a known ID before its name, updating renamed
repositories without following a reused name to a different ID. Repository pins
remain application preferences. Add from Clipboard never creates a window.
Clipboard drafting and comparison target the presented main workspace, or
Scratchpad when there is no presented main window; startup requests wait for the
initial Scratchpad load.

The slate GPUI Kit sidebar has a fixed Projects/search header, a scrolling project
list containing named projects, and fixed Scratchpad and Create Project controls
at the bottom. Scratchpad is excluded from the list and Primary section, including
when used for Add from Clipboard; its button keeps its notebook icon. Named projects
keep folder icons when pinned, with pin icons only in the Pin/Unpin menu actions.
The named project used for
Add from Clipboard shows a star and ranks first in the
Primary section, followed by explicitly pinned projects, then a two-point divider
using half-opacity muted foreground with extra vertical spacing, and the
remaining projects. Both latter groups sort by update time. Project pins and the
optional Add from Clipboard project folder name are application preferences. Null means
Scratchpad; unmarking a named project restores that default. Search has only a placeholder. Command+B hides the sidebar entirely;
the expand control appears beside Logs. The sidebar state persists in settings.
Project menus offer Use for "Add from Clipboard" / Stop using for "Add from Clipboard" and Pin project /
Unpin project. Named projects also offer Edit metadata and Delete; Scratchpad's
name and deletion remain reserved. The heading displays only the project name.
Editing changes the manifest title
and description without moving the folder. Refresh discovers project folders in
`~/.gatto/projects` and drops missing folders. Delete validates the manifest
and removes the folder. Recursive removal never follows child symlinks. Delete All
Projects removes project folders in `~/.gatto/projects` and preserves Scratchpad.
Reset additionally clears Scratchpad and preferences.

`app/attachment_actions.rs` supplies per-item menus and selection toolbar actions,
annotation selection, and a destination picker that excludes the current project.
The toolbar uses an unlabeled select-all checkbox and icon buttons with tooltips;
unavailable actions are omitted. Combining requires exactly two compatible images
that are not already comparisons; the composer also rejects comparison inputs,
and the selected/total attachment count appears at the right only when something
is selected, followed by “selected.”
The upload button counts all attachments needing upload, regardless of selection,
and omits the count when it is zero.
`app/upload_actions.rs` renders the joined upload control. Both its main area
and shaded chevron are separate buttons with independent hover, focus, and
pressed feedback. The main button runs the remembered action directly; the
chevron opens the choices above the control. Selecting an action persists
`upload_action` in global settings before starting the batch. The batch captures
its copy mode and attachment IDs; automatic copying includes only successful
uploads from that batch, skips cancellation and total failure, and honors
close-after-copy only when the whole batch succeeds. The selection toolbar shows
copy only for entirely uploaded selections. Copy all uploaded appears only when
more than one attachment is uploaded and retains Command+C in a tooltip
rather than a separate keyboard hint, and matches the icon buttons' height.
`app/attachment_drag.rs` keeps full-card drag previews,
live sibling shifts, edge scrolling, transactional sidebar drops, and Escape
rollback; preview order is not saved before release. Interactive card controls
consume pointer events so they cannot start a reorder.

`theme/button.rs` and `theme/checkbox.rs` add shared lighter hover fills and brighter
borders while retaining the component library's input, focus, and accessibility
behavior. Button borders are painted without adding a layout node; disabled
controls keep their ordinary appearance. Attachment copy actions say “Copy” in
either output format.

The main question mark opens workspace information: name, description, total
storage, repository, and dates. The native Help menu opens documentation links.
Modal Escape dispatches the dialog Cancel action before window-level dismissal,
retaining cancellation and close hooks. Shared DialogFeedback renders validation
within the owning modal.
GPUI's Dock menu API exposes five projects in last-updated order; global typed
actions open the main window and select the corresponding project.

## Loading, batches, and window lifecycle

All app windows open through `theme::open_window`, which keeps GPUI's Root and
view identity while installing shared native macOS styling in `window_limits.rs`.
Title bars use the theme's sidebar background, with a one-point bottom divider
using `sidebar_border`, matching the sidebar's right border. The native divider
sits at the top of the content view without changing content layout or the native
title and traffic-light controls. Bounds observers keep it full-width and restore
title-bar transparency after GPUI's fullscreen transitions; the divider is hidden
while fullscreen.

Opening the main window loads Scratchpad and restores its repository without a
GitHub request. A new Scratchpad starts without a repository selection.
Opening the repository picker loads the account or organization's paginated
repository list via GraphQL `repositoryOwner` (owned repositories only) and token
through `gh`; uploading can instead resolve just the remembered repository ID
and token when needed. The token is cached in process
memory for the uploader view's lifetime. Restart Gatto after changing the active
GitHub CLI account to avoid using a previously loaded session. Add from Clipboard
reads the current CLI token for each background job.

Media validation and CLI calls run on background tasks. Each HTTP upload uses
Foundation URLSession through the typed Rust implementation in `github/native.rs` on a
background worker. A stock 64 KiB bound stream connects the existing Rust
attachment reader to the OS transport without buffering a whole video. A serial
delegate queue reports actual bytes sent, rejects redirects and request-body
replay, and limits response accumulation to 64 KiB. Ephemeral sessions disable
cookie, credential and cache storage; only default server-trust handling is
permitted. Monotonic watchdogs enforce the 300-second total and 90-second
inactivity deadlines. These are transport deadlines rather than a separate
15-second TCP/TLS connect timer. Cancellation cancels the native task; the worker
waits for delegate invalidation before releasing callback state, including on Rust
unwinding. Delegates own an `Arc` of synchronized Rust state and a response `Vec`;
there are no borrowed worker-stack callback pointers or C response allocations.
The sole explicit `Send` exception transfers an unopened input stream exactly once
to Foundation; it does not make streams generally thread-safe. A completed
response wins a simultaneous cancellation. A batch uploads sequentially; each
attachment transitions from preview or failed to queued, then uploading with
byte progress, then uploaded or failed. Retrying skips uploaded attachments. The
copy result is rebuilt from successful attachments in their current order.

`desktop.rs` checks and retains `MainThreadMarker` for desktop owners.
`desktop/menu.rs` owns typed retained AppKit objects and a Rust target with an
owned event sender; teardown clears unretained targets before releasing it.
`native/Hotkey.m` contains only Carbon registrations and media-key event taps; callbacks
enqueue integer action IDs without reentering GPUI. Menu shortcuts are cleared
directly, and keypad equivalents retain their numeric-pad modifier. Shortcut
replacement registers the candidate before releasing the previous key; failed
replacement leaves the old registration and observer state intact. Generation
IDs reject queued events from earlier registrations. Ordinary keys use Carbon
without a new permission prompt. The optional media-key event tap is created
only for a selected media shortcut and preserves existing permission and
suppression behavior. Failed native cleanup detaches callbacks before Rust frees
their context. `build.rs` compiles only this small hotkey adapter with Apple's
clang/libtool into the app binary; the Sparkle Swift bridge remains separately
bundled and signed. The [Rust migration record](research/safe-rust-migration-2026-10-23.md)
explains the remaining platform exceptions and their verification.
Image and video variants share staging, retry, upload, and copy interfaces.
File drops and Finder pastes append every accepted file in order.

`NSServices` declares **Send to Gatto**, limited to Finder and accepting file
items broadly so mixed selections reach the existing validator intact. macOS
controls its visibility and context-menu placement. No Automator workflow or
separate installer is shipped.

`desktop.rs` installs the Rust-defined AppKit service provider at app startup.
`desktop/file_service.rs` reads file URLs (or legacy filename lists) from the service's supplied pasteboard,
without reading or changing the general clipboard. Foundation's
[`filePathURL`](https://developer.apple.com/documentation/foundation/nsurl/filepathurl)
resolves Finder file references before GPUI serializes them; an unresolvable
reference reports a service error instead of forwarding a synthetic `/.file/id=`
path. It forwards the whole resolved selection
to GPUI's `application:openURLs:` delegate, which only enqueues events. The file
picker, paste, drop, and Finder file opening share one background path loader and
import-busy guard, including cancellation and reset guards. Both the
service and Open With use the early `on_open_urls` channel until the controller is
ready, including at cold launch. `open_request.rs` decodes local file URLs without
changing existing `gatto://` actions. File events activate the main window and use
the existing validation and compression flow. They never authorize
upload. Busy operations and dialogs reject selections with retry feedback; an
in-flight validation blocks reentry. Cancellation and reset invalidate pending results.

Apple documents [document types](https://developer.apple.com/documentation/bundleresources/information-property-list/cfbundledocumenttypes)
and [Services properties](https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/SysServices/Articles/properties.html).
Bare source executables support drop/paste; Finder registration needs an app bundle.
The local build script packages that bundle separately from release signing,
Sparkle, and publication. See [Contributing](../CONTRIBUTING.md#local-app-bundle).

The shared image formatter defaults to an HTML `<img>` element accepted in GitHub
Markdown, with `width` and `height` from the validated pixel dimensions and
escaped `alt` and `src` attributes. The saved `copy_images_as_markdown` preference
defaults to false, including for older settings. Enabling it selects
`ImageCopyFormat::Markdown`, producing escaped `![title](URL)` snippets
without dimensions. Individual copy, batch copy, and Add from Clipboard use the same
format selection; preference changes rebuild completed copy results without
uploading again. Image titles supply alt text; optional descriptions appear only
in an escaped HTML `data-description` attribute. Video snippets remain bare URLs
in both modes.
GPUI's clipboard image exposes encoded bytes without an
original display size or scale factor; copied dimensions therefore describe
pixels rather than the source app's logical size.
Video snippets are bare URLs, and bulk copy uses blank paragraphs between all
snippets so GitHub can promote each video URL to a player.
Generation counters prevent stale task completions from restoring state after
reset or workspace cancellation. Closing preserves the workspace, and pending
work must finish or be canceled. Cancellation cannot remove remote attachments
that GitHub has already accepted.

Images are limited to 10,000,000 bytes. `allow_large_videos` defaults to false,
including for older saved settings; `video_upload_limit()` maps it to a typed
10,000,000- or 100,000,000-byte video policy. Selection, drop, Finder paste,
Add from Clipboard, bounded snapshots, and compression share that policy. Gatto does
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
and numeric repository ID query parameters. Attachment titles determine the
upload filename; when a title omits the extension, the existing file extension
is retained. Changing an uploaded video's filename clears its previous upload
so it can be re-uploaded; changing an image title reuses its uploaded asset and
updates the generated text. Returned URLs must use HTTPS,
`github.com`, the standard HTTPS port, no embedded credentials, and a nonempty
`/user-attachments/assets/` path. The endpoint is
hard-coded for GitHub.com; GitHub Enterprise Server hosts are not configurable.

Oversized video imports carry a typed `OversizedVideo` result into an ordered
import batch. The main window offers Cancel / Compress per file; Add from Clipboard
instead returns a main-window recovery hint. The batch waits for each choice
and compression result before staging the accepted items in their input order.
A separate cancellation flag and generation guard prevent stale native jobs
from restoring discarded imports. Uploads and updates are disabled during import.

`compression.rs` owns the offline AVURLAsset → AVAssetReader → AVAssetWriter
pipeline through narrowly enabled `objc2` framework bindings.
`compression/native.rs` denies unsafe code in orchestration, budgeting, progress,
deadlines and errors. Its private `bindings.rs` owns reader/writer/sample operations,
while `settings.rs` holds the documented native dictionary schemas. Session ownership
prevents thread transfer; reader/writer Drop cancels partially completed operations.
It uses native
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
close cancels shortcut recording and preserves saved workspace attachments and
copy results. Active work must finish or be canceled before closing.
Utility windows close independently: confirmed preview exit releases its image references,
logs refresh on reopening, and Rooftops resets its run on close. Other platforms
use native destruction.

The shared image/GIF preview starts with maximized window bounds and no content
size cap. Reusing its retained window preserves subsequent manual resizing.

`image_composer.rs` owns a separate, non-maximized **Side-by-Side Comparison**
attachment picker, opened from the native Advanced menu or status item. The main
controller retains one picker under the existing macOS window lifecycle. It
contains two labeled slots, using main-window-style image drop zones and the
shared attachment component. Populated cards show a thumbnail, filename, size,
and removal action, without a description editor. File/drop/paste imports accept
one PNG, JPEG, or WebP per side; content validation rejects disguised GIFs.

The picker stacks the left slot above the right slot. Clipboard paste targets
the first empty slot in left-to-right image order, independently of pointer
focus; only that slot advertises Command+V. Clicking or dropping can fill either
slot first. Loading state gates input and generation checks ignore stale results.
When both sources are loaded, `ComparisonRequested` transfers them to the main
controller. Once the main window accepts the request, the picker closes under
its normal retained-window lifecycle. Conflicting main-window imports/uploads
leave the picker available to retry by removing and reattaching a source.

`UploaderApp::begin_comparison` shows the normal main page and runs combination
on a background worker. An attachment card with an animated thumbnail spinner
reports resizing/combining while import/upload controls are gated. The completion
appends the result to the current workspace. Canceling combination invalidates
pending results through its upload generation. Combining does not
upload or modify either original file.

An individual PNG, JPEG, or WebP attachment can open the same composer with its
image fixed in one slot. A segmented control chooses the new image's side. The
populated slot wraps its 64-point attachment card; only an empty slot reserves
160 points for dropping an image. Source changes resize the window and update
its native minimum height after rendering, preserving its width. Switching sides
keeps the same compact window height. The result replaces the original attachment in its current position and keeps its
identity, title, and description. The new PNG clears the previous upload URL.
Project manifests record `is_comparison` for each attachment; reopening a
workspace uses that flag to determine whether conversion is available.

One common scale fits the complete result within 3840 × 2160, including the
reserved caption area, padding, and gap. Photos keep their proportions and relative
scale, align at the top, and pad below the shorter side. Small sources are never
upscaled. Each source is decoded and resized with Lanczos3 directly into one
bounded canvas. The renderer encodes a lossless PNG and inspects its actual size.
Outputs over 9,000,000 bytes are rerendered at a smaller scale, estimated from the
byte ratio with margin. Retried exports use the retained base rather than a
previous PNG; there is no intermediate PNG decode or second annotation editor.
The completed PNG is staged with known dimensions, avoiding a validation decode.
The [sizing research](research/comparison-image-sizing.md) records tldraw's distinct
export/display behavior and the GitHub limit behind this policy.

`annotation/comparison.rs` owns column geometry and two caption strings,
separate from normal drawing marks. Columns have a minimum width of 400 source
pixels so captions can be wider than small images. Each original is centered
horizontally in its column without upscaling. Both columns share the same top
and height, anchoring captions below the image row on a common baseline.
The attachment session retains an unlabeled base and comparison metadata; the
staged PNG already contains **Before**/**After**. Ordinary images have no
comparison metadata or caption controls.

Captions use 32–64-pixel text scaled with the comparison dimensions, with fixed
column widths, automatic wrapping, explicit line breaks, and a fixed left/top
anchor. They have no field background, border, or tab shape. Expanded caption
heights extend document export bounds downward without moving or covering the
source images. PNG export and native preview reserve the shared caption band
above all annotation paint; drawing cannot start in that band. The preview
covers saved caption pixels before painting current glyphs. The editor's fitted
comparison leaves 96 screen pixels below it for floating controls and room to
edit additional lines. The fitted extent stays frozen during caption typing;
finishing editing refits the complete footer without stretching the raster.
Floating drawing and zoom controls hide while a caption has keyboard focus.

Double-clicking a caption creates the same `TextInput` used for freeform
annotations. It supplies caret, selection, IME, clipboard, and text undo, while
the canonical annotation text renderer remains the only glyph painter. This
avoids the previous combination of a resizing toolkit input and independently
measured annotation glyphs. Shift+Return inserts a line break; Return, Escape,
or blur finishes editing. Text stays metadata rather than becoming a draggable
or resizable annotation. Caption pointer handlers allow Space, pan-tool, and
middle-button panning. During annotation drags they also remove their occluding
hit region and text cursor, so the canvas owns pointer tracking across the row.

For comparisons only, the shared image row's lower edge is a hard geometry
boundary. New gestures cannot start below it. Live single-object and group
transforms are projected before committing: moves clamp their vertical offset,
while drawing, resizing, bending, and rotation find the nearest valid gesture
position when their full visual bounds would cross the edge. The bounds include
stroke thickness, arrow caps, and labels. Most plain strokes stop directly at a
stroke-width inset; the boundary solver handles rotated or expanded geometry.
Publishing only one constrained result avoids repeated cache updates during the
search. Pen release, click-created defaults, text/style changes, and pasted
objects obey the same bottom boundary; pasted groups retain relative spacing.
Ordinary image gestures retain their existing unbounded behavior.

Document checkpoints include both marks and comparison metadata. Saved session
state also records both, so caption-only edits enable Save and survive save/reopen,
while Discard restores saved captions along with saved annotations. Comparison
PNG exports fit expanded annotation bounds within 3840 × 2160 and recheck a
9,500,000-byte budget, independently of editor pan, zoom, or window size. The
initial 9 MB target leaves headroom; the final check handles annotation growth.
Metadata is retained within the attachment session;
reimporting an exported PNG starts an ordinary raster document. Historical
measurements of the superseded split-editor design and the current reproduction
command live in [the composition profile](research/side-by-side-composition-profile.md).

The rationale, trade-offs, and conditions for removing this workaround are recorded
in [ADR 0001](adr/0001-retain-macos-windows-to-avoid-touch-bar-crashes.md).

Dock reopening activates existing presented windows without changing their pages.
If none are presented, it opens the main page. AppKit visibility includes occluded
and minimized windows; GPUI's drawing visibility is not used for this decision.

`annotation.rs` owns image-coordinate marks, full-session undo/redo, hit testing, and
text layout. `annotation/geometry.rs` supplies the shared rotated frame for
hit testing, selection, painting, and export. Marks retain local points and a
rotation about their bounds' center. `annotation/text.rs` defines fixed-size
labels, horizontal/vertical alignment, rectangular label padding, content minima,
and extra vertical growth above a shape's base height. Every closed geometry can
host a label. Closed-shape labels use white glyphs with black outlines in native
preview, live editing, and export, independently of the shape palette. Standalone
text and arrow/line labels keep their selectable palette colors. Normal resizing
clamps the shape to its text rather than shrinking
or hiding the letters; group scaling changes both geometry and text size.
Standalone text stores automatic width or a wrapping width, and preserves its
alignment anchor while editing. Arrow labels retain a fraction along the actual
path, clamp away from endpoints where possible, and project/snap during label
movement. Their shaft geometry excludes the label rectangle rather than painting
an opaque cover over the source image. Labels contribute to hit and export bounds.

`annotation/text/layout.rs` supplies shared line boxes, wrapping, caret positions,
and glyph paths with whole-pixel line heights and preserved blank lines.
CoreText handles contextual line breaking and shapes Arial with native kerning,
ligatures, and system glyph fallback. Color glyphs draw into a fallibly allocated,
Core Graphics-owned, explicitly cleared bitmap before a bounded copy into Rust
pixels. No native context retains a pointer into a Rust pixel allocation. Its outlines feed the same native geometry
and PNG export. Glyphs without outlines, including color emoji, retain CoreText's
fallback font and positions and are drawn into bounded RGBA bitmaps. The shared
layout carries those bitmaps through scaling, hit testing, preview, and export.
Annotation text uses macOS CoreText exclusively, without a direct font-parser
dependency or a separate layout path for unsupported platforms.
Layout caches are bounded by entry count and estimated memory, shared across
preview geometry and measurement. A separate bounded line cache reuses unchanged
shaping as paragraphs are edited. Shaped outlines retain positioned words,
splitting unbroken runs into short fragments while preserving CoreText glyph
placement. The native preview
shares bounded fragment meshes across edits, movement, and reflow, transforms only
visible fragments, and submits two combined text layers for outline and ink.
Editing does not tessellate a whole paragraph on every keystroke; PNG export still
uses the shared vector outlines and bitmap glyphs.
`image_preview/editor/text_input.rs` implements GPUI's native `EntityInputHandler`
over those same line boxes and rotated coordinates. It only paints the caret,
composition underline, and translucent text selection; the annotation renderer
continues to paint the glyphs during editing. There is no second text widget,
editing-only background, width clamp, or padding. UTF-16 platform ranges, native
composition, grapheme navigation/deletion, plain-text clipboard operations, and
local text undo/redo share this input. The first actual change checkpoints the
artwork; opening editing alone does not. Left/right and clipboard/history actions
have their own text key context so preview navigation cannot consume caret keys.
Opening existing text focuses it with its entire UTF-8 value selected; native
selection reporting converts that range to UTF-16. Double-clicks within an active
input also select its full value. Left/right collapse the selection to the start
or end through the existing caret navigation, and typing replaces it. Empty new
text starts with a collapsed selection as before; selecting alone adds no history.
Rich formatting and font-family choices are deliberately excluded by user request.
Arrow/line labels use natural-width layout until they exceed the sampled path
length minus cap geometry and a small gap. Wrapped labels retain at least the
measured width of their longest word, rounded up in annotation units; they never
feed a fractional natural width into the fixed-width floor. Cap dimensions share
constants with the rendering paths. Text changes extend an undersized host;
endpoint drags preserve the fixed tip and font while enforcing that content
minimum. Clearance includes the padded label rectangle, stroke radius, and 12
annotation units of visible shaft beyond each cap. Multiline labels use their
actual two-dimensional footprint at every angle. If no safe placement exists,
the host grows around the fixed endpoint or label anchor, remeasuring wrapping
while retaining the font and relative curve geometry. Bending and group
transforms enforce this constraint too; an expanded member invalidates the
cached group frame. Label placement tests a balanced position between asymmetric
caps before sampling and refining toward the requested position. Refinement
always retains a verified safe point, including curves with disconnected safe
intervals. Preview, editing, and export share this layout and clearance policy.
`annotation/shapes.rs` supplies sixteen closed shapes, a line option, and clipped
hatch fills. Directional polygon templates normalize winding before optional
geometry reflections; native fills remain separate from strokes. Shape icons
use the same 16-pixel square as other tool glyphs, with centered geometry and no
flex shrinking, in the toolbar, properties panel, and shared shape menu.
One palette choice supplies the brighter stroke tone and a matching fill at
80 percent of each RGB channel. Native preview paints fills separately from
strokes so the two tones remain distinct. X boxes use corner-to-corner diagonals;
checkboxes use a centered, rising tick. Both symbols use the border's color,
width, and dash pattern. Preview and export share the stroked symbol geometry;
hit testing and tool icons retain the complete shape, including its symbol.
`annotation/marker.rs` owns numbered walkthrough badges. Documents and undo
snapshots carry the next number, starting at 1 with a maximum of 99 markers.
Deletion renumbers surviving markers in sequence order rather than stacking
order; descriptions remain attached to the same marks. Clones and pastes allocate
fresh numbers atomically and use the destination image's existing marker size.
Corner resizing changes all markers' scale together, including their numerals,
while retaining a square frame. Group transforms keep numerals upright and update
the shared size; marker selections do not expose rotation or edge stretching.
Each marker's palette color stays independent. The badge painter returns shared
vector paths for the fill, two concentric border lines, and a fitted one- or
two-digit numeral; text and border ink contrast with the fill. Preview uses native
vector geometry at every zoom, and PNG export rasterizes the same paths.

Marker descriptions live separately from canvas text. Saving stores every
marker's number and description in numeric order in `ImageData` alongside the
exported PNG, retaining blank descriptions. Single and bulk copying append all
items to that image's snippet, with indented
continuation lines and no heading. The editing session preserves the source,
badges, descriptions, colors, and size across reopening; Discard restores the
saved state. `image_preview/editor/marker_notes.rs` uses a native multiline
textarea in a full-width bottom overlay for a single selected marker. Opening
or closing it does not participate in camera scale or position calculations.
Input focus keeps typing and clipboard shortcuts inside the textarea. The
attachment's ordered-list icon is present whenever markers exist, even if every
description is blank. Design sources are in [numbered marker research](design/numbered-markers.md).

`annotation/preview.rs` caches tessellated native GPUI geometry per
mark; pointer updates rebuild changed geometry only, without copying base pixels,
allocating full-image layers or encoding PNGs. The same
paths feed the `tiny-skia` export rasterizer. Opaque vector marks reuse their
local mesh during rotation; frame painting transforms its vertices and bounds.
`annotation/raster.rs` caches composed textures for translucent marks and marks
containing bitmap glyphs. It uses the same whole-mark painter as PNG export,
compositing opacity once after fill, stroke, glyph outlines, and color bitmaps.
Textures cover only the mark, use two pixels per annotation unit, and cap each
side below 2,048 pixels. Moving a mark reuses its texture; geometry, rotation,
text, or style changes replace it. Replaced, deleted, and closed textures are
explicitly retired from GPUI's atlas alongside pixelation textures.
CoreText supplies outlined glyph paths from the system Arial font, without
bundling another font. GPUI and its SVG dependencies still bring in `ttf-parser`
transitively; Gatto does not depend on it directly.

PNG export runs from a background snapshot, composites opacity once per whole
mark, expands to include off-image marks, and enforces dimension/allocation limits.
An export reuses one transparent annotation layer. It paints in the original
full-canvas coordinates, then composites and clears only rows containing pixels;
it does not estimate bounds or clip glyphs to a new rectangle. PNG encoding consumes
the completed raster for straight-alpha conversion, retaining the existing
compression settings and rounding. Editor decoding and comparison resizing also
consume their RGBA results rather than cloning them. Preview textures convert
their completed bitmap in place before handing its buffer to GPUI.
Pixelation replaces the covered area with opaque mosaic tiles sampled from the
source, using the same cached colors for 16-pixel blocks in preview and export.
`annotation/pixelation.rs` rasterizes round-ended brush segments at source pixel
centers. Preview caches sparse 256-pixel BGRA tiles and uploads only tiles with
newly covered pixels as a stroke grows. Transparent coverage retains mosaic RGB
so linear filtering cannot introduce dark fringes. Each texture has a one-pixel
gutter sampling adjacent coverage, cropped to the logical tile when painted;
boundary edits also refresh existing neighboring gutters to prevent tile seams.
Cached interiors are copied into padded textures without recalculating colors.
Undo/redo rebuilds the covered tiles from
the remaining strokes. Each frame paints cached tile textures, without per-block
primitives or full-image rasterization. Replaced tiles are explicitly evicted
from the GPU atlas; closing or replacing a preview also releases its textures.
Export uses the same brush coverage. `annotation/pixelation/hover.rs` paints a
transient circular mosaic preview between committed source effects and annotation
geometry. It shares source mosaic colors, caches smaller color grids for subpixel
cells, clips coverage to the circle/image/viewport, and batches the cells into one
GPUI paint layer. Pointer movement creates no document marks, history checkpoints,
encoded images, or texture uploads. Leaving the canvas or hovering over controls
removes the preview; the toolbar's circular photo icon sits after shape overflow.
Export uses only committed marks. Pixelation size and opacity share the same
slider renderer. Brush diameter is continuous from 30 to 138 logical screen points
(default 64); squared slider travel gives small brushes finer control. Size
changes update only the next stroke, without checkpoints or modifying existing
strokes. Hover and new strokes divide that screen diameter by the current camera
scale; committed coverage remains fixed in image space. The minimum remains a
paintable brush.
Both renderers paint pixelation only within the source image and below every
annotation; hit testing excludes pixelation entirely. It cannot be selected,
transformed, erased, or reordered, but each stroke remains undoable. Pixelation alone cannot enlarge
the output. Saved outputs
contain no recoverable source layer when uploaded. Local workspaces retain the
original source and annotation JSON. Each staged image owns an in-memory `Session`
that retains the original document and source image independently of the flattened
upload bytes. The open preview paints from the original coordinate system, rather
than annotating an already flattened replacement. Each preview image and retained
session stores a saved-mark baseline separately from undo/redo history. Save compares
the current marks to that baseline, exports all changed images in a background batch,
and shows a loading indicator while blocking additional edits. Each uploader event
is acknowledged after the staged replacement is accepted; errors leave the preview
open and retryable. Only after every acknowledgement does Save close the preview,
retaining the original document and saved baseline for reopening. Window-local
undo/redo is cleared by the retained snapshot when the editor closes.
Both the native close button and Command+W close clean previews immediately.
Unsaved edits open a Save/Discard/Cancel dialog. Save uses the same acknowledged
batch export as the toolbar action; Cancel preserves the session, while Discard
restores each document's saved-mark baseline and clears undo/redo before closing.
The restored documents retain their original source image, annotation scale, and
editable saved marks in the shared sessions. Reopening after Discard uses that
original source rather than the flattened upload PNG. All closes preserve the hide/reuse workaround
on macOS. `preview_open` records presentation separately from the retained native
window handle, so a hidden clean preview cannot block workspace switching.
An incrementing session generation suppresses late Save results after exit,
including if the same attachment is reopened before PNG export completes.
The top-right Discard action opens a confirmation dialog, then uses the same
restore-and-close behavior as the exit dialog: it retains the latest saved marks
as editable objects and keeps previously staged saved pixels. It cannot
be undone. It is disabled with no unsaved work in any preview image;
Save is disabled if every image matches its saved baseline. Both actions close the preview.

`image_preview/editor.rs` owns native GPUI pointer gestures; its `toolbar.rs`
module supplies the bottom icon toolbar, contextual twelve-color palette,
shape picker, and opacity slider. Pen uses a continuous, rounded scribble glyph
painted as native vector geometry at the other toolbar icons' size and weight;
its label and P shortcut remain unchanged. Rectangle and ellipse have dedicated
toolbar buttons. A shared 17-item picker anchors above the bottom chevron for choosing
a drawing tool, or beside the properties panel for changing the selected shape.
Ellipse is a continuous ellipse; oval is a capsule with semicircular ends.
The editor coordinates
selection handles, text input, style controls, and the text-labelled Save/Discard actions.
Creation gestures clear object selection on release. New text clears selection
when editing finishes; editing explicitly selected existing text retains that
selection. Unlocked tools return to Select, while locked tools remain active.
Palette changes with no selection update defaults for the next object without
changing document history, including opacity changes.
`editor/selection.rs` resolves corner/edge resizing, rotation just outside corners,
and movement with rotated opposite anchors held fixed. Shared handle math
normalizes crossed dimensions, swaps the effective drag handle, and preserves the
opposite anchor through reflections. Shape geometry stores horizontal/vertical
flip flags; pen points reflect directly, while text dimensions and fonts stay
positive and glyphs never mirror. Polygon outlines use short quadratic corner
curves, shared by fills, strokes, hit testing, icons, and PNG export. Transparent interiors pass
through until selected; selected interiors move the object. `editor/cursor.rs`
adds cached native macOS rotate and directional resize cursors. They use 24-point
images with dark arrow silhouettes, thin white outlines, outer dark keylines,
and centered hotspots; moving uses the regular pointer. Two-times rasterization
keeps cursor edges crisp on Retina screens. `editor/group.rs` adds geometry-aware marquee selection, additive
selection, a shared rotated frame, and snapshots for uniform group transforms.
Arrow points transform directly; boxed marks retain their local rotation during
ordinary group scaling. Group reflections map points through the rotated frame,
reflect member geometry, and adjust member rotation without mirroring labels.
Preview cache reuse includes flip flags so a reflection always rebuilds its mesh.
Selection never enters document history; a group edit checkpoints once.
Shared style edits change only the requested property on each mark.
`editor/clipboard.rs` clones selections one full visible width to the right and
copies editable marks through versioned GPUI clipboard metadata with readable
label text as the plain-text representation. Paste validates the metadata and
centers copies at the canvas pointer after converting pan/zoom coordinates;
controls use the viewport center. Geometry, labels, rotation, stacking order,
and per-object units survive copying. Each insertion checkpoints once and selects
the new marks. Command+D/C/V apply outside the native text editing context.
Visible copy extents include stroke thickness, arrow caps, labels, and rotation.
Text hit testing uses padded character advance boxes over the shared shaped line
geometry. Blank rows and whitespace do not make an unfocused transparent shape
opaque to selection. Focused shapes retain interior dragging; selected arrow/line
labels now move their entire host rather than sliding or editing on a single click.
`annotation/stack.rs` preserves selected objects' relative order and fixed
pixelation slots during group stacking. Mixed selections expose common controls.
Cursor requests paint after the floating controls and take window-level priority
only over the artwork. Rotate gestures retain their corner index; the cursor
orientation combines that corner with the current individual or group frame's
rotation. Resize cursors follow the rotated handle axis rather than snapping to
45-degree directions. Native cursor images are generated lazily and cached at
one-degree precision (up to 360 rotate and 180 resize orientations). Active drag
cursors update after geometry changes in the same pointer event. Every pointer
move reapplies the semantic cursor, even when its kind is unchanged. Hover changes caused by child hitboxes do not clear the canvas
cursor while the pointer remains inside the artwork area.
Escape leaves input/selection focus and returns keyboard handling to the
artwork. Shape labels open on a double-click of the outline or selected interior;
plain typing preserves letter tool shortcuts. Return edits standalone text only.
Double-click targeting gives the selected object's body priority over objects
underneath, including transparent shapes that have no label yet.
The native input remains upright while
editing rotated text; finished glyphs rotate with the object.
Fill swatches depict the same transparent, soft, solid, and hatched treatments.
Floating controls suppress idle brush hover, but let movement propagate during
an active canvas gesture or pan so dragging across a palette or toolbar cannot
freeze the object. A release over a control completes the existing drag.
Selection boxes use a blue outline and four blue corner handles with black
centers. Edge hit testing and resize cursors remain active without midpoint
indicators, for individual objects and groups. Arrows use a matching blue path
highlight and endpoint/bend handles without a bounding box.
The shared arrow path trims the shaft inside filled triangle caps, including
curved arrows, so its rounded end does not protrude beyond the tip.
Each continuous pen stroke is a single mark whose points move and resize together.
Native right-click menus select the hit object and offer the four standard
stacking commands; `StackOrder` makes them undoable. Forward/backward skips
nonoverlapping marks. Pixelation strokes stay in the source-effect layer, below
the annotation stack. Zoom/pan
transform pointer coordinates into original-image space. The uploader subscribes
to apply events, rejects changes during import/upload, replaces the staged image
with a validated flattened PNG, preserves its identity and description, and clears
stale upload results. Save and Command+S apply background PNG snapshots and close
after the uploader acknowledges them. Workspace persistence writes the original
source and current annotation JSON together with the rendered PNG. Stroke presets
use 2/3.5/5/10 annotation units. Each document establishes
one annotation scale from its first fitted view, before user zooming, and retains
it through snapshots, save/reopen, undo/redo, and reverting the artwork. Creation
uses that shared unit to convert stroke, font, label spacing, and tap-shape size
into image coordinates. Editor defaults and palette values
stay in annotation units; selection normalizes them, and style edits convert back
using each mark's retained scale. Group scaling and standalone text corner scaling
update that unit too. Text layout measures in annotation units before scaling the
shared glyph paths and caret positions. New and existing marks therefore have
matching widths and font sizes regardless of creation zoom; both zoom with the
image. Pixelation instead keeps its cursor diameter constant on screen, converting
it to image coordinates when each stroke starts. Window resizing does not
recalibrate the document's annotation unit.

The bottom-left percentage opens a native zoom menu. Displayed percentages and
100% use image pixels per logical screen pixel; the internal zoom remains relative
to the fitted image, capped at 1 so fitting never enlarges small sources. The
renderer, pointer transform, and percentage share that fit rule. Zoom limits apply to actual scale. Command+/− changes zoom,
Shift+0 sets 100%, and Shift+1 or Command+0 fits the source image and resets pan.
GIFs omit editor controls and keep their existing animation path.

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
preferences, quit and the Add from Clipboard shortcut. Successful global Add from Clipboard
registrations notify the menu controller, which refreshes its shortcut hint;
disabling or temporarily unregistering during recording clears that hint.
The menu bar uses the favicon's front-facing kitten as a monochrome AppKit
template, displayed at 18 × 18 points from a 36 × 36 pixel PNG. Its editable
source and export instructions live in the [brand artwork record](design/explorations/art/brand-cat/artwork.md#macos-menu-bar).
The Dock, Finder and Applications icon uses the same portrait in color on a
cream-to-lavender Icon Composer background. Bundles ship the compiled asset
catalog and ICNS; source runs embed that same ICNS via `window_limits`.
Close captures the
focused window, defers access until keyboard dispatch releases its borrow, then
uses the existing native close request and per-window cleanup callbacks.
An unscoped Escape action uses the same close path for extra windows other than
the image preview, even when no control has keyboard focus. It identifies preview
content through the GPUI Root and routes Escape to its focus/selection cleanup,
never the close callback. Preview Command+W requests confirmation only for unsaved edits.
The same Root inspection identifies main-window content and routes Escape to that
view to cancel shortcut recording first,
then return from Preferences to the main page on the next press. The main page
keeps its draft. Scoped dialog actions dismiss an open dialog first. Key bindings
run before raw key listeners, so the global action must handle main-window
navigation as well as closing extra windows. Regression tests use the full app
keyboard setup and the same Root wrapper as native windows.

Add from Clipboard runs through a process-lifetime background service with no
window requirement. Menu, global shortcut, and `gatto://add-from-clipboard` invoke
that service. It snapshots the clipboard, waits for pending workspace writes,
loads preferences, and saves a new draft to the chosen project as a direct child
of the projects directory. Null `add_from_clipboard_project` uses Scratchpad and
creates it if necessary. Staging needs no account or repository selection and
never changes the clipboard. Opening the app shows the saved draft, including
before account setup. The separate Draft from Clipboard menu and URL action are
retired; focused Command+V still dispatches the normal paste action.

`automatically_copy_add_from_clipboard` defaults to false. The default completion
posts a native system notification with **Upload and copy**. With the preference
on, staging commits first, then the service uploads the saved item, commits its URL,
and copies its snippet. Upload failures retain the saved draft. Upload and copy
requires a matching owner and project repository, resolves the repository name,
and rejects a different database ID. An already uploaded item reuses its URL.
Every attempt posts a system notification, regardless of window focus, including
busy, updating, unsupported clipboard, and failure results. The native delegate
requests foreground banners as well as notification-center delivery.

Notification tags contain the project folder, immutable project UUID, and
attachment UUID. The button loads that saved item with current preferences;
later clipboard contents or a different chosen project cannot redirect it.
Removed items, replaced project folders, and paths outside project storage are
rejected. The button carries no foreground activation option and creates no window.
Gatto owns the app-global notification response handler for these raw system
notifications; GPUI component toasts remain in-app.

Open destinations share their ordered writer with the main view. Upload completion
updates the existing UUID, rather than appending another attachment. Background
content checks and a final in-memory snapshot check preserve edits made during an
upload; metadata-only image changes survive and use the latest copy text. Closed
destinations reopen, update, and save in the background. Repeated clicks reuse the
saved URL; requests for an item already queued in the open uploader are refused.
Concurrent service requests are rejected. Project navigation, deletion, metadata
editing, and transfer wait until the service finishes. Windows created during a
job defer their initial workspace load until it commits. Closing a window cannot
stop the job. Clipboard writes follow successful persistence only.

Attachment presentation uses a stable partition: items needing upload first, then
completed uploads, with a divider and no category labels. Edited images return to
the first group. The saved/manual order is retained within each group, and drag
reordering stays within its group. Batch upload/copy ordering therefore remains
stable even as individual uploads finish.

## App Preferences and local state

The repository owner (personal account or organization), repository favorites,
project pins, the project folder used for Add from Clipboard, shortcut configuration,
and general preferences are saved as JSON in
`~/.gatto/settings.json`. Previous `quick_copy_project` and global Quick Copy
shortcut keys are accepted on read and rewritten under the new names; automatic
upload remains off unless explicitly enabled. The GitHub token and image bytes
are never written there.
Until an account or organization is set, the main screen points the user to
App Preferences. The `owner` setting accepts the former
`organization` key when reading existing preferences. Changing owners clears
the active repository selection and repository favorites after successfully saving
settings. Project repository records retain their owner and are inactive while
another owner is configured. Global settings do not store repository selections.

Start at Login is recorded in settings and synchronized with the macOS-required file
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

The **Reset app** preference deletes projects, clears Scratchpad, removes settings
and the login LaunchAgent, then returns the UI to first-run setup. Delete All
Projects preserves Scratchpad and preferences. Neither operation modifies GitHub
CLI authentication, which is managed outside of Gatto.

The global Add from Clipboard shortcut uses macOS hotkey registration rather than global keyboard event
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
- Scratchpad and projects persist media copies, metadata, and completed upload
  URLs in workspace folders; see [storage locations](../SUPPORT.md#local-data-and-recovery).
  Annotated images retain original sources and editable annotation JSON locally;
  undo/redo history and GitHub credentials are not persisted.
- Uploads send media directly to GitHub. Annotated images upload as flattened PNGs.
  Staged videos use private temporary snapshots, shared with the configured macOS
  player for preview.
- Custom action URLs do not accept parameters.
- Upload results are copied after an explicit copy action or a user-configured Add from Clipboard action.
- Persistent settings contain repository names and app preferences, never tokens or image bytes;
  Start at Login stores the executable path in a per-user LaunchAgent.
- Logs and errors should never include tokens or raw image bytes.
- Diagnostic events are retained only in memory (up to 500 entries) and are lost on quit.

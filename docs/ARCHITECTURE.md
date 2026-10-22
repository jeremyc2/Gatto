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

## App buttons

`theme.rs` owns `AppButtonStyle` and the shared confirmation footer. Ordinary
text actions use 32-point buttons with the same type size, corner radius, and
horizontal padding. Primary actions use the app's purple fill. Secondary actions
use the landing page's neutral background and subtle outline. Compact utility
buttons use the same outline at 20 points, including attachment description,
copy, and remove actions and log-window controls. Accessible names, keyboard
focus treatment, loading indicators, and disabled states stay on the underlying
GPUI Button.

Upload uses the standard text-button size and icon proportions, with enough width
for its progress label. Preview Save and Discard use the same styles and dimensions
as their exit-dialog equivalents. Confirmation footers constrain GPUI's full-size
wrappers so their buttons stay compact; the three-action exit dialog separates
Discard from the Cancel/Save group. The annotation tool palettes and illustrated
Rooftops score card retain their distinct controls.

This applies Refactoring UI's [hierarchy-first button guidance](https://medium.com/refactoring-ui/7-practical-tips-for-cheating-at-design-40c736799886):
make the primary action obvious, with quieter treatments for supporting actions.

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
File drops and Finder pastes preserve all input paths through validation and the
replacement dialog. Switching to bulk keeps the existing attachment and appends
the whole accepted batch in order; a multi-file input to an empty draft enables
bulk upload directly. Cancel keeps the draft, while Replace uses the first new
attachment.
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
Utility windows close independently: confirmed preview exit releases its image references,
logs refresh on reopening, and Rooftops resets its run on close. Other platforms
use native destruction.

The shared image/GIF preview starts with maximized window bounds and no content
size cap. Reusing its retained window preserves subsequent manual resizing.

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
and glyph paths with whole-pixel line heights and preserved blank lines. On macOS,
CoreText handles contextual line breaking and shapes Arial with native kerning,
ligatures, and system glyph fallback. Its outlines feed the same native geometry
and PNG export. Glyphs without outlines, including color emoji, retain CoreText's
fallback font and positions and are drawn into bounded RGBA bitmaps. The shared
layout carries those bitmaps through scaling, hit testing, preview, and export.
Other platforms use the existing TrueType outline fallback.
Layout caches are bounded by entry count and estimated memory, shared across
preview geometry and measurement; the portable fallback also caches character
advances. A separate bounded line cache reuses unchanged shaping as paragraphs
are edited. Shaped outlines retain positioned words, splitting unbroken runs into
short fragments while preserving CoreText glyph placement. The native preview
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
`ttf-parser` supplies outlined glyph
paths from the macOS Arial font, without bundling another font.

PNG export runs from a background snapshot, composites opacity once per whole
mark, expands to include off-image marks, and enforces dimension/allocation limits.
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
contain no recoverable source layer. Each staged image owns an in-memory `Session`
that retains the original document and source image independently of the flattened
upload bytes. The open preview paints from the original coordinate system, rather
than annotating an already flattened replacement. Each preview image and retained
session stores a saved-mark baseline separately from undo/redo history. Save compares
the current marks to that baseline, exports all changed images in a background batch,
and shows a loading indicator while blocking additional edits. Each uploader event
is acknowledged after the staged replacement is accepted; errors leave the preview
open and retryable. Only after every acknowledgement does Save close the preview,
retaining the original document, saved baseline, and history for reopening.
Both the native close button and Command+W close clean previews immediately.
Unsaved edits open a Save/Discard/Cancel dialog. Save uses the same acknowledged
batch export as the toolbar action; Cancel preserves the session, while Discard
clears every preview document and shared retained session before closing. After
that destructive exit, reopening decodes staged bytes as a fresh document, retaining
saved pixels without editable marks/history. Clean closes and Save preserve history.
All closes preserve the hide/reuse workaround on macOS.
An incrementing session generation suppresses late Save results after exit,
including if the same attachment is reopened before PNG export completes.
The top-right Discard action opens a confirmation dialog, then uses the same
destructive close as the exit dialog: it releases every preview document and
its shared retained session, keeping previously staged saved pixels. It cannot
be undone. It is disabled with no work or history in any preview image; Save is
disabled if every image matches its saved baseline. Both actions close the preview.

`image_preview/editor.rs` owns native GPUI pointer gestures; its `toolbar.rs`
module supplies the bottom icon toolbar, contextual twelve-color palette,
shape picker, and opacity slider. Rectangle and ellipse have dedicated toolbar
buttons. A shared 17-item picker anchors above the bottom chevron for choosing
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
after the uploader acknowledges them, without writing separate files or resetting
history. Stroke presets use 2/3.5/5/10 annotation units. Each document establishes
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
preferences, quit and clipboard preview shortcuts. Successful global Quick Copy
registrations notify the menu controller, which refreshes its shortcut hint;
disabling or temporarily unregistering during recording clears that hint.
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

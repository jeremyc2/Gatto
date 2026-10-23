# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.17.7] - 2026-10-23

### Changed

- Custom URLs now match menu wording with `draft-from-clipboard` and
  `app-preferences`. Added `side-by-side-comparison` to open the comparison picker (#27).

## [0.17.6] - 2026-10-23

### Fixed

- **Send to Gatto** resolves Finder file references before importing, preventing
  `Could not inspect /.file/id=…` errors for selected files.

## [0.17.5] - 2026-10-23

### Added

- Finder integration through **Open With → Gatto** and a built-in **Send to Gatto**
  service, with no separate action installation. Opening files stages them for
  review, preserves batch and draft choices, and keeps supported files from mixed
  selections. Upload still requires an explicit click.
- `scripts/build-app.sh` compiles and packages a fresh local `dist/local/Gatto.app`
  without publishing. `scripts/build-dmg.sh` packages that bundle by default and
  prints the finished DMG's absolute path.

### Changed

- File imports consistently protect the current selection while files are being read.
- The image editor's Pen tool uses a freehand scribble icon, matching its drawing
  action. Its label and **P** shortcut are unchanged (#26).

## [0.17.4] - 2026-10-23

### Added

- Place up to 99 numbered walkthrough markers per image with **N**. Crisp vector
  badges use individual palette colors and double-circle borders. A corner drag
  resizes all markers and their numbers together; numbers stay upright. Deletion
  closes numbering gaps, and clones/pastes receive the next numbers. Edit optional
  descriptions in a full-width bottom overlay without moving or zooming the image.
  Only badges are embedded; single and bulk text copying append descriptions as
  numbered lists beneath their images without headings, including blank items
  for markers without descriptions. The ordered-list attachment icon appears
  whenever an image has markers. Editing supports undo/redo, Save, reopening,
  and Discard.

## [0.17.3] - 2026-10-23

### Added

- Open **Side-by-Side Comparison** from the native Advanced menu or menu bar
  icon. A small window stacks Left image above Right image, replacing drop zones
  with removable attachment cards as images are added. Command+V fills the first
  empty side. Adding both images closes the picker and prepares one PNG in the
  main window with an attachment loading spinner, ready for the regular editor.
  Fit the complete comparison, including labels and padding, within 3840 × 2160
  without enlarging small sources. Use high-quality resizing and reduce further
  only when needed to fit 9 MB initially or 9.5 MB after annotations. Preserve
  image proportions, top alignment,
  and a gap between the photos. File selection, dropping, and clipboard paste
  accept PNG, JPEG, and WebP images; GIFs and videos receive a warning.
  Comparison images have editable **Before** and **After** captions below the
  images, without a background, border, or tab shape. Captions share a baseline,
  wrap within independent columns, and accept Shift+Return for new lines.
  Center small images above wider caption columns without upscaling them.
  Double-click to edit with the freeform text tool's native caret and selection,
  keeping the caption anchor and width stable. Protect the caption row from
  annotations and allow pan gestures across it. Stop comparison annotations at
  the image row's lower edge when drawing, moving, resizing, or rotating; include
  stroke thickness, arrow caps, and grouped selections. Active drags bypass
  caption hover hitboxes so they resume immediately above the boundary.
  Caption edits participate in
  undo, redo, Save, and Discard, and appear in the saved PNG. Ordinary images
  keep their normal editor. Combining does not upload or modify either original.

### Changed

- Update GPUI Kit from 0.7.3 to 0.8.0 while retaining the compatible GPUI 0.3.9
  backend and existing features.
- Rename the menu bar action to **Draft from Clipboard**, keeping its **⌘V** shortcut.

## [0.17.2] - 2026-10-22

### Changed

- Separate the upload footer with a full-width divider. Show a denied cursor over
  disabled Upload buttons and a reminder to add an image or video when empty.

## [0.17.1] - 2026-10-22

### Changed

- Update tiny-skia to 0.12, Core Graphics to 0.25, and Core Text to 22 for
  annotation rendering and native text layout.
- Make the image-editing walkthrough easier to follow with short, task-focused
  guidance and screenshots for text, shapes, selection, pixelation, and saving.

### Fixed

- Refresh walkthrough screenshots before reviewing and committing each release;
  stop the release if screenshot generation fails.

## [0.17.0] - 2026-10-22

### Changed

- Rename the menu bar action to **Stage from Clipboard**, keeping its **⌘V** shortcut.

### Fixed

- Keep the latest saved annotations editable after Discard. Restore saved objects
  on the original image while clearing unsaved changes and undo/redo history,
  including across multiple preview images. Reopening still allows moving,
  resizing, editing, and saving those objects; uploaded PNGs contain flat pixels.


- Update Gatto's Cargo.lock version during release preparation, including when
  reusing a prepared release, so locked release checks pass without changing
  dependency pins.

## [0.17.0-beta.1] - 2026-10-22

### Added

- Choose Stable or Beta releases in App Preferences. Beta includes newer stable releases; returning to Stable keeps an installed beta until Stable catches up. Channel changes clear old offers and never install automatically.
- Choose release operations from a numbered menu showing the resulting version
  and channel, including advancing a beta or promoting it to Stable.

- Hold Shift to draw closed shapes with equal width and height or resize corners
  proportionally, including rotated shapes. Keep selection handles active when
  Shift is already held, preserve label minimums, and retain 15-degree Shift rotation.

- Anchor the properties shape menu beside the Shape button, centered on its row
  and kept inside the preview. Tighten floating palette and toolbar spacing and
  enlarge opacity and pixelation slider handles. Join the history bar directly to
  the tool bar with a small right inset, and keep XL labels fully visible.

- Speed up rapid typing by reusing unchanged text outlines and line shaping,
  keeping long words in bounded fragments, and batching visible text drawing.
  Apply the same rendering path to standalone text and shape, arrow, and line labels.

- Show blue wrapping bounds while creating or editing fixed-width text, without
  resize handles. Keep free-form text unboxed while typing, and turn it into a
  fixed-width box when directly resized. Selecting either kind shows its handles.

- Highlight all existing text when opening it for editing or double-clicking
  within an active input, including standalone text and shape/arrow/line labels.
  Typing replaces the selection; arrow keys move to its beginning or end.

- Move objects with their labels when dragging the text, including arrows and
  lines. Transparent shapes pass through blank text-box areas until selected.
  Confirmed Discard closes the preview and clears its editing history.
- Rename Duplicate to Clone, place clones a full selection width to the right,
  and add Command+D. Copy editable selections with Command+C and paste at the
  canvas cursor with Command+V, preserving grouped geometry and styles.
- Narrow the pixelation brush slider to 30–138 screen points, keeping its 64-point
  default and continuous sizing with a paintable minimum.

- Preserve visible shaft beyond both arrow caps when multiline labels grow or
  arrows are shortened, bent, or rotated in a group. Check the full label footprint
  at every angle without shrinking text or moving a fixed opposite endpoint.

- Keep the pixelation brush circle and hover preview at a constant screen size
  while zooming, with new strokes matching the circle and existing strokes fixed
  to the image. Remove dark brush outlines and tile seams during zoomed previews.

- Keep short arrow and line labels on one line, wrap longer labels between words
  using the path length and cap clearance, and preserve manual Enter line breaks.
  Prevent fractional-width rounding from splitting words. Keep enough space for
  text while typing or resizing endpoints, without shrinking its font.

- Move Pixelate to the far right after shape overflow and give it a circular,
  colored icon. Preview mosaic pixels inside the blue brush circle on hover,
  beneath annotations, without changing artwork or history until painting.

- Replace pixelation brush size presets with a continuous slider, matching the
  opacity control. Offer fine control for small brushes with a paintable minimum.

- Simplify selection indicators to blue outlines and blue corner handles with
  black centers. Keep edge resizing and hover cursors active without visible
  midpoint handles, including group selections.

- Standardize app actions around the landing page's outlined neutral buttons and
  purple primary buttons. Match attachment, preference, log, preview, and dialog
  actions; balance Upload's label and icon proportions. Share button sizing and
  confirmation footers while preserving annotation-tool styling.

- Simplify the unsaved-changes dialog with compact, consistently sized buttons:
  Save is primary, Cancel and Discard share a neutral style, and spacing separates
  Discard from the other choices. Shorten the heading and explanation.

- Render text inside closed shapes in white with a black outline regardless of
  the shape color, while keeping standalone text colors selectable.

- Give rectangles and polygon shapes slightly rounded corners. Allow resize
  handles to cross the opposite edge or corner and continue on the other side,
  including rotated objects and groups. Reflect geometry while keeping text
  readable, preserving label minimums, and updating resize cursor direction.

- Calibrate annotation presets once to each image's opening view, including text,
  labels, and click-created shapes. Drawing before and after zooming
  produces matching stroke widths and font sizes. Retain those units when
  resizing the window or reopening saved work. Fit large images into the preview
  without enlarging small images; retain one actual zoom percentage.
- Place default-size closed shapes with a click, including transparent shapes.
  Line clicks place a visible dot with overlapping, draggable endpoints; arrow
  clicks leave the artwork and undo/redo history unchanged.
- Add a native Rust image annotation editor with a tldraw-style bottom toolbar,
  contextual twelve-color palette, outlined text, wrapping and resize handles,
  sixteen closed shapes plus a line, patterned fills, curved arrows with independent caps, pen,
  pixelation, erasing, and undo/redo. Preserve GIF playback as preview-only.
  Save flattened edits, including off-image annotations, then close the preview
  while retaining the original and full history for reopening. Use text-labelled
  Save and Discard actions, disable unavailable actions, show saving progress,
  and offer Save, Discard, and Cancel when exiting with unsaved edits. Discard confirms restoring the
  original and clearing all undo/redo history. Stroke presets use tldraw’s 2/3.5/5/10 size steps.
  Live drawing caches native GPU geometry; PNG rasterization runs in the background.
  Right-click objects to bring them forward/to front or send them backward/to back.
  Move and resize each continuous pen stroke as one object. Give pixelation a
  photo-thumbnail icon.
  Paint pixelation with a circular brush and blue cursor preview, with four brush
  sizes. Strokes stay fixed and can be undone/redone, but cannot be selected,
  moved, resized, or erased.
  Put rectangle and ellipse directly in the bottom toolbar, with the shared
  17-item shape picker above its chevron and beside the shape properties button.
  Add a zoom percentage menu with zoom in/out, actual 100%, and fit commands.
  Keep the preview open on Escape, clearing only focus/selection. Confirm exit
  through the close button or Command+W, discard preview edits/history on exit,
  and reopen from the last applied image.
- Resize objects from selection borders and corners with directional cursors,
  move selected transparent shapes from their interiors, and rotate shapes,
  text, and pen strokes just outside selection corners. Keep rotation consistent
  across preview, hit testing, history, and exported images.
- Rework text using tldraw's documented behavior: fixed text presets, automatic
  or fixed-width standalone text, left/center/right alignment, and shape-label
  top/middle/bottom alignment. Shape labels wrap across padded rectangular bounds,
  grow their hosts, and enforce content minima during resizing rather than
  shrinking or disappearing. Include labels on X boxes and checkboxes.
  Add editable arrow labels with path-relative positioning, dragging, center snap,
  endpoint clearance, and transparent shaft cutouts.
  Use one native glyph layout for editing, hit testing, preview, and export,
  including rotated caret/selection coordinates, native composition, plain-text
  clipboard operations, and local text undo/redo. Remove opaque editing fields
  and editing-only width/padding changes. Preserve one font and plain text.
  Replace generic fill-option icons with swatches showing transparent, soft,
  solid, and hatched fills.
- Add box selection, Shift-click selection, and Command+A, with shared movement,
  proportional resizing, rotation, styling, stacking, duplication, deletion,
  and one-step undo. Keep pixelation fixed and outside object selection.

### Changed

- Give Release channel its own section in App Preferences, including the channel
  dropdown, update check, and update status.
- Replace the app's direct HTTP, menu-bar, global shortcut and dynamic-loader
  dependencies with bounded macOS adapters. Keep upload progress, cancellation,
  safe retry behavior, menu actions and configurable shortcuts.
- Replace Toastify with the website's own accessible notification and use Bun's
  built-in TypeScript checker. Require locked local tools, disable install
  scripts, audit JavaScript dependencies with visible expiring exceptions, and
  run builds before signing credential checks.

- Widen the preview's Save button in both idle and loading states so its spinner
  and full “Saving…” label fit without resizing or truncation.

- Remove Gatto's direct `ttf-parser` dependency and the unused non-macOS annotation
  text fallback. Annotation text continues to use macOS CoreText for layout,
  preview, and PNG export.

### Fixed

- Preserve native color emoji in annotation text and labels, including selection
  and PNG export. Match whole-object opacity between preview and saved images.

- Open image and GIF previews maximized on first use, allow resizing without a
  maximum content size, and preserve the chosen size when reopening the window.
- Leave completed drawings and new text unselected, so palette changes prepare
  the next object without restyling the last one or adding artwork undo steps.
- Hide selection outlines and handles while drawing, preserving selection and
  transform controls for finished objects.
- Orient rotate cursors around all four selection corners and follow object or
  group rotation. Align edge and corner resize cursors to the rotated axes, and
  update cursor direction immediately during a drag.
- Update rotation and directional resize cursors on hover with small outlined
  double-arrow silhouettes, keeping the regular pointer while moving objects.
  Preserve canvas cursor priority over floating element defaults.
- Always highlight one active size preset, including default stroke and brush
  sizes and the closest preset for custom text or stroke sizes.
- Give every shape a darker fill and matching brighter strokes from one palette
  choice. Extend X-box diagonals to the corners and match checkbox proportions
  to the reference, with both symbols following their border's color and dash
  style in the preview and exported image.
- Keep mirrored filled arrow shapes solid where their borders overlap the fill.
- Match rectangle, ellipse, and shape-menu icons to the other tool icons' size
  and proportions.
- Keep object handles and canvas drags moving across floating controls instead
  of freezing until the pointer leaves a menu.
- Keep filled arrowheads solid where their caps overlap the shaft.
- Render pixelation with cached textures, updating only newly painted tiles, to
  avoid rebuilding thousands of mosaic primitives per frame. Apply it only to
  the source image beneath annotations, in both the editor and exported output.
- Highlight selected arrows along their path, with endpoint and bend handles,
  without a bounding rectangle.
- Trim arrow shafts inside filled triangle caps so their rounded ends cannot
  protrude beyond the tip, in both preview and exported output.

## [0.16.3] - 2026-10-20

### Fixed

- Restore Escape navigation from App Preferences when the app-wide Escape action
  is enabled, including focused inputs and cleared control focus. Keep dialog
  dismissal and shortcut-recording cancellation ahead of returning to the main page.

## [0.16.2] - 2026-10-20

### Changed

- Use `diskutil image attach` and `diskutil eject` for installer packaging and
  release-size measurements instead of the deprecated `hdiutil` commands.

## [0.16.1] - 2026-10-20

### Fixed

- Keep every dropped or pasted file when switching an existing attachment to
  bulk upload. Enable bulk upload for multi-file input into an empty draft.
- Close the focused extra window with Escape, including Application logs, image
  previews, and Rooftops, even without a focused control. Preserve the main draft
  and existing dialog and Preferences Escape behavior.

## [0.16.0] - 2026-10-19

### Added

- Add an unchecked **Copy images as Markdown** preference to use
  `![description](URL)` instead of sized HTML image tags across individual copy,
  bulk copy, and Quick Copy, including existing uploads.
- Add an unchecked **Allow videos up to 100 MB** preference for eligible paid
  GitHub plans. Apply it to every import path, Quick Copy, and compression;
  retain the 10 MB image/GIF limit and recheck staged videos before upload.
- Offer Cancel / Compress for oversized video imports. Use native hardware
  H.264/AAC MP4 compression with original resolution, progress, cancellation,
  bounded retries, and strict final size validation. Keep source files unchanged
  and provide recording guidance, including clips under 15 seconds.
- Upload MP4, MOV, and WebM videos through drag/drop, file selection, or copied
  Finder files, including mixed batches and Quick Copy. Open video previews in
  the default macOS video app with its playback and audio controls.
- Play animated GIF frames in the image preview, with their recorded timing,
  pausing when inactive and respecting Reduce Motion.
- Stream attachment uploads with byte progress, queued/waiting states, cancel,
  a five-minute total timeout and 90-second inactivity timeout, and actionable
  network, access, format, size, rate-limit, and server errors.

### Changed

- Default each image/video to 10 MB for Free-plan compatibility. Keep videos as
  private temporary snapshots for stable previews/retries without full-memory
  buffering. Separate copied snippets with blank lines for GitHub video players.
- Copy uploaded images as HTML `<img>` elements by default, with width, height, and alt text
  for GitHub Markdown. Normal, bulk, and Quick Copy uploads use the image's pixel
  dimensions and escape HTML attributes.

### Fixed

- Restore the original gap between the drop zone and attachment previews by
  rendering the compression panel only while a compression job is active.

## [0.15.4] - 2026-10-19

### Added

- Add **App Preferences** with **⌘,** to the Gatto application menu. Show **⌘Q**
  for Quit and **⌘V** for Preview from Clipboard in the cat icon's menu, plus the
  enabled Quick Copy shortcut, which stays in sync with shortcut preferences.

## [0.15.3] - 2026-10-19

### Added

- Add **Gatto → Check for Updates…** to the macOS application menu, with a native
  version prompt, GitHub release link, and Update/Cancel choices. Report when no
  update is available or checking fails; preserve quiet scheduled checks and the
  existing Update controls.

## [0.15.2] - 2026-10-19

### Added

- Press **Escape** in App Preferences to return to the main page, just like **Done**.

## [0.15.1] - 2026-10-19

### Added

- Open App Preferences with **Command+,** while Gatto is focused.

## [0.15.0] - 2026-10-19

### Added

- Add a Homebrew cask that installs Gatto and its GitHub CLI dependency.
- Add quiet update checks while Gatto runs, using Sparkle’s four-hour scheduler.
  A compact Update control downloads, verifies, installs, and restarts only after
  a click, with retry support and protection for active uploads.
- Add linked game-development research covering engines, gameplay, physics,
  assets, NPCs, story structure, and project scope.

### Changed

- Guide version selection and release notes, run release checks, commit and push
  code, verify uploaded assets before publication, and automatically refresh,
  verify, commit, and push the README figures and Homebrew cask. Interrupted
  publication or metadata updates can resume without rebuilding.
- Start at Login now runs quietly in the menu bar without creating a window or
  showing a Dock icon; shortcuts and update checks start independently of the UI.
- Keep the website's Share scene pinned while a final paw trail reaches an
  unnumbered rainbow pawprint marker. Reverse scrolling retraces the trail.
- Adapt website chapter controls and pawprint density to the available width,
  center the tracker on wide screens, and preserve keyboard focus when compact
  controls are hidden.

### Fixed

- Load Bun's types for release scripts in editors and provide a dedicated script
  type check.

## [0.14.0] - 2026-10-17

### Added

- Add a scroll-revealed Gattoverse portal between the website download area and
  footer, with matching torn-paper edges, a watercolor setting, and a jetpack cat
  that approaches and floats. Reduced motion shows a static composition.
- Show a dismissible “Gattoverse mini game coming soon.” notice when the portal
  is activated, with keyboard support and a contrasting notification that slides
  into the top right.

### Fixed

- Keep the website paw trails progressing to each chapter, stop them at the
  selected marker when using chapter controls, and return to normal page scrolling
  as Share arrives. Remove the divider above the chapter tracker.
- Keep website illustrations inside their scenes on wide, shorter screens,
  preventing overlap with the chapter tracker.
- Refine the website cats’ materials, lighting, limbs and tail poses, and correct
  the GitHub tray and picture placement across the story.
- Avoid a Solid development warning when synchronizing the initial story chapter
  and keyboard focus.

## [0.13.0] - 2026-10-17

### Added

- Support repositories owned by personal GitHub accounts as well as organizations.
  App Preferences now accepts a GitHub account/organization login, preserving
  existing organization settings. Changing owners clears the previous selection
  and pins to keep uploads tied to the chosen owner.
- Added the public Gatto website with an illustrated scrolling story, chapter
  navigation, download links, and links to setup, walkthrough, and support guides.
  The story supports fast scrolling, reduced motion, and narrow screens, with
  individual paw prints that adapt to the available space.
- Added a black cat favicon inspired by the website's luminous character, with
  cyan and violet eyes, scalable artwork, browser fallback sizes, and an Apple
  touch icon.
- Added `bun deploy:web` to build and update the existing Cloudflare site using
  the beta `cf` CLI, keeping account configuration in the local environment.

### Changed

- Updated GPUI Kit from 0.7.1 to 0.7.3 and serde_json from 1.0.152 to 1.0.154.
- Replaced the README download link with a badge; the release-size updater keeps
  its target aligned with the measured release.

## [0.12.1] - 2026-10-14

### Added

- Added a Bun/TypeScript release-size updater and documentation maintenance guidance.

### Changed

- Shortened the README around macOS use, key features, and measured release sizes;
  moved setup and developer instructions into linked guides.

## [0.12.0] - 2026-10-14

### Added

- Automatically eject the matching installer disk in the background after Gatto
  launches from Applications, without extra instructions or cleanup prompts.
  Busy or ambiguous disks are left alone; the downloaded DMG is preserved.

### Fixed

- Restore Command+W to close the focused window and Command+Q to quit from any
  app window, including while a text field is focused. Add native Close Window
  and Quit Gatto menu items showing their shortcuts.

## [0.11.1] - 2026-10-14

### Changed

- Simplified the DMG installation window to show "Drag and Drop" without
  persistent names beneath the app and Applications icons.

### Fixed

- Prevented horizontal scrolling from exposing a white strip beside the DMG artwork.

## [0.11.0] - 2026-10-14

### Changed

- Replace the ZIP download with a signed, notarized DMG featuring original dark
  anime-cat artwork, a curved matte arrow, and an Applications folder shortcut
  for drag-to-Applications installation.

## [0.10.5] - 2026-10-12

### Changed

- Documented the Touch Bar window-close workaround, its upstream GPUI issue,
  validation limits, and requirements for safely restoring native window closing.

## [0.10.4] - 2026-10-12

### Fixed

- Mitigated window-close crashes on Touch Bar Macs by hiding and reusing macOS
  windows instead of destroying them. Closing still clears preview images and
  resets Rooftops; reopening logs refreshes their contents. Dock reopening keeps
  its main-page behavior, and Quick Copy remains entirely windowless.

## [0.10.3] - 2026-10-12

### Fixed

- Clicking the Dock icon opens the main page when no windows are open, while
  preserving the current page of existing windows.
- Windows now close independently instead of being hidden and retained, keeping
  Gatto running after the last window closes and avoiding crashes from
  programmatic close actions such as Close the window after copy.
- Quick Copy uploads entirely in the background without creating a window or
  disturbing staged images. Successful uploads copy Markdown and send a macOS
  notification; missing configuration or upload failures notify without changing
  the clipboard.

## [0.10.2] - 2026-10-11

### Changed

- Clarified that images are uploaded directly to GitHub and Gatto has no backend
  or image-hosting service of its own.
- Expanded setup, bulk upload, preview, recovery, uninstall, release, and
  architecture documentation, with guidance for contributors.
- Corrected security guidance for Quick Copy, supported versions, and disabled
  CI and dependency-scanning workflows.

## [0.10.1] - 2026-10-11

### Changed

- Updated Rooftops coins with a clearer baby-kitten design and ornate gold detailing.

### Fixed

- Applied the app's window-teardown workaround to Rooftops to address crashes when
  closing its window in bundled builds. Closing resets the game, so reopening
  starts a fresh run.

## [0.10.0] - 2026-10-11

### Added

- Added Rooftops, a three-level black-cat platformer.
- Added cat-face coins, rats and owls, lantern checkpoints, parallax rooftops, and illuminated
  apartments with animated residents to Rooftops.
- Added a results screen with coins, kills, deaths, and active play time, plus a
  high-resolution scorecard download to Downloads.
- Added Rooftops documentation and walkthrough screenshots.

### Changed

- Refined hover feedback in the main window.

### Fixed

- Improved Rooftops rendering performance and kept its status and completion prompts readable over
  the scrolling scene.
- Prevented score values from clipping in the results screen and walkthrough screenshots.
- Corrected the cat's ear placement to match its profile view during gameplay and on scorecards.

## [0.9.1] - 2026-10-11

### Fixed

- Fixed the default global Quick Copy shortcut to Command+Shift+G so it matches the documented shortcut and the app's built-in default.

## [0.9.0] - 2026-10-10

### Added

- Added a Settings link to the Gatto GitHub repository with author attribution.

## [0.8.2] - 2026-10-10

### Changed

- Changed the Markdown copy shortcut from Command+Shift+M to Command+C for single and bulk uploads.
- Show the Markdown copy shortcut hint after any successful upload, including a single image.

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

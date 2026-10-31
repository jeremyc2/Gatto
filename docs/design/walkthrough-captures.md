# Walkthrough captures

Captured on 2026-10-25 from the installed drawing application using
[Scout, the screenshot cat](art/screenshot-cat/ARTWORK.md), original pixel art
created for these examples. Its editable Aseprite source and native PNG live in
the repository. The source art is independent of Gatto’s logo and website
illustrations.

The app's header artwork was refreshed on October 27, 2026 using the
[dark-background pixel cat](explorations/art/app-cat/artwork.md). App captures
were regenerated through the same native renderer; the illustrated editor
fixtures and their source crops remain independent.

Main-window captures show the current interface at 800 logical pixels wide,
including the 220-pixel project sidebar. Most use a 600px height; Preferences
fits the full settings page, compression uses 640px, and reorder/collapsed-sidebar
examples use 760px. Preferences height is measured from the rendered content,
and capture generation fails if any part of that content is clipped.
The batch example uses a 1024 × 768 window so the sidebar and three attachment
rows fit a compact 4:3 landscape frame. Editor and game captures use their own
window sizes. Decorative motion is disabled while capturing.

Run `./scripts/update-walkthrough-screenshots.sh` to regenerate all captures from
the production interface. The native renderer exports at 2×. Raw PNGs live in
`docs/images/walkthrough/`; the website exports its WebPs from these captures.
Markdown guides use the generated `framed/` copies, with transparent rounded
corners and a thin white edge (16-point radius and 1-point border). The frame is
rendered into the PNG because [GitHub removes inline styles](https://github.com/github/markup#github-markup).

All screenshots embedded in Markdown use the rounded frame, including the
scorecard. `bun refresh:assets` also exports the editable artwork, compiles app
icons and generates the [installer preview](../images/dmg-install.png) from the
DMG's shared layout/background and actual macOS icons. It omits the title bar and
uses the same frame. The release and web deployment commands run this complete
refresh automatically; the standalone walkthrough command refreshes only app captures.
See the [release guide](../RELEASING.md#preview-the-installation-window).

Refresh only the project examples with
`cargo run --features walkthrough-screenshots --bin walkthrough-screenshots -- docs/images/walkthrough --projects-only`.

## Source images

The committed PNGs in `docs/images/walkthrough/source/` are crops of actual
1412 × 1012, 1412 × 1280 and 1416 × 1012 window captures, without reconstructed controls or
composited UI.
They exclude the title bar, menus, document tabs, and capture pointer. Coordinates
below use pixels in the original window capture, with an exclusive lower edge.

| File                    | Window crop          | View                                                                                                    |
| ----------------------- | -------------------- | ------------------------------------------------------------------------------------------------------- |
| `canvas.png`            | x 0–1408, y 164–948  | Centered canvas at 300%, timeline hidden.                                                               |
| `timeline.png`          | x 0–1408, y 164–1220 | Centered canvas at 200%, timeline and artwork layers visible.                                           |
| `detail.png`            | x 0–1100, y 164–864  | Closer crop of the canvas capture, keeping the character, complete shovel, palette and annotation area. |
| `path.png`              | x 0–1408, y 164–1276 | Timeline view including the visible local document-path prefix in the status bar.                       |
| `comparison-orange.png` | x 0–1408, y 164–948  | Original orange gardener at 300%, timeline hidden.                                                      |
| `comparison-black.png`  | x 0–1408, y 164–948  | Same pose and clothes with a black coat, at the same 300% framing.                                      |

The unannotated path capture is a fixture for the privacy demonstration; the
published editor image covers that path with Gatto’s pixelation tool. Other
captures, including both comparison sources, exclude the status bar and its
file paths. If a later crop includes a path, redact it with Gatto’s pixelation
brush before publishing. Treat the sources as intentionally retained
screenshot fixtures, not temporary working documents.

## Composition

`src/image_preview/editor.rs` seeds the real editor with one scene at a time.
`src/walkthrough_screenshots.rs` renders it through the production Metal renderer.
Run `./scripts/update-walkthrough-screenshots.sh` to regenerate the published PNGs.
Recapturing the source windows is a separate step: open the committed
`scout.aseprite`, frame the two views above, capture the real window, and crop
without scaling or changing its pixels. Keep the pointer above y 164. Recheck
the palette, frame-marker and path-redaction coordinates in the editor fixture
when the workspace layout changes.

Use one arrow and one note for the overview. The note asks for a stronger
active-color outline. The group example selects that related pair. The text
example edits the same note in the closer view; the shape example uses a simple
outline around the relevant swatch. Two numbered markers collect the active-color
and selected-frame feedback in the timeline view. The privacy example uses only
pixelation over the path, without decorative annotations. Its editor view is
zoomed out and panned upward to keep the path and brush cursor above the floating
toolbar.

Use the full relevant workspace for context, with a closer view for text editing.
No collage is needed for these tasks. Keep the walkthrough prose about Gatto’s
controls; leave the example feedback in the images. Alt text describes the
visible operation for readers who cannot see the image.

The example organization and repository names are fixed demonstration data, not
an upload destination or a claim that feedback was filed with another project.
No screenshots are uploaded while generating the walkthrough. Inspect the
staged-image thumbnail as well as the full editor: the cat’s ears, face, scarf,
blue outfit, curved tail and shovel should remain recognizable at the smaller
size.

Keep single-file staging and copy examples focused on one attachment. The mixed
batch includes an uploaded interface image, a video currently sending, and a
queued timeline image, so the extra row both fills the window and explains the
remaining work. These are in-memory states with no network activity.

`comparison-editor.png` shows the two matched Aseprite captures open together in
Gatto’s editor, with captions renamed through its actual editing controls to
“orange cat” and “black cat”. Keep this comparison free of arrows and other
annotations: the changed coat, unchanged clothes and matching pose are the
comparison. The original orange cat and the separate
[black gardener variant](art/screenshot-cat/ARTWORK.md#black-gardener-comparison-variant)
remain editable. These inputs replace the earlier Gatto main-window captures;
no published editor or attachment example now uses Gatto itself as its image.
The regular screenshot command captures it at 1060 × 740 logical pixels; use
`--comparison-editor-only` with the same native screenshot executable to refresh
just this view. Picker and caption diagnostics remain opt-in with
`--comparison-only`. The website reuses the batch, annotation overview and
comparison editor captures; its framing and exports belong to the
[website design system](../../web/DESIGN-SYSTEM.md#feature-screenshots).

## Licenses and attribution

The captured interface is Aseprite 1.3.18.6, copyright Igara Studio S.A. and its
contributors, governed by its [license agreement](https://github.com/aseprite/aseprite/blob/main/EULA.txt).
Its appearance here demonstrates Gatto’s image tools and does not imply endorsement.
Third-party interface artwork retains its owners’ rights; Gatto’s MIT license
does not relicense it. The original Scout drawing is covered by Gatto’s MIT
license; it does not reproduce an existing game character.

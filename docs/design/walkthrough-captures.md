# Walkthrough captures

Captured on 2026-10-25 from the installed drawing application using
[Scout, the screenshot cat](art/screenshot-cat/ARTWORK.md), original pixel art
created for these examples. Its editable Aseprite source and native PNG live in
the repository. The source art is independent of Gatto’s logo and website
illustrations.

## Source images

The committed PNGs in `docs/images/walkthrough/source/` are crops of actual
1412 × 1012 and 1412 × 1280 window captures, without reconstructed controls or
composited UI.
They exclude the title bar, menus, document tabs, and capture pointer. Coordinates
below use pixels in the original window capture, with an exclusive lower edge.

| File | Window crop | View |
| --- | --- | --- |
| `canvas.png` | x 0–1408, y 164–948 | Centered canvas at 300%, timeline hidden. |
| `timeline.png` | x 0–1408, y 164–1220 | Centered canvas at 200%, timeline and artwork layers visible. |
| `detail.png` | x 0–1100, y 164–864 | Closer crop of the canvas capture, keeping the character, complete shovel, palette and annotation area. |
| `path.png` | x 0–1408, y 164–1276 | Timeline view including the visible local document-path prefix in the status bar. |

The unannotated path capture is a fixture for the privacy demonstration; the
published editor image covers that path with Gatto’s pixelation tool. Other
captures exclude the status bar. Treat the sources as intentionally retained
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

## Licenses and attribution

The captured interface is Aseprite 1.3.18.6, copyright Igara Studio S.A. and its
contributors, governed by its [license agreement](https://github.com/aseprite/aseprite/blob/main/EULA.txt).
Its appearance here demonstrates Gatto’s image tools and does not imply endorsement.
Third-party interface artwork retains its owners’ rights; Gatto’s MIT license
does not relicense it. The original Scout drawing is covered by Gatto’s MIT
license; it does not reproduce an existing game character.

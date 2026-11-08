# Product demo

Revision, November 7, 2026: a 27.87-second, 1920 × 1080, 30 fps product film.
The source builds the complete project workflow.

There are no added titles, captions, subtitles, narration, or music. The feedback
is an actual text annotation typed in Gatto: “Stronger outline for / the active
color”, displayed on two lines without the slash. Other text belongs to the app
and source screenshot.

## Sequence and sources

The film creates **Palette review**, briefly opens the project's ellipsis menu,
adds the Scout screenshot, opens the attachment's ellipsis menu, and chooses
Annotate. In the editor it circles the active palette swatch, draws an arrow to
it, types the feedback, and saves. It returns to the same project with the
flattened annotation visible in the thumbnail, presses Upload, shows sending
progress and Uploaded, then presses Copy and holds Gatto's native “Copied to the
clipboard.” confirmation before the final pullback.

The native walkthrough renderer captures 24 sparse introductory states and nine
closing states at 2000 × 1520. The Create project dialog, incremental name entry,
and popup menus are real app UI. Creating the project and the final save outcome
are represented by in-memory fixtures, leaving personal projects unchanged.
Upload progress and completion use the production attachment model, and clipboard
confirmation uses the production success feedback. These fixture transitions
perform no network requests or system clipboard writes. Staging uses the
production attachment model. A small floating copy of the source
image illustrates the drag into the drop area; it is a presentation layer in
EffectCraft, rather than a desktop recording.

The editor capture produces 420 native frames at 30 fps. The film uses its first
348 frames, ending after Save. Circle and arrow previews use production pointer
handlers; arrow and text selection use Gatto's actual A and T shortcut handlers.
Feedback enters the native text input incrementally and is committed normally.
The production document raster produces the final image used by the project's
closing thumbnail. Pointer position, tool cursor, text bounds, and action events
are exported as metadata.

No desktop recording or visible app window is used. GPUI creates an invisible
off-screen window, although initialization needs macOS services and can require
execution outside an OS sandbox. Demo hooks are restricted to the
`walkthrough-screenshots` feature.

## Camera and sound

EffectCraft composes the app over a cream/lavender background with rounded corners
and a soft shadow. Camera and cursor transforms share the native 30 fps clock.
The project dialog and menu shots are framed for reading. During annotation the
camera stays close, reframing gently from circle to arrow to note. Keyboard tool
changes avoid returning to the toolbar. After Save, the camera follows the Upload
press, reframes the sending progress, and pans to the Uploaded row and Copy
confirmation. Separate vector pointer, crosshair, I-beam, and restrained press
rings clarify the action; the cursor fades out during the final pullback.

Original Foley uses deterministic noise and small resonances, without samples
or external audio dependencies. Drawing grain accompanies gestures, soft contacts
mark placement, and very small key sounds accompany typing. The master gain is
**10 dB below the first cut** (`10 ** (-10 / 20)`); individual event gains retain
headroom. There is no constant sound bed. These sounds are editorial additions,
not a claim that Gatto plays them. The mix and individual WAV cues stay editable.

## Rebuild and edit

### Repository source of truth

Version the generators and native capture fixtures, together with the existing
source artwork. Do not version the rendered MP4, generated `.ecproj`, numbered
PNG sequence, synthesized WAVs, or media archive. All those outputs live under
the root `dist/`, which Git already ignores.

The `.ecproj` is a JSON composition, not a self-contained media bundle. It
references `media/frames/`, SVG/PNG assets, and `media/audio/mix.wav`. Keeping that
file alone would lose its footage. The complete source recipe is:

- [Composition, camera, cursor, and export generator](../../scripts/build-product-demo.ts).
- [Original sound generator](../../scripts/generate-demo-audio.ts).
- [Editor gesture capture](../../src/walkthrough_screenshots/annotation_demo.rs)
  and [project workflow capture](../../src/walkthrough_screenshots/product_demo.rs),
  with their screenshot-only hooks in the app and editor.
- The already versioned [Scout interface capture](../images/walkthrough/source/canvas.png).

This source-only policy needs no Git LFS configuration or new Git attributes:
the generators are ordinary text files and PNG artwork already has a binary
attribute. The approved cut was built with EffectCraft 0.22.0. Rebuild from the
corresponding repository revision to retain that revision's UI and artwork;
later app or renderer versions can change the generated frames. If editing a
generated project interactively, transfer approved changes back into the
generator before discarding the output folder.

### Commands

Requires macOS, the repository's normal Rust dependencies, Bun, and EffectCraft
CLI at `~/.local/bin/effectcraft-cli` (or `EFFECTCRAFT_CLI`). The CLI uses the same
engine exposed by EffectCraft's MCP automation; no interactive editor is needed.

From the repository root:

```sh
bun build:demo
```

Output is the ignored `dist/product-demo/` directory:

- `gatto-product-demo.mp4` and editable `gatto-product-demo.ecproj`.
- `native/editor/` gesture frames, metadata, and flattened annotation image.
- `native/workflow/` project/dialog/menu states and their manifest.
- `media/frames/` the assembled 836-frame sequence, plus vector assets and audio.
- `chapters.json`, `camera.json`, `sound-timeline.json`, and engine command JSON.
- `previews/` representative diagnostic exports; these are not slides in the film.

Media references are relative to the project, so the complete output folder can
be moved together. Its composition is **Gatto — Palette review**. The script
checks for missing footage before rendering. Assembled frames use hard links to
native captures; tar archives preserve these shared files more efficiently than
ZIP when packaging the entire folder.

To adjust camera/audio without recapturing the native UI:

```sh
bun --no-env-file scripts/build-product-demo.ts dist/product-demo --reuse-captures
```

To capture the native inputs separately:

```sh
cargo run --offline --locked --features walkthrough-screenshots --bin walkthrough-screenshots -- dist/product-demo/native/editor --annotation-demo
cargo run --offline --locked --features walkthrough-screenshots --bin walkthrough-screenshots -- dist/product-demo/native/workflow --product-demo --demo-annotation dist/product-demo/native/editor/final-annotated.png
```

To recapture only the ending while keeping the existing native introduction:

```sh
cargo run --offline --locked --features walkthrough-screenshots --bin walkthrough-screenshots -- dist/product-demo/native/workflow --product-demo --product-demo-outro-only --demo-annotation dist/product-demo/native/editor/final-annotated.png
```

Use `--demo-intro <existing-workflow-directory>` when capturing that ending into
a different output folder. Then rebuild with `--reuse-captures`.

The audio generator accepts an optional timeline JSON with `duration`, `events`,
and `masterGain`. The builder supplies the full workflow timeline:

```sh
bun --no-env-file scripts/generate-demo-audio.ts dist/product-demo/media/audio dist/product-demo/sound-timeline.json
```

When changing geometry or timing, review the dialog, both menus, drag landing,
both live annotation gestures, incremental and committed text, Save, upload
progress, Uploaded, the Copy press and confirmation, and the closing project.
Verify frame count, rate, dimensions, duration, and stereo audio
with FFprobe. Keep the saved thumbnail synchronized with the actual flattened
annotation. Use the [walkthrough attribution](walkthrough-captures.md#licenses-and-attribution)
for the Scout artwork and captured third-party interface.

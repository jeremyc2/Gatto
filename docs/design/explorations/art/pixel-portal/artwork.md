# Pixel portal ring

Created October 26, 2026 as an Aseprite pixel-art adaptation of the original
transparent portal. The [earlier artwork record](../download-portal.md) retains
the original generation prompt and the website's independent reveal mechanics.
The original ring supplies the actual strand flow, glow and spark placement;
the pixel version keeps that wispy structure and narrow luminous edge.

## Master and geometry

[portal.aseprite](portal.aseprite) is the editable source: **384 × 384, one still
frame**, with three visible layers:

1. Ember glow: the partly transparent amber support around the main strands.
2. Luminous strands: connected bright filament and ember detail.
3. Sparks: detached streaks and small bright clusters.

The original transparent PNG was registered at native size with Aseprite's
nearest-neighbor scaling, quantized to twelve warm RGB colors, and cleaned up
with four nonzero alpha levels (64, 128, 192, 255). The palette runs from burnt
amber through orange, gold and pale cream. Low-opacity fringe noise and dark
isolated scraps were removed, while bright streaks and ember clusters remain.
The larger native grid preserves finer filaments than the cat without smooth
resampling or an unrestricted color gradient.

Keep the original proportions and translucent spaces between strands. Do not
replace these with a uniformly filled orange band: the broad procedural trial
was too heavy and was rejected. The open strand structure and stepped glow meet
the existing dark center without the gap exposed by the earlier thin outline.
The central aperture and full exterior are transparent. No cat, dark center,
paper background or animation is baked into this asset.

`lint_sprite` intentionally reports the detached spark clusters. Inspect their
coordinates and continuity rather than deleting them as stray anatomy. The
complete drawing has no isolated exterior single pixels; tiny hot pixels within
connected spark clusters and filaments are deliberate. Many single-color
orphan pixels are intentional ember texture, not detached stray pixels.

## Export and use

Use Aseprite MCP `export_frame`, frame 1, scale 2, to write
`web/assets/images/download-portal-pixel.png` (768 × 768). Pass absolute paths
to MCP. The automated exporter reads the saved master and uses Bun to encode
the lossless WebP; run it from the repository root:

```sh
bun --no-env-file scripts/export-artwork.ts
```

`PortalCat.tsx` imports the WebP into the existing SVG reveal mask, with
`image-rendering: pixelated`. The separate CSS radial dark center remains
behind it. The existing 240-second rotation and scroll-driven reveal still
operate independently of the [website cat](../portal-cat/artwork.md).

Keep the current source and exports; the superseded painted `download-portal`
PNG/WebP are removed. After edits, inspect at native size and in the actual
website composition, check alpha at the center and exterior, and compare the
lossless WebP pixels with the PNG. Confirm that the ring doesn't clip, the
dark center remains separate, and the cat can pass in front of the rim.

# Repeating courtyard paper

October 31, 2026. [portal-paper.aseprite](portal-paper.aseprite) replaces the
direct-generated portal texture with an Aseprite-authored tile. The former tile
was a visual reference only; no raster pixels were imported.

The 256 × 256 RGB still has two editable layers: warm courtyard plaster and
fine pigment grain. Six visible tan colors form quiet material variation.
The palette follows the courtyard film's plaster: sampled wall pixels at
(550,150) and (620,180) in frame 1 are `#d8b890` and `#f0d8a0`. The new tile
uses related tan ramps with softer contrast; it matches the material family
rather than copying an exact patch of wall and its directional lighting.

Periodic multiscale pigment and wrapped short grain clusters were authored
directly in Aseprite. The construction scripts were removed after completion
at the user's request; the saved master remains the editable source.
Noise lattices wrap in both axes, and every grain stroke wraps modulo 256.
Opposite edges are adjacent samples of one continuous toroidal field; there is
no mirrored border, edge fade, directional seam or fixed large composition.

Routine exports read the saved master. MCP `export_frame`, frame 1, scale 2
writes the 512 × 512
[`portal-watercolor-paper.png`](../../../../web/assets/images/portal-watercolor-paper.png).
`bun --no-env-file scripts/export-artwork.ts` refreshes that PNG and encodes its
lossless WebP with no resizing or filtering.

The [design system](../../../../web/DESIGN-SYSTEM.md) owns the existing fixed
512 × 512 CSS-pixel repeat and torn-paper masks. No layout change is necessary;
the pattern repeats across arbitrary viewport sizes without stretching.
Inspect a 3 × 3 repeat and an offset view across both joins after edits, as well
as `view_sprite`, color statistics and lint. Pigment-grain color orphans are
intentional; alpha must remain fully opaque throughout the tile.

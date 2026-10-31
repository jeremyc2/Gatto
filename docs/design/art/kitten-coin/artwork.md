# Baby-kitten gold coin

October 31, 2026. [kitten-coin.aseprite](kitten-coin.aseprite) is an original
Aseprite redraw of the previous generated coin, used only as a visual reference.
No generated pixels were imported. The kitten remains a baby: oversized head,
large round eyes with catchlights, short muzzle, small torso and tiny paws.

The RGB 64 × 64 still has 11 gold/bronze colors, transparent exterior and six
editable layers: milled rim, recessed field, engraved sprigs, body/paws, head/ears
and expression. Relief light comes from upper left; the lower-right bevel is
amber. Dark eye recesses, gold irises and cream speculars keep the face readable.
The composite is one connected silhouette, with no detached or lone pixels.
Small milled-rim accents and eye reflections are intentional color orphans.

The construction scripts were removed after the redraw at the user's request.
The saved Aseprite master remains the editable source; routine exports read it
directly and preserve manual edits.

Export frame 1 at 2× with MCP `export_frame` to
[`packaging/kitten-coin.png`](../../../../packaging/kitten-coin.png), 128 × 128.
`bun --no-env-file scripts/export-artwork.ts` includes the same export.
[Packaging notes](../../../../packaging/kitten-coin.md) record runtime filtering.
Check transparency, upper-left relief, the baby face and tiny paws at the game's
28-point display size and an enlarged integer preview. Keep review boards
outside the repository. Use `view_sprite`, color statistics and lint after edits.

# Delivered artwork provenance

Audited October 31, 2026 from consumed source paths, artwork records and export
scripts. Generated visual references are allowed. A saved Aseprite container by
itself does not prove that imported/generated pixels were redrawn.

| Delivered artwork | Current source and provenance |
| --- | --- |
| Installer background, anime cat, jetpack, ribbon and marks | [Aseprite redraw](installer/artwork.md), authored from scratch using the former generated composition as reference. |
| Repeating portal paper | [Aseprite procedural pixel tile](portal-paper/artwork.md), no generated raster inputs. |
| Rooftops baby-kitten gold coin | [Aseprite redraw](kitten-coin/artwork.md), former generated coin used as reference only. |
| App header cat and website flying visitor | [App master](../../design/explorations/art/app-cat/artwork.md) and [visitor master](../../design/explorations/art/portal-cat/artwork.md), Aseprite drawings developed from the approved Pocket Kitten concept. |
| Website portal ring | [Aseprite ring](../../design/explorations/art/pixel-portal/artwork.md); former generated ring is retired. |
| README cat, favicon, Dock and menu icons | [Aseprite masters](../../design/explorations/art/brand-cat/artwork.md). Dock background/bevel comes from Apple's Icon Composer; Finder supplies the Applications shortcut. |
| Screenshot gardener cats | [Scout masters](screenshot-cat/ARTWORK.md), drawn in Aseprite; actual editor captures feed the walkthrough and feature images. |
| Courtyard film and its three stills | [Journey master](../../design/explorations/art/scroll-story/journey.md). The record says generated drawing studies were registered, recolored, repaired and animated in Aseprite. This is substantially edited source art, not an untouched generated export, but it is not documented as a complete redraw of every generated pixel. |
| Torn-paper edges and application/game interface shapes | Authored SVG/CSS or code drawing, rather than image-generation outputs. |

The three direct-generated production raster assets found by this audit were the
installer, paper tile and coin; all three are replaced above. The courtyard's
remaining generated-study lineage is explicit rather than inferred away from
its `.aseprite` extension. The retired exploratory paintings, contact sheets, prototype pages and
intermediate scene assets were removed October 31, 2026. All Aseprite masters,
including the courtyard's embedded model references, remain intact.

Exporter: `scripts/export-artwork.ts` reads saved masters directly. Normal
refreshes preserve manual Aseprite edits. The temporary artwork construction
scripts were removed at the user’s request.
No app behavior, marketing copy or published release-size figures change here.

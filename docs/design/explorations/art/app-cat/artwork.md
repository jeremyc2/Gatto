# App header cat

Created October 26, 2026. This is a separate dark-background variant of the
[website rocket cat](../portal-cat/artwork.md), preserving its three-quarter
flight, cyan/violet eyes, rounded paws, back-mounted rockets and hot exhaust.
The native app's 72-point header mark uses deep purple fur and muted lavender
lit planes against the `#0e1017` background and purple controls.

## Master and palette

[app-cat.aseprite](app-cat.aseprite) is the editable source: **256 × 256, one
still frame**, with the same nine named layers as the website visitor. The
masters are independent; edits and exports do not update each other.

The fur keeps its upper-left light and surface shading, with these five
value-separated purple/lavender steps. The coat is deliberately richer and
less gray than the first app variant. The five fur tones were darkened by one
additional exposure stop on October 26 (linear RGB × 0.5), keeping their hue and
surface separation:

| Role | Color |
| --- | --- |
| Outline and deepest seams | `#170b24` |
| Shadow | `#231430` |
| Coat | `#361e4a` |
| Lit planes | `#503068` |
| Upper-left highlight | `#74538a` |

Keep outlines darker than adjacent fills. Preserve the eyes, lavender ears,
metal barrels, cyan bands and white/yellow flame cores. Avoid a uniform glow
around the silhouette; light and shadow should describe the same surfaces as
the website cat. Keep the background fully transparent.

## Export and use

Use Aseprite MCP `export_frame`, frame 1, scale 2, to write
[`packaging/app-cat-pixel.png`](../../../../../packaging/app-cat-pixel.png)
(512 × 512). Pass absolute paths to MCP. `src/app.rs` embeds that PNG for
`header_mark`; its existing 72-point image geometry and click target remain.
The original rocket-cat asset still supplies historical artwork references and
test fixtures; the Dock and menu bar have their own assets.

Inspect the source with `view_sprite`, `get_color_stats` and `lint_sprite` after
edits. Small eye highlights and material-edge pixels are intentional color
orphans; detached anatomy and stray exterior pixels are not. Verify the mark
at its actual size through the native renderer:

```sh
SEED=0 cargo run --locked --quiet --features walkthrough-screenshots --bin walkthrough-screenshots -- /tmp/gatto-app-cat-captures
```

Inspect `01-first-run.png` against the real app background. Regenerate the
published app captures with `scripts/update-walkthrough-screenshots.sh` when
changing this header. Keep temporary proof images outside the repository.

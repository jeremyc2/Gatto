# App header cat

Created October 26, 2026; redesigned October 27 with the user-approved Pocket
Kitten concept. The app's 72-point header mark shares the website visitor's
rounded head, black-purple coat, cyan/violet eyes, tucked paws, lavender rim and
mostly hidden twin thrusters.

## Master and shared design

[app-cat.aseprite](app-cat.aseprite) is the transparent **256 × 256** RGB master:
one still frame, a 56-color palette and nine named layers matching the
[website master](../portal-cat/artwork.md). The masters are independent; edits
and exports do not automatically update each other. Keep them synchronized
unless a distinct app variation is requested.

The website artwork record owns the shared anatomy, palette, lighting, layer
order, hidden barrel construction and flame direction. Keep the approved eye
pixels and knee shading, fur-covered paw tops, broad wrists and ankles, a clean
head-to-neck join, continuous tail and uniform lavender silhouette rim. The
coat stays dark against the app's `#0e1017` background, with no near-white fur
highlights. The Dock/menu-bar assets are separate artwork. Image tests and
comparison diagnostics reuse the current PNG; the retired raster cat is removed.

## Export and verification

Use Aseprite MCP `export_frame`, frame 1, scale 2, with absolute paths to write
[`packaging/app-cat-pixel.png`](../../../../../packaging/app-cat-pixel.png)
at **512 × 512**. `src/app.rs` embeds this PNG for `header_mark`; its existing
72-point geometry and click target remain.

Inspect `view_sprite`, `get_color_stats` and `lint_sprite` after edits, following
the shared artwork record. Check the mark against the actual app background:

```sh
SEED=0 cargo run --locked --quiet --features walkthrough-screenshots --bin walkthrough-screenshots -- /tmp/gatto-app-cat-captures
```

Inspect `01-first-run.png`. Regenerate published app captures with
`scripts/update-walkthrough-screenshots.sh`, then refresh their website exports
with `bun export:features`. Keep temporary previews outside the repository.

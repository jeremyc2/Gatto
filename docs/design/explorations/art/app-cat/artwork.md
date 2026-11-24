# App header cat

Created October 26, 2026; redesigned October 27 with the user-approved Pocket
Kitten concept; regenerated November 22 with the website visitor's model. The
app's 72-point header mark is the visitor's three-quarter profile: rounded head,
black-purple coat, cyan/violet eyes, reaching forelegs, trailing hind legs and
tail, dark outline and twin thrusters.

## Master and shared design

[app-cat.aseprite](app-cat.aseprite) is the transparent **256 × 256** RGB master:
one still frame with `Harness and thrusters`, `Cat` and `Exhaust` layers.
`bun render:portal-cat` writes it together with the
[website masters](../portal-cat/artwork.md). That record owns the anatomy,
palette, lighting and equipment. Keep the coat dark against the app's `#0e1017`
background. The Dock and menu-bar assets are separate artwork.

## Export and verification

`bun --no-env-file scripts/export-artwork.ts` exports frame 1 at scale 2 to
[`packaging/app-cat-pixel.png`](../../../../../packaging/app-cat-pixel.png)
at **512 × 512**. `src/app.rs` embeds this PNG for `header_mark`; its existing
72-point geometry and click target remain.

Check the mark against the actual app background:

```sh
SEED=0 cargo run --locked --quiet --features walkthrough-screenshots --bin walkthrough-screenshots -- /tmp/gatto-app-cat-captures
```

Inspect `01-first-run.png`. Regenerate published app captures with
`scripts/update-walkthrough-screenshots.sh`, then refresh their website exports
with `bun export:features`. Keep temporary previews outside the repository.

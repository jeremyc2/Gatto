---
status: superseded
date: 2026-10-17
---

# Use the luminous black cat for the website favicon

Superseded October 25, 2026 at the user's request: the faceted SVG treatment
did not meet the desired character quality. A seated pixel-art kitten now
connects the README and favicon; the flying website visitor remains separate.
See the [current artwork record](../design/explorations/art/brand-cat/artwork.md)
for sources and exports. The original decision and commands below are historical.

The favicon must remain recognizable at 16 pixels while belonging to the same
world as the website illustrations. Use a simplified SVG of the luminous black
cat, retaining its cyan and violet eyes, uneven ears, cheek tufts, and violet coat
highlight against warm paper. The user approved this character treatment.

A cream cat on green conflicted with Gatto's exclusively black-cat identity.
The next symmetric black face with narrow amber eyes felt generic and evoked
Halloween. Preserve the approved curious expression and distinctive character
features; the detailed painted fur is intentionally omitted for small-size clarity.

## Original asset maintenance

The source was `web/public/favicon.svg` (removed by the superseding change). Vite copies
`web/public/` to the site root. The document declares the SVG, a 96 px PNG, an ICO
containing 16, 32 and 48 px sizes, and a 180 px Apple touch icon. Keep these exports
in sync when editing the SVG. The Apple icon uses an opaque background so the
device can apply its own corner mask. With ImageMagick installed, regenerate the
exports from the repository root:

```sh
magick -background none -density 768 web/public/favicon.svg -resize 96x96 -depth 8 -strip web/public/favicon-96.png
magick -background '#f7f3e9' -density 768 web/public/favicon.svg -resize 180x180 -alpha remove -depth 8 -strip web/public/apple-touch-icon.png
magick -background none -density 768 web/public/favicon.svg -define icon:auto-resize=48,32,16 web/public/favicon.ico
```

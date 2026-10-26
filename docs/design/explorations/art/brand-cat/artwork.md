# README cat and favicon

Created October 25, 2026; whiskers and lighting refined October 26. A seated
charcoal-black kitten connects the README illustration
and website favicon. Cyan and violet eyes, lavender ears, a round face and small
paws carry Gatto's identity. The README adds a compact seated body and curled
tail. The favicon keeps the face against warm paper. Neither includes a portal
or jetpack; the [flying visitor](../portal-cat/artwork.md) is separate artwork.

All three masters were drawn directly with the Aseprite MCP pixel tools. They
are single-frame stills with transparent exterior pixels and separate editable
layers. A generated combined cat/portal study was rejected and is not used in
any delivered asset. No third-party logo was used as a drawing reference.

## Sources

| Master | Native size | Layers |
| --- | --- | --- |
| [brand-cat.aseprite](brand-cat.aseprite) | 64 × 64 | Cat silhouette; coat and ears; eyes and expression |
| [favicon-32.aseprite](favicon-32.aseprite) | 32 × 32 | Warm paper; cat portrait |
| [favicon-16.aseprite](favicon-16.aseprite) | 16 × 16 | Warm paper; cat portrait |

Keep the 16 px portrait as an optical redraw. Its eyes, ears and nose use
deliberate small clusters; reducing the 32 px picture would move those features
between pixels. Preserve the cyan/violet eye order across all three drawings.
Paws are rounded feline shapes, with no protruding finger shapes. Light comes
from the upper left: the forehead and left cheek receive the broad highlight,
with a darker right cheek, a contact shadow beneath the head, and small lit
planes on the paws and tail. Keep the charcoal/lavender coat ramp consistent;
avoid a centered forehead spotlight or a bright lower rim.

The README has two neutral warm-gray whiskers per cheek, rooted beside the
muzzle below both irises. Keep a visible gap from the eye colors and keep the
lower right tips clear of the tail. Avoid lavender strokes extending from eye
height: those looked like iris pixels bleeding out of the face.

The favicon omits whiskers. Shorter muzzle-rooted gray alternatives were tried
at native 16/32 px sizes, but they still read as extra dashes. Preserve the clean
portrait across PNG, ICO and Apple touch exports. Review images belong outside
the repository; keep only the masters and consumed exports here.

The warm-paper backing keeps the small dark face visible on light and dark
browser tabs. The favicon is supplied as native raster sizes; the rejected
SVG is no longer declared or shipped. Browser selection uses the declared
icon sizes ([MDN link metadata](https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Attributes/rel#icon)).

## Exports

Use Aseprite MCP `export_frame`, frame 1, with these integer scale factors.
Resolve paths against the repository root and pass absolute paths to MCP.

| Master | Export | Scale |
| --- | --- | ---: |
| `brand-cat.aseprite` | `docs/images/readme-cat.png` | 4 |
| `favicon-16.aseprite` | `web/public/favicon-16.png` | 1 |
| `favicon-32.aseprite` | `web/public/favicon-32.png` | 1 |
| `favicon-32.aseprite` | `web/public/favicon-96.png` | 3 |
| `favicon-32.aseprite` | `/tmp/gatto-favicon-64.png` | 2 |
| `favicon-32.aseprite` | `/tmp/gatto-apple-160.png` | 5 |

Then assemble the ICO and opaque Apple touch canvas with ImageMagick:

```sh
magick web/public/favicon-16.png web/public/favicon-32.png /tmp/gatto-favicon-64.png -strip web/public/favicon.ico
magick /tmp/gatto-apple-160.png -background '#f7f3e9' -alpha remove -alpha off -gravity center -extent 180x180 -depth 8 -strip web/public/apple-touch-icon.png
magick web/public/favicon-96.png -filter point -resize 480x480 -background '#f7f3e9' -alpha remove -gravity center -extent 1200x630 -depth 8 -strip PNG24:web/public/social-image-pixel.png
```

The README displays its 256 px export at 256 CSS pixels, preserving a 4× native
grid. The ICO contains 16, 32 and 64 px entries. The Apple touch export centers
160 px of artwork in a 180 px opaque canvas, leaving masking to the device.
The social preview centers a crisp 480 px enlargement of the favicon on a
1200 × 630 warm-paper canvas.
Keep declarations in `web/src/Document.tsx` synchronized with those files.

Inspect native 16/32 px versions on light and dark tab backgrounds, their integer
enlargements, and the README at its declared size. Use `view_sprite`,
`get_color_stats` and `lint_sprite` after edits; eye catchlights and small
iris accents are intentional single-color details, while disconnected exterior
pixels are not. Run the website build after changing public exports.

# Flying website visitor

Created October 25, 2026; redesigned October 27 from the user-approved Pocket
Kitten concept. The visitor is a compact black-purple kitten with cyan/violet
eyes, tucked paws and two mostly obscured back-mounted thrusters.

The [app header cat](../app-cat/artwork.md) uses the same drawing in an independent
master. The seated brand cat, favicon, Dock icon and menu bar icon are separate
artwork. Superseded concept images, flame references and review previews were
removed after the design was captured in the editable sprites.

## Master and layers

[portal-cat.aseprite](portal-cat.aseprite) is the transparent **256 × 256** RGB
master: one still frame, a 56-color palette and nine visible layers, bottom to top:

1. Exhaust flames.
2. Tail.
3. Far rocket.
4. Near rocket.
5. Body and hind legs.
6. Mounts and harness.
7. Forelegs and paws.
8. Head and ears.
9. Eyes muzzle and whiskers.

The source contains the cat, equipment and exhaust. The [portal ring](../pixel-portal/artwork.md),
dark aperture, reveal masks and website motion remain independent.

## Cat and lighting

Preserve the oversized rounded head, lavender inner ears, small closed smile,
cyan-blue viewer-left eye and violet-purple viewer-right eye. Keep the approved
eye pixels intact when editing other parts.

The coat reads as black with rich purple shading. Its main value ramp is
`#1c0d29`, `#251133`, `#30133f`, `#3b184e`, `#47205e`, `#54296e`,
`#5b3077`, `#60357e` and `#6a3c87`. Broad diffuse light comes from above-left;
local purple shadows and small fur clusters separate the limbs. Avoid ring
bands, faceted planes, mottled spots, black hairlines and near-white fur patches.

Forelegs sweep gently into broad wrists. All four paws show fur-covered tops
with short plum toe divisions, without exposed undersides or pads. Preserve
the knee shapes and shading; keep hind ankles substantial through the softer
hock bend, with an open gap between the forward-facing feet. The tapered torso
has no separate back bulge, belly lobe or underside flap. Complete anatomy
occludes the equipment; never cut gaps into a leg or paw to accommodate a nozzle.

The neck joins the rounded lower jaw without a transparent seam, hooked chin
ledge or bright internal outline. Keep torso fragments out of the head layer.
A continuous one-native-pixel `#ce96e1` rim frames the exposed body and entire
tail. Keep it uniformly lavender, without blue or orange flame reflections.

The hooked tail is one continuous curve, about 7–14 native pixels thick,
tapering toward its tip. Its solid root passes behind the rump and rocket
assembly. Fur strokes follow its length without crosswise bands or slices.
Inspect it alone and remove stray tail fragments from other layers after edits.

## Equipment and exhaust

Exactly two straight, parallel silver-lavender barrels with cyan bands sit on
the back. A partial near barrel and two visible lavender shoulder straps with
a small light buckle establish the attachment. Keep the straps smooth and
behind the forelegs. The far barrel and nozzle are entirely hidden; its flame
communicates the second engine. Keep both engines close together.

Both rocket layers contain complete cylinders beneath the cat. Their caps,
collars and nozzles share continuous centerlines. The near axis runs roughly
from `(113,106)` to `(82,154)`; the far axis runs from `(136,126)` to `(105,173)`.
Both point down-left along `(-2,+3)`, with lengths about 57 native pixels and
near/far diameters about 16/14 pixels.

Each flame flares rapidly from a narrow nozzle along the barrel's straight
axis, with a broad blue/cyan exterior, violet detail, orange and yellow layers,
and a white hot core. Preserve the approved wide teardrop shapes, connected
spiky fringe and pointed trailing tips. Near/far lengths are about 96/86 native
pixels. Keep the flames continuous, without sideways waves, floating sparks,
transparent pinholes or a black gap between the visible nozzle and hot core.

## Export and verification

Use Aseprite MCP `export_frame`, frame 1, scale 2, with absolute paths to write
[`web/assets/images/download-rocket-cat-pixel.png`](../../../../../web/assets/images/download-rocket-cat-pixel.png)
at **512 × 512**. The automated exporter reads the saved master and uses Bun to
encode its lossless WebP; run it from the repository root:

```sh
bun --no-env-file scripts/export-artwork.ts
```

`PortalCat.tsx` imports the lossless WebP at intrinsic size 512 × 512 with
`image-rendering: pixelated`. Keep runtime exports transparent. Generate review
backgrounds and enlarged close-ups outside the repository as needed.

After edits, inspect `view_sprite`, `get_color_stats` and `lint_sprite`. The
current composite has binary alpha, one connected silhouette and no detached
parts or lone exterior pixels. Small catchlights, fur transitions and flame tips
can be intentional color orphans. Check hidden rocket continuity and compare
exports against the source at exact integer scale. Inspect the cat against black,
the app's `#0e1017` background and warm paper, including the 72-point app mark
and website visitor. Export both independent masters for shared design changes.

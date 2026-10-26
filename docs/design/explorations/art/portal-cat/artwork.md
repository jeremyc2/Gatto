# Flying website visitor

Created October 25, 2026; redrawn October 26 at the user's request. The website
visitor is a separate pixel-art flying cat, drawn directly with Aseprite MCP.
The earlier [angled rocket cat](../../../../../packaging/rocket-cat-transparent.png)
guides the flight direction and equipment placement. Its charcoal coat,
lavender ears, and cyan/violet eyes match the
[seated README cat and favicon](../brand-cat/artwork.md), while the pose and
equipment remain specific to this separate asset. The
[app header variant](../app-cat/artwork.md) preserves this pose in its own master
with lighter indigo/lavender fur for the native dark interface.

The first front-facing pixel drawing was rejected: the round closed engine caps
looked like objects held in the paws. The current three-quarter view exposes
long rocket barrels mounted on the back, recognizable nozzles, and visible
exhaust. The user specifically requested bright white/yellow cores within the
colored flames.

## Master and anatomy

[portal-cat.aseprite](portal-cat.aseprite) is the editable source: **256 × 256,
one still frame**, with nine visible layers in depth order:

1. Exhaust flames.
2. Tail.
3. Far rocket.
4. Body and hind legs, including the harness and buckle.
5. Near rocket.
6. Forelegs and paws.
7. Head and ears.
8. Mounts and harness.
9. Eyes, muzzle and whiskers.

The round charcoal kitten flies toward the upper right in a three-quarter view,
with both eyes visible. It has lavender inner ears and silver-lavender rocket
pods with cyan bands. Its attached forelegs reach forward while the hind legs
trail behind the body. Paws are
continuous rounded feline shapes; short underside notches imply toes without
separate finger-like protrusions. Keep four paws total and one attached tail.

The rockets are long cylinders mounted behind the shoulders with visible
hardware, cyan bands, segmented collars and dark exhaust openings. Their thrust
points backward along the barrel axes. Keep both flames clear of the hind legs;
do not place the legs across the exhaust path. The white/yellow hot core begins
at each nozzle and tapers inside orange, violet and cyan clusters. Keep the
harness on the torso and the forelegs in front of it. Light comes from the upper
left; avoid bright lower fur rims and pillow shading.

Only the cat, equipment and its exhaust are in this transparent asset. The
separate [pixel portal](../pixel-portal/artwork.md), dark aperture, masks and
page seams remain independent.
The master has no portal, background, floor shadow, or baked animation. The
exhaust has its own editable layer. The named layers leave room for
later animation work without choosing an animation technique now.

## Export

Use Aseprite MCP `export_frame` with frame 1 and scale 2 to write
`web/assets/images/download-rocket-cat-pixel.png` (512 × 512). Pass absolute
paths to MCP. Then, from the repository root with libwebp installed:

```sh
cwebp -quiet -lossless -z 9 web/assets/images/download-rocket-cat-pixel.png -o web/assets/images/download-rocket-cat-pixel.webp
```

`PortalCat.tsx` imports the WebP with intrinsic dimensions 512 × 512 and
`image-rendering: pixelated`. The existing CSS controls the independent forward
approach and floating motion; the portal keeps its existing rendering and motion.
The superseded side-view and forward-facing raster exports were removed during
asset cleanup; retain the current editable master and its PNG/WebP exports.

After edits, inspect `view_sprite`, palette statistics and `lint_sprite`, then
check the exported PNG and the visitor at its actual website size. Catchlights,
small edge transitions and toe notches can be intentional color-orphan pixels;
detached anatomy or stray exterior pixels need correction. Check the front-paw
silhouette and equipment direction before adding detail.

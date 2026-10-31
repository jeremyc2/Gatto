# Installer anime scene

October 31, 2026. Redrawn with Aseprite MCP, authored contours and pixel scripts.
The former generated illustration and the user's mood board are visual
references. No reference raster is imported into the delivered drawing.

## Master and construction

[installer.aseprite](installer.aseprite) is the **840 × 520 RGB master**, one
still frame, **16 editable layers** and **106 visible colors**. The PNG remains
840 × 520, smaller than the former 1554 × 1012 generated image. The Finder
window, native icons and typeset instruction retain their existing layout.

The scene was authored with Aseprite MCP using contours, projected pixel glyphs
and quantized material lighting. Contours rasterized on the native grid without
antialiasing; no reference bitmap was imported or sampled into the drawing.
The construction scripts were removed after the redraw at the user's request.
The saved Aseprite master is the editable source; routine exports read it
directly and preserve manual edits.

Layers, bottom to top:

1. Midnight air.
2. Exhaust shared turbulent volume.
3. Exhaust overlapping blue cooling folds.
4. Exhaust aperture cores and flowing accents.
5. Broad arched tail in perspective.
6. Far legs receding behind the body.
7. Ribcage rump and near hind leg together.
8. Pack saddle harness and two parallel barrels.
9. Near shoulder forearm and turned paw.
10. Continuous neck and shoulder coat.
11. Tall ears skull cheek and muzzle planes.
12. Angled anime eyes nose and quiet smile.
13. Foreshortened sole pads along the paw planes.
14. Arrow sweeping toward Applications.
15. Quiet orbit lines and glints.
16. Reflected light and small fur planes.

## Cat, expression and perspective

The original's body drawing is the reference for the full arched tail, tall
near ear, broad cheek, small muzzle and gentle closed smile. The upper thigh
flows into the rump. Its joint is partly hidden by the haunch, rather than
outlined as an isolated oval muscle or a deeply pinched hinge. Compact rear
feet trail behind; far limbs sit lower and recede behind the near limbs.

The near foreleg starts in a full shoulder, tapers through the forearm and
continues into a turned paw. The far forepaw reaches farther right and lower.
Both soles follow those limb planes. A shared pixel pattern supplies four toe
pads and one broad main pad, rotated toward the direction of reach and
foreshortened into the sole. The farther pattern is smaller and more compressed.
The distal paw contour continues the forearm without a raised knuckle bulge;
its fur fills are constrained to that silhouette. This replaces the earlier
front-facing circular paw treatment. Visible beans
must fit the toe contours; projection arithmetic does not replace visual review.

The coat uses the visitor's black-purple materials with restrained blue
reflected light. The near eye is cyan and the far eye violet, using the existing
visitor palette. Eye positions, inclined lids, a narrower far eye and unequal
muzzle planes express the three-quarter view. The smile curves share a consistent baseline around the muzzle. The far ear
base remains an unbroken coat plane, with the forehead shadow beginning below
it. A continuous neck plane replaces the ink-dark shoulder seam. Ear and cheek
fur stay in connected groups. References and construction caveats are recorded in the
[anime-cat study](../../../research/anime-cat-installer-2026-10-31.md) and
[spatial construction study](../../../research/installer-spatial-construction-2026-10-31.md).

## Exhaust and hardware

The new flame direction combines the approved cat's white/yellow/orange heat
and cyan/violet edges with an oversized shared flowing volume. Its authored
trailing contour has cooling folds and torn tips. A quantized, flow-warped
light field supplies internal warm volumes; it is an illustration technique,
not a physical fluid simulation. The former smooth blue pointed envelope is
superseded. The tail naturally overlaps the downstream plume.

The close silver-lavender barrels retain matching cyan sleeves. Their shared
axis is `(0.991, 0.132)` in composition coordinates. Rear-face centers are
`(221,66)` near and `(224,60)` far, or `(442,132)` and `(448,120)` on the native
grid. The far barrel is drawn first, then partly concealed by the near barrel
and the head. Supports meet a common saddle and a strap around the ribcage.

The near flame exits at approximately `(436.05,131.21)` native pixels; the far
one at `(442.05,119.21)`. Both aperture cores follow the barrel axis and broaden
into the shared volume. The upper blue-violet cooling edge continues across the small emission
bridges to the metal lips, matching the lower edge. The belly tuft uses coat
purple rather than blue, and the far ear has one curved, continuous root.
The outer plume retains its full contour; it is not
cropped with a local nozzle mask. That former mask removed part of the outer
flame and is superseded.

Each barrel's full inner opening is illuminated in cream/white. A short white
bridge extends from inside that opening across its left edge, then the right
metal lip overlaps it. The near barrel is drawn after the far barrel, hiding
part of the far equipment and stream. There is no unlit inner sliver or closed
metal rim across the escaping core. The streams mix downstream without angling
apart. Inspect this connection in the composite, not just the background flame.

Review the exhaust alone, then with equipment and fur. Inspect aperture depth,
initial direction, the merge, and natural tail occlusion. A sampled centerline
check finds no fur crossing the first 49 native pixels behind either rear face.
That check cannot establish the visual quality of the flame or the anatomy.

## Export and visual review

Use MCP `export_frame`, frame 1, **scale 1** for
[`packaging/dmg-artwork.png`](../../../../packaging/dmg-artwork.png).
`bun --no-env-file scripts/export-artwork.ts` reads the same saved master.
The [packaging guide](../../../../packaging/dmg-artwork.md) owns native icons,
text, Finder label handling and the 840 × 520-point window.

Compare the full installer with the original for body thickness, continuous
joints, ear and muzzle shapes, paw direction and depth. Also inspect the face,
soles and nozzle outlets at an integer enlargement. Review boards are temporary
artifacts outside the repository. The refreshed
[installer preview](../../../images/dmg-install.png) shows the delivered layout.

Use `view_sprite`, color statistics and lint after edits. Separated pad clusters,
fur reflections and metal highlights can be intentional on their own layers;
inspect the complete cat and flame masks before treating them as detached art.
These checks do not establish that the illustration is emotionally successful.

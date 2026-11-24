# Flying website visitor

Created October 25, 2026; redesigned October 27 from the user-approved Pocket
Kitten concept; rebuilt November 22 from a single model so every angle keeps the
same cat. The visitor is a compact black-purple kitten with cyan/violet eyes,
flying in a stretched "Superman" pose with two back-mounted thrusters. It flies
straight out of the portal toward the viewer. It starts at a front
three-quarter angle, face to the viewer, turned just enough that both barrels
and plumes show past the big head. Its turn banks through a three-quarter
profile to a rear view.

The [app header cat](../app-cat/artwork.md) uses the three-quarter profile. The seated
brand cat, favicon, Dock icon and menu bar icon are separate artwork.

## Source of truth

[`scripts/render-portal-cat.ts`](../../../../../scripts/render-portal-cat.ts) holds
one rigid model of the cat, harness, thrusters and exhaust. It renders native
pixel art in the approved palette and writes the Aseprite masters. Change the
model, then regenerate; do not hand-edit cels, because the next render replaces
them. From the repository root:

```sh
bun render:portal-cat
bun --no-env-file scripts/export-artwork.ts
```

For iteration, `--preview <dir> <turns>` renders turn amounts (0 = outbound,
1 = rear) to PNGs, and `--debug <dir>` renders side, top, front and back views.
Set `PORTAL_CAT_PARTS=1` to colour each body part separately when checking how
limbs join the body.

Each master has three layers: `Harness and thrusters`, `Cat` and `Exhaust`. Their
pixels never overlap, so the composite equals the render.

- [portal-cat.aseprite](portal-cat.aseprite): **256 × 256**, `Thrust` tag, six
  75 ms cels of the front three-quarter pose with flickering exhaust.
- [portal-cat-turn.aseprite](portal-cat-turn.aseprite): **320 × 256**, `Turn` tag
  with `Bank away` (eighteen 48 ms cels, front three-quarter to profile to rear) and
  `Rear thrust` (six 75 ms cels). Bank timing matches `web/src/portal/flight.ts`.

The body origin sits at (154, 144) in the square tile and 32 pixels further right
in the turn tile, so the first turn cel lines up with the thrust loop. Every pose,
including the rear flame tips, stays inside its cel.

## Anatomy

Both sides use one mirrored limb definition, so legs never differ in length.
Limbs are chains of tapered segments smoothly blended into the torso, giving
soft joints and a thick thigh that narrows into the shin without a seam.

- **Head:** the neck stretches forward in flight, so the head leads the
  shoulders instead of sitting on top of them. Oversized and round with full cheeks and cheek tufts in the
  silhouette, lavender inner ears, a small pink nose and dark "w" mouth. The
  cyan eye is the cat's right eye (viewer-left when facing us); the violet eye
  is the left. Eyes are painted on the face surface, so they foreshorten and
  disappear around the cheek. They never appear on the back of the head.
  Three whiskers per side are drawn as depth-tested lines.
- **Forelegs:** reach forward from low on the chest, with the elbow as the lowest
  point and the forearms extended under the chin, paws close together, as in a
  stretch. From the front, the paws sit centred under the chin. The underside of each
  paw (facing the belly) shows a pad and three toe beans; two creases split the
  top into toes.
- **Hind legs:** trail straight back close together, thigh to hock to paw.
- **Tail:** straight back from the rump between the hind legs, without curling.

From the side the plume, tail and hind legs sit in three tiers; the legs never pass
through the exhaust. When the plumes face the viewer they cover the legs, and
only the paw tips show.

## Equipment and exhaust

Two identical, parallel silver-lavender barrels with cyan bands sit side by side
along the spine, raised on a backplate so they show over the shoulder,, nozzles toward the tail. Their size never
changes. A dark chest strap wraps the whole body behind the forelegs, under the
pack, with a metal buckle on the cat's right flank.

Each plume is a 3D teardrop along its barrel axis, starting at the nozzle. Its
layers run white, pale yellow, yellow, orange, deep orange, violet, blue and cyan,
with a ragged outer fringe. Seen end-on, the layers form concentric rings. The
flicker comes from loop-safe noise: six cels animate the fringe and core,
without changing the overall length. Plumes that point toward the viewer are
lengthened to keep their on-screen length.

## Lighting and style

Light comes from above-left in world space, so shading stays consistent as the
cat turns. The coat uses `#1c0d29`, `#251133`, `#30133f`, `#3b184e`, `#47205e`,
`#54296e`, `#5b3077`, `#60357e`, `#6a3c87` and `#87509e`. Fur clusters come from
object-space noise, so they stay attached to the cat. The head gets a soft front
fill to keep the face readable, and it does not shade the limbs, so paws under
the chin stay distinct. A two-pixel `#120a1c` outline sits just outside the
cat and harness, never on the exhaust. It crisps the silhouette on light
backgrounds and blends into dark ones. Only the whiskers use lavender. Where one part overlaps another, the farther pixel takes the darkest
coat or equipment shade, separating limbs without drawing lines across the torso.

## Verification

After a render, view the turn strip as a contact sheet. Check that limbs, the
strap, both barrels and both plumes keep their size and attachment in every cel,
and that the rear cels show the barrels side by side on the back. Inspect the
outbound frame at 4× against black, the app's `#0e1017` background and warm
paper. Check the portal in the browser from arrival through the fly-through.

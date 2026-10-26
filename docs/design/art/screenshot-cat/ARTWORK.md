# Scout, the screenshot cat

Original artwork created on 2026-10-25 for [issue #29](https://github.com/jeremyc2/Gatto/issues/29).
An orange cat adventurer with pointed ears, a cream muzzle, red scarf, cobalt
outfit and rising tail gives the walkthrough a recognizable retro game sprite.
The tail grows from the hip, sweeps upward with a tapered tip, and stays separate
from the raised paw so it reads as part of the cat rather than a held object.
He holds a shovel with a D-shaped grip, long wooden handle and broad steel
spade. The scarf hangs straight down his chest so it cannot be mistaken for a
held prop.
Both arms use the same bent silhouette, rotated 180 degrees for the raised arm,
with matching shoulder-to-paw length and thickness. The other hand grips the
shovel.
Large flat color clusters and a dark outline preserve the face and silhouette
when an Aseprite screenshot becomes a small Gatto attachment thumbnail.

## Source and export

- [scout.aseprite](scout.aseprite): authoritative editable 48 × 48 RGBA source,
  one static frame. Head, Body, Scarf, Tail, Shovel and Arms are separate layers;
  the default Layer 1 is empty. Both paws belong to Arms. No animation is intended.
- [scout.png](scout.png): native-size transparent PNG export of frame 1.
- Fifteen opaque colors; transparent background. Highlights come from the upper
  left, with orange shadows on the right and under the head.
- Aseprite MCP inspection reports one connected silhouette, 978 opaque pixels,
  and no detached pieces, lone pixels or orphan pixels.

Export frame 1 at 1× through Aseprite MCP’s `export_frame`. Use integer
nearest-neighbor scaling for enlarged previews. Captures use 300% and 200%
Aseprite zoom; [capture framing](../../walkthrough-captures.md) records the real
window crops and Gatto screenshot regeneration procedure.

The artwork is original and uses the repository’s MIT license. The retro
platform-game styling does not use a Nintendo character or third-party sprite.

## Black gardener comparison variant

Created on 2026-10-26 for the website and walkthrough comparison. The original
orange Scout remains unchanged. [scout-black.aseprite](scout-black.aseprite) is
a separate 48 × 48, one-frame source with the same editable layers, pose and
silhouette; [scout-black.png](scout-black.png) is its native transparent export.

Head, Tail and Arms use a continuous charcoal coat: `#434B5B` in the upper-left
light, `#282D39` for the main fur, `#191D27` in shadow and `#111520` for the outline.
The cream muzzle is removed rather than turned into another light patch. The
nose and mouth use `#F4FFFF` to stay legible against the dark face. Pink inner
ears and white eye highlights remain. Body, Scarf and Shovel are unchanged,
preserving the blue overalls, red scarf, cream shoes and wooden-handled tool.

The variant has 18 opaque colors, one connected 978-pixel silhouette and no
detached or lone pixels. The lint report's single color orphan at (32, 24) is
the preserved scarf outline where it meets the raised arm, not a loose pixel.
Export frame 1 at 1× with Aseprite MCP. The comparison uses real, matched 300%
editor-window captures of the two saved sources, as recorded in
[walkthrough captures](../../walkthrough-captures.md).

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

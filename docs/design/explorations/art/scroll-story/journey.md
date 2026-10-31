# Courtyard animation

October 25, 2026. One pixel-art courtyard story replaced the website’s
three static cat paintings. Image generation supplied drawing studies; Aseprite
was used to register, recolor, repair and animate them. The approved animation
and reusable drawing references now live in one file, without old versions or
procedural reconstruction scripts.

## Master and references

[journey.aseprite](journey.aseprite) is the authoritative editable source:
**1152 × 768, 131 frames, 12 fps on export, 10.916667 seconds**. Its nine visible
layers contain the complete approved scene. Character bodies and tails share a
layer, while the picture, mouth grip and front rim retain their own depth.
Linked cels hold the static courtyard, rim and foreground plants.

The hidden **Reference library — isolate one layer** group preserves 155 native
reference cels in 15 layers. These are current model drawings and components,
not alternate versions of the film. Their colors already match the scene’s coat
ramps, before character scaling.

| Reference layers                                                | Cel numbers in this document |
| --------------------------------------------------------------- | ---------------------------- |
| Paper walking / standing; pickup; seated                        | 1–6 and 9; 1–6; 1 and 3      |
| Watercolor walking / sniffing / standing; seated                | 1–9 and 12; 1–3              |
| Round walking / sniffing; seated                                | 1–4 and 6–8; 1–4             |
| Four tail banks: paper, watercolor, round seated, round walking | 1–28 each                    |
| Clean courtyard, GitHub box, front rim, flower picture          | 1 each                       |

Reference cel numbers identify model poses, independently of the story playing
at those frame numbers. Isolate one reference layer to inspect it. Layer user
data records native dimensions, mouth/ground landmarks, pelvic anchors and tail
profiles. Scene scale is 1.08× for paper, 0.76× for the kitten and 0.95× for
round. Preserve one scale per cat across poses; visible scene cels are already
scaled.

Edit the visible scene cels to change the film. Reference edits do not
recompose the scene automatically; copy or redraw the affected visible cels.
Keep the reference group hidden when saving for export. There is no generator
to rerun over manual corrections.

## Approved art and timing

The kitten has charcoal watercolor blooms, cream markings and teal eyes. The
tall cat has folded matte charcoal paper and amber eyes. The round cat has dense
velvety fur and violet/blue eyes. One warm upper-left light serves all three:
highlights follow the lit planes; folds and overhangs receive occlusion. Avoid
pillow shading, a dark border around every edge, lower rim lights and floor glow.
Cream kitten markings are pigment, not a second light.

The round cat’s violet iris has the blue iris’s saturated, luminous treatment,
with its dark pupil and small warm catchlights preserved. Its tail is tapered
from a 36-pixel native root through a roughly 30-pixel midshaft to a rounded
14-pixel tip, before the final tip taper and 0.95× scene scale. Both round-cat
tail banks and all 131 scene cels use this slimmer anatomy. The 230-pixel native
centerline bends continuously along its length; changing depth and asymmetric
gestures let it extend, rise and draw back without tracing a circular orbit.
The final bend flows into a smoothly tapered, rounded end. Keep the raised
tip aligned with that arc, without a sideways hook or a pinched terminal bend.
Broad depth sweeps curl it around the lower haunch, swing it toward the viewer,
recede behind the rump and lift it beside the back. Foreshortening and modest
perspective follow that travel. The returning arc can cross in front of the
haunch, with a small cast shadow on the fur; the higher rear arc stays behind
the body. Keep these turns broad in the projected drawing, including views
along the tail, so depth changes never produce a sharp folded silhouette.
The root follows the saved pelvic anchor for each body pose, with the rump
covering and blending into the attachment. Fur uses the body’s charcoal ramp.
Highlights and underside shade follow the local surface orientation under the
fixed light; the rump occludes the root, and lower arcs cast a faint floor
shadow that softens and weakens as the tail rises.
Its floor projection follows depth as well as height, moving across the paving
as the tail sweeps closer or farther away.
In the wrapped pose, the tail follows the lower haunch and paw line closely,
with its end tucked near the paw. Preserve length through the depth wrap;
leave no open loop beside the body. Keep its contact shadow close to the fur
and floor so the resting tail sits against the cat.

The round cat leads the choreography. Keep the tail quiet between body cues,
with one restrained lift instead of a succession of independent flourishes.
Bending begins near the root 1–2 frames after each cue; the tip follows up to
3 frames later. The pelvic attachment stays fixed to the body. Ease into and
out of each gesture without overshooting. The small
walking-stop sway and final tip response finish once, without repeating.

| Body cue            | Tail response                                        |
| ------------------- | ---------------------------------------------------- |
| Sniff begins at 23  | Small counterbalance, 24–37, then hold.              |
| Sit at 46           | Draw into the close curl, 47–62, then hold.          |
| Look forward at 72  | One moderate lift, 74–93, then hold.                 |
| Look left at 95     | Slowly relax back into the curl, 97–123.             |
| Look forward at 117 | Small tip response, 119–126; hold through the blink. |

The box sits flat, with taller sage sides, botanical lining and large GitHub
lettering. The flower picture is a rigid 180 × 120 rectangle while carried. Its
straight top edge meets a small lip at the mouth; it stays in front of the carrier.
It settles into floor perspective behind the rim without changing physical size.

| Frames  | Action                                                  |
| ------- | ------------------------------------------------------- |
| 1–34    | Explore, sniff; kitten moves ahead; paper cat crouches. |
| 35–46   | Grip and lift through complete body poses.              |
| 47–96   | Carry and lower, keeping paws outside the box.          |
| 97–105  | Release and settle behind the rim.                      |
| 102–117 | Withdraw and back away.                                 |
| 118–131 | Gather and rest; kitten’s tongue appears at 119–123.    |

The kitten’s paws stay planted once it stops. Each cat has a distinct rhythm;
the round cat’s tail follows its body and head cues. It blinks at 128–130;
frame 131 reopens its eyes and is the approved endpoint.
Find/Send/Share tags are 1–45, 46–100 and 101–131. Website copy
changes at 31% and 69% scroll progress, aligned to frames 46 and 101. Delivery
adds a reversible 100%–120% camera push toward the lower-right during 47–110;
the master keeps the complete uncropped scene.

## Lessons from the repairs

- **Lock a model before generating poses.** Keep body, paw and head dimensions,
  pixel scale, coat colors and one ground baseline consistent. Request complete
  feline poses, not independently moving arms or replacement body fragments.
  Use the saved model cels as references for any new generated drawings.
- **Inspect the silhouette before texture.** Removing a baked-in tail with broad
  masks cut into rumps, hind legs and ears, and left duplicate tails and fur spikes.
  Complete bodies plus deliberate pelvic overlap worked better. Keep each cat,
  including its tail, in one depth layer so another animal or the card cannot
  pass between its parts.
- **Keep tail anatomy fixed while changing the curve.** The root stays anchored
  inside the pelvis. Vary bends along the whole shaft, rather than swinging a
  rigid tail or repeatedly hinging at its midpoint. Preserve length and taper:
  reference centerlines are 190 native pixels for paper, 160 for watercolor and
  230 for round; approximate midshaft widths are 29, 36 and 30 before scene scale.
  The paper tail follows the slimmer wrapped-tail reference. A small internal
  contact blend joins the materials without a dark seam or sliding connector.
- **Keep the material continuous.** Different paint sources on the two halves
  of a tail produced a visible texture/color seam. Use one coat treatment along
  its length. The lit surface changes as the tail bends; the light’s direction
  and color stay fixed. Don’t bake a second highlight onto the underside.
- **Review every used pose, including holds.** Trimming a broad dark hind-thigh
  projection in one walking frame missed it in pickup and standing drawings.
  Hide the far thigh behind the near hip; redraw its connection where necessary,
  preserving a substantial leg and the planted paw. Excessive trimming pinches
  the leg. Move complete ears and heads through their necks so tips and noses
  cannot shear off. Stop the walk cycle when forward travel stops.
- **Plan contact and depth together.** Carry the rigid picture at the mouth,
  not the chin; use a small curved lip instead of a patch that cuts its edge.
  Settle it into the box’s perspective without shrinking it. Keep the box’s
  contact shadow tight and the paws clear of its rim. Leave room between the
  kitten’s raised tail and the paper cat’s head.
- **Judge the encoded frames.** Length and pixel-count checks passed while
  anatomy still looked wrong. Inspect decoded video close-ups, adjacent frames
  and onion skins for hips, tail roots/tips, ear edges, mouths and landing. Check
  forward/reverse scrolling, chapter controls and the final frame in the browser.

## Export and check

Configure [Aseprite](../../../../agents/aseprite.md), install FFmpeg, and run
**`bun export:story`**. Bun encodes the lossless WebP stills without a separate
image converter. It reads this saved master directly,
checks its dimensions/frame count and hidden references, applies the delivery
camera with nearest-neighbor sampling, and writes one MP4 plus three WebP stills.
Intermediate PNGs use an automatically removed system temporary directory.

H.264 uses CRF 24, the veryslow preset, YUV 4:2:0, no audio/B frames, one-second
keyframes and a front-loaded index. The current MP4 is **7,317,894 bytes**;
stills are **613,552 / 646,770 / 498,620 bytes** using Bun's lossless WebP encoder.
Reproduce with the export command. The encoder changes compressed sizes while
preserving the source pixels.
The page loads the video only for the enhanced motion-enabled story. Reduced
motion, normal flow, unavailable video and disabled JavaScript use the stills.
The muted video autoplays once on page load and holds its final frame;
chapter copy, paw trails and the finale remain scroll-driven.

Run `bun build:web`, `bun test:web`, `bun lint:web` and `bun typecheck:web` after
changes. Build tests retain the 8 MiB video budget and fast-seeking index check;
runtime tests cover chapter progress and video byte ranges. Update
`web/src/story-video.ts` if the native dimensions, frame clock or chapter timing
change. Source timing is 83 ms per cel; delivery assigns exactly 12 fps.

# Organic picture integration

The production website now uses the [October 17 material and lighting revision](website-cat-materials.md).
The exploration studies retain the `v3` sources below as historical artwork.

October 16, 2026. Built-in image-generation tool. The user rejected the independent prop overlay because paws and box rims did not correctly overlap it. These edits integrate the shared picture reference into each illustration instead.

| Active file | Generated source |
| --- | --- |
| [story-bring-v3.png](story-bring-v3.png) | `exec-73e64ee5-d160-4afe-bff9-1c55778d0640.png` |
| [story-upload-v3.png](story-upload-v3.png) | `exec-425f2135-2e9e-4969-893c-f6f62cf614b3.png` |
| [story-copy-v3.png](story-copy-v3.png) | `exec-51d7550b-b82c-4092-b2ef-3e93478ec1bf.png` |

The source plates and [flower reference](flower-picture-v2.png) remain unchanged. The source outputs are preserved in the conversation's generated-image folder and copied into this project. No manual raster editing was used. Small rendering differences remain; natural contact and scene depth take priority over the rejected pixel-identical overlay method. The user called these integrated images fantastic and perfect; the current website proposal uses all three.

## Exact prompts and inputs

```json
[
  {
    "id": "bring",
    "prompt": "Use case: compositing. Image 1 is the scene EDIT TARGET. Image 2 supplies the EXACT botanical picture to integrate: cream landscape paper with a five-petaled violet pansy, yellow center, short green stem and two leaves, left leaf higher than right. Insert that one picture into the target illustration NATURALLY, painted in the same watercolor light and paper grain, not a crisp pasted-on layer. Preserve its subject, botanical arrangement and approximate physical paper proportions. The paper is about the width of the small kitten's body, not a giant sheet spanning the scene. Preserve all three cats, their identities, body sizes, materials, eyes, poses, limbs and tails, the scene composition and framing. Small local paw adjustments ONLY if essential for believable contact. No extra limbs, new characters, costumes, flowers, boxes or text. Preserve GitHub lettering exactly where present. The goal is real depth, contact, correct perspective and occlusion, NOT an image floating on top of all foreground objects.\nThe cats are finding the picture. Lay the card FLAT on the floor directly beneath the small kitten's nose, just to the right of its near forepaw, with a soft, very close contact shadow. Its nearest small paw may overlap the card's corner slightly. All remaining edges rest on the floor. Match the viewing angle of the floor. Keep the tray in the background empty. The small kitten gazes at this specific picture, and the others attend to it. Do not put the card far away from their paws or floating above the floor.",
    "inputs": [
      "story-bring-plate-v2.png",
      "flower-picture-v2.png"
    ]
  },
  {
    "id": "upload",
    "prompt": "Use case: compositing. Image 1 is the scene EDIT TARGET. Image 2 supplies the EXACT botanical picture to integrate: cream landscape paper with a five-petaled violet pansy, yellow center, short green stem and two leaves, left leaf higher than right. Insert that one picture into the target illustration NATURALLY, painted in the same watercolor light and paper grain, not a crisp pasted-on layer. Preserve its subject, botanical arrangement and approximate physical paper proportions. The paper is about the width of the small kitten's body, not a giant sheet spanning the scene. Preserve all three cats, their identities, body sizes, materials, eyes, poses, limbs and tails, the scene composition and framing. Small local paw adjustments ONLY if essential for believable contact. No extra limbs, new characters, costumes, flowers, boxes or text. Preserve GitHub lettering exactly where present. The goal is real depth, contact, correct perspective and occlusion, NOT an image floating on top of all foreground objects.\nThe two cats are lowering the picture INTO the tray. Its LEFT/rear cream edge must sit directly UNDER the tall paper cat's EXTENDED front paw near image coordinate (675,725); its upper-right edge must sit directly UNDER the small teal-eyed kitten's FRONT paw near (807,727). Both paws VISIBLY OVERLAP the paper, with small contact shadows; the paper must not cover those paws. The far/lower edge of the picture is LOWERED INSIDE the box, resting at its interior floor, BEHIND the FRONT RIM, never lying across or above that rim. Slightly tilted card plane is fine while lowering it, but its bottom must visibly enter the box. Preserve GitHub painted on the front face and keep it readable. Correct perspective can foreshorten the paper. This is a controlled precise integration with two visible contact points and box-rim occlusion.",
    "inputs": [
      "story-upload-github-plate-v2.png",
      "flower-picture-v2.png"
    ]
  },
  {
    "id": "copy",
    "prompt": "Use case: compositing. Image 1 is the scene EDIT TARGET. Image 2 supplies the EXACT botanical picture to integrate: cream landscape paper with a five-petaled violet pansy, yellow center, short green stem and two leaves, left leaf higher than right. Insert that one picture into the target illustration NATURALLY, painted in the same watercolor light and paper grain, not a crisp pasted-on layer. Preserve its subject, botanical arrangement and approximate physical paper proportions. The paper is about the width of the small kitten's body, not a giant sheet spanning the scene. Preserve all three cats, their identities, body sizes, materials, eyes, poses, limbs and tails, the scene composition and framing. Small local paw adjustments ONLY if essential for believable contact. No extra limbs, new characters, costumes, flowers, boxes or text. Preserve GitHub lettering exactly where present. The goal is real depth, contact, correct perspective and occlusion, NOT an image floating on top of all foreground objects.\nThe picture is now fully INSIDE the tray, resting FLAT on its INTERIOR FLOOR. Its complete paper footprint fits within the inner walls, not extending over ANY rim, not touching the top of the box walls. FRONT and SIDE BOX WALLS must visibly overlap and occlude the near paper edge where appropriate. Match perspective of the tray's inner floor; add very soft ambient/contact shadow at the card edges. Show the flower clearly, but it must belong inside the box. Keep all cat paws OUTSIDE this completed tray, exactly as in the target, naturally looking at the successful result. Do not add a second card, external result slip, or anything hovering above the tray.",
    "inputs": [
      "story-copy-plate-v2.png",
      "flower-picture-v2.png"
    ]
  }
]
```

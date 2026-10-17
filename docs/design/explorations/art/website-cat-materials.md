# Website cat materials and shared light

October 17, 2026. Implements [GitHub issue #10](https://github.com/jeremyc2/Gatto/issues/10)
and the user's follow-up corrections to the GitHub tray and final-scene tails.
Generated with the built-in imagegen tool; no manual raster retouching.

## Production assets

The website uses these PNG masters and lossless WebP delivery copies at the
original 1536 × 1024 dimensions. Historical exploration sources remain unchanged.

| Scene | PNG master | Generated source |
| --- | --- | --- |
| bring | [story-bring.png](../../../../web/assets/images/story-bring.png) | `exec-bdcc2911-7c8d-4405-bad1-c61b53943fb7.png` |
| upload | [story-upload.png](../../../../web/assets/images/story-upload.png) | `exec-6f226061-6a6b-469b-b32e-4adfdf2caf23.png` |
| copy | [story-copy.png](../../../../web/assets/images/story-copy.png) | `exec-fbd6afba-c64d-46d7-b32a-f00bba866a4c.png` |

## Treatment and continuity

- The small teal-eyed kitten uses broad charcoal watercolor washes, soft pigment
  edges, paper grain and delicate ink contours. Repeated cut-paper fur was removed.
- The tall amber-eyed adult has crisp overlapping matte paper layers and visible
  edges. The round cyan/violet-eyed adult keeps its smooth animated treatment.
- All three share soft warm upper-left daylight and grounded shadows. The round
  cat's separate blue/purple rim illumination was removed; its eye colors remain.
- The GitHub tray has one shallow rim and a low far wall. The broad triangular
  inner panel that read as a stacked second tier was lowered, exposing the flat
  interior beside the picture. Both cats still grip the picture's top corners.
- Across all three scenes, the paper cat has a slim, smoothly curving tail with
  crisp layered cardstock from base to tip. Its forearms, wrists and paws use the
  same construction. The watercolor kitten uses a lower flowing tail with visible
  anatomical attachment, and the round cat keeps its thick rounded curl.
- The upload picture rests farther back in the tray, with visible floor before
  the front rim; its original landscape proportions and approximate scale are
  retained while both cats hold the top corners.
- The approved scene composition, botanical world, picture interaction, page copy,
  layout, app icon, packaging and favicon remain in place. Small generated pigment
  and flower details vary; these are illustrations, not identical prop composites.

## Export

From the repository root, with libwebp's `cwebp` available:

```sh
cwebp -quiet -lossless -z 9 web/assets/images/story-bring.png -o web/assets/images/story-bring.webp
cwebp -quiet -lossless -z 9 web/assets/images/story-upload.png -o web/assets/images/story-upload.webp
cwebp -quiet -lossless -z 9 web/assets/images/story-copy.png -o web/assets/images/story-copy.webp
```

The browser consumes only WebP; PNGs are the editable-generation masters. Keep
both copies synchronized. Lossless export preserves the generated pixels and
existing image geometry.

## Exact prompt record

Inputs were the corresponding historical `story-*-v3.png` scenes, then the
preceding generated revision of that scene. The revised discovery scene anchored
the other scenes' material and light; the original `cat-watercolor.png` study
supported its refinement. The user-provided tray crop identified the rejected
back-wall geometry. The final scene then supplied the watercolor medium reference
for the upload kitten's last refinement. Intermediate outputs remain in the
conversation's generated-image folder; the three final masters above are in the repo.

### Bring

```text
Use case: style-transfer.
Asset type: finished Gatto website story illustration, landscape 1536x1024.
Input image 1: EDIT TARGET, the approved discovery scene.
Improve ONLY the rendering materials of the three existing cats and unify their lighting. Preserve the exact composition, crop, identities, relative sizes, silhouettes, poses, anatomy, faces, gaze, ears, paws, tails, floor, empty celadon tray, plants and the organically integrated pansy picture in the foreground.
The SMALL TEAL-EYED KITTEN at lower left must be unmistakably a traditional watercolor-and-reed-pen illustration: charcoal-black translucent pigment washes, soft bleeding edges, visible ivory paper grain through pigment, subtle wet-on-wet blooms, sparse delicate expressive ink contours and whiskers. REMOVE the small hard overlapping scales and cut-paper fur from this kitten. Preserve its tiny low crouch, teal eyes, cream muzzle and paw contact with the picture; charcoal black coat, not brown tabby. Painterly and tender, not 3D fur.
The TALL AMBER-EYED ADULT at center must be unmistakably a physical charcoal-black cut-paper sculpture: larger visibly separate overlapping curled paper tufts, fibrous matte cardstock, crisp hand-cut contours and thin exposed paper edges with tiny occlusion shadows. Its slender anatomy, face and pose remain the same. No watercolor fur on this cat.
The ROUND BLACK ADULT at right retains the smooth polished animated/cel-rendered style, round proportions, cyan and violet eyes and lilac inner ears. Give its charcoal/velvet-black coat coherent warm upper-left highlights and soft right-facing shadows. REMOVE the electric blue/purple neon rim light and colored glowing outline; retain cyan/violet eye colors as intrinsic character colors, not separate light sources. It should still feel like the same character.
ONE SHARED LIGHT: broad soft warm daylight from upper left/front illuminates all three, the prop and floor. Each medium interprets the same direction: pale reserved pigment on kitten, warm cut-paper edge highlights on adult, broad warm soft/cel planes on round cat. Match soft floor contact shadows falling lower right. All three inhabit this same room.
Preserve the existing warm ivory watercolor world and delicate botanical background with breathing room. No new objects, no clothing, no text, no watermark, no split panels. Preserve the pansy picture, paper size, foreshortening and contact exactly. Dramatic material distinction at website display size, without redesigning the cast or scene.
```

### BringRefine

```text
Use case: style-transfer. Image 1 EDIT TARGET, keep its entire scene and large two cats exactly unchanged. Image 2 is ONLY a watercolor medium reference, never copy its poses or contact sheet layout.
Make ONE LOCALIZED STRONG CHANGE: repaint ONLY the SMALL CROUCHING TEAL-EYED KITTEN at lower left in unmistakably fluid traditional watercolor and delicate black ink. Completely eliminate ALL its sharply overlapping leaf/scale shaped pieces, hard platelets, brown mottled cut-paper patches and sculptural texture, including on its tail and forehead. Replace its entire coat with broad connected soft charcoal-black wash shapes, wet-on-wet gray pigment blooms, soft ragged brush edges and occasional dry brush strokes, visible unpainted ivory grain. Flat painted watercolor with expressive thin reed-pen contours; zero cut-paper texture and zero photoreal/3D fur. At small display size it must look like a watercolor painting living beside a paper sculpture. Keep teal eyes, cream muzzle, pink ears, little paws and exact crouching anatomy and raised curled tail, exactly the same place, gaze and nose/paw contact with the picture. Preserve the warm upper-left illumination by reserving pale paper highlights on top-left surfaces and washes for lower-right shadows.
Everything outside that small kitten is locked: large amber-eyed paper adult, round cyan/violet-eyed black adult with warm light and NO blue/purple rim glow, background, plants, empty tray, exact foreground pansy picture and perspective, framing, image proportions. No new objects, no text. Landscape 1536x1024.
```

### Upload

```text
Use case: style-transfer. Asset: finished website story illustration, landscape 1536x1024.
Image 1 is the EDIT TARGET: upload scene. Image 2 is the finished discovery scene, ONLY a reference for the three cats' distinct materials and shared lighting.
Change only the rendering materials of the three existing cats and their lighting, matching image 2. Preserve image 1 composition, silhouettes, anatomical poses, relative sizes, exact paw positions, faces/gazes, ears, tails, plants, floor, tray, GitHub lettering and foreground flower picture. DO NOT copy image 2's arrangement, crouches or empty tray.
SMALL TEAL-EYED KITTEN in center must be fluid traditional watercolor and fine reed-pen ink: broad connected charcoal-black translucent washes, soft bleeding pigment edges, visible ivory grain, wet-on-wet blooms, sparse dry brush strokes and expressive delicate ink whiskers. Completely replace the sharply overlapping leaf/scale/cut-paper patches, including forehead and tail, with watercolor wash shapes. Flat painted watercolor with expressive outline; no paper scales or 3D fur. Retain cream muzzle, pink ears, teal eyes, little paws and identity. Clear material difference from tall adult at website size.
TALL AMBER-EYED ADULT at left must be a physical charcoal-black cut-paper sculpture: larger separate overlapping curled matte cardstock tufts, fibrous paper surfaces, crisp hand-cut edges and tiny interlayer shadows, matching image 2. Preserve slender body, reaching foreleg and all other anatomy.
ROUND CYAN/VIOLET-EYED ADULT at right retains smooth polished animated/cel rendering and round velvety-black proportions with lilac ears. Remove ALL electric blue/purple rim light, glowing outline and colored fur illumination. Keep both intrinsic eye colors. Warm highlights and broad smooth charcoal shadow planes as image 2.
One broad warm daylight source from upper left/front lights all cats, props and floor; soft shadows fall lower right, coherent with source. Watercolor reserves pale pigment/paper highlights, cut paper catches warm light on upper-left edges, round cat has warm soft planes.
CRITICAL preserve the exact flower painting, card size and perspective, tall cat's extended paw resting OVER the upper-left card edge and small kitten's near paw resting OVER the upper-right card edge, card lowered into the celadon box, near rim visibly IN FRONT of lower card, exact readable painted word GitHub on front wall. No new objects, text, limbs, split panels or watermark. Keep scene and background unchanged.
```

### UploadTray

```text
Use case: precise-object-edit. Edit target: image 1, the Gatto upload illustration. One localized correction to the celadon GitHub tray ONLY. Keep all cats, distinct materials, warm shared lighting, exact paws gripping the picture, pansy picture, plants, floor, lettering and composition unchanged.
The box currently has confusing stacked/double-height construction on its FAR/BACK side, visible behind and around the large leaning flower card. Redraw it as ONE simple shallow rectangular open tray with FOUR uniform low walls and ONE continuous top rim in correct perspective. Back wall must be the SAME physical height as the front and side walls; its top edge must join the side-wall top edges at the same plane. Remove any second horizontal lip, doubled back rail, extra tier, inner raised ledge or stacked box. Only one far wall, one thin cream top edge, one continuous celadon interior face. Where the card occludes the back wall, continue that same low wall invisibly behind it. Do not draw a high extra wall behind the card or a rim running across the card. The outer footprint and front GitHub wall stay in exactly the same place and size; readable painted text verbatim GitHub. Keep natural card insertion: both cats' paws overlap the upper card corners, card lower edge sits inside the tray behind the near wall. Preserve pansy painting unchanged. No new props or text. Landscape 1536x1024.
```

### UploadTrayLow

```text
Use case: precise-object-edit.
Image 1 is full-scene EDIT TARGET. Image 2 is a close-up identifying a REJECTED DEFECT in its tray geometry, NOT a style/reference to copy.
Correct ONLY the pale celadon tray in image 1: the TALL TRIANGULAR GREEN INNER PANEL visible to the RIGHT of the pansy card under the round cat's paws looks like a second stacked level. REMOVE THAT TALL PANEL. LOWER the FAR/BACK rim dramatically so the far wall is a narrow low rail near the tray floor, with only a THIN 10–15 pixel inner vertical face, not a big tall triangle/ramp. There must be one unmistakably simple shallow tray, uniform low walls, one top rim, no second tier or high raised back. Both top and bottom edges of each wall must be parallel in perspective with constant physical height. The back-right corner must be low, leaving a clearly visible strip of warm ivory FLOOR between the low far rim and the round cat's paws. Cats do not stand on the tray or form part of its wall.
The near/right botanical wall can remain visible, but simplify its junction with the far wall to ONE low corner of a shallow rectangular tray. No broad green triangular face behind or above this near wall. The interior should read as a mostly FLAT FLOOR around the inserted card, with subtle thin near-contact shadows, not sloping walls or a deep well. Reduce ALL walls to the same shallow physical height as needed to fix perspective. Cream edge is a single thin line.
Keep the exact readable word GitHub painted on front face, botanical decoration, approximate tray footprint and foreground position. Preserve both cats' paws overlapping the picture's upper corners, the leaning pansy card and its lower edge inside behind the near rim, the pansy artwork, all cats/poses/eye colors/materials/shared warm lighting, plants, background, framing and aspect ratio. No new objects, text or limbs. Landscape 1536x1024.
```

### UploadRefine

```text
Use case: style-transfer. Image 1 EDIT TARGET, Gatto upload scene. Image 2 is a reference ONLY for watercolor kitten's loose medium. ONE LOCAL CHANGE: the SMALL TEAL-EYED KITTEN in CENTER behind the pansy card.
Make its coat decisively fluid HAND PAINTED WATERCOLOR matching image 2's kitten: a few broad connected charcoal-black/gray pigment washes, wet-on-wet blooms, ivory grain showing through, subtle bleeding edges, sparse expressive fine ink contours/whiskers. REMOVE every repeated small sharp overlapping cut-paper leaf/scale tuft from its forehead/body/tail and replace with flat flowing broad ink washes. No paper cut layers, no 3D sculptural fur. Preserve original kitten's size and exact silhouette, upright little head looking at card, teal eyes, cream muzzle, pink ears, curled tail, paws. Keep its front paw exactly overlapping upper-right corner of picture with natural contact, do not move it.
EVERYTHING OUTSIDE THIS SMALL KITTEN IS LOCKED TO IMAGE 1. In particular preserve the corrected LOW shallow GitHub tray, its single rim, narrow far-wall inner face, visible flat floor to right of card and gap beneath round cat's paws. Do NOT raise the back wall or recreate a large green triangular inner panel or second tier. Preserve exact word GitHub and all tray details as image 1. Keep tall paper cat's exact materials, body/pose, round smooth cat's rendering and pose, all eye colors and tails, shared warm light, pansy painting/card/geometry/contact, floor, plants, background, framing and aspect ratio. Do NOT copy image 2's composition, final scene or lower kitten tail. No new objects, limbs, text or watermark. Landscape 1536x1024.
```

### Copy

```text
Use case: style-transfer. Asset: finished Gatto website story illustration, landscape 1536x1024.
Image 1 is EDIT TARGET: final completed-picture scene. Image 2 is finished discovery scene, reference ONLY for the three cats' rendering materials and shared lighting.
Change only rendering materials and lighting of the three cats in image 1 to match image 2, preserving image 1's composition, exact anatomical poses, relative sizes, silhouettes, faces/gazes, eyes, ears, paws, tails, plants, floor, tray, flower picture and framing. DO NOT copy image 2 poses or arrangement.
SMALL TEAL-EYED KITTEN at right must be traditional fluid watercolor and delicate reed-pen ink: broad connected translucent charcoal-black washes, soft pigment blooms and bleeding edges, visible ivory paper grain, sparse dry brush marks and expressive thin ink whiskers. Remove ALL sculptural overlapping leaf/scale/cut-paper fur patches on kitten, including forehead/body/tail. Flat painted watercolor, not textured 3D fur, no hard paper scales. Retain teal eyes, cream muzzle, pink ears, little paws and play-bow with curled tail. Clearly different medium from tall paper cat.
TALL AMBER-EYED ADULT at center must be physical charcoal-black cut-paper sculpture, with larger visibly separate curled matte cardstock tufts, fibrous paper surfaces, crisp hand-cut edges and tiny interlayer occlusion shadows, matching image 2. Same slender upright body, seated pose and amber eyes.
ROUND CYAN/VIOLET-EYED ADULT at left retains smooth polished animated/cel rendering and round velvety black proportions with pink/lilac inner ears. REMOVE all electric blue/purple rim glow and colored fur lighting; preserve eye colors, identity and smooth broad modeling. Warm soft highlights from shared upper-left daylight, charcoal shadow planes.
ONE broad soft warm daylight from upper left/front illuminates all three and floor, with coherent soft lower-right shadows and grounded feet. Paint highlights as reserved paper on watercolor, thin warm upper-left edge catches on paper sculpture, broad warm soft planes on round cat.
Keep organically integrated violet pansy picture exactly as in image 1: card FLAT within celadon tray's interior floor, all card edges inside tray, front and side rims overlap it. Keep all paws outside tray, no hovering card or second sheet. Preserve warm ivory botanical watercolor background, floor and small plants. No new text, no GitHub label in this scene, no new objects or characters, no extra limbs, no split panels or watermark.
```

### CopyRefine

```text
Use case: style-transfer. Image 1 EDIT TARGET, final Gatto story scene. A very strong LOCALIZED medium change on ONLY the little teal-eyed kitten at RIGHT. Everything outside this kitten is locked.
Repaint this kitten as a loose, unmistakably HAND PAINTED BLACK WATERCOLOR illustration with fine reed-pen outlines. Its body and head must have only a FEW LARGE SOFT CONNECTED CHARCOAL INK WASHES, fluid smoky gray pigment pooling, transparent watercolor edges, soft bleeding wet-on-wet shading and visible ivory grain. Remove every little leaf-shaped tuft/scale/plate/tile of fur completely. The existing kitten looks like another paper sculpture: change this decisively. Broad brushed masses with NO repetitive pointed fur shapes, no cut layers, no tiny detailed fur rendering, no three-dimensional sculptural texture. Loose transparent picture-book watercolor. Ink lines can define ear contours, paws and whiskers with sparse sketchy strokes, but the body interior should be mostly soft flowing dark washes and unpainted highlights. Match the original pose and silhouette exactly: play-bow, front paws stretched toward tray, hindquarters raised, long tail curling up. Same teal eyes, pink ears, cream muzzle, cream little paws, friendly face. Keep it clearly black/charcoal with cream muzzle, not a brown tabby or a gray cat.
Keep upper-left warm light suggested through pale reserved paper, and soft painted contact shadows on same floor. Do not change large amber-eyed adult's sharp cut-paper layers, large smooth round cat, their faces and eye colors, warm shared lighting, tray, flower painting, foliage, composition or framing. Do not add props, text, limbs or watermark. Landscape 1536x1024.
```

### CopyTail

```text
Use case: precise-object-edit. Image 1 EDIT TARGET is finished Gatto final story scene.
Make ONLY ONE LOCALIZED CHANGE: the SMALL WATERCOLOR KITTEN'S TAIL at FAR RIGHT. It must have a completely different silhouette from the tall paper cat's upright hooked tail behind its shoulder.
Remove the small kitten's current high vertical hook/S-curl. Replace with a soft slender watercolor tail extending OUTWARD to the RIGHT at a LOW ALMOST HORIZONTAL angle from its rump, then relaxing DOWN in a single gentle arc, with its pointed brushy tip resting just ABOVE the floor inside the right image boundary. The tail stays behind its hind legs, stays fully visible within canvas, does not overlap any other cat/tail. Make it feel relaxed, fluid and flexible, NOT a second tall upright hooked curl. Preserve natural tail attachment at rump. No extra tail.
Render this changed tail with loose smoky charcoal watercolor wash and sparse fine ink contour, soft pigment edges, paper grain, no paper-cut tufts. Keep the kitten's black watercolor body, cream paws/muzzle, teal eyes, exact play-bow pose, head/ears/gaze and body unchanged.
LOCK EVERYTHING ELSE: the tall paper adult including its ORIGINAL upright hooked tail, the round adult including its ORIGINAL thick tail, the cats' distinct materials and shared warm light, plants, tray, exact pansy card lying flat inside tray, floor, composition, crop and all other pixels. No new objects/text/watermark. Landscape 1536x1024.
```



## Follow-up: tail anatomy, paper construction and card placement

The user clarified that tail distinction includes posture, position and shape
throughout the sequence, and that the paper cat's limbs and tail must retain the
same cut-paper material as its torso. An angular-tail attempt was rejected as
broken-looking; final paper tails use smooth anatomical curves with crisp paper
layers. The kitten's upload tail root was then joined visibly to its rear haunch.
The upload card was moved away from the front corner, with its earlier scale
restored after an intermediate size reduction was rejected. The final images
above supersede the earlier pass while retaining its prompts below.

### BringPrompt

```text
Use case: style-transfer / precise-object-edit.
Asset: Gatto discovery scene, landscape 1536x1024.
Image 1 is EDIT TARGET. Correct paper-cat material continuity and differentiate tail POSTURE, POSITION and SHAPE. Preserve scene composition, all faces/eye colors, body poses, paw placements, lighting, pansy picture, plants and tray.

TALL AMBER-EYED PAPER CAT in center: it is a physical sculpture made entirely of charcoal matte cardstock. Keep the good crisp overlapping pointed paper layers on its torso and head, and apply EXACTLY THAT SAME CONSTRUCTION to EVERY limb, every forearm, wrist, paw, toe and its ENTIRE TAIL. Visible cut edges, tiny cast shadows between layers, fibrous matte cardstock, rigid folded/curled paper surfaces. Forelegs must have overlapping small paper plates and clearly folded paper toes, not smooth realistic furry legs or watercolor strokes. Paper tail must have clear overlapping cut-paper shingles from base through tip, clean solid silhouette and tiny layer shadows. No smoky pigment, soft watercolor edges, fluffy brush fur or fading paper texture anywhere on this cat.

REDESIGN PAPER CAT TAIL: remove its existing fluffy upright C-hook. At the same natural tail base, create a LONG SLIM TAPERED FACETED paper tail rising diagonally UP then bending once in a clear angular kink, terminating in a small POINTED paper tip. This is a narrow elegant sculpted tail, NOT a round hooked curl, loop or S. Keep it wholly inside canvas, with clear negative space from ears and other tails. Its layered paper construction must be just as crisp as the shoulder, all the way to tip.

SMALL WATERCOLOR KITTEN at lower left: replace its upright hooked tail with a LOWER loose lateral swish extending LEFT from its rump, softly bending toward floor with a slender brushy taper. Fully within left canvas margin, no loop or high hook. Tail is watercolor, continuous smoky ink washes and loose contour matching kitten body, never paper scales. Preserve kitten body/head/paws exactly.

ROUND SMOOTH CAT at right: preserve its thick rounded upright curled tail and all its smooth rendering. Its tail now contrasts with slim angular paper tail and low flowing watercolor tail.

Lock the remaining world: warm shared upper-left daylight, soft grounded shadows, same three black cats, same body proportions, eyes and expressions, same exact pansy picture and contact under kitten's nose/paw, empty celadon tray, botanical background and framing. No objects, limbs or text added. No redesign of faces or body poses. No neon rim light.
```

### BringSmoothPrompt

```text
Use case: precise-object-edit. Image 1 EDIT TARGET. One local correction ONLY to the tall amber-eyed PAPER CAT'S TAIL in the center-right upper region.
Remove the abrupt angular bend that makes this tail look broken. Replace its centerline with a SINGLE SMOOTH GENTLE NATURAL ARC rising from the same rump and sweeping up-right, gradually tapering to a narrow rounded feline tip. No elbow, sharp kink, zigzag, crease across tail, spear tip, right angle, tight hook or loop. It should feel like a flexible cat tail in an alert relaxed diagonal pose, not a rigid branch or a feather. Tail's OUTLINE has smooth curvature; individual surface paper layers still have crisp cut edges.
CRITICAL MATERIAL: this is still physical charcoal cut-paper sculpture, entirely covered from base to tip by overlapping matte cardstock shingles with crisp edges, visible paper fibers and tiny interlayer cast shadows EXACTLY matching its torso and arms. Do NOT make tail soft furry or watercolor. Do NOT soften the individual paper edges; only smooth the underlying anatomical centerline. Full tail within image margin with breathing room at top.
Everything else remains identical: three cat faces/eyes/body poses/paper forearms/paws, low loose watercolor kitten tail at left, thick rounded animated cat tail at right, warm light, pansy picture and contact, empty tray, plants, framing. Landscape 1536x1024.
```

### UploadPrompt

```text
Use case: style-transfer / precise-object-edit. Gatto upload scene, 1536x1024 landscape.
Image 1 is EDIT TARGET. Image 2 is MATERIAL/TAIL-DESIGN REFERENCE ONLY: revised discovery scene. Keep image 1's composition and poses; do not copy image 2 layout or tray.
Two corrections: paper-cat material consistency throughout EVERY limb and tail, plus three distinct tail silhouettes.

TALL AMBER-EYED ADULT at LEFT is the SAME physical charcoal cut-paper sculpture as image 2. Preserve torso/head's overlapping matte cut-cardstock layers and extend this construction all the way along EVERY limb including the LONG HORIZONTAL REACHING FOREARM, wrist, gripping paw, toes, hind legs and tail. The extended arm must clearly be built from overlapping hand-cut paper shingles with sharp exposed edges and small interlayer shadows, not a smooth furry tube or watercolor smudge. At bends, show folded cardstock joints, especially wrist and gripping paw. All surfaces matte fibrous paper, not natural fur or ink wash.
REPLACE this adult's high fluffy hooked tail with the SAME slim tapered faceted paper-tail design as image 2: a long narrow paper tail rising diagonally up from its natural rump, one distinct angular bend, small pointed tip. Here it rises along left side of body, fully within canvas and away from its head. No loop, thick rounded hook, smoky edge, fluffy fur or watercolor on tail. Crisp layered paper shingles extend continuously from base to pointed tip.

WATERCOLOR KITTEN at CENTER: its tail must have a different posture/position/shape, a loose slender smoky watercolor swish extending LOW and LEFT behind its body above the paper card's top edge; a gently flowing horizontal/diagonal curve with tapered brush tip, NOT a tall upright hook/C/S. Must stay visually separate from paper adult's extended arm; tail connects only to kitten's rump. Preserve broad soft charcoal pigment washes on kitten, cream muzzle/paws, teal eyes, exact head pose and paw overlap on the card.

ROUND SMOOTH CAT at RIGHT: retain thick rounded tail curled close beside rump, preserve its smooth animated rendering and cyan/violet eyes. Thick compact curl versus tall angular paper tail versus lower loose painted tail.

LOCK THE CORRECTED TRAY: the single LOW shallow celadon GitHub tray, narrow far-wall interior strip, flat interior floor visible right of card, one top rim, no tall triangular rear panel, no stacked tiers. Word GitHub exactly on front. Preserve two cats' paws visibly gripping the two upper corners, leaning pansy painting/paper size/perspective/lower edge inside behind near rim. Keep all other cats' body poses, faces, sizes, ears, eyes, plants, floor, background, framing and shared warm upper-left light. No extra limbs, props, text or neon rim glow.
```

### UploadPlacementPrompt

```text
Use case: precise-object-edit. Image 1 EDIT TARGET, Gatto upload scene. Image 2 material/curve reference ONLY (not composition). Landscape 1536x1024.
Two necessary localized corrections:
1. PAPER CAT'S TAIL at upper left: replace the sharp bent/kinked centerline with ONE SMOOTH NATURAL GENTLE ARC rising from same rump. A slender elegant feline tail extending up-left with mild gradual curvature, NO angular elbow, zigzag, kink, tight hook or loop; tip well within canvas. Its SURFACE must remain entirely physical crisp cut-cardstock layers matching its torso and reaching forearm: overlapping charcoal matte shingles, clear cut edges, small layer shadows from base through tip. Smooth underlying anatomy, crisp individual paper pieces; no watercolor or fluffy fur.
2. REPOSITION THE PANSY CARD INSIDE THE TRAY. Currently lower paper corner butts against tray's FRONT corner/rim. Move the card's LOWER EDGE into the BACK-CENTER HALF of the tray interior, well away from front and side walls. Make card somewhat smaller and more upright if needed, while keeping same recognizable violet pansy painting and physical rectangular paper proportions. There must be a CLEAR BROAD VISIBLE STRIP OF EMPTY IVORY TRAY FLOOR between the card's bottom edge and the entire front wall, including front-right corner. Card must NOT rest on, touch, cover or wedge against front rim/corner. Leave visible interior margin to BOTH side walls too. The bottom edge rests on the interior FLOOR nearer the far side, with close contact shadow, not floating.
Both paper cat and little kitten still hold the card's two upper corners with paws visibly overlapping paper; make SMALL NATURAL ADJUSTMENTS to their reaching forearms/paws and card tilt as needed to fit this physically correct placement, while keeping overall body poses and positions. Preserve crisp overlapping paper layers on ENTIRE long reaching arm, wrist, paws and toes, no furry smooth joints. Little kitten remains watercolor with cream paw.
Keep corrected tray geometry: one LOW shallow uniform wall height, single thin cream rim, narrow far-wall interior strip, no large triangular tall rear panel or second tier. Keep readable GitHub painted exactly on front face and botanical decoration. Keep tray footprint and position. Keep all cats' faces/eyes/body sizes/poses, lower loose watercolor tail behind kitten, thick compact rounded tail of smooth round cat, warm shared light, plants, floor and framing. No new objects, extra tails/limbs, text, neon glow. Do not copy image 2 arrangement.
```

### UploadAttachCardPrompt

```text
Use case: precise-object-edit. Gatto upload illustration, landscape 1536x1024.
Image 1 EDIT TARGET. Image 2 is the PREVIOUS CARD-SIZE REFERENCE ONLY: keep its original cream flower card's physical dimensions, rectangular proportions and size relative to cats/paws. Do NOT copy image 2 tray placement or old tail. Image 3 is canonical flower paper reference, landscape 3:2, content/proportions only.
Two precise local corrections in image 1:
A. THE LITTLE WATERCOLOR KITTEN'S TAIL IS DISCONNECTED-LOOKING. Repaint its ROOT and the small rear HAUNCH so the tail visibly grows from the kitten's RUMP at the end of its spine, behind its body. Show an unbroken outline and continuous charcoal watercolor washes from rear haunch into a thicker tail base that gradually tapers along the low sideways tail. There must be NO floating separate brushstroke, no gap at attachment, no attachment to neck/shoulder, and no attachment to adult cat's forearm. Reveal a little rounded rear haunch behind the kitten's left flank if necessary to make its actual root unmistakable. Keep lower leftward flowing tail, natural anatomy and kitten's original head/face/teal eyes, watercolor medium and grip on card.
B. RESTORE ORIGINAL FLOWER CARD'S PHYSICAL SIZE AND SHAPE from IMAGE 2. Image 1 made it too small and changed dimensions. It is the SAME landscape cream rectangular 3:2 botanical picture across all scenes, not a miniature square or portrait card. Enlarge the card back to image 2's scale relative to the cats' paws (roughly one-quarter larger than image 1), preserving recognizable exact violet pansy arrangement and original paper aspect ratio. Position and tilt this rigid unchanged-size card so its LOWER EDGE rests on the tray FLOOR toward BACK-CENTER, with CLEAR FLOOR SPACE in front of it and beside it. It must not butt into front corner, lie on rim, float, or extend outside tray. Use a more upright tilt and modest backward shift; do NOT shrink or squash the paper to make it fit. Both cats' paws still visibly overlap the top two corners; adjust only their reaching wrists/forearms locally as needed for natural contact.
Preserve ALL crisp layered charcoal cardstock on paper adult's entire reaching forearm, wrists, paws and tail; its newly smooth gently curved slender tail and all other adult body poses/face/amber eyes stay. Keep round smooth cat, eye colors, thick rounded tail. Preserve single shallow tray wall/rim, no stacked tiers or tall triangular back panel, readable GitHub label, background, floor, plants, warm shared light and framing. No extra objects, tails, limbs or text.
```

### UploadRootPrompt

```text
Use case: precise-object-edit. Image 1 EDIT TARGET. Change ONLY the small watercolor kitten's REAR BODY AND TAIL ATTACHMENT, centered between adult's reaching arm and kitten's head.
Current horizontal tail looks like a floating brushstroke coming from near its shoulder. REMOVE this ambiguous floating tail shape. Reveal a small anatomically clear ROUNDED REAR HAUNCH to the LEFT of kitten's shoulder, behind the card. Connect it continuously to the kitten's existing visible torso with a gentle arched back. The tail must visibly emerge from the LOW REAR END OF THIS HAUNCH, immediately above the hindquarters, at the end of the spine. Draw an unbroken, clearly readable charcoal silhouette and pigment continuity across hip into a substantial natural tail base.
From this unmistakably attached root, let the slender tail flow LEFT in a low gentle natural curve behind the adult forearm, then taper with relaxed brushy tip. Its base must be lower than the kitten's shoulder, NOT attached near neck/head. No gap of background at base, no isolated stroke, no extra tail or limb, no tail attached to adult's arm. If necessary move the low tail's curve locally to show the root, but do not make a high upright hook.
Watercolor medium stays broad soft charcoal washes and ink contours, cream muzzle/paw, exact teal eyes/head/ear positions and expression. Everything else LOCKED, especially current restored-size LANDSCAPE flower card and its exact shape, painting, tilt and location with empty tray floor in front, both paws gripping top corners, GitHub tray's geometry/lettering, paper adult's crisp layered material on forearm/paws/tail, smooth natural adult tail curves, round cat, lighting, plants and framing. Do not shrink/resize card, change other cats, or add objects. Landscape 1536x1024.
```

### CopyPrompt

```text
Use case: style-transfer / precise-object-edit. Gatto final scene, landscape 1536x1024.
Image 1 EDIT TARGET, image 2 REFERENCE ONLY for consistent paper construction and NATURAL SMOOTH tail curvature. Preserve image 1 composition/poses and do not copy image 2 scene.
TALL AMBER-EYED SEATED ADULT in CENTER must be entirely a physical matte charcoal cut-paper sculpture with the exact crisp overlapping hand-cut cardstock layers of its own torso and image 2. Apply same material continuously to ALL legs, arms, wrists, paws, toes and ENTIRE TAIL. No soft fuzzy/watercolor edges or smooth furry limbs. Visible sharp cut-paper edges, fibrous matte surfaces, curled layered shingles and tiny interlayer cast shadows all the way to tail tip and paw tips. Wrist and toe joints should look folded from paper, not realistic skin covered with a few tufts. Keep face/eyes/ears/body silhouette and proportions.

REPOSE THIS PAPER CAT'S TAIL: replace existing rounded upright hooked/C-curl with a SLIM TAPERING tail rising diagonally OUTWARD TO THE RIGHT behind its body in a LONG GENTLE SMOOTH ARC, through the open space above/right of its haunch and above the watercolor kitten. A relaxed flowing feline centerline, no abrupt angle, elbow, crease, zigzag, kink, broken-looking bend, loop, short round hook or spear. Distinct from other cats' tail positions. Maintain negative space from kitten's head/ears and paper adult's shoulder; fully within canvas. The anatomical shape is curved and flexible, but all individual surface layers remain sharply cut physical PAPER continuously base to tip, not watercolor or fluffy brush fur.

Preserve SMALL WATERCOLOR KITTEN at RIGHT with its already DIFFERENT LOW DROOPING tail behind hind legs, loose pigment washes and smoky ink-contour edge. Do NOT raise, hook, stiffen, or turn kitten tail into paper. Keep its exact play-bow, teal eyes, cream muzzle/paws and head.
Preserve ROUND SMOOTH CAT at LEFT with its THICK UPRIGHT ROUNDED CURL and smooth animated fur, cyan/violet eyes.
Three distinct tail postures/shapes: slender diagonally outward paper arc, low loose watercolor droop, thick upright rounded animated curl.
Lock all other details: pansy picture FLAT inside tray behind rims, all paws outside completed tray, botanical floor/background, warm upper-left shared light, soft ground contact shadows, sizes/poses/gazes/eye colors and framing. No new props, text, extra limbs or tails, no neon light, no sharp broken anatomical tail bend.
```


# Download portal artwork

Originally created October 17, 2026 for the website download section; the ring
was redrawn as [Aseprite pixel art](pixel-portal/artwork.md) on October 26. The cat's identity is
based on [the existing logo artwork](../../../../packaging/rocket-cat-transparent.png).
The original portal was generated with the built-in image-generation tool as a
separate transparent layer. The current ring is drawn in Aseprite. Both its
aperture and exterior are transparent, allowing the
website background to show through in the source. The website now places a black
circular aperture behind the ring at the user's request. No film still or third-party character art
was supplied.

## Files and motion

The portal master is now [pixel-portal/portal.aseprite](pixel-portal/portal.aseprite)
(384 × 384). Its 2× PNG and lossless WebP exports are
`web/assets/images/download-portal-pixel.png` and `.webp` (768 × 768). The earlier
painted portal PNG/WebP were removed; the original prompt remains below as history.
On October 25, 2026 the cat was replaced by a separate
[pixel-art visitor](portal-cat/artwork.md), drawn in Aseprite and exported as
`web/assets/images/download-rocket-cat-pixel.png` and lossless WebP (512 × 512).
The superseded side-view and forward-facing cat PNG/WebP exports were removed
during the October 26 asset cleanup. The prompts and generated-output IDs below
record the earlier illustrations. The portal and cat are
separate assets. WebP exports are served lazily. Original generated portal source:
`exec-4c768f84-d109-493c-9f82-790ac6ddffce.png`.

The portal rotates once every 240 seconds; the separate cat moves just one CSS
pixel above and below center over twenty-eight seconds, with no rotation. Its only
ambient animated property is vertical `translate`; a separate wrapper controls
its scroll-driven forward approach without rotation. The cat finishes at
42% of the scene width, leaving space inside the ring. The user requested removal
of the animation control.
The composition sits in its own tan watercolor section between the download
section and footer. A named native view timeline starts opening it from zero
height once the footer reaches the viewport bottom. At 15%–55%, an SVG stroke
mask traces the painted rim clockwise. Rounded caps, a displaced edge and a
soft blur remove radial wedge cuts; the broad caps overlap as the loop closes.
The mask retains the original painted ring rather than replacing it with a
vector stroke. At 20%–90%, the rim and void scale from zero to full size
together. The void uses a black radial core with deep violet and warm translucent
falloff under the painted rim, eliminating a hard circular cutoff. From
30%–100%, the cat approaches on an independent foreground layer: it starts small,
dim and slightly defocused, then grows into its existing final size with normal
lighting and focus. It is never circularly clipped. Neither cat nor void uses
an opacity entrance. The cat remains upright, with no rotation.
Reversing scroll closes it. Its growing height and a shrinking spacer after the
footer sum to a constant full section height, so changing layout cannot feed
back into the timeline. The footer remains stationary during the reveal.
The timeline uses `entry-crossing` ranges so it also works in short viewports.
Unsupported range/timeline syntax shows the complete composition with no spacer.
Reduced motion also stops the ambient animations. No wheel, touch, or keyboard
events are intercepted.

## Watercolor setting

The background tile was generated with the built-in image-generation tool using
`story-bring.png` as color and material guidance. Source:
`exec-8f3f097d-fc3e-4a0e-b361-1b65bf442f41.png`. The master is
`web/assets/images/portal-watercolor-paper.png` (1254 × 1254); the served WebP is
1024 × 1024. It repeats at 512 × 512 CSS pixels for 2× density rather than
stretching across larger viewports. The image's pale tan pigment and paper grain
remain visible in both appearance modes without a blend that washes them out.
The initial code-authored `portal-seam.svg` used a 192 × 16 repeatable alpha mask.
The dramatic-tear revision on 2026-10-17 replaces it with a 768 × 48 vector
repeat: irregular shallow runs, deeper notches and small jagged fibers. Phones
render it at 640 × 40 to preserve the mask's aspect ratio and seamless joins.
`PortalSeam` attaches it only to the download section's bottom and footer's top,
using each section's theme color over a tan paper underside. The initial dramatic
revision shifted the footer pattern by 211 CSS pixels; the matching-cut revision
on 2026-10-17 removes that shift and rotation. `web/scripts/build-portal-seam.ts`
derives `portal-seam-complement.svg` by keeping the exact cut coordinates and
closing the path along the bottom rather than the top. `bun build:web` runs this
generator before bundling. Both masks therefore use the same origin. Their
mask bands coincide when closed; separation equals the revealed section height.
Colored faces inset by 2 CSS pixels (1 on phones), exposing thickness inside the
cut instead of extending one paper edge past the other. The shared finale paper
background continues behind the footer's masked band. A tight contact shadow
and broader soft shadow follow the torn contours. The watercolor
section itself has no shadow. See the
[opening research](../../../research/portal-opening.md) for primary sources and
alternative approaches.

### Texture prompt

Use case: stylized-concept. Asset type: seamless repeating watercolor paper background texture tile for a website. Reference image is STYLE AND COLOR GUIDANCE ONLY. Extract the feeling of its pale warm ivory/tan watercolor paper world, without depicting any objects from it. Generate a square perfectly seamless tile: fine cotton paper fibers, extremely subtle translucent warm beige and pale raw-sienna watercolor granulation, delicate irregular pigment blooms and paper grain. Very quiet low contrast, predominantly pale creamy tan with tiny soft warm peach/ochre variations. Even diffuse light, completely flat surface, no vignetting, no center composition, no edges, no borders, no crease, no diagonal light streak, no shadows. No cats, animals, characters, plants, flowers, leaves, tray, picture, props, text, shapes or recognizable objects. No green pigment. No large blotches or distracting speckles. Opposite edges must match seamlessly for repeating in both axes, no visible seam or grid. This is background paper material only, sophisticated subtle watercolor, not fabric or grunge. Entire image opaque.

## Forward-facing cat prompts

Built-in image generation used the existing logo cutout as identity guidance.
The initial head-on pose was `exec-a040eeff-fb4a-47b8-ab20-26731a48cec7.png`;
the intermediate cutout `exec-30c3e789-e119-4fc0-95f4-331cf2a71f73.png` reverses
the thrusters. The final cutout `exec-ecd988a0-2a79-4652-a039-8d831b7825f9.png`
removes the visible flames while preserving the jetpack. The earlier side-view
and forward-facing raster exports are superseded by the current Aseprite master.

### Head-on pose

Use case: identity-preserve. Make a NEW standalone transparent cutout of this exact Gatto logo jetpack cat, preserving its polished dimensional animated illustration style, midnight indigo coat, violet highlights, big bright violet-white eyes, purple inner ears, tiny purple nose, whiskers, and silver-purple twin rocket pack with cyan light bands and small multicolor thrust flames. Change the camera and pose: cat faces DIRECTLY FORWARD toward the viewer, centered head-on, both eyes looking directly at us, face level with no head tilt or sideways turn. Cat leans forward slightly in flight as though coming OUT OF a portal toward the viewer. Use gentle perspective foreshortening: face and two front paws closer to camera, body and little hind paws receding behind, jetpack visible behind both shoulders. Four feline paws total, one graceful tail curving behind. Friendly curious expression, natural compact kitten anatomy, no humanoid arms. Keep the whole character and jetpack flames fully in frame with generous transparent margins. Square composition with optical center at canvas center, balanced silhouette, no ground, no shadow plane, no portal or ring drawn in this asset. Genuine alpha transparency everywhere outside character. No black background, no text, no symbols. The reference image is identity/style guidance, do not retain its sideways flight direction. Camera must be straight in front, cat emerging toward camera.

### Thruster correction

Use case: precise-object-edit. Edit this transparent Gatto character cutout. Keep exactly the same cat identity, face, head-on gaze toward viewer, leaning-forward flight pose, paw positions, indigo/violet colors, polished dimensional illustration, canvas and optical centering. CORRECT ONLY THE JETPACK DIRECTION. The cat is flying OUT OF the screen toward the viewer. Its jetpack is on its BACK, behind its shoulders. The rocket thrust must point AWAY FROM THE VIEWER, receding BEHIND the cat into depth. Current image incorrectly shows both open exhaust nozzles and flames pointing down and toward the camera. Reverse the physical orientation of both engines: show their closed rounded purple-silver FRONT caps beside/behind the shoulders facing the camera, with cyan bands wrapping the body. Their open exhaust nozzles must face the distant background, completely behind the cat and not visible head-on. Remove the two large flames hanging down beside the front paws. Instead show small foreshortened cyan/violet/gold exhaust trails peeking out BEHIND the shoulders/ears and tapering UPWARD and BACK into depth, mostly occluded by the cat's head and engine bodies. The flames must be clearly BEHIND the character and never in front of paws or face. Natural perspective, no extra rockets. Preserve the fully transparent background, no portal, no backdrop, no text. Keep complete character within square canvas.

### Remove visible flames

Use case: precise-object-edit. Remove ONLY the two visible rocket exhaust flames sticking up behind the ears/shoulders of this transparent jetpack cat. Replace all flame pixels with genuine transparency. Keep the jetpack itself fully intact, including rounded purple-silver front caps, silver body, cyan bands, straps, and its placement on the cat's back. Exhaust nozzles point away from camera and their flames are entirely hidden behind the cat in depth, so NO visible flame or smoke anywhere. Preserve EXACTLY the same forward-facing cat identity, pose, head, ears, eyes, whiskers, paws, tail, dimensional polish, indigo-violet colors, framing, scale, and transparent background. No portal, no new objects, no background. This is a localized flame removal only.

## Generation prompt

Use case: stylized-concept. Asset type: isolated transparent website illustration
layer, square composition. Create a magical interdimensional portal ring inspired
by the spinning golden spark portals of Doctor Strange, original artwork with no
symbols or franchise emblems. Front-facing circular aperture, centered exactly,
generous transparent margin. A delicate energetic ring of fine molten gold and
amber sparks, luminous ivory hot inner edge, small tangential sparks swirling
clockwise, subtle painterly texture that complements refined storybook
illustrations. Ring occupies about 75 percent of square width. BOTH the entire
center hole and the entire outside must be fully transparent alpha, including no
background, no black disk, no scene inside the portal, no haze filling the hole.
Only the narrow gold ring and its small sparks are visible. No cat, no people, no
text, no border, no ground shadow. Balanced circular shape suitable for seamless
CSS rotation behind a separate jetpack cat.

The result fills more of the canvas than the requested margin; the composition
reserves its own space around the artwork.

## Exports

From the repository root with libwebp installed:

```sh
cwebp -quiet -lossless -z 9 web/assets/images/download-portal-pixel.png -o web/assets/images/download-portal-pixel.webp
cwebp -quiet -q 90 -m 6 -resize 1024 1024 web/assets/images/portal-watercolor-paper.png -o web/assets/images/portal-watercolor-paper.webp
```

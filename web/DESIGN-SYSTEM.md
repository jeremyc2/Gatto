# Gatto design system

The design language is independent type, warm paper, green ink, restrained
controls, and detailed illustrations. Preserve the approved website while making
these choices reusable. Add a shared abstraction when multiple consumers need
one contract; keep individual compositions local.

Landing-page copy stays brief and focused on sharing. The existing three-item
feature row covers batches, annotations, and side-by-side comparison; video
support appears in the sharing copy. Keep tool lists, file-format lists, and
advanced workflows out of this page.

Read the copy in scroll order when editing it. State the app's purpose once,
then let each section add something. Cut repeated explanations before reaching
for synonyms. Use ordinary product terms where they help people act; avoid
stock marketing promises, forced contrasts, and giving every line the same
clipped slogan rhythm. The license link in the footer supplies the license
information without repeating the download section's open-source claim.

## Structure

| Layer             | Location                                              | Responsibility                                                                                                                  |
| ----------------- | ----------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| Tokens            | `src/design-system/tokens.stylex.ts`                  | Semantic colors, type roles, fixed spacing and radii, viewport conditions, browser preferences and shared layout dimensions.    |
| Primitives        | `src/design-system/`                                  | `Container`, `Heading`, `Text`, `Link`, `Button`, `MediaFrame` and `VisuallyHidden`, with adjacent `*.stylex.ts` style modules. |
| Compositions      | `src/components/`                                     | Header, brand, features, download section and footer. These compose primitives and own their specific layout.                   |
| Story             | `src/Story.rx`, `src/story-layout.stylex.ts`          | Pinned scenes, responsive story geometry and chapter state. These details do not belong to the shared token scale.              |
| Document          | `src/design-system/document.stylex.ts`, `src/Page.rx` | Page canvas and browser preferences. The client imports the foundation for build-time CSS extraction.                           |
| Font registration | `src/document.css`                                    | Only the two bundled `@font-face` declarations. Font licenses remain in the HTML.                                               |

## Using the system

```tsx
<Container>
  <Heading as="h2" size="title">
    A useful heading
  </Heading>
  <Text muted>A brief explanation.</Text>
  <Link href="/guide">Read the guide</Link>
</Container>
```

Headings have `display`, `headline`, `title` and `subheading` sizes, independent
of their semantic HTML level. Text has `body`, `lead`, `small` and `caption`
sizes, plus a muted tone. Buttons have `quiet` and `selected` variants. Their
native disabled state, keyboard focus and touch behavior belong to the primitive.

Text links always use an underline with a 5px offset. `Link` adds a small ↗ arrow
to destinations beyond the current page; do not include arrows in link copy.
Absolute HTTP(S) and protocol-relative links open in a new tab with
`noopener noreferrer`, with the new-tab behavior announced to screen readers.
Fragment links stay in the current tab and have no arrow. The `graphic` variant
omits both underline and arrow for the home brand and download badges.
The `icon` variant also omits both decorations and provides a
44px minimum target. Use it for the header's 24px GitHub SVG, with an accessible
GitHub label and the inherited ink color in either appearance. GitHub appears
only in the header; the footer keeps Support and MIT license.
Both header and footer wordmarks include the same spaced green dot at every
viewport size, using the shared brand component and accent color.

The optional `xstyle` prop accepts typed StyleX styles for composition-specific
spacing or layout. Prefer an existing variant for a shared visual decision. When
another consumer needs a new role, add a named variant rather than reproducing
its typography or interaction rules in the page.

Use `stylex.defineVars` for themeable or responsive values and `defineConsts` for
fixed scales and media conditions. Constants are inlined and emit no custom
properties. Components consume semantic roles such as `colors.text` or
`colors.border`; palette values live in the token file. Use dynamic styles only for values that
need to change at runtime, such as footprint placement. Prefer native CSS scroll
timelines for decorative progress.

Author `stylex.create` in the consuming component's adjacent `*.stylex.ts` module.
ReactiveScript owns the `.rx` view and equations; StyleX's Vite integration extracts
these ordinary TypeScript style declarations. Keep each style map local to its
component rather than creating one giant global catalog. `attrs` composes these
extracted rules at the view boundary.
Repeated declarations across components are deduplicated by the compiler, so a
catalog of hand-written utility classes is unnecessary. Production classes are
hashed atomic rules; development includes readable source labels. Do not couple
markup, tests or JavaScript behavior to generated class names.

Keep property ownership clear. Prefer individual logical properties for spacing,
and explicit border width/style/color. The default StyleX resolution makes a
longhand more specific than its shorthand. Avoid overlapping shorthand overrides
that depend on application order. StyleX validation runs inside Oxlint, including
`valid-styles`, `valid-shorthands` and `no-unused`.
Bun 1.4.4's native formatter owns formatting through the root `bunfig.toml`,
with a 100-column width and exclusions for generated output and artwork.
Oxlint keeps semantic and StyleX checks because stable Bun has no native linter.

Viewport conditions that select different values for one responsive token use
exclusive ranges where practical. This prevents a long chain of overlapping
media overrides from expanding into large boolean selector combinations. Do not
move every one-off measurement into a global variable; story geometry stays local.

## Build and CSS budget

The official `@stylexjs/unplugin/vite` integration extracts the styles; the official
`@solidjs/vite-plugin` 3.0.1 loads `@reactivescript/compiler` automatically for
`.rx` modules and compiles the remaining Solid entry points. The development stylesheet
uses StyleX's CSS-only hot-reload module. Production extracts and minifies one
stylesheet with cascade layers, without runtime injection. Vite serves fonts as
separate hashed files, which can be cached independently of the stylesheet.

All site components, including the full `Page.rx` document, are authored in
ReactiveScript. `entry-client.ts` hydrates its compiled program; `entry-server.ts`
streams it with Vite's asset manifest and the request's nonce and abort signal.
`reactivescript.ts` creates one runtime per root and passes JSX children directly
to Solid on both targets: ReactiveScript 0.3.1's default `show` serializes text-only
arrays and double-escapes SSR strings. Keep this compatibility boundary covered by
the compiled-rendering regression test and manual hydration checks until the upstream runtime handles JSX
children directly. No `.rx` module is declared as `any`; `?program` types only the
compiler's JavaScript factory boundary.

Use Solid's native `ref` attribute to capture component-owned elements. In
ReactiveScript, bind it to a named action that writes a typed source:

```rx
source video: HTMLVideoElement | undefined := undefined
action captureVideo(element: HTMLVideoElement) { video := element }
<video ref={captureVideo} />
```

Keep DOM handles out of module-level state. Effects that depend on a handle list
it in their inputs and wait until it is attached. Put setup and cleanup in those
owned effects; a native ref callback only captures the element. `Button` and
`SiteHeader` forward an optional typed callback ref and clear it
with `undefined` when their component is removed. IDs remain for fragment links, CSS and ARIA
relationships; component behavior receives element references rather than finding
them through IDs or selectors.

`Story.rx` owns the active chapter, the observed browser frame and video readiness,
along with refs for its section, stage, video, chapter articles, copy and controls.
The application passes the header ref into the story. The observer starts only
after every required ref is attached, and its cleanup runs when refs change or
the story is removed. Chapter selection passes directly through its callback
contract; it does not dispatch a DOM event to find its own observer. Focus
synchronization uses the captured chapter/control refs and `document.activeElement`.
Pinned state, copy height, terminal state and live status are equations.
`story-browser.ts` owns native layout reads, event listeners, View Transitions and
autoplay, with no Solid state or effects. Its cleanup belongs to the `.rx` ref-dependent
effect. The hidden primitives have explicit span, heading and status roots so
ReactiveScript can verify HTML nesting through component boundaries.

Vite builds both the browser and server graphs. A small Bun script renders the
root response into static HTML, so all three scenes remain readable without
JavaScript. CSS selects the pinned story before hydration on script-enabled screens taller
than 500 CSS pixels. Hydration updates its chapter state without changing its
geometry. The root color-scheme controls the light and dark tokens. The initial chapter update is
scheduled for the next animation frame so focus synchronization can flush outside
Solid's setup effect. The sticky range includes a final paw trail after Share's
arrival. The three trail legs cover 2%–31%, 33%–69% and 71%–100% of the pinned
range; Share remains visible through the last leg in either scroll direction.
Chapter controls stop at their marker before the next trail begins. A fourth,
unnumbered circle is decorative and reveals a large, still rainbow pawprint at
sticky release. It has no label or button behavior. A named inline-size container
on the tracker wrapper keeps the chapter controls visible while at least 740px
is available, leaving room for the labels and five prints per gap. Below 740px
(currently a viewport narrower than 796px), the controls disappear, leaving
three connected paw trails and the final circle across the available width.
The tracker fills its container up to 960px,
then stays centered so wider screens do not add trail length or prints.
Each gap fits up to twelve prints, with
exclusive container ranges keeping the visible count and fade timing aligned.
This replaces the release-at-Share behavior recorded on 2026-10-17 in ADR 0002.
Chapter text changes use native View Transitions with directional
transforms; reduced motion and unsupported
browsers update immediately. `src/story-transitions.css` owns the document-level
snapshot pseudo-elements and keyframes that cannot be scoped to component
selectors. It also sets the route’s `animation-range`, which StyleX 0.19’s
property validator does not yet accept.

## Portal finale

`PortalCat.rx` reserves a responsive stage between the download and footer.
A Canvas 2D surface spans the stage and footer, using adjacent section colors
from StyleX. Embers can continue below the stage, pass behind the native footer
content, and fade before the bottom of the page. The stage and footer keep
stable positions in the document throughout playback. The stage height and
available width determine the aperture and hit target. The paper lips also
clear the bright rim at their tightest point. The page root disables vertical overscroll
bounce at its boundaries.

`portal-browser.ts` loads the scene as the stage approaches the viewport.
An IntersectionObserver starts the animation when the stage is fully in view.
A requestAnimationFrame clock runs the opening shot once and keeps the
settled scene moving at up to 30 frames per second. It pauses when the scene or
document leaves view. The icon-only pause button works with keyboard, pointer,
and touch. Resize and appearance changes repaint the current composition.
Reduced motion draws a still of the open portal. Without JavaScript or Canvas
2D, the stage shows its two-tone background and the native Rooftops link still
works. The link takes pointer input when the aperture
opens and retains its circular keyboard focus indicator.

The closed seam uses the hand-drawn contour of the original paper edge.
Its opposing faces separate at different rates into a tear wider at the center.
Fine torn fibers and soft contact shadows catch the light at the edges. Small
fragments tumble from the seam and fall under gravity. Warm ochre and olive
undertones bring the sandstone closer to the courtyard’s plaster. The
[Aseprite paper master](../docs/design/art/portal-paper/artwork.md) is
256 × 256 and exports at 2×. Canvas repeats the 512 px lossless WebP at its
native size so the grain stays consistent across viewport sizes.

Rotating filaments gather first. Overlapping ribbons then turn them into a
continuous white-gold rim with a glowing orange fringe and moving embers.
Detached sparks leave tangentially, curve downward, and cool. Short radial
sparks also fan upward and sideways.
A soft amber spill lights the cut. The opening is a cropped view of the actual
Rooftops harbor. It frames the skyline, bridge and water before the title has
appeared. A diagonal
crossing plane reveals the foreground portion over the sparks. Inside the
aperture, Canvas draws the complete cel underneath so that the clipping edge
stays invisible across the fur. One continuous flight curve brings it out at a steady
speed with short acceleration and braking at the ends. The cat holds its
approved orientation and drifts subtly after arrival. Its flame comes from six attached Aseprite poses sampled
with nearest-neighbor scaling. The original film’s visual effects
team used animated guides, separate spark motion, and light from the particles
to illuminate the foreground. [RISE and SideFX breakdown](https://www.sidefx.com/community/rise-fx-doctor-strange/) ·
[RISE VFX supervisor interview](https://www.artofvfx.com/doctor-strange-florian-gellinger-vfx-supervisor-rise-visual-effects-studios/).

The cat flies straight out of the middle of the portal, its face toward the
viewer at a front three-quarter angle that shows the pack and plumes.
Activating the portal banks it away from the viewer through eighteen poses
rendered from the visitor's model, through profile to a rear view. Each cel is
enlarged by a whole number with nearest-neighbour sampling, then drawn with
smoothing, so pixels stay even while the cat moves and scales. The return holds the rear angle while both
flames flicker. The cat flies forward through the opening and
brakes at a fixed point in the harbor. The camera continues forward until it
draws level with the cat. Perspective takes the stationary cat out of view
as the camera looks ahead. The camera enlarges the opening in front of the harbor. The harbor has its
own framing so the visible portion grows until the renderer fills the viewport
at the menu’s native scale. Its projection keeps every on-screen part of the
aperture covered throughout the move.

The portal and Rooftops use the same harbor renderer and opening aim.
The final frame’s harbor time passes once through session storage so the
first game frame matches it. A same-origin View Transition holds the outgoing
frame while the render-blocking game initializes. The harbor stays visible
through the handoff. The Rooftops title and native menu controls fade in over
it. On mobile the landscape and fullscreen prompt appears when the player
selects Play. The native link handles reduced motion, modified clicks, and
pages without Canvas 2D.

The footer's icon-only radio group offers System, Light, and Dark. An early,
nonce-aware script applies a saved preference to the color-scheme meta tag
before first paint. StyleX tokens use `light-dark()`. Canvas reads that same
preference and repaints when the tag changes. Returning to System clears the
override and follows the OS.

The renderer caches the viewport-resolution harbor frame, spark glow, and
settled sandstone. While the portal is visible it samples the harbor at 18 fps.
It groups particle strokes by color and opacity and redraws only the active
area of the stage after the seam opens. The backing store is capped at about
1.65 million pixels. The production finale is a lazy-loaded chunk that shares
scenic drawing with Rooftops. The game’s simulation, controls, and Web Audio
remain in the standalone `/rooftops/` build. Its interactive dialogs and buttons
use StyleX (`web/src/rooftops/rooftops.stylex.ts`) with native top-layer and
range/checkbox pseudo-elements in `web/src/rooftops/rooftops-controls.css`.

For visual verification, jump directly to the page bottom and check that the
crack precedes ignition and the cat clears the rim. Scroll away during playback
and return to check that it resumes in place. Check sustained ember motion
after the arrival and pause it from the keyboard. Check a short landscape
viewport and a high-DPR phone for rim clearance. Verify the theme control in
both OS appearances. During the fly-through, check that the cat slows beyond
the opening and track the same bridge towers through the opening and into the menu.
Check the mobile Play prompt, reduced
motion, and the native link with JavaScript disabled.

Run `bun build:web` and `bun size:web` to measure the actual stylesheet links in
`web/dist/client/index.html`. Bun reports raw bytes and gzip bytes. The build test
limits the combined CSS to **24 KiB raw and 7 KiB gzip**, leaving room above the
current page while catching large regressions. Raise a budget only with a measured
reason. Font and image payloads are separate from this CSS budget.

The 2026-11-21 fly-through build ships one main-page stylesheet with
**20,212 raw bytes and 5,985 gzip bytes**, measured with
`bun build:web && bun size:web`. The continuous finale build measured
19,048 raw bytes and 5,750 gzip bytes. The initial Canvas 2D build measured
18,781 raw bytes and 5,668 gzip bytes.

Rechecked on 2026-11-21 after replacing the portal notice with the split `/rooftops/`
page: one production stylesheet on the main page, **22,843 raw bytes and 6,736 gzip
bytes**, measured with `bun build:web && bun size:web`, well within the 24 KiB / 7 KiB limits.

The earlier 2026-11-21 build with the split `/rooftops/` page measured
22,843 raw bytes and 6,736 gzip bytes.

Historical measurements use Vite 8.3.4 and StyleX 0.19.1.

Rechecked on 2026-10-22 after removing Toastify: the production stylesheet remains
23,326 raw bytes and 6,971 gzip bytes with the same Vite/StyleX versions. Removing
the toast dependency changes its implementation, not the approved styling.

Rechecked on 2026-10-23 after removing the file-format footnote: 23,300 raw
bytes and 6,959 gzip bytes, within the same budgets.

The subsequent copy pass removes the duplicate footer license sentence and its
unused styles: 23,135 raw bytes and 6,910 gzip bytes, measured the same day.

| Build                                                                     | Styling bytes | Gzip bytes |
| ------------------------------------------------------------------------- | ------------: | ---------: |
| Initial direct StyleX translation, before Vite/minification               |        19,218 |      4,233 |
| Refactored system, all production CSS including font declarations         |        11,497 |      3,967 |
| Native snapshot transitions and connected chapter tracker                 |        16,369 |      4,969 |
| Responsive scene height correction                                        |        16,360 |      4,963 |
| Approaching cat, feathered void and soft brush seam composition           |        20,316 |      6,122 |
| Portal button and themed in-page notifications                            |        21,166 |      6,380 |
| Contrasting notification surface and top-right slide entry                |        21,576 |      6,528 |
| Story release at Share with continuous paw fade intervals                 |        21,632 |      6,535 |
| Deeper layered paper tears and contour shadows                            |        22,177 |      6,688 |
| Remove the chapter tracker's top divider                                  |        22,047 |      6,657 |
| Complementary tear masks with a continuous paper setting                  |        22,322 |      6,704 |
| Final rainbow paw marker and compact continuous trails (2026-10-18)       |        23,208 |      6,950 |
| Keep chapter controls until the tracker runs out of space                 |        23,326 |      6,971 |
| Continuous story video and measured copy slot (2026-10-24)                |        23,274 |      6,956 |
| Equal-height chapter snapshots without text overlap (2026-10-25)          |        23,476 |      6,986 |
| Rounded story frame and inset canvas blend (2026-10-25)                   |        23,781 |      7,050 |
| Consistent links, header GitHub icon and matching brand dots (2026-10-26) |        23,674 |      7,032 |
| Feature screenshots with a shared media frame (2026-10-26)                |        23,858 |      7,100 |

The original CSS finale added 3,956 raw bytes and 1,159 gzip bytes over the responsive scene
correction. The independent cat approach and feathered void added 324 raw bytes
and 154 gzip bytes over the brush-mask composition. The portal button, collapsed
focus handling and notification styles added 850 raw bytes and 258 gzip bytes.
The budgets increase to 22 KiB raw and 7 KiB gzip for this measured addition.
The contrasting notification treatment and slide add another 410 raw bytes and
148 gzip bytes, remaining within those budgets.
The story release geometry and retimed paw intervals add 56 raw bytes and 7
gzip bytes.
The larger tear masks, paper underside and contour shadows add 545 raw bytes and
153 gzip bytes, still within the same budget.
Removing the chapter tracker's divider saves 130 raw bytes and 31 gzip bytes.
Complementary masks and the shared paper setting add 275 raw bytes and 47 gzip
bytes, remaining within the same budget.
The final paw marker, expanded print density and centered compact tracker add
886 raw bytes and 246 gzip bytes. Raise the raw budget to 23 KiB for this addition;
the gzip budget remains 7 KiB.
Sizing the compact switch from the tracker container adds 118 raw bytes and
21 gzip bytes, within the same budgets.
Equal-height copy snapshots add 202 raw bytes and 30 gzip bytes over the
continuous-video build, remaining within the same budgets.
The rounded story frame, inset canvas blend and aspect-ratio constraints add
305 raw bytes and 64 gzip bytes. Raise the raw budget to 24 KiB for this addition;
the gzip budget remains 7 KiB.

The initial figure excludes its separate stylesheet with embedded font data;
this is a comparison of generated styling, not total page transfer. The change
reflects both the source refactor and production minification. Refresh this dated
measurement after intentional design-system or build changes.

## Feature screenshots

The three feature columns use real Gatto screenshots above their existing copy,
replacing the short dividers. Bring a batch uses the three-file upload progress
view; Make your point uses the annotation overview; Side by side uses the
combined comparison open in the editor, with matching Aseprite views of the
orange and black gardener and captions “orange cat” and “black cat”. The pose,
outfit and shovel match; the black coat has a light mouth for contrast. There
are no arrows or other annotations in the comparison.
All three sources are also published in the walkthrough. Keep the approved views.

`MediaFrame` owns the shared 20px corners and the video’s exact canvas-colored
edge blend: a 1px inner rim plus a 12px blur with 2px inset spread. Both story
media and feature screenshots use this primitive so the treatment stays in sync.
The screenshot frame reserves a 4:3 landscape area before loading. The batch
capture uses a 1024 × 768 app window to show the current sidebar and all three
attachment rows. All screenshots use `object-fit: contain` against
`colors.mediaSurface`, preserving their full interface. On phones, cards stack and stop at 530px
wide. Verify the row and stacked layouts, the visible batch rows, both comparison
panels and captions, and the editor tools after changing screenshots or spacing.

After regenerating walkthrough PNGs with `./scripts/update-walkthrough-screenshots.sh`,
run `bun export:features`. It uses the pinned Bun runtime's `Bun.Image` API to export
620px and 1240px WebPs from the committed unframed PNGs; documentation's `framed/`
copies are separate. The website's `MediaFrame` supplies its own edge treatment.
The page uses `srcset` with matching `sizes`, native lazy loading, asynchronous
decoding and explicit image dimensions. Asset exports are committed; ordinary
website builds do not need Rust or separate image converters. Lossless WebP
encoding and aspect-preserving screenshot resizing use Bun's bundled codecs.

`bun deploy:web` and app releases run the complete `bun refresh:assets` pipeline:
artwork/icon exports, story video and stills, native captures, framed installer
preview, then feature WebPs. Deployment builds only after these exports finish;
failures prevent deployment. Ordinary `bun build:web` still uses committed exports.

## Download badge

The website and README share `docs/images/download-macos.svg`, a 380 × 64
vector badge. The label is “Download for macOS”, following
[Apple's spelling](https://support.apple.com/en-us/102662). Keep the existing pale
blue fill, black outline and rounded corners. This badge uses smooth vector art.

The Apple mark is 24 px high, with a 12 px gap before the label. The visible
logo-plus-label group is centered at x=190; both visible shapes center at y=32.
The system semibold 22 px label is outlined, so font fallback and SVG baseline
interpretation cannot shift its alignment between the website and GitHub's
README renderer. Preserve the accessible SVG title and image alt text. Copy
changes must update both the title and outlined lettering. Check the actual
visible bounds rather than the old independently positioned text and icon boxes.

## README identity and favicon artwork

The README uses a seated charcoal kitten with cyan and violet eyes, lavender
inner ears, rounded paws and a curled tail. The favicon uses the same face on
warm paper, redrawn separately at 16 and 32 pixels. The README cat's two neutral
gray whiskers per cheek fan from beside the muzzle, below and clear of the eyes.
The favicon omits whiskers: even shorter muzzle-rooted trials looked like stray
dashes at native tab sizes. Keep that simplification in its larger raster exports.
Light falls from the upper left, shading the right cheek and the body beneath
the chin while picking out the forehead, left cheek and upper paw planes.
These identity assets contain
no portal or jetpack. The [artwork record](../docs/design/explorations/art/brand-cat/artwork.md)
owns their editable Aseprite sources and export instructions. This direction
supersedes the SVG treatment in ADR 0003 at the user's request on 2026-10-25.

`Page.rx` declares the native 16 px and 32 px PNGs, a 96 px PNG, and an ICO
with 16, 32 and 64 px entries. The Apple touch icon places a 160 px integer-scale
portrait in an opaque 180 px warm-paper canvas. Keep these exports in sync;
the smallest portrait is an optical redraw rather than a downsample.
The Open Graph and Twitter preview uses the same 32 px portrait via its 96 px
export, enlarged with nearest-neighbor sampling to 480 px on a 1200 × 630
warm-paper canvas. Its separate `social-image-pixel.png` URL prevents cached
previews from retaining the former SVG-based card.

The website's flying visitor is separate artwork. One model in
[`scripts/render-portal-cat.ts`](../scripts/render-portal-cat.ts) renders the
[256 px thrust master](../docs/design/explorations/art/portal-cat/portal-cat.aseprite)
and the 320 px turn master, so every angle keeps the same anatomy, harness,
thrusters and exhaust. The compact Pocket Kitten has an oversized rounded head,
a black-purple coat, the approved cyan/violet eyes and a two-pixel dark outline around the cat and harness.
It flies in a stretched pose: forelegs reach forward under the chin, while the
hind legs and tail trail straight back. Two identical barrels sit side by side
along the spine with a chest strap and buckle. Their layered white-to-cyan
teardrop plumes start at the nozzles and keep their on-screen length at every
angle. Light stays fixed in world space as the cat turns. The portal ring is
drawn entirely by the finale’s moving sparks. The
[visitor artwork record](../docs/design/explorations/art/portal-cat/artwork.md)
owns exports and anatomy guidance.

## Scroll-story artwork

The story is a 1152 × 768, 131-frame pixel-art courtyard animation at 12 fps.
Watercolor pigment, folded matte cardstock and dense charcoal fur distinguish
its three cats. All share one warm upper-left light, with no pillow shading,
underside glow or changing coat colors. Preserve body proportions, connected
ears and tail roots, consistent tail length/girth, and the distinct materials.

The kitten discovers the pansy picture, clears the carrier’s head, then holds
planted paws when it stops. The paper cat picks up the rigid picture in its mouth,
carries it in front of its body, and lowers it into a grounded sage GitHub box.
A small lip overlaps its straight top edge; the rim occludes its landing. Paws
stay outside the box. The picture settles into floor perspective without
shrinking. The round cat listens, turns, moves its tail and blinks once.
The scene ends immediately after that blink, with reopened eyes, at 10.916667
seconds. Each cat has a distinct motion rhythm.

The round cat’s violet eye matches the blue eye’s bright, saturated iris while
retaining its pupil and catchlights. Its tail has a roughly 30-pixel native
midshaft, tapering from a 36-pixel root, at the existing 0.95× scene scale.
Continuous bends and changing depth produce extension, lift and retreat with
smooth transitions. It curls around the lower haunch, reaches toward the viewer,
recedes behind the body and swings high beside the back. Foreshortening,
perspective and correct front/behind overlaps make the depth visible. Keep
projected turns broad even when looking along the tail. Each pose keeps its
pelvic attachment covered by the rump.
The charcoal fur ramp stays consistent with the body; the fixed upper-left light
shades each local surface as it turns, with a darker underside and root contact.
Its faint floor shadow softens and fades with height and moves with depth;
front arcs can cast a small shadow on the haunch.
The last bend flows into a smooth, rounded taper without a hooked tip. Resting
wraps hug the lower haunch and paw line, with a tucked end and close contact
shadow; avoid an open loop beside the body.
The round cat's body and head lead its tail: small sniff counterbalance, a curl
after sitting, one moderate lift after looking forward, then a slow return to
the curl after looking left. Bending begins near the root 1–2 frames after the
body cue and the tip follows up to 3 frames later, with the attachment fixed to
the pelvis. Hold between gestures; keep the final tip
response small and finish it before the blink. Avoid continuous waves or extra
flourishes while the body holds a pose. The artwork record owns the exact cues.

One [Aseprite master](../docs/design/explorations/art/scroll-story/journey.aseprite)
contains the nine visible scene layers and a hidden library of current models,
tail poses and prop references. Edit the visible cels directly and keep the
reference library hidden for export. The [artwork record](../docs/design/explorations/art/scroll-story/journey.md)
owns the reference inventory, staging, repair lessons and export workflow.

`Story.rx` owns one muted, inline video shared across chapters. It autoplays
once on page load, independently of scrolling, and remains on its final frame
when playback ends. Chapter copy and paw trails remain scroll-driven.
`story-video.ts` retains only the export dimensions and frame clock; playback
has no scroll mapping, queued seeks or scroll-triggered loading.

Static HTML declares a typed MP4 source, `autoplay`, `muted`, `playsinline`
and `preload="auto"` so loading and playback can begin before hydration.
The video stage is visible from the start, with its poster covering loading;
hiding it while awaiting decoded data can prevent WebKit autoplay. Hydration
sets the live muted property and requests playback, and checks existing readiness
in case the initial loaded-data event already fired. Loaded-data and can-play
events reveal the video over the chapter still.
Video does not support `fetchpriority`; avoid unsupported video preloads or a
separate high-priority fetch that duplicates range requests.

The Cloudflare deployment routes only `/assets/*.mp4` through
`cloudflare/worker.ts`, using the static asset binding for the source bytes.
It supplies `206 Partial Content` with exact `Content-Range` and `Content-Length`
headers, including open-ended and suffix requests. Unsatisfiable ranges return
416; nonmatching `If-Range` validators receive the full file. Other assets keep
direct static serving. Partial requests buffer the bundled video at the edge,
bounded by the build's 8 MiB video budget, then return only the requested bytes
to the browser. This invokes Worker code for MP4 requests and uses the account's
Worker request allowance. Worker logs and traces are enabled.

On 2026-10-25, the deployed asset host returned 200 and all 7,340,120 bytes for
`Range: bytes=0-1`. The browser stayed at frame zero after scrolling. Replaying
that response through the range handler moved the video to the requested
3.834333-second Send frame; direct serving stayed at zero. The deployment probe
in `scripts/check-story-video.ts` checks exact byte ranges at the start, middle
and end of the actual hosted file. Use it after deployments, together with a
cold-load browser scroll check; the Vite preview's built-in ranges concealed this
production failure during the earlier loading-state investigation.
After deployment on the same day, all three hosted range checks returned 206
with the exact requested bytes. Chrome's cache-bypassing Command-Shift-R restored
the video to the current scroll position, and forward and reverse scrolling
continued to select frames.

Short-screen flow, video failures and disabled JavaScript retain the three
lossless chapter stills. The video has no playback controls or tab stop.
Autoplay is requested on page load, including with reduced motion; browser
autoplay policies may still prevent playback. Reduced motion continues to disable
chapter transitions and the other preference-aware decorative motion.
Accessible status changes only at major markers. Each cat, including its tail,
stays in one scene depth layer.

The video and all three fallback stills share a symmetric rounded frame.
The `radius.media` token is 20px, matching the download badge's source corner
radius. A clipped wrapper fits the actual 3:2 artwork, capped at 1.5 times its
height limit on short, wide screens. A noninteractive overlay paints a thin
canvas-colored rim and a narrow inset shadow over the artwork's edges. This
blends the perimeter into either page appearance while preserving crisp detail
inside and the full courtyard composition. All frame styling lives in StyleX.

The chosen edge treatment is preview setting 4 (100%): a 1px inner rim with a
12px blur and 2px inset spread. Development and production use the same fixed
treatment; the comparison picker, alternative strengths and keyboard shortcuts
are removed.

Only the active copy receives the `story-copy` View Transition name. The video
stays outside the snapshot so chapter changes cannot freeze its playback.
The old and new snapshots both fill the animated group's height so their vertical
slides travel the same distance, even when the chapter copy heights differ.
Top-left positioning and `object-fit: none` preserve the text's size while the
group and each snapshot clip their contents, preventing outgoing and incoming
copy from overlapping.
Its overlay shares the article grid, padding, fit and responsive ordering. A
ResizeObserver measures copy height, including font loading and line wrapping,
to keep the illustration above the text on phones. All three flow articles and
meaningful image descriptions remain available without JavaScript.

`bun export:story` reads the saved master, validates its frame clock and hidden
references, and produces H.264/MP4 plus the three WebPs. Temporary frame PNGs are
automatically removed. The build ships no Aseprite sources or reference sheets.
The October 25 export measures 7,340,120 bytes for CRF 24 H.264 with one-second
keyframes and a front-loaded index, within the 8 MiB video budget. Its WebP stills
measure 385,756, 402,070 and 303,774 bytes. Build tests enforce the transfer budget
and index order; the obsolete scroll-seeking runtime tests have been removed.
`bun size:web` measures the stylesheet budget.

## References

- [StyleX principles: co-location, low-cost abstractions and atomic output](https://stylexjs.com/docs/learn/thinking-in-stylex/)
- [Official StyleX Vite integration](https://stylexjs.com/docs/learn/installation/vite/)
- [StyleX DOM attributes for Solid](https://stylexjs.com/docs/api/javascript/attrs/)
- [Solid Vite plugin and start mode](https://github.com/solidjs/solid-vite-plugin)
- [ReactiveScript language and compiler integration](https://github.com/solidjs/reactivescript)
- [Oxlint JavaScript plugin support](https://oxc.rs/docs/guide/usage/linter/js-plugins.html)

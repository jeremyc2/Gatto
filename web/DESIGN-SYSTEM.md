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

| Layer             | Location                                                   | Responsibility                                                                                                               |
| ----------------- | ---------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| Tokens            | `src/design-system/tokens.stylex.ts`                       | Semantic colors, type roles, fixed spacing and radii, viewport conditions, browser preferences and shared layout dimensions. |
| Primitives        | `src/design-system/`                                       | `Container`, `Heading`, `Text`, `Link`, `Button` and `VisuallyHidden`, with styles beside their markup.                      |
| Compositions      | `src/components/`                                          | Header, brand, features, download section and footer. These compose primitives and own their specific layout.                |
| Story             | `src/Story.tsx`, `src/story-layout.stylex.ts`              | Pinned scenes, responsive story geometry and chapter state. These details do not belong to the shared token scale.           |
| Document          | `src/design-system/document.stylex.ts`, `src/Document.tsx` | Page canvas and browser preferences. The client imports the foundation for build-time CSS extraction.                        |
| Font registration | `src/document.css`                                         | Only the two bundled `@font-face` declarations. Font licenses remain in the HTML.                                            |

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

Author `stylex.create` beside its consuming JSX. Local static calls can compile
away; importing one giant style map forces more composition to happen at runtime.
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
`@solidjs/vite-plugin` compiles Solid and provides HMR. The development stylesheet
uses StyleX's CSS-only hot-reload module. Production extracts and minifies one
stylesheet with cascade layers, without runtime injection. Vite serves fonts as
separate hashed files, which can be cached independently of the stylesheet.

Vite builds both the browser and server graphs. A small Bun script renders the
root response into static HTML, so all three scenes remain readable without
JavaScript. CSS selects the pinned story before hydration on script-enabled screens taller
than 500 CSS pixels. Hydration updates its chapter state without changing its
geometry. System appearance is a CSS token condition. The initial chapter update is
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
property validator does not yet accept. The download composition similarly keeps
its feature-detected finale timeline in `src/components/portal-entry.css`, including
the paired section/spacer height animations, the SVG brush's stroke-dashoffset,
and the void expansion and independent cat approach ranges. Rounded overlapping brush caps and a displaced,
feathered mask edge preserve the ring's sparks without a hard angular cut.
The section grows as the compensating spacer shrinks, keeping total scroll
geometry constant. Its appearance, spin and floating animations remain in StyleX.
The imported paper asset URL is passed through an inline custom property; its
repeat size is a component style. `PortalSeam` owns the shared repeating SVG
mask; the download and footer compositions own their respective lower and upper
edge geometry. The 768 × 48 SVG repeat has varied tear depths and a tan paper
underside; phone masks retain its aspect ratio at 640 × 40. Both masks share the
same origin; `scripts/build-portal-seam.ts` derives the footer mask by closing the
same contour along the opposite side, instead of rotating or offsetting it.
The build regenerates this vector asset from the source path. The footer mask's
empty top margin is clipped by 4 pixels to avoid a rasterization hairline before
applying its shadows. The paper halves meet at the same cut;
the colored faces inset 2 pixels (1 on phones) to expose their thickness inward.
Directional drop shadows follow each contour. The finale owns the continuous
paper background, while the footer's background begins below its masked band.
The portal content shifts by half that band to stay centered between the cuts.
The watercolor composition has no shadow. All ordinary component
styling remains in StyleX.

`SiteNotifications` owns a small native DOM notification with a persistent polite
status region. It uses a manual popover for the top layer and
fixed positioning as its fallback. Dedicated notification tokens keep its cream
surface, dark green text, gold border and shadow distinct in either appearance.
It sits in the top right and uses a 320 ms slide on the individual `translate`
property. Reduced motion disables the entry animation. It has no external toast
library or vendor stylesheet. It keeps one active notice and pauses its timer
while hovered or keyboard-focused. Removing the component clears its timer and
notice. The portal's native button
uses a circular focus indicator and stays hidden from keyboard navigation while
the scroll section is collapsed.
Portal activation displays “Gattoverse mini game coming soon.” for six seconds;
explicit dismissal restores focus to its trigger. For notification changes,
check activation, repeated clicks, dismissal, focus and timeout behavior on
desktop and phone widths.

Run `bun build:web` and `bun size:web` to measure the actual stylesheet links in
`web/dist/client/index.html`. Bun reports raw bytes and gzip bytes. The build test
limits the combined CSS to **24 KiB raw and 7 KiB gzip**, leaving room above the
current page while catching large regressions. Raise a budget only with a measured
reason. Font and image payloads are separate from this CSS budget.

Measured through 2026-10-18 with Vite 8.3.4 and StyleX 0.19.1:

Rechecked on 2026-10-22 after removing Toastify: the production stylesheet remains
23,326 raw bytes and 6,971 gzip bytes with the same Vite/StyleX versions. Removing
the toast dependency changes its implementation, not the approved styling.

Rechecked on 2026-10-23 after removing the file-format footnote: 23,300 raw
bytes and 6,959 gzip bytes, within the same budgets.

The subsequent copy pass removes the duplicate footer license sentence and its
unused styles: 23,135 raw bytes and 6,910 gzip bytes, measured the same day.

| Build                                                               | Styling bytes | Gzip bytes |
| ------------------------------------------------------------------- | ------------: | ---------: |
| Initial direct StyleX translation, before Vite/minification         |        19,218 |      4,233 |
| Refactored system, all production CSS including font declarations   |        11,497 |      3,967 |
| Native snapshot transitions and connected chapter tracker           |        16,369 |      4,969 |
| Responsive scene height correction                                  |        16,360 |      4,963 |
| Approaching cat, feathered void and soft brush seam composition     |        20,316 |      6,122 |
| Portal button and themed in-page notifications                      |        21,166 |      6,380 |
| Contrasting notification surface and top-right slide entry          |        21,576 |      6,528 |
| Story release at Share with continuous paw fade intervals           |        21,632 |      6,535 |
| Deeper layered paper tears and contour shadows                      |        22,177 |      6,688 |
| Remove the chapter tracker's top divider                            |        22,047 |      6,657 |
| Complementary tear masks with a continuous paper setting            |        22,322 |      6,704 |
| Final rainbow paw marker and compact continuous trails (2026-10-18) |        23,208 |      6,950 |
| Keep chapter controls until the tracker runs out of space           |        23,326 |      6,971 |
| Continuous story video and measured copy slot (2026-10-24)          |        23,274 |      6,956 |
| Equal-height chapter snapshots without text overlap (2026-10-25)    |        23,476 |      6,986 |
| Rounded story frame and inset canvas blend (2026-10-25)             |        23,781 |      7,050 |

The finale adds 3,956 raw bytes and 1,159 gzip bytes over the responsive scene
correction. The independent cat approach and feathered void add 324 raw bytes
and 154 gzip bytes over the brush-mask composition. The portal button, collapsed
focus handling and notification styles add 850 raw bytes and 258 gzip bytes.
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

`Story.tsx` owns one muted, inline video shared across chapters. It autoplays
once on page load, independently of scrolling, and remains on its final frame
when playback ends. Chapter copy, paw trails and the finale remain scroll-driven.
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
- [Oxlint JavaScript plugin support](https://oxc.rs/docs/guide/usage/linter/js-plugins.html)

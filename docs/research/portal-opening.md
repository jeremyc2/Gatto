# A portal that opens instead of fading

Research date: October 17, 2026 (America/Chicago).

This note supports the existing website portal between the download section and
footer. The user wants the surrounding sections to feel like the edges of a
world being opened, and the portal to form physically as scrolling continues.
The proposals below are art-direction inferences from CSS capabilities, not
behavior prescribed by the standards.

## Current implementation: a rounded brush, not a wedge

After browser review on October 17, the conic-gradient proposal below was
replaced because it left two straight radial cuts through the painted sparks.
The ring is now rendered as an SVG image through a broad stroked-circle mask.
It retains the original bitmap artwork, so the SVG does not replace its texture.

The circle has `pathLength="1"`, one full-length dash and a scroll-driven
dash offset from one to zero. `pathLength` normalizes distance calculations,
including dash lengths. Round caps extend past both endpoints, so the head and
tail overlap before the loop completes rather than meeting on a radial line.
[Path length](https://developer.mozilla.org/en-US/docs/Web/SVG/Reference/Attribute/pathLength),
[round stroke caps](https://developer.mozilla.org/en-US/docs/Web/SVG/Reference/Attribute/stroke-linecap).

A fixed-seed turbulence/displacement filter gives the matte a slightly irregular
edge; a Gaussian blur feathers it. These filters affect the mask, preserving the
painting's sharp detail inside the visible area. The brush band is broad enough
to cover the entire painted rim when complete. The ring image rotates inside the
stationary mask.
[Displacement](https://developer.mozilla.org/en-US/docs/Web/SVG/Reference/Element/feDisplacementMap),
[Gaussian blur](https://developer.mozilla.org/en-US/docs/Web/SVG/Reference/Element/feGaussianBlur).

### Depth revision

The user then requested forward motion and removal of hard edges on the void
and cat. The current implementation removes the circular clip entirely. A
radial gradient keeps the void black in its center, then blends through dark
violet and translucent warm haze into the painted rim. The void scales with the
rim. The cat has an independent foreground wrapper that scales from distant to
its existing final size over 30%–100% of the section opening, with a small upward
translation and subtle focus/brightness cues. Because the approaching cat fits
inside the growing aperture at every stage, no mask needs to cut its body. The
only continuous movement afterward remains its ±1px, 28-second vertical bob;
it never rotates. These are art-direction choices, not standards requirements.
[Radial gradient geometry](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Values/gradient/radial-gradient).

## Initial proposal

1. Keep the existing fixed scroll geometry and named view timeline. As the seam
   opens, reveal the watercolor world between the green download area and footer.
2. Trace the existing painted ring clockwise. Apply a conic-gradient alpha mask
   to an outer ring wrapper, changing its visible angle from zero to one turn.
   A conic gradient distributes stops around its center; registering an angle
   custom property allows the angle to interpolate in keyframes. This reveals
   the actual artwork progressively instead of changing its opacity.
   [Conic gradients](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Values/gradient/conic-gradient),
   [typed custom properties and animated stops](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/At-rules/@property).
3. Once the ring has begun forming, open the black aperture with a circular
   `clip-path` growing from zero radius to its final radius. Clip the cat within
   the same aperture wrapper so the opening uncovers it. A clipping region shows
   only the enclosed pixels, and basic shapes can animate. Keep circle-to-circle
   keyframes rather than changing from `none` to a circle. This creates an iris
   opening rather than an image fading into existence.
   [Clipping and basic shapes](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/clip-path).
4. Finish with the existing very slow ambient ring rotation and slight vertical
   cat bob. The ring mask wrapper and aperture wrapper should be siblings. Put
   rotation on the ring image inside its masked wrapper; put only vertical
   translation on the cat inside the aperture. This separates the construction
   sweep from ambient rotation and keeps the cat upright. Transforms create a
   stacking context and a containing block for positioned descendants, so a
   shared rotating parent would move the cat too.
   [Transforms](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/transform).

A reasonable first tuning is: section reveal begins immediately; the ring trace
starts after a little watercolor is exposed; the aperture opens while the ring
finishes; the final portion settles into the complete composition. Those timings
are design choices to inspect in the browser. Do not add an opacity entrance to
the cat: its appearance should be caused by the aperture uncovering it.

## Implemented tuning

Browser inspection showed a visible gap when a nearly full-size ring surrounded
a smaller opening. The implemented rim therefore expands from zero scale to
full size over the same 20%–90% range as the aperture clip grows from zero to
38% of the scene width. Their boundaries remain aligned while the cat stays at
its existing size. The painted rim traces clockwise over 15%–55% of the section
opening. No opacity entrance is used. These percentages are local art-direction
choices, verified in the live page, rather than requirements from the sources.

A 192 × 16 SVG alpha tile makes the torn-paper edges. The download section owns
only the lower edge and casts a downward shadow; the footer owns only the upper
edge, mirrored with an upward shadow. Their material colors use the existing
theme roles, while the watercolor section owns neither shadow.

## Practical pitfalls

- The `mask` shorthand resets mask size, position and repeat as well as image.
  Prefer an explicit `mask-image` on the tracing wrapper and explicit
  `mask-size: 100% 100%`, `mask-position: center`, `mask-repeat: no-repeat`. Do not
  overwrite a repeating seam texture's mask using a later shorthand.
  [Mask shorthand and initial values](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/mask).
- Register the angle with `syntax: "<angle>"`, `inherits: false` and
  `initial-value: 0deg`. Set its animated value on the masked element itself;
  a noninheriting property will not flow from a different wrapper. MDN's
  registration example demonstrates animated gradient stops.
  [Custom-property registration](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/At-rules/@property).
- Put `animation-timeline` after any `animation` shorthand, which resets the
  timeline to `auto`. Keep the current feature gate and static fallback; a
  named view timeline can drive descendants independently of its subject.
  [Timeline behavior](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/animation-timeline).
- Clip the aperture, not the complete painted ring, so sparks can extend beyond
  the hole. Clipping creates its own stacking context; assign explicit sibling
  ordering so the ring remains in front of the aperture.
  [Clipping and stacking contexts](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/clip-path).
- Under reduced motion, show the final static composition without a long
  construction sequence, zoom, spin or bob. The preference communicates a desire
  to reduce or replace nonessential motion; large scaling can be a trigger.
  [Reduced motion](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/At-rules/@media/prefers-reduced-motion).

## Alternatives considered

A pure expanding circular clip is simpler and remains a useful fallback, but it
can look like a growing sticker unless the painted rim forms in a different
phase. Using a clean SVG circle as the visible artwork would replace the texture
of the painted rim; using it only as a mask, as in the current implementation,
avoids that drawback. A center
slit expanding into an ellipse is a plausible dimensional-rift treatment, though
it changes the current circular portal's visual language. The traced painted
rim plus circular aperture best preserves the existing artwork while giving the
opening a clear physical cause.

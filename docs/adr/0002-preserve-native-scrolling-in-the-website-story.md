---
status: accepted
date: 2026-10-17
---

# Preserve native scrolling in the website story

The illustrated story should be noticeable without slowing visitors who want to
reach the rest of the site. Keep native scrolling without gesture interception
or mandatory snapping, shorten the pinned section, and use the existing chapter
tracker rather than adding scroll instructions or a visible skip control.

Select the pinned geometry in CSS before hydration to avoid the startup layout
jump. Small viewport units keep it stable as mobile browser controls move. With
JavaScript disabled or insufficient screen height, show all chapters in normal
flow so the story remains readable.

Use native View Transitions for directional chapter changes, with immediate
updates for fast scrolling, reduced motion, or unsupported browsers. Pending
snapshot callbacks read the latest chapter so rapid input cannot restore stale
content. Move focus out of a chapter only if its focused control becomes hidden.

Keep the tracker borderless, with cumulative marker colors and individual paw
SVGs in dedicated gaps. Container queries choose the print count from each gap's
actual width, preserving physical spacing instead of stretching a fixed count
across viewport breakpoints. Each visible print gets its own fade-in range.

## Loading trade-offs

Preload the bundled fonts and use `font-display: optional` to prevent late swaps;
a slow connection may retain a fallback font for that visit. Serve lossless WebP
illustrations while retaining the PNG artwork. The exports preserve pixels but
still send full-size images to phones.

## Local evidence

On 2026-10-17, a production-preview load probe at 390 × 844 recorded zero layout
shift and unchanged scene/page positions across hydration. The original scene
moved 17.5 CSS pixels at 624 × 930. These are local observations, not field Core
Web Vitals measurements. Spacing checks at 17 widths from 320 to 1920 CSS pixels
found no overlapping prints or controls and no horizontal overflow.

Summing the three files in each image format gives 7,458,406 PNG bytes versus
5,210,858 WebP bytes, a 30.1% reduction. Browser QA is still needed for layout and
snapshot timing; the Bun tests do not measure them.

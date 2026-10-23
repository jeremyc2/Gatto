# Numbered markers and descriptions

Researched 2026-10-23 for [issue #20](https://github.com/jeremyc2/Gatto/issues/20).
These are design findings and recommendations; implementation behavior belongs
in the [walkthrough](../WALKTHROUGH.md) and
[architecture guide](../ARCHITECTURE.md).

## Primary-source precedents

- **CleanShot:** its “Outline Steps” example uses small, solid pink circles with
  centered white numerals and a subtle shadow. The numbered badges stand beside
  the points of interest without embedding explanatory copy. Its product page
  presents this separately from ordinary markup. This observation comes from
  the first-party example image, not from an inferred feature specification.
  [Product page](https://cleanshot.com/),
  [counter example](https://cleanshot.com/_ipx/f_png%26q_90%26s_380x380/img/home/annotate/counter.png)
- **Shottr:** the official FAQ calls this annotation a “Counter object.” Typing
  a replacement value changes the selected counter, and later counters increment
  from it. This establishes automatic sequence progression as an existing
  screenshot-tool convention; it does not establish how deletion should behave
  in Gatto. [FAQ](https://www.shottr.cc/kb/faq)
- **Snagit:** the Step tool adds sequential numbers or letters and supports
  configurable fill, outline, shape, shadow, and opacity. Its documented cursor
  shows the next sequence value. Its product features page explicitly separates
  numbered markers from shapes and text callouts.
  [Official help, page 83](https://assets.techsmith.com/Downloads/ua-tutorials-snagit-11/Snagit_11.pdf),
  [current feature categories](https://www.techsmith.com/snagit/features/)
- **Ordered-list icon:** Lucide has an established `list-ordered` glyph, tagged
  “number,” “order,” and “queue.” This is a concrete visual vocabulary for the
  attachment's sequence indicator, rather than a letter abbreviation.
  [Official icon](https://lucide.dev/icons/list-ordered)

## Recommended Gatto treatment

Use a compact filled circular badge, a centered numeral, and a thin contrasting
outer halo. The halo keeps the edge legible over both photographs and interface
screenshots. Choose black or white numeral ink from the fill's luminance so
light colors remain readable. Treat these as deliberate Gatto choices, informed
by the precedents above, rather than claims about other products.

The badge's silhouette, solid fill, and numeral distinguish it from ordinary
outline circles and shape labels. Keep it circular as numbers grow; size its
diameter to the measured numeral plus padding. Avoid a speech-bubble tail: a
tail would imply that prose belongs inside the annotation. Only the badge and
its sequence number are rendered into the image.

Start each image's sequence at 1 and increase the stored next value whenever a
new marker is created. Keep descriptions attached to their marker identity and
number. They are metadata, not a shape label. Sequence and undo/deletion rules
must be defined in the issue and verified independently of visual styling.

For the attachment indicator, use the ordered-list silhouette: a short column
of small numerals beside horizontal strokes. Draw it through Gatto's existing
icon system, with the same size and stroke treatment as adjacent attachment
controls. Show it whenever the attachment has markers, including blank descriptions. Its
tooltip can say “Marker descriptions”; the icon itself needs no adjacent text.
This design communicates the exact structure users will receive on copy.

When a marker is selected, reveal a full-width multiline description area along
the bottom of the editor. Identify the selected number there, retain focus while
typing, and keep the input outside the image canvas. This is the requested
notes-pane interaction, not a documented behavior of the screenshot tools above.

Text copy should put each image's existing attachment text first, then its own
numbered descriptions directly below it, without a heading or a “Marker” prefix.
Use every marker's actual number and leave an empty list item when its
description is blank. Images with markers always include their numbered list,
even when all descriptions are blank. Multiple attachments each keep their descriptions with their own
image; descriptions are never collected into a detached global list.

## Implemented treatment

Following interactive feedback, badges use individual palette colors and two
concentric border lines. Border and numeral ink switches between dark and white
for contrast. Shared vector paths keep preview crisp and feed PNG export. A
corner drag scales every marker and its numeral on that image together; new
markers inherit that size. Numbers remain upright even in group transforms.

Each image supports 1 through 99; deleting a marker closes its numbering gap.
The notes pane overlays the bottom of the editor without changing the image's
camera and shows only the textarea, without a numbered label or helper subtext. The native ordered-list glyph is shared by
the tool and attachment indicator. These feedback-driven choices supersede the
initial fixed-color, fixed-size recommendation above.

## Review targets

Inspect badge contrast on light/dark/photographic backgrounds; one- and two-digit numerals; editing at different zooms; attachment icon legibility;
single and batch text copy; blank descriptions; description edits followed by
upload; and exact preview/export agreement. Neither these references nor this
note are a substitute for testing Gatto's rendered output.

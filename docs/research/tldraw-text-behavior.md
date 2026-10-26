# tldraw text behavior for the native image editor

Researched 2026-10-21. Scope: standalone text, geometric shape labels, and arrow
labels in Gatto's Rust image annotator. The user explicitly wants one font family
(Arial), while matching tldraw's interactions and layout as closely as practical.
The user subsequently clarified that text must remain plain text: no bold,
italic, rich formatting, or lists. That is an explicit scope decision, not an
unimplemented expectation of font-family parity.
This note records research and implementation recommendations; it does not claim
that every upstream feature is implemented. Current Gatto instructions belong in
[the walkthrough](../WALKTHROUGH.md), and implementation boundaries in
[the architecture guide](../ARCHITECTURE.md).

The upstream source inspected is tldraw commit
[`e8eb1cf5635d28a2d516f0c0d823ed35f3ea9dc9`](https://github.com/tldraw/tldraw/commit/e8eb1cf5635d28a2d516f0c0d823ed35f3ea9dc9),
dated 2026-10-20. A sparse checkout was inspected without installing JavaScript
dependencies. Official SDK documentation supplements the pinned source; this
research did not assert that the deployment at tldraw.com is the identical build.

## The important distinction

tldraw preserves a chosen text size. A geometric label does not repeatedly shrink
to fit a conservative interior or disappear below an arbitrary legibility
threshold. It wraps inside the shape's rectangular bounds, and the shape grows
or refuses to shrink below the measured label. A standalone text object instead
has either automatic width or an explicitly chosen wrapping width. Arrow labels
have their own path-relative positioning and wrapping policy.
[Geo lifecycle and resize source](https://github.com/tldraw/tldraw/blob/e8eb1cf5635d28a2d516f0c0d823ed35f3ea9dc9/packages/tldraw/src/lib/shapes/geo/GeoShapeUtil.tsx#L524-L714),
[standalone text source](https://github.com/tldraw/tldraw/blob/e8eb1cf5635d28a2d516f0c0d823ed35f3ea9dc9/packages/tldraw/src/lib/shapes/text/TextShapeUtil.tsx#L239-L393),
[arrow label source](https://github.com/tldraw/tldraw/blob/e8eb1cf5635d28a2d516f0c0d823ed35f3ea9dc9/packages/tldraw/src/lib/shapes/arrow/arrowLabel.ts)

## Standalone text

The default `autoSize` mode has no soft wrapping. Explicit newlines still create
lines. Fixed-width mode wraps and derives height from the resulting lines; there
is no independent stored text height. Automatic width has a 16-pixel minimum and
adds one pixel to measured width to avoid rounding accidentally introducing a
wrap. Fixed width is floored to whole pixels and constrained to at least 16.
[Measurement implementation](https://github.com/tldraw/tldraw/blob/e8eb1cf5635d28a2d516f0c0d823ed35f3ea9dc9/packages/tldraw/src/lib/shapes/text/TextShapeUtil.tsx#L364-L392)

The text tool creates automatic text on a click. It intentionally ignores tiny or
quick accidental drags: fixed-width creation requires at least 150 milliseconds
and horizontal movement greater than six times the configured base drag
distance, adjusted for zoom. Once that threshold is crossed, dragging controls
width and release enters editing. The click path vertically centers the empty
line at the pointer; its horizontal anchor follows left/center/right alignment.
Double-clicking empty canvas through Select also creates automatic text, although
that source path initializes the origin directly at the pointer rather than
applying the text tool's empty-line offset.
[Text tool pointing state](https://github.com/tldraw/tldraw/blob/e8eb1cf5635d28a2d516f0c0d823ed35f3ea9dc9/packages/tldraw/src/lib/shapes/text/toolStates/Pointing.ts),
[Select double-click creation](https://github.com/tldraw/tldraw/blob/e8eb1cf5635d28a2d516f0c0d823ed35f3ea9dc9/packages/tldraw/src/lib/tools/SelectTool/childStates/Idle.ts#L707-L739)

The text tool's pointer-down state does not branch on whether the pointer hit an
existing text, geo, or arrow object. Its click completion creates a new
standalone text shape. Enter with an already selected editable host does switch
to Select and edit that host. This differs from treating the text tool as a
universal click-to-edit label tool; Select double-click is the normal explicit
label entry path.
[Text tool idle state](https://github.com/tldraw/tldraw/blob/e8eb1cf5635d28a2d516f0c0d823ed35f3ea9dc9/packages/tldraw/src/lib/shapes/text/toolStates/Idle.ts#L20-L39),
[unconditional click creation](https://github.com/tldraw/tldraw/blob/e8eb1cf5635d28a2d516f0c0d823ed35f3ea9dc9/packages/tldraw/src/lib/shapes/text/toolStates/Pointing.ts#L116-L127)

Left and right side handles change wrapping width and convert automatic text to
fixed width. Other handles scale the complete object proportionally. While
automatic text changes, left alignment holds the left anchor, center alignment
holds the horizontal center, and right alignment holds the right anchor; rotated
text applies that anchor correction in its local coordinates. The commented-out
edge-double-click reset is explicitly disabled upstream, so it should not be
treated as a current automatic-width shortcut.
[Resize and anchoring implementation](https://github.com/tldraw/tldraw/blob/e8eb1cf5635d28a2d516f0c0d823ed35f3ea9dc9/packages/tldraw/src/lib/shapes/text/TextShapeUtil.tsx#L239-L362)

Text enters editing on double-click or Enter while selected. Command/Ctrl+Enter
finishes editing; Enter alone remains text input. Whitespace-only standalone text
is removed when editing ends. The empty test calls `trimEnd`, but does not trim
the stored value of nonempty text: trailing spaces and newlines must not be
silently deleted as an edit-end normalization step. Plain-text normalization
replaces CRLF or CR with LF. Escape finishes editing and restores canvas focus;
it does not discard the typed changes. Upstream supports left/center/right paragraph
alignment; standalone vertical alignment remains centered in the geometry and
the corresponding style-panel button is disabled.
[Text edit-end and keyboard handling](https://github.com/tldraw/tldraw/blob/e8eb1cf5635d28a2d516f0c0d823ed35f3ea9dc9/packages/tldraw/src/lib/shapes/text/TextShapeUtil.tsx#L266-L273),
[shared editing shortcut](https://github.com/tldraw/tldraw/blob/e8eb1cf5635d28a2d516f0c0d823ed35f3ea9dc9/packages/tldraw/src/lib/shapes/shared/useEditableRichText.ts#L29-L40),
[selected Enter behavior](https://github.com/tldraw/tldraw/blob/e8eb1cf5635d28a2d516f0c0d823ed35f3ea9dc9/packages/tldraw/src/lib/tools/SelectTool/childStates/Idle.ts#L650-L700),
[style panel](https://github.com/tldraw/tldraw/blob/e8eb1cf5635d28a2d516f0c0d823ed35f3ea9dc9/packages/tldraw/src/lib/ui/components/StylePanel/DefaultStylePanelContent.tsx#L253-L324),
[plain-text newline normalization](https://github.com/tldraw/tldraw/blob/e8eb1cf5635d28a2d516f0c0d823ed35f3ea9dc9/packages/tldraw/src/lib/shapes/shared/TextHelpers.ts),
[edit completion and cancellation](https://github.com/tldraw/tldraw/blob/e8eb1cf5635d28a2d516f0c0d823ed35f3ea9dc9/packages/tldraw/src/lib/tools/SelectTool/childStates/EditingShape.ts#L241-L251)

## Geometric labels

Label font size comes from a preset, independent of geometric bounds. All geo
types use a rectangular label area with 16 unscaled pixels of padding on each
side. They do not reserve a special central polygon for triangles, hearts,
arrows, or stars. This explains why labels can visually cross a pointed or curved
outline before the bounding rectangle needs to grow. The HTML container clips to
the overall rectangular bounds, not the actual geometric silhouette. Horizontal
alignment is left/center/right; vertical alignment is top/middle/bottom. New geo
labels default to centered horizontally and vertically.
[Display values and default properties](https://github.com/tldraw/tldraw/blob/e8eb1cf5635d28a2d516f0c0d823ed35f3ea9dc9/packages/tldraw/src/lib/shapes/geo/GeoShapeUtil.tsx#L194-L260),
[label rendering container](https://github.com/tldraw/tldraw/blob/e8eb1cf5635d28a2d516f0c0d823ed35f3ea9dc9/packages/tldraw/src/lib/shapes/geo/GeoShapeUtil.tsx#L409-L448)

Measurement uses a minimum inner width of 12/14/16/20 pixels for S/M/L/XL plus
stroke-dependent extra padding, or the current shape width minus 32 pixels,
whichever is larger. The returned required width and height include the outer
padding. Word wrapping can break long words. On first label addition the shape
expands both dimensions to accommodate the label. If both previous dimensions
were below 51 unscaled pixels, it grows to a square of at least 51 pixels.
[Minimum widths](https://github.com/tldraw/tldraw/blob/e8eb1cf5635d28a2d516f0c0d823ed35f3ea9dc9/packages/tldraw/src/lib/shapes/geo/GeoShapeUtil.tsx#L69-L79),
[measurement and first-label expansion](https://github.com/tldraw/tldraw/blob/e8eb1cf5635d28a2d516f0c0d823ed35f3ea9dc9/packages/tldraw/src/lib/shapes/geo/GeoShapeUtil.tsx#L799-L866)

After the first label, the base shape height stays separate from additional
`growY`. Required label height above base height becomes that extra growth. When
text is deleted, extra growth can return to zero, retaining the base height.
Required label width may enlarge the stored width. Clearing all text resets
growth. Resizing measures the label at the proposed dimensions, clamps width and
height to the resulting requirements, adjusts top/left position when the dragged
side hits its minimum, and commits the resulting total height as the new base
height with growth reset. This is a content minimum, not proportional font
shrinking.
[Create/update hooks](https://github.com/tldraw/tldraw/blob/e8eb1cf5635d28a2d516f0c0d823ed35f3ea9dc9/packages/tldraw/src/lib/shapes/geo/GeoShapeUtil.tsx#L613-L713),
[resize constraints](https://github.com/tldraw/tldraw/blob/e8eb1cf5635d28a2d516f0c0d823ed35f3ea9dc9/packages/tldraw/src/lib/shapes/geo/GeoShapeUtil.tsx#L524-L610),
[growth calculation](https://github.com/tldraw/tldraw/blob/e8eb1cf5635d28a2d516f0c0d823ed35f3ea9dc9/packages/tldraw/src/lib/shapes/geo/GeoShapeUtil.tsx#L775-L795)

Gatto previously excluded X boxes and checkboxes from labels. That exclusion is a
Gatto policy rather than a general limit of upstream GeoShapeUtil, which uses
the same label machinery for all its geometric forms. Matching the user's new
request broadly should allow their labels too, unless that older exception is
deliberately retained and documented.
[Shared geo label rendering](https://github.com/tldraw/tldraw/blob/e8eb1cf5635d28a2d516f0c0d823ed35f3ea9dc9/packages/tldraw/src/lib/shapes/geo/GeoShapeUtil.tsx#L395-L448)

## Arrow labels

Arrows are editable text hosts. The label begins halfway along the actual path,
not at the rectangle around its endpoints. Text is centered horizontally and
vertically in its label rectangle and does not rotate to follow the shaft's
tangent; the parent shape transform still applies. Initial editing of an empty
label resets its position to the path default. Moving endpoints or bending the
arrow recalculates the label's position along the new path.
[Arrow defaults and editability](https://github.com/tldraw/tldraw/blob/e8eb1cf5635d28a2d516f0c0d823ed35f3ea9dc9/packages/tldraw/src/lib/shapes/arrow/ArrowShapeUtil.tsx#L197-L260),
[label component](https://github.com/tldraw/tldraw/blob/e8eb1cf5635d28a2d516f0c0d823ed35f3ea9dc9/packages/tldraw/src/lib/shapes/arrow/ArrowShapeUtil.tsx#L846-L887),
[first editing position reset](https://github.com/tldraw/tldraw/blob/e8eb1cf5635d28a2d516f0c0d823ed35f3ea9dc9/packages/tldraw/src/lib/shapes/arrow/ArrowShapeUtil.tsx#L994-L1005)

The label is first measured without wrapping. Mostly horizontal arrows constrain
long text using the body width with a 64-pixel margin, while preserving a minimum
width up to 64 pixels for short arrows. Mostly vertical arrows cap label width at
16 times font size. Text is remeasured with wrapping when constrained; padding
adds 4.25 scaled pixels on each side. Label sizes are cached, and moving only
`labelPosition` does not invalidate measurement.
[Arrow measurement cache](https://github.com/tldraw/tldraw/blob/e8eb1cf5635d28a2d516f0c0d823ed35f3ea9dc9/packages/tldraw/src/lib/shapes/arrow/arrowLabel.ts#L59-L138)

Dragging the label projects its center onto the arrow path, with a 10-screen-pixel
snap back to the default center. It is constrained away from capped or bound
ends using intersections of the label rectangle and path, plus stroke-dependent
padding. The shaft is cut out beneath a rounded label rectangle rather than
covered with a canvas-colored opaque rectangle. For an image editor, this is
particularly important: the photo behind the label must remain visible.
[Drag and center snap](https://github.com/tldraw/tldraw/blob/e8eb1cf5635d28a2d516f0c0d823ed35f3ea9dc9/packages/tldraw/src/lib/tools/SelectTool/childStates/PointingArrowLabel.ts#L79-L143),
[path range and clamping](https://github.com/tldraw/tldraw/blob/e8eb1cf5635d28a2d516f0c0d823ed35f3ea9dc9/packages/tldraw/src/lib/shapes/arrow/arrowLabel.ts#L140-L282),
[shaft label cutout](https://github.com/tldraw/tldraw/blob/e8eb1cf5635d28a2d516f0c0d823ed35f3ea9dc9/packages/tldraw/src/lib/shapes/arrow/ArrowShapeUtil.tsx#L937-L977)

## A shared layout contract

Upstream measurement, static rendering, editing, and export deliberately use the
same text settings. `resolveLineHeightPx` rounds font size times the theme's line
height to whole pixels and documents cross-engine drift as the reason. The
default theme uses base font size 16 and line height 1.35. The resulting default
presets are:

| Text host       |   S |   M |   L |  XL |
| --------------- | --: | --: | --: | --: |
| Standalone      |  18 |  24 |  36 |  44 |
| Geometric label |  18 |  22 |  26 |  32 |
| Arrow label     |  18 |  20 |  24 |  28 |

These are unscaled document pixels, separate from zoom and object scale.
[Default theme](https://github.com/tldraw/tldraw/blob/e8eb1cf5635d28a2d516f0c0d823ed35f3ea9dc9/packages/editor/src/lib/editor/managers/ThemeManager/defaultThemes.ts#L15-L20),
[font preset multipliers](https://github.com/tldraw/tldraw/blob/e8eb1cf5635d28a2d516f0c0d823ed35f3ea9dc9/packages/tldraw/src/lib/shapes/shared/default-shape-constants.ts#L22-L43),
[line-height and whitespace contract](https://github.com/tldraw/tldraw/blob/e8eb1cf5635d28a2d516f0c0d823ed35f3ea9dc9/packages/editor/src/lib/editor/managers/TextManager/TextManager.ts#L6-L73)

The displayed text and editable element share their parent layout and inherited
font, width, line height, alignment, and padding. Static content is hidden while
editing, preventing two copies at slightly different positions. The text and
editing wrappers have transparent backgrounds. A thin background-colored text
outline is distinct from an opaque rectangular field.
[Shared rich text label](https://github.com/tldraw/tldraw/blob/e8eb1cf5635d28a2d516f0c0d823ed35f3ea9dc9/packages/tldraw/src/lib/shapes/shared/RichTextLabel.tsx#L157-L249),
[text CSS including transparency and edit/display visibility](https://github.com/tldraw/tldraw/blob/e8eb1cf5635d28a2d516f0c0d823ed35f3ea9dc9/packages/editor/editor.css#L538-L740)

## Formatting beyond font families

Current tldraw text is rich text, including geometric and arrow labels. Its
toolbar includes bold, italic, code, links, bullets, and highlighting. The
underlying extensions also support underline, strike, headings, lists, paragraph
direction, typography substitution, and selection wrapping. Its document is
structured content rather than one plain string. Omitting multiple font-family
choices does not itself implement or exclude these independent features.
[Official rich-text overview](https://tldraw.dev/sdk-features/rich-text),
[toolbar actions](https://github.com/tldraw/tldraw/blob/e8eb1cf5635d28a2d516f0c0d823ed35f3ea9dc9/packages/tldraw/src/lib/ui/components/Toolbar/DefaultRichTextToolbarContent.tsx)

Gatto intentionally keeps the plain-string model under the user's subsequent
plain-text-only instruction. The target is layout and interaction parity for
plain text, including alignment, wrapping, geometry growth, arrow labels, native
editing, clipboard, undo, and IME composition. Do not introduce rich-text toolbar
actions or structured formatting merely because upstream supports them. These
upstream capabilities remain recorded here to distinguish researched behavior
from the narrower behavior the user authorized.

## Recommended Gatto changes and verification

1. Replace fitting-to-interior with a shared measured text block: lines, baselines,
   advances, wrapping width, chosen size, horizontal and vertical alignment,
   origin, and rotation. Have the editable text field, outlined preview glyphs,
   hit bounds, and exported PNG use that block. Keep empty lines and trailing
   editing space in the line-box calculation, instead of centering only visible
   ink.
2. Store automatic versus fixed-width mode explicitly for standalone text. Use a
   gesture threshold for deliberate width creation, side-handle wrapping, and
   proportional scaling from other handles. Preserve the selected alignment
   anchor as automatic width changes.
3. Give geo labels fixed preset sizes, full rectangular bounds with padding,
   content minima, and reversible extra vertical growth. Keep the font unchanged
   during ordinary geometric resize. Clamp all single and group resize paths
   consistently so text never disappears to satisfy a smaller rectangle.
4. Add arrow label position as a path parameter, automatic width constraints,
   endpoint clearance, drag projection and center snap, hit testing, and a true
   shaft cutout. Do not use opaque rectangles to conceal the shaft on photos.
5. Make the editor transparent, suppress duplicate static glyphs, remove the
   unrelated editing-only width clamp, and calculate font/line spacing/position
   identically before, during, and after editing. Convert canvas coordinates to
   screen coordinates exactly once and apply the same rotation transform.
6. Expose icon controls for horizontal alignment and geo vertical alignment.
   Enter edits a selected host; Command+Enter/Escape finishes editing without
   closing the preview; tool shortcuts must not consume typing while editing.
   Preserve Gatto's explicit user choice to leave newly finished objects
   unselected, returning to Select unless the creating tool is locked. This is
   a deliberate difference from upstream automatic selection.

Verification should cover click and drag text creation in either direction;
accidental tiny drag; explicit newlines and blank lines; long words; all three
horizontal alignments; rotation; side resize versus corner scaling; multiline
editing at several zoom levels; transparent/photo backgrounds; tiny labeled
shapes growing on first input; vertical growth followed by deletion; minimum
width and height from every drag edge and corner; group resizing; straight and
curved arrow labels moving with their paths and remaining clear of caps; empty
label cleanup; retained trailing whitespace in nonempty text; undo/redo;
save/reopen; and preview/export alignment. Include native paste, selection, and
IME checks rather than assuming native text input guarantees parity by itself.

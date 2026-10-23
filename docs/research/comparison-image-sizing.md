# Side-by-side image sizing and tldraw export

Researched 2026-10-23 for Gatto's comparison attachment workflow. This is evidence
and a sizing recommendation, not a claim that every recommendation is implemented.
Current behavior belongs in the [architecture guide](../ARCHITECTURE.md); this
advanced feature is currently omitted from the user walkthrough.

The inspected upstream source is tldraw commit
[`71e9eae0502a06b8299b9a67b881539d28516f0d`](https://github.com/tldraw/tldraw/commit/71e9eae0502a06b8299b9a67b881539d28516f0d).
Official SDK documentation supplements that pinned source. The deployment at
tldraw.com was not measured, and need not match that commit or SDK defaults.

## What two 4K images actually mean

Taking “4K” to mean 3840 × 2160 pixels, two images placed next to each other at
their original size produce 7680 × 2160 pixels before gaps, labels, or padding.
That is 16,588,800 pixels: twice one source's pixel count. It is half the pixel
count of an 8K UHD image, 7680 × 4320. A wide image can therefore have an
“8K-wide” edge without being an 8K UHD frame. These figures are arithmetic,
independent of any particular application's implementation.

An RGBA working buffer for that pair needs about 66.4 MB (63.3 MiB), before source
decodes, intermediate surfaces, annotations, or encoding. A PNG's compressed file
size cannot be inferred reliably from its dimensions or from the sum of two
source files: the combined image is encoded again, and the source formats and
content may differ. File size and decoded working memory are separate budgets.

## What tldraw does

### Imports do not establish a final PNG size budget

The inspected source defines a default image dimension of 5000 and an asset
limit of 10 × 1024 × 1024 bytes. `ImageAssetUtil.getAssetFromFile` fits the image
asset's logical width and height into that dimension. Importantly, the raster
file handler passes the original file to `uploadAsset`; this path does not
automatically encode a smaller raster file. The image's logical size and its
stored pixel data can differ, including for high-DPI screenshots with pixel-ratio
metadata. [Import constants and handler](https://github.com/tldraw/tldraw/blob/71e9eae0502a06b8299b9a67b881539d28516f0d/packages/tldraw/src/lib/defaultExternalContentHandlers.ts#L64-L85),
[original file upload](https://github.com/tldraw/tldraw/blob/71e9eae0502a06b8299b9a67b881539d28516f0d/packages/tldraw/src/lib/defaultExternalContentHandlers.ts#L171-L187),
[image asset sizing](https://github.com/tldraw/tldraw/blob/71e9eae0502a06b8299b9a67b881539d28516f0d/packages/tldraw/src/lib/assets/ImageAssetUtil.ts)

The SDK's options guide lists 5000, while the generated property reference says
the default is infinity. The implementation above resolves that documentation
disagreement for the inspected commit; neither statement proves the deployed
website's configuration. [Options guide](https://tldraw.dev/sdk-features/options),
[property reference](https://tldraw.dev/reference/tldraw/TLExternalContentProps)

### Display and export use different work

The image shape renders through HTML image elements. The asset hook skips URL
resolution updates for culled shapes and debounces resolution changes during
zooming. It selects image URLs using the displayed size and zoom level rather
than re-encoding the entire board for every interaction. These mechanisms explain
why fluid editing does not imply that a subsequent PNG export is small.
[Image shape rendering](https://github.com/tldraw/tldraw/blob/71e9eae0502a06b8299b9a67b881539d28516f0d/packages/tldraw/src/lib/shapes/image/ImageShapeUtil.tsx#L348-L498),
[asset resolution hook](https://github.com/tldraw/tldraw/blob/71e9eae0502a06b8299b9a67b881539d28516f0d/packages/tldraw/src/lib/shapes/shared/useImageOrVideoAsset.ts)

The dotcom asset store can request server image variants based on zoom, device
pixel ratio, source size, and network conditions. It avoids near-native resize
variants when savings would be small. It returns the source when explicitly
asked for original quality. During PNG export, the export context supplies the
requested pixel ratio and shape scale, so a suitable variant can still be used;
SVG export without a pixel ratio requests the original. This is an asset-loading
optimization, not a 1080p output policy.
[Dotcom asset store](https://github.com/tldraw/tldraw/blob/71e9eae0502a06b8299b9a67b881539d28516f0d/apps/dotcom/client/src/utils/multiplayerAssetStore.ts),
[export resolution context](https://github.com/tldraw/tldraw/blob/71e9eae0502a06b8299b9a67b881539d28516f0d/packages/editor/src/lib/exports/getSvgJsx.tsx#L261-L280)

### PNG export can be very large

`Editor.toImage` defaults to PNG, logical scale 1, and raster `pixelRatio: 2`.
The export pipeline gathers the selection's bounds, creates a self-contained
SVG, then draws it into a raster canvas. The raster stage multiplies dimensions
by the pixel ratio and clamps them to browser canvas limits. It is not a promise
to fit a compressed byte budget. [Default export options](https://github.com/tldraw/tldraw/blob/71e9eae0502a06b8299b9a67b881539d28516f0d/packages/editor/src/lib/editor/Editor.ts#L10209-L10215),
[raster conversion implementation](https://github.com/tldraw/tldraw/blob/71e9eae0502a06b8299b9a67b881539d28516f0d/packages/editor/src/lib/exports/getSvgAsImage.ts),
[official export guide](https://tldraw.dev/sdk-features/image-export)

For example, if each 3840 × 2160 source occupies 3840 × 2160 logical canvas
units, a pair exported at the default 2× ratio requests roughly
15360 × 4320 raster pixels, plus decoration and subject to browser clamping.
If those are Retina screenshots represented at half their physical dimensions,
or the shapes were resized on the canvas, the output differs. The user's
over-10-MB result is consistent with this export policy, but it does not establish
the exact settings or dimensions of their particular export.

Copy-as-PNG uses the image export pipeline. Where supported, it writes both a
normal PNG MIME entry and a custom tldraw PNG MIME entry from the same blob,
preserving resolution metadata that browsers might strip. These are not separate
low- and high-resolution images. [PNG clipboard export](https://github.com/tldraw/tldraw/blob/71e9eae0502a06b8299b9a67b881539d28516f0d/packages/tldraw/src/lib/utils/export/copyAs.ts),
[clipboard MIME explanation](https://github.com/tldraw/tldraw/blob/71e9eae0502a06b8299b9a67b881539d28516f0d/packages/tldraw/src/lib/utils/clipboard.ts#L1-L18)

## GitHub's applicable limit

GitHub documents a 10 MB limit for images and GIFs. The 100 MB allowance concerns
video attachments to repositories on eligible paid plans; it does not raise the
PNG limit. A PNG that exceeds the image limit cannot simply upload more slowly.
[GitHub attachment limits](https://docs.github.com/en/get-started/writing-on-github/working-with-advanced-formatting/attaching-files)

## Recommended Gatto policy

These are product and implementation recommendations derived from the use case,
not tldraw defaults:

1. Replace the hard 1920 × 1080 rectangle with a maximum 3840-pixel long edge and
   an 8,000,000-pixel area for the entire comparison, including labels, gap, and
   padding. Preserve aspect ratios and the sources' relative scale. Never enlarge
   either source merely to fill the output budget. A two-4K landscape comparison
   then retains approximately twice the per-side width and four times the detail
   of the old 1920-wide output, while staying around four million output pixels.
2. Use a high-quality resampler when reduction is required. Resize each source
   directly to its final planned dimensions, then assemble it once. Avoid
   repeatedly resizing an already downsized output. PNG keeps screenshot edges
   and text free of JPEG artifacts; its compression setting does not control
   spatial sharpness.
3. Encode the candidate PNG and inspect its actual byte length. Initially target
   at most 9,000,000 bytes, leaving some space for annotations. If it already
   fits, keep that resolution. If it does not, reduce dimensions and retry from
   the source data. A useful first estimate is
   `sqrt(target_bytes / encoded_bytes) × 0.95` for both axes; always verify the
   resulting encoding, because compression is content dependent. Bound retry
   count and fall back to further safe reductions rather than looping forever.
4. On annotation save, render the retained base plus current vector marks and
   protected labels within the same dimension/pixel caps, and check the final
   PNG against 9,500,000 bytes. Apply the same adaptive reduction if necessary.
   The initial spare bytes cannot guarantee that later annotations fit: marks
   can expand bounds and alter compressibility. Ordinary screenshots should not
   shrink again when the final PNG already fits.
5. Run decode, resample, render, and encode away from the UI thread. Show the
   existing attachment loading state in the main window while combining. Keep
   original inputs available until the result is successfully staged, so errors
   or an unacceptable result do not destroy the user's only copy.

The decimal 9.0/9.5 MB budgets avoid depending on whether an endpoint interprets
“10 MB” as decimal MB or binary MiB. The dimensional caps bound working memory;
the measured byte budget protects upload compatibility. Text-heavy screenshots
that compress well retain more pixels than noisy photographs, so this policy
uses the available file budget rather than assuming that every image needs the
same severe reduction.

Some loss is unavoidable when combining large detailed sources into a bounded
PNG. This policy does not promise that every source's small text stays legible;
it preserves materially more detail than the earlier 1080p cap and reduces only
as much as the chosen resource and upload budgets require. Source-resolution
matching and a perfect visual-quality threshold are different goals, and the
compressed byte limit alone cannot decide either one.

## Implemented policy, 2026-10-23

Gatto now uses a 3840 × 2160 rectangle for the complete output (at most
8,294,400 pixels), rather than the long-edge/8 MP variant recommended above.
This gives a straightforward 4K envelope including decoration and annotations.
Both paths use Lanczos3 when resizing is needed and preserve small sources.
Initial encoding targets 9,000,000 bytes; annotation save targets 9,500,000 bytes.
Retries are capped at twelve, with each retry reducing both axes by at least
10 percent and usually more based on the measured byte ratio. Resampling uses
the retained base on every attempt, and staging trusts the renderer's known
PNG dimensions instead of decoding it again.

The picker stacks its slots and hands off automatically when both are filled.
Background processing shows the shared attachment component's processing spinner
in the main window. Original files are never modified. If import validation
fails, the picker keeps the other source; a composition failure in the main
window reports an error, and the user can retry from the original files.

## Validation to run when implementing

Use a screenshot pair, a photo pair, a mixed screenshot/photo pair, mismatched
portrait/landscape sizes, tiny inputs, and an incompressible/noisy fixture. Record
final pixel dimensions, PNG bytes, elapsed combine time, and peak working memory
on a named machine/build. Verify no source upscaling, no repeated save degradation,
readable labels, and an annotation that expands beyond the base image while the
final file remains inside the byte budget. The recommendation above is not a
substitute for those application-specific measurements.

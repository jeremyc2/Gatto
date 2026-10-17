# Website concept

October 16, 2026. [Open the single HTML file](website-concept.html). This is a complete local website proposal, not a deployed site or a framework implementation.

## Decisions carried forward

- Coconat headings and Apfel Grotezk text, both free open-source fonts with licenses embedded in the file.
- The three black cats and the three developing encounters, with the GitHub label painted into the upload box.
- Product copy stays outside the illustrations: one heading and one short paragraph per scene. No image captions, appearance controls, repeated narrative explanations, or questionnaire UI.
- The first view names the job and offers a download. The next views explain uploading to GitHub and copying Markdown. Three brief feature descriptions cover batches, preview/descriptions and Quick Copy.
- The opening and bottom download links use the exact macOS badge image from the README, embedded unchanged. There is no header download button. Supporting destinations appear once: the walkthrough in the header, setup beside requirements, and source/support/license in the footer.
- The upload explanation follows the actual order: choose a repository, paste or drop an image, then upload. Small scrolling hints are omitted.
- Download and setup links lead to the project's real GitHub pages. Compatibility and setup requirements come from the repository's current setup and support guides; no release version, download size, testimonial or performance figure is invented.

## Scrolling and access

The browser scrolls natively through a tall story section. Its stage remains sticky, while the illustration and copy crossfade at chapter boundaries. There is no wheel/touch interception, forced snap or custom scrolling engine. The fade completes even if a visitor stops scrolling between chapters, avoiding a frozen double exposure.

Chapter buttons offer direct navigation, and a skip link bypasses the story. There is no Still view button or alternate reading layout. The operating system's `prefers-reduced-motion` preference disables fades and smooth scrolling while preserving the same JavaScript-driven chapter changes and pinned composition. Short screens use a more compact responsive layout. JavaScript being unavailable leaves the three scenes in normal document flow. Appearance follows the operating system without a visible settings control.

The file embeds all artwork, fonts, CSS and JavaScript, so it can be opened by itself. External links require internet access. The embedded source artwork makes this a comparatively large design artifact; production image delivery and a framework migration are later work, not measured performance claims.

## Organic picture integration

The earlier exact prop overlay was rejected because it sat on top of the artwork and did not meet the paws or fit behind the box rim. The new active images are [story-bring-v3.png](art/story-bring-v3.png), [story-upload-v3.png](art/story-upload-v3.png), and [story-copy-v3.png](art/story-copy-v3.png). The flower remains a shared reference, but the picture is integrated into each painting. Two paws overlap the paper during the handoff; the finished picture lies inside the tray, behind its rim. These replace the overlay in the current studies as well.

The user approved the resulting integrated images. Preserve them while refining the website. Small painted differences remain; these are not pixel-identical props or rigid 3D models. The source images and prior overlay experiment remain preserved. The [organic integration prompt record](art/organic-integration.md) records the source outputs and exact instructions.

## Research basis

The existing [website research](../../research/distinctive-website-design.md) informs concise factual copy, a direct download route, optional motion, and a subject-specific composition. [Font and reference notes](../../research/moodboard-reference-notes.md) own font provenance and source observations. [Mixed-media research](../../research/mixed-media-character-worlds.md) supports the material and staging hypotheses. These are design interpretations, not evidence of a conversion benefit.

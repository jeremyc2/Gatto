# Shared discovery — artwork redraw

**Later update:** the user rejected the independent picture overlay's lack of physical contact and depth. The [organic integration](organic-integration.md) supersedes that approach in the current website and studies. The notes below preserve the earlier experiment and its limitations.

October 16, 2026. Built-in image-generation tool, followed by native SVG scene assembly. Exploratory artwork; the app icon and packaging remain unchanged.

## What changed

- A small teal-eyed watercolor kitten, a tall slender amber-eyed paper adult, and a broad round luminous adult with cyan right eye and violet left eye. The two adults use smaller heads and less exaggerated eyes than the earlier sheets.
- Three encounters use distinct actions: investigate an image, send it to a destination, then inspect the result. All are curious peers. No dogs or mentor role.
- The upload tray has **GitHub** painted into its front face, following its perspective and watercolor grain. This is confined to the upload scene, at the user's request.
- Postcards use Coconat and a practical explanation: bring an image, upload it to GitHub, copy the Markdown. A small illustrative output accompanies the last beat. These are studies, not screenshots or a working upload interface.
- The character studio, postcards, shared-world study, motion study, shelf, original mood board, and original system study now show the redraw. Earlier favorite sheets remain available as comparison references.

## The flower continuity fix

The first redraw still changed the flower painting and paper size between pictures. The user spotted this. Asking the generator to match the flower more closely did not eliminate the drift.

The final three scenes therefore use generated **clean plates with the picture removed**, then insert the **same unchanged flower-picture-v2.png** through native SVG. Each insertion has identical 430 × 286.666667 dimensions and the identical affine projection `.9 .08 -.48 .28`; only its position changes. The flower pixels, paper proportions, projected size and angle cannot drift independently. The PNG scene intermediates that contain separate generated flowers are not the active scene artwork.

The SVG files embed their raster inputs, so they work when opened as images without network requests. The surrounding HTML remains editable. This is an illustration assembly method, not a claim that Gatto edits image contents.

[compose-scenes.py](compose-scenes.py) reproduces the three SVG assemblies from the unchanged source assets with Python's standard library. No raster editing or resampling is performed.

After seeing the corrected sequence, the user said they love it and want all three images retained as a developing scene while visitors scroll. The [scrolling story study](../story-sequence.html) now places practical text beside each moment. Artwork approval here concerns these encounters; a complete website layout remains open.

## Final sources

All generated files originated in the current conversation's generated-image folder. Originals are preserved.

| Project file                                                         | Generated source / role                                                                                                       |
| -------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| [cast-v2.png](cast-v2.png)                                           | `exec-a04648d9-bf5d-498a-bb06-dd85b9a8560d.png`; cast reference refined from `exec-88e4015e-4829-4252-81ab-184cd545f2f7.png`. |
| [story-bring-plate-v2.png](story-bring-plate-v2.png)                 | `exec-69e66c5d-7a9d-4e24-83e4-a181679a0460.png`; clean discovery scene.                                                       |
| [story-upload-plate-v2.png](story-upload-plate-v2.png)               | `exec-f61565b6-f8dd-48a7-ba85-8a91fa7b346e.png`; clean upload scene before lettering.                                         |
| [story-upload-github-plate-v2.png](story-upload-github-plate-v2.png) | `exec-f9c9baef-3555-4e2e-a66a-5f018ebe4944.png`; localized GitHub lettering edit, active upload plate.                        |
| [story-copy-plate-v2.png](story-copy-plate-v2.png)                   | `exec-15873442-9aad-4fc5-a8d8-67455e8acf1a.png`; clean result scene.                                                          |
| [flower-picture-v2.png](flower-picture-v2.png)                       | `exec-6886ec95-9d07-4b59-be1d-b1fbb4a5228d.png`; canonical picture prop.                                                      |
| [story-bring-v2.svg](story-bring-v2.svg)                             | Clean discovery plate and canonical prop.                                                                                     |
| [story-upload-v2.svg](story-upload-v2.svg)                           | GitHub-lettered upload plate and canonical prop.                                                                              |
| [story-copy-v2.svg](story-copy-v2.svg)                               | Clean result plate and canonical prop.                                                                                        |

The [exact prompt record](redraw-prompts.md) includes intermediate attempts and refinements. The earlier [artwork notes](README.md) retain the favorite sheets and first encounters.

## Remaining limits

The reusable flower is exact; the generated cats, scenery and tray are still illustrations rather than fixed models. Small rendering differences remain, especially watercolor fur edges and the tray decoration between scenes. The paper and watercolor cats now differ more clearly, although the watercolor kitten retains some sharply drawn fur edges. The prop's overlap and contact shadows can be refined further. These are not transparent cutouts or animation rigs.

Response exports identify artwork revision `redraw-2`. Earlier browser answers on substantially redrawn studies are retained in an archived section of the Markdown export, separately from new reactions. Earlier font preferences and narrative feedback remain recorded in their original feedback documents.

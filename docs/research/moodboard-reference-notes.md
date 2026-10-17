# Mood-board reference notes

Researched October 16, 2026 (America/Chicago). This is a collection of precedents and material for an exploratory board, not a website specification. The research uses first-party websites, their delivered stylesheets, font owners’ source repositories, and museum/publisher records. Source colors below are observations; the board’s proposed colors are separate experiments.

The aim is a black cat with a distinct voice: capable, companionable, a little strange. Fonts need not all be used together. “Less familiar” is a selection judgment relative to common interface defaults, not a measured claim about font adoption.

## Open-source type drawer

All five candidates below have owner-maintained SIL Open Font License 1.1 files. Free downloads or trial offers alone were not treated as evidence of an open license. Font redistribution must retain the copyright and license text; preserve any reserved font names when making derivatives. Collletttivo also requests designer/foundry credit. [Collletttivo licensing](https://www.collletttivo.it/licensing), [OFL official text](https://openfontlicense.org/open-font-license-official-text/)

| Family | Verified ownership / license | What to try on the board — our interpretation |
| --- | --- | --- |
| **Coconat** | Sara Lavazza; Collletttivo. Three styles, Latin. [Owner specimen](https://www.collletttivo.it/typefaces/coconat), [OFL file](https://github.com/collletttivo/coconat/blob/75f0bf21470de362ebee2f173e99505b7a7e977b/LICENSE.txt). | Friendly flourishes with a proud, wide stance. Try a small, almost theatrical headline next to an unadorned label; avoid turning every sentence into a performance. |
| **Ortica** | Ben Bovani; Collletttivo. Linear Light/Regular/Bold and Angular Bold. [Owner specimen](https://www.collletttivo.it/typefaces/ortica), [OFL file](https://github.com/collletttivo/ortica/blob/27bf44d3ea5057b7abcd09a8922d43d0b37a4f06/LICENSE.txt). | A botanical, slightly prickly storybook voice. Compare the curved Linear and cut-paper Angular letterforms; test real reading sizes before appointing it as text type. |
| **Apfel Grotezk** | Luigi Gorlero; Collletttivo. Five styles in the current owner repository. [Owner specimen](https://www.collletttivo.it/typefaces/apfel-grotezk), [OFL file](https://github.com/collletttivo/apfel-grotezk/blob/8b4c9f2682d274d7fae81d8eceef24f7f2fba4ea/LICENSE.txt). | A more everyday, soft-spoken companion for the expressive families. Useful for captions, controls, and small paragraphs where the art should lead. |
| **Basteleur** | Keussel, with contributions by George Triantafyllakos; Velvetyne. Bold and Moonlight webfonts. [Owner repository and design account](https://gitlab.com/velvetyne/basteleur/-/blob/master/README.md), [OFL file](https://gitlab.com/velvetyne/basteleur/-/blob/master/LICENSE.txt). | A curious little spellbook. Moonlight is sharp and eccentric; Bold is softer. Try one or two short words against rough ink silhouettes. Its theatrical forms could overwhelm small UI. |
| **Shantell Sans** | Concept/creative direction: Shantell Martin. Type design/development: Stephen Nixon / Arrow Type. Cyrillic: Anya Danilova. [Owner repository](https://github.com/arrowtype/shantell-sans), [OFL file](https://github.com/arrowtype/shantell-sans/blob/43932aa9375e60fbcb930b0f865f2f60eae9d206/OFL.txt). | Handwritten notes that still behave like type. Use for a circled thought, little route annotation, or the cat’s aside. Compare calm and bouncing settings; do not cover every surface with handwriting. |

Direct webfont files, verified from the owners’ trees:

- [Coconat Regular WOFF2](https://raw.githubusercontent.com/collletttivo/coconat/75f0bf21470de362ebee2f173e99505b7a7e977b/fonts/Coconat-Regular.woff2); change `Regular` to `Demi` or `Bold` for the companion cuts.
- [Ortica Linear Regular WOFF2](https://raw.githubusercontent.com/collletttivo/ortica/27bf44d3ea5057b7abcd09a8922d43d0b37a4f06/fonts/OrticaLinear-Regular.woff2); the same directory also contains `OrticaLinear-Light.woff2`, `OrticaLinear-Bold.woff2`, and `OrticaAngular-Bold.woff2`.
- [Apfel Grotezk Regular WOFF2](https://raw.githubusercontent.com/collletttivo/apfel-grotezk/8b4c9f2682d274d7fae81d8eceef24f7f2fba4ea/fonts/ApfelGrotezk-Regular.woff2); the same directory contains `Fett`, `Mittel`, `Satt`, and `Brukt` cuts.
- [Basteleur Moonlight WOFF2](https://gitlab.com/velvetyne/basteleur/-/raw/master/fonts/webfonts/Basteleur-Moonlight.woff2), [Bold WOFF2](https://gitlab.com/velvetyne/basteleur/-/raw/master/fonts/webfonts/Basteleur-Bold.woff2). Moonlight returned a valid `wOF2` file of 44,820 bytes during research.
- [Shantell Sans variable WOFF2](https://raw.githubusercontent.com/arrowtype/shantell-sans/43932aa9375e60fbcb930b0f865f2f60eae9d206/fonts/Shantell%20Sans/Web/ShantellSans%5BBNCE%2CINFM%2CSPAC%2Cital%2Cwght%5D.woff2), also [hosted on its official specimen site](https://shantellsans.com/fonts/ShantellSans%5BBNCE,INFM,SPAC,ital,wght%5D.woff2). The latter returned a valid `wOF2` file of 725,396 bytes. The owner documents weight, italic, bounce, informality, and spacing axes; static cuts are available in the repository’s `Web/Static` directory. [Axis documentation](https://github.com/arrowtype/shantell-sans#variable-axes)

Possible pairings are Coconat + Apfel (warm and direct), Ortica + Apfel (quiet drawing-book), or Basteleur + Apfel (small strange workshop). Shantell is a marginal note rather than a fourth hierarchy level. These are experiments, not selections.

## Websites to pin beside the type

### TanStack: a system people can inspect

The current [TanStack design system](https://tanstack.com/ds) explicitly presents a living catalog with light/dark viewing, brand/styles, palette and semantic tokens, and component specimens. Its hierarchy is the useful precedent: raw colors become roles, and roles become component states. This is more helpful than collecting unrelated buttons. [Public overview](https://tanstack.com/ds), [source overview](https://github.com/TanStack/tanstack.com/blob/9ab080e75cf29aad53224e2d73f930d47c58462d/src/routes/ds.index.tsx)

Measured source tokens: neutral-100 `#eeebd4`, neutral-500 `#111111`, blue-300 `#61adbf`, blue-500 `#003e53`, terracotta-300 `#e06e49`, terracotta-400 `#d3481b`, amber-300 `#f4d648`, green-300 `#69bc75`. Display and mono roles use Bricolage Grotesque and IBM Plex Mono; the live document also preloads Inter. These are observations, not Gatto font candidates. [Pinned token source](https://github.com/TanStack/tanstack.com/blob/9ab080e75cf29aad53224e2d73f930d47c58462d/src/styles/app.css), [live typography](https://tanstack.com/ds/typography), [live colors](https://tanstack.com/ds/colors)

The button source has primary, secondary, ghost, icon, link, subtle-link, and gradient variants; its defaults are pills with a squircle utility and 150 ms transitions. The general lesson is deliberate variants and visible states, rather than adopting these particular shapes. [Pinned component source](https://github.com/TanStack/tanstack.com/blob/9ab080e75cf29aad53224e2d73f930d47c58462d/src/components/ds/ui/index.tsx)

### Teenage Engineering: precise objects, restrained punctuation

The delivered root stylesheet declares proprietary `te-20` / `te-40` faces and named colors: `--te-white: #f5f5f5`, `--te-black: #0f0e12`, orange `#f05a24`, yellow `#fab413`, blue `#0071bb`, green `#006837`. Its field rules explicitly use radius `0` at desktop widths, while mobile fields use a viewport-scaled radius. [Official site](https://teenage.engineering), [root CSS](https://teenage.engineering/assets/root.NRW9iUxr.css), [field CSS](https://teenage.engineering/assets/form.DWmR17wv.css)

Board translation: put one lively mark against a large calm surface; try precise labels alongside a handmade animal. The tension between instrument-like clarity and a friendly imperfection is the idea to explore. We are not licensing or importing the site’s fonts.

### Poolsuite: an interface as a complete little world

The current [poolside.fm](https://poolside.fm) title is Poolsuite. Its delivered CSS defines `--primary: #f7d5d5`, `--secondary: #faf1e8`, `--tertiary: #faf5c6`, and `--secondary-button: #afe5dd`. It includes faces named ChiKareGo2, ITC Garamond, and Perfect DOS VGA 437. These names are evidence about the reference, not an assertion that those fonts are available under an acceptable license. [First-party stylesheet](https://poolside.fm/css/index.17a39892.css)

One inspectable slider rule uses a 4 px corner radius, black border, and a 2 px bottom edge. Small edge treatments make its desktop fantasy feel tangible. Board translation: try a tiny beveled ticket or stamp among paper scraps; preserve the sense of a coherent little world without borrowing a whole old-Mac interface.

### Shantell Sans specimen: play that is the content

The [official specimen](https://shantellsans.com) makes type variation explorable. Its global CSS sets black/white surfaces with `--uiAccent: hsl(228, 100%, 50%)` in light mode and `hsl(4, 100%, 70%)` in dark mode. Neutral 0.5 rem corners coexist with fully rounded 999 px controls. [Global CSS](https://shantellsans.com/global.css)

Board translation: a live type specimen is more informative than a screenshot of a font. Give the viewer a phrase to edit or a small slider to move. One responsive, purposeful toy can carry more personality than constant decorative movement.

## Art reference drawer

| Reference | What we are looking at — interpretation | Attribution / use status |
| --- | --- | --- |
| [Kiki’s Delivery Service official still collection](https://www.ghibli.jp/works/majo/) — `majo011.jpg`, `majo027.jpg` | A small black-cat silhouette; ordinary errands made adventurous; character scale. The earlier dog/cat still is historical provenance only and has been removed from the active board after the user ruled out dogs. These are compositional references, not proposed Gatto characters. | Publisher credit: © 1989 Eiko Kadono/Hayao Miyazaki/Studio Ghibli, N. The page permits image use within common sense; it does not supply an open license. Keep as attributed research references. |
| [The Tale of the Princess Kaguya official still collection](https://www.ghibli.jp/works/kaguyahime/) — `kaguyahime013.jpg`, `kaguyahime029.jpg` | Watery color, incomplete edges, paper left visible, and expressive marks rather than elaborate rendering. Study which few marks are enough to suggest place and weather. | Publisher credit: © 2013 Isao Takahata, Riko Sakaguchi/Studio Ghibli, NDHDMTK. Same publisher instruction about use within common sense; not open licensed. |
| [Théodule Ribot, *Cats (recto)*, Cleveland Museum of Art](https://www.clevelandart.org/art/2010.285.a) | A remarkably useful black-cat study: weight, gesture, and pooled shadow with very few marks. Compare the heavy silhouette to the barely drawn central cat. | The record leaves the date unknown; medium is black ink applied with reed pen and brush on beige laid paper. Bequest of Muriel Butkin, 2010.285.a. The museum explicitly permits copying, modification, and distribution through its Open Access program. |

For new artwork, describe material and construction: transparent pigment, dry-brush contour, a two-tone cel shadow, or a three-mark ink silhouette. A material vocabulary gives us room to make an original cat instead of reproducing a recognizable film character.

## Questions the board should keep open

- Does the cat feel better as one dark shape in watery space, a dry-brush gesture, or a clean cel with one shadow?
- Is the light a warm lamp, an overcast window, or a little colored night sky? Test colors beside the actual black silhouette, not only as isolated chips.
- Current constraint: no dogs. A secondary cat or alternate self may have a different material, eye color, height, or proportions. Study whether the relationship earns its place.
- Which type pairing stays pleasing after the novelty of a single large word has passed?
- Can texture be sparse enough to preserve crisp reading and quiet enough that it does not become a repeating overlay?

The mood board can happily contain incompatible answers. The companion design-system experiment should make one provisional combination inspectable; the storyboard should explain how that combination earns its place in the eventual visitor experience.

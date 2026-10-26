# Replacing Gatto's direct font parser

Researched 2026-10-21 against Gatto commit `886d259` and primary upstream sources.

## Decision for the current macOS app

The user confirmed that Gatto supports macOS only. Remove the direct `ttf-parser`
dependency and the unused non-macOS font loading, outline adapter, measurement,
and wrapping fallback. Keep the existing CoreText implementation for annotation
editing, preview, and PNG export; no replacement parser is needed.

After removal, the configured dependency audit passes. `ttf-parser` remains in the
locked dependency graph through GPUI and its SVG dependencies. The current
`deny.toml` scopes unmaintained advisories to workspace dependencies; passing the
audit does not mean every indirect dependency has migrated away from this crate.
Verification: `cargo deny --locked check --hide-inclusion-graph advisories licenses bans sources`
and `cargo tree --locked -i ttf-parser`.

## Recommendation

If support for other platforms is introduced later, **Skrifa**, part of Google Fonts' Fontations project, is a suitable parser to evaluate. It supplies the metadata, character mapping, advances, and outline extraction that the removed non-macOS annotation fallback needed. Keep the macOS CoreText implementation. This would be a focused parser migration, not a replacement text-layout engine. Fontations explicitly positions Skrifa between its low-level `read-fonts` parser and higher-level layout. [Skrifa documentation](https://docs.rs/skrifa/latest/skrifa/), [Fontations project](https://github.com/googlefonts/fontations)

## Maintenance status

RustSec still lists `RUSTSEC-2026-0192` as an **informational unmaintained advisory**, with no patched versions, and recommends Skrifa. This is a maintenance advisory, not itself a published memory-safety vulnerability. However, the current `ttf-parser` README now promises maintenance-mode bug fixes, including security fixes, and also recommends Fontations for new projects. The historical statement that no fixes will ever be accepted is therefore an incomplete description of current upstream policy. [RustSec advisory](https://rustsec.org/advisories/RUSTSEC-2026-0192.html), [current upstream README](https://github.com/harfbuzz/ttf-parser/blob/main/README.md), [original discussion and later maintainer response](https://github.com/harfbuzz/ttf-parser/issues/217)

## Fit and migration scope

At the researched commit, the direct dependency was `ttf-parser = "0.25.1"` in `Cargo.toml`. Its application usage was confined to `font()` and `Outliner` in `src/annotation.rs`, plus the non-macOS `char_width()` and `shape_line()` functions in `src/annotation/text/layout.rs`. These read one system font, map individual characters, measure advances, and draw paths into tiny-skia. The macOS path already used CoreText. The API mapping below records what a future portable implementation could use.

| Current need                  | Skrifa equivalent                                                                                                                                                                                                                                                                                                                                                                                                                          |
| ----------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Parse font index zero         | `FontRef::from_index(bytes, 0)` accepts single TTF/OTF files and font collections; `FontRef::new` accepts single fonts only. [FontRef](https://docs.rs/skrifa/latest/skrifa/font/struct.FontRef.html)                                                                                                                                                                                                                                      |
| Map a character               | `font.charmap().map(c)` returns its nominal glyph identifier. [Outline example](https://github.com/googlefonts/fontations/blob/main/skrifa/src/outline/mod.rs)                                                                                                                                                                                                                                                                             |
| Units per em, ascent/descent  | `font.metrics(size, location)` exposes these. [Metrics source](https://github.com/googlefonts/fontations/blob/main/skrifa/src/metrics.rs)                                                                                                                                                                                                                                                                                                  |
| Horizontal advance            | `font.glyph_metrics(size, location).advance_width(glyph)` supplies glyph advances. [Metrics source](https://github.com/googlefonts/fontations/blob/main/skrifa/src/metrics.rs)                                                                                                                                                                                                                                                             |
| Vector outline into tiny-skia | `font.outline_glyphs().get(glyph)` and `draw(DrawSettings::unhinted(...), pen)`. `OutlinePen` has the same move, line, quadratic, cubic, and close operations as the existing adapter. Retain the Y-axis inversion and apply size scaling exactly once. [Outline example](https://github.com/googlefonts/fontations/blob/main/skrifa/src/outline/mod.rs), [OutlinePen](https://docs.rs/skrifa/latest/skrifa/outline/trait.OutlinePen.html) |

Skrifa's global metrics select among OS/2 and hhea metrics. Verify baseline, line width, wrapping, caret positions, and exported paths against the existing behavior rather than assuming numerical parity. For exact compatibility, compare its table-selection policy with the current parser's ascender/descender accessors. [Metrics source](https://github.com/googlefonts/fontations/blob/main/skrifa/src/metrics.rs)

The current Fontations workspace declares Rust 1.85 and `MIT OR Apache-2.0`; Gatto declares Rust 1.90, so those requirements fit. Recheck the selected published version when implementing. [Workspace manifest](https://github.com/googlefonts/fontations/blob/main/Cargo.toml), [Skrifa manifest](https://github.com/googlefonts/fontations/blob/main/skrifa/Cargo.toml)

## Limits

Skrifa maps codepoints, not graphemes, and sits below a layout engine. Replacing this parser alone will not add shaping, ligatures, bidi layout, font discovery/fallback, or native emoji behavior. Skrifa does expose bitmap/color APIs, but using them requires additional rendering integration; the existing macOS CoreText emoji implementation should remain. [Skrifa documentation](https://docs.rs/skrifa/latest/skrifa/)

Removing Gatto's direct dependency does **not remove `ttf-parser` from the dependency graph**: the locked graph also contains it through `gpui-pre` directly and through `usvg` → `fontdb` / `rustybuzz`. Evidence: `cargo tree --locked -i ttf-parser` in this checkout. Those dependencies require separate upstream upgrades or changes for complete removal. The application no longer calls the parser directly, and no advisory suppression was added.

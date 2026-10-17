# Gatto design system

The design language is independent type, warm paper, green ink, restrained
controls, and detailed illustrations. Preserve the approved website while making
these choices reusable. Add a shared abstraction when multiple consumers need
one contract; keep individual compositions local.

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
geometry. System appearance is a CSS token condition. Chapter changes use native
View Transitions with directional transforms; reduced motion and unsupported
browsers update immediately. `src/story-transitions.css` owns the document-level
snapshot pseudo-elements and keyframes that cannot be scoped to component
selectors. It also sets the route’s `animation-range`, which StyleX 0.19’s
property validator does not yet accept. All ordinary component styling remains in StyleX.

Run `bun build:web` and `bun size:web` to measure the actual stylesheet links in
`web/dist/client/index.html`. Bun reports raw bytes and gzip bytes. The build test
limits the combined CSS to **16 KiB raw and 5 KiB gzip**, leaving room above the
current page while catching large regressions. Raise a budget only with a measured
reason. Font and image payloads are separate from this CSS budget.

Measured on 2026-10-17 with Vite 8.3.4 and StyleX 0.19.1:

| Build                                                             | Styling bytes | Gzip bytes |
| ----------------------------------------------------------------- | ------------: | ---------: |
| Initial direct StyleX translation, before Vite/minification       |        19,218 |      4,233 |
| Refactored system, all production CSS including font declarations |        11,497 |      3,967 |
| Native snapshot transitions and connected chapter tracker         |        16,369 |      4,969 |

The initial figure excludes its separate stylesheet with embedded font data;
this is a comparison of generated styling, not total page transfer. The change
reflects both the source refactor and production minification. Refresh this dated
measurement after intentional design-system or build changes.

## References

- [StyleX principles: co-location, low-cost abstractions and atomic output](https://stylexjs.com/docs/learn/thinking-in-stylex/)
- [Official StyleX Vite integration](https://stylexjs.com/docs/learn/installation/vite/)
- [StyleX DOM attributes for Solid](https://stylexjs.com/docs/api/javascript/attrs/)
- [Solid Vite plugin and start mode](https://github.com/solidjs/solid-vite-plugin)
- [Oxlint JavaScript plugin support](https://oxc.rs/docs/guide/usage/linter/js-plugins.html)

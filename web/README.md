# Gatto website

Solid 2.0.0-rc.15 and StyleX 0.19.1. Vite 8.3.4 provides development HMR and
production builds. Bun runs Vite, installs packages, runs scripts and tests, and
prerenders the production page. Tests use Bun; Vitest is not used.

From the repository root, with Bun 1.4.3 or later:

```sh
bun install --frozen-lockfile
bun dev:web
```

Open `http://127.0.0.1:3000/`.

Other commands:

```sh
bun build:web
bun preview:web
bun lint:web
bun typecheck:web
bun test:web
bun format:web
bun size:web
```

The deployable static site is `web/dist/client/`.

For responsive story changes, check all three chapters in the browser at
1397 × 716 and 1920 × 720, plus phone and tablet sizes. Illustrations should fit
inside their scenes above the chapter tracker without cropping. Bun tests check
the build, story logic and renderer-independent game simulation; they do not
measure browser layout.

[Design system](DESIGN-SYSTEM.md) owns the styling conventions, component APIs,
CSS budget and measurements.

Story illustrations live in `assets/images/`: `story-bring`, `story-upload`, and
`story-copy`. Each PNG master has a matching lossless WebP served by the website;
keep both copies synchronized at 1536 × 1024. The cast uses watercolor, layered
paper and smooth animated rendering under shared warm light. The
[artwork record](../docs/design/explorations/art/website-cat-materials.md) contains
the source provenance, prompts and export commands.

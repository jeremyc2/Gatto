# Gatto website

Solid 2.0.0-rc.15 and StyleX 0.19.1. Vite 8.3.4 provides development HMR and
production builds. Bun runs Vite, installs packages, runs scripts and tests, and
prerenders the production page. Tests use Bun; Vitest is not used.

From the repository root, with Bun 1.4.4 or later:

```sh
bun install --frozen-lockfile --ignore-scripts
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

For deployment, put `CLOUDFLARE_ACCOUNT_ID` and any API token in the ignored root
`.env.cloudflare`, then run `bun deploy:web`. Deployment reads that file explicitly
and supplies Cloudflare settings only to `cf`, after building. Keep credentials
out of root `.env` so regular Bun commands do not automatically load them.

Bun's built-in formatter uses the root `bunfig.toml`; a separate formatter package
is not required. Its built-in checker runs both TypeScript projects and checks
tests before execution. Oxlint retains typed and StyleX-specific lint rules;
Bun's native linter is not available in the pinned stable release.
Package commands require the locked local
installation and never fetch a missing executable automatically. Dependency
installation scripts are disabled explicitly. Run `bun audit:dependencies` for
the complete JavaScript dependency audit and any visible temporary exceptions.

For responsive story changes, check all three chapters in the browser at
1397 × 716 and 1920 × 720, plus phone and tablet sizes. Illustrations should fit
inside their scenes above the chapter tracker without cropping. Bun tests check
the build and story logic; they do not measure browser layout.

[Design system](DESIGN-SYSTEM.md) owns the styling conventions, component APIs,
CSS budget and measurements.

For story artwork exports, configure [Aseprite](../docs/agents/aseprite.md), install
FFmpeg and libwebp's `cwebp`, then run `bun export:story` after editing the saved
animation. The [artwork record](../docs/design/explorations/art/scroll-story/journey.md)
owns the master, references and export workflow; visual behavior belongs in the design system.

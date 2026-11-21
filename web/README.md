# Gatto website

Solid 2.0.3, ReactiveScript 0.3.1 and StyleX 0.19.1. Vite 8.3.4 and
`@solidjs/vite-plugin` 3.0.1 compile the `.rx` components for development and
production. Bun runs Vite, installs packages, runs scripts and tests, and
prerenders the production page. Tests use Bun.

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

The static site is `web/dist/client/`. Deployment also bundles a small Worker
that serves MP4 byte ranges; other files use direct static asset serving.

For deployment, put `CLOUDFLARE_ACCOUNT_ID` and any API token in the ignored root
`.env.cloudflare`, then run `bun deploy:web`. Deployment reads that file explicitly
and supplies Cloudflare settings only to `cf`, after refreshing artwork/screenshots
with `bun refresh:assets` and building. The refresh requires macOS and the
[artwork refresh requirements](../CONTRIBUTING.md#artwork-refresh-requirements). Keep credentials
out of root `.env` so regular Bun commands do not automatically load them.

Validate deployment without uploading with `bun deploy:web --dry-run`. After a
deployment, run `bun --no-env-file web/scripts/check-story-video.ts <website-url>`
to check video ranges against the real host. A local Vite preview already
supports ranges and cannot catch missing production range support.

For `.rx` highlighting and diagnostics, install the SolidJS ReactiveScript editor
extension. `bun typecheck:web` runs the installed `rx-tsc` under Bun; builds and
linting run this check too, so `.rx` sources cannot bypass language or type checks.
TypeScript 6.0.3 is pinned at the repository root for the current
ReactiveScript/Volar integration. This also gives the editor extension's automatic
SDK detection a compatible JavaScript compiler API when opening the whole repo;
TypeScript 7's native SDK cannot initialize this language server. After updating
dependencies, use **Developer: Reload Window** if the extension previously failed
to start.

Bun's built-in formatter uses the root `bunfig.toml` for TypeScript, CSS and
Markdown. It does not format `.rx`; keep those files consistently indented and
check their grammar with `bun typecheck:web`. Its built-in checker checks ordinary TypeScript tests before execution
and the root scripts project. ReactiveScript checks the website project, including
its TypeScript browser adapters and tests. The check fails if the experimental
checker reports an internal analysis failure, even when it exits zero. Oxlint
retains typed and StyleX-specific lint rules;
Bun's native linter is not available in the pinned stable release.
Package commands require the locked local
installation and never fetch a missing executable automatically. Dependency
installation scripts are disabled explicitly. Run `bun audit:dependencies` for
the complete JavaScript dependency audit and any visible temporary exceptions.

For responsive story changes, check all three chapters in the browser at
1397 × 716 and 1920 × 720, plus phone and tablet sizes. Illustrations should fit
inside their scenes above the chapter tracker without cropping. Bun tests check
the build, story logic and Rooftops simulation/audio/storage. Check hydration,
chapter navigation, portal navigation to `/rooftops/`, and rendering without
JavaScript manually in the browser.

[Design system](DESIGN-SYSTEM.md) owns the styling conventions, component APIs,
CSS budget and measurements.

For story artwork exports, configure [Aseprite](../docs/agents/aseprite.md), install
FFmpeg, then run `bun export:story` after editing the saved
animation. The [artwork record](../docs/design/explorations/art/scroll-story/journey.md)
owns the master, references and export workflow; visual behavior belongs in the design system.

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

[Design system](DESIGN-SYSTEM.md) owns the styling conventions, component APIs,
CSS budget and measurements.

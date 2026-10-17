## JavaScript and TypeScript tooling

Use Bun for the runtime, package manager, test runner, and scripts. The website
uses Vite for its development server and production builds, running under Bun.
Use Bun's bundler for other JavaScript bundling tasks.
Use `bunx --bun` for package executables and Bun APIs wherever Bun provides the
capability. Never invoke `node`, `npx`, `npm`, `pnpm`, or `yarn`, including commands
suggested by skills or dependency documentation. Keep `bun.lock` committed.

For website changes, read [web/README.md](web/README.md) and preserve the approved
design, copy, and artwork unless the user asks to change them.
Follow [the design system](web/DESIGN-SYSTEM.md) for tokens and StyleX components.

## Agent skills

### Issue tracker

Issues and specs live in GitHub Issues; use the `gh` CLI. See `docs/agents/issue-tracker.md`.

### Triage labels

Use the default labels: `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, and `wontfix`. See `docs/agents/triage-labels.md`.

### Domain docs

This is a single-context repo with root `GLOSSARY.md` and `docs/adr/`. See `docs/agents/domain.md`.

### Documentation

When changing app behavior, assets, dependencies, packaging, or releases, read
[documentation upkeep](docs/agents/documentation.md). Update the owning guide and
verify README claims and measured release figures.

Change READMEs only for setup or essential usage. Put website visuals, motion, and asset mechanics in [the design system](web/DESIGN-SYSTEM.md) or artwork records; rationale in `docs/adr/`.

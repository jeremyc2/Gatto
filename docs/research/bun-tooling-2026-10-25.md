# Bun tooling consolidation

Researched 2026-10-25 for [issue #28](https://github.com/jeremyc2/Gatto/issues/28).
The baseline is Gatto's existing Bun 1.4.3 package requirement, installed Bun
1.4.4 (`38afee6c8`), website Oxfmt/Oxlint configuration, and two TypeScript
projects. The applied changes below preserve the staged website appearance
work. The owning development guides describe the resulting commands.

## Available in the requested stable releases

- **Type checking:** Bun 1.4.3 introduced `bun check`, reading `tsconfig.json`
  without a standalone TypeScript dependency. It also added checks before
  running tests, scripts or bundles through `--check`. The existing
  `typecheck:web` and `typecheck:scripts` commands already use the native checker.
  The flag extends enforcement to tests without introducing another dependency.
  [Bun 1.4.3 release](https://bun.com/blog/bun-v1.4.3),
  [type checking](https://bun.com/docs/runtime/check).
- **Formatting:** Bun 1.4.4 adds `bun fmt` and `--check`, accepting Oxfmt config,
  ignore patterns and formatter flags. This directly replaces the root
  `oxfmt` dependency and its script invocations. The required/pinned Bun release
  is now 1.4.4. The release also improves type-checker and package-manager
  performance; these do not require additional tool replacements.
  [Bun 1.4.4 release](https://bun.com/blog/bun-v1.4.4).

## Formatting migration details

Formatting defaults must survive uninstalling Oxfmt: Bun selects Oxfmt's
defaults when it discovers its config or dependency, and otherwise uses different
defaults. At baseline, `bun fmt --doctor web package.json` reported 43 native
files and that root defaults came from the dependency. The final root
`bunfig.toml` explicitly sets `printWidth = 100`, `tabWidth = 2`, `semi = true`
and `trailingComma = "all"`. It excludes website assets, cache and build output,
replacing the nested `web/.oxfmtrc.json` without retaining a deleted package's
schema reference.
[Oxfmt migration guide](https://bun.com/guides/format/migrate-from-oxfmt).

The compatibility target is Oxfmt 0.72; the baseline package was 0.73.0. The
website scope's 42 files passed native formatting unchanged. Newly covered
developer scripts were formatted. `bun fmt` has no language server. Its
`--staged` replacement for lint-staged is unnecessary here and would restage
files.
[Oxfmt migration guide](https://bun.com/guides/format/migrate-from-oxfmt),
[formatter reference](https://bun.com/docs/format).

## Applied changes

- Removed the direct `oxfmt` package and updated `bun.lock`. Pinned
  `packageManager` and `@types/bun` to 1.4.4 and required `engines.bun >=1.4.4`.
- Replaced website formatter invocations with native `bun fmt`; added
  `format:scripts` and `lint:scripts` so release tooling gets formatting and
  typed semantic lint checks too.
- Added `test:scripts`; both it and `test:web` now run `bun test --check`.
  Release preparation now invokes `lint:scripts` and `test:scripts`.
- Fixed four floating promises exposed by script linting: await two subprocess
  stdin writes and its stdin close in `aseprite-mcp-check.ts`, and await the
  HTTP server stop in `test-update-install.ts`.
- Final `bun fmt --doctor scripts web package.json tsconfig.json bunfig.toml .oxlintrc.json`
  reports 60 files formatted natively and no remaining formatter packages needed.
  These are local executable observations, not a claim about every file in the
  repository. Rust formatting and checks remain Rust-owned.

## Linting is still a stable-release limitation

The official linter documentation explicitly limits `bun lint` to **canary**.
The installed stable 1.4.4 confirms this: `bun lint --help` exits 1 with
`Script not found "lint"`, and `bun --help` lists `fmt` and `check`, but no
native lint command. Removing Oxlint on this executable would remove lint
coverage. [Bun linter documentation](https://bun.com/docs/lint).

Future Bun linting supports `.oxlintrc.json`, `--type-aware`, `--type-check` and
JavaScript `jsPlugins`, so the eventual migration can remove both `oxlint` and
`oxlint-tsgolint`. Its compatibility target is Oxlint 1.87, compared with this
repository's 1.88.0. Validate diagnostics and custom rules on the actual selected
runtime before switching. [Oxlint migration guide](https://bun.com/guides/lint/migrate-from-oxlint).

Keep `@stylexjs/eslint-plugin` independently of which linter hosts it: the
configuration actively uses `stylex/valid-styles`, `stylex/no-unused` and
`stylex/valid-shorthands`. Those are project-specific checks, not replaced by a
type checker or formatter. Bun's documented JavaScript plugin support still
loads such packages. [Bun ESLint plugin support](https://bun.com/docs/lint/plugins),
[current configuration](../../web/.oxlintrc.json).

## Other boundaries

The runtime, package manager, test runner, lockfile, file I/O, audit driver and
most process execution already use Bun. Standard `node:` imports do not imply
an installed third-party dependency. Opportunistic conversions can simplify
scripts but do not themselves remove packages. Keep API behavior covered by
the existing release-script tests.

Keep Vite and its Solid/StyleX integrations: the website configuration needs
Solid compilation, CSS extraction, server rendering and development HMR.
Neither requested release announces a drop-in replacement for that configured
pipeline. The repository explicitly requires Vite for website builds and Bun's
bundler for other JavaScript bundling.
[Website configuration](../../web/vite.config.ts),
[repository tooling policy](../../AGENTS.md).

The resulting stable-release boundary replaces Oxfmt, retains the native type
checker, broadens script/test checks, and preserves semantic plus StyleX linting
until a supported Bun executable provides equivalent coverage.

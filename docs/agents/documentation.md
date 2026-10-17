# Documentation upkeep

## Ownership

Keep each instruction in its owning guide; other pages link to it. Use direct,
concrete language and measured claims in the README. Keep the header illustration.

| Document | Owns |
| --- | --- |
| [README](../../README.md) | macOS product introduction, download, measured release sizes, key capabilities, URL action table, links to project Markdown documents. |
| [Setup](../SETUP.md) | Installation, checksum verification, GitHub CLI login, organization and pins, initial preferences. |
| [Walkthrough](../WALKTHROUGH.md) | Upload/copy workflows, batches and retries, preview controls, shortcuts, automation requirements, window behavior. |
| [Support](../../SUPPORT.md) | Troubleshooting, diagnostics, local file paths, reset, uninstall, recovery. |
| [Contributing](../../CONTRIBUTING.md) | Developer tools, source builds, checks, dependency compatibility, screenshot regeneration. |
| [Website](../../web/README.md) | Solid website development, Bun scripts, web assets, dependencies and verification. |
| [Architecture](../ARCHITECTURE.md) | Modules, state, data flow, integration and security boundaries. |
| [Releasing](../RELEASING.md) | Signing setup, packaging, publication, verification, size measurement method. |
| [Security](../../SECURITY.md) | Authentication and data handling policy, dependency audits, vulnerability reporting. |
| [Rooftops](../EASTER_EGG.md) | Game controls and scorecards. |

Changelogs, ADRs, research notes, and artwork prompts record dated history or
rationale. Preserve their evidence and dates; link to current instructions when
needed. Agent docs and contribution templates keep their own operational scope.

## Release figures

After publishing, run `bun scripts/update-readme-size.ts` on macOS, then
`bun scripts/update-readme-size.ts --check`. Commit the resulting README update.
See [Releasing](../RELEASING.md#readme-size-figures) for prerequisites and measurement
semantics. This measures the downloadable release, including its installed bundle.

When code, assets, dependencies, or packaging change, revisit affected claims and
refresh size figures once the build is released. Keep figures tied to their
measured release version until then. Source any new numeric claim with units,
measured artifact/version, and a reproducible method.

## Change verification

When behavior changes, update its owning guide and the README summary if relevant.
Move details with their caveats when shortening a page. Keep the README’s project
index linked to project Markdown documents, excluding bundled agent skills and
the Easter egg guide. Keep the Easter egg’s entry action undocumented everywhere,
including its guide and historical changelog entries.
Check relative links, heading anchors, image paths, and cross-links after moves.
Keep setup, walkthrough, and README navigable in both directions. Add user-visible
changes to the changelog without rewriting historical releases.

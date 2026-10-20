# Security Policy

## Dependency audit status

The dependency graph was audited on **2026-10-07** after resolving and locking the
macOS application dependencies.

- RustSec `cargo-audit` 0.22.2 scanned 858 locked packages against advisory
  database commit `b8a1a33e246a0a9a3b5f377248c41a503defec74`.
- OSV-Scanner 2.6.0 independently scanned the same `Cargo.lock`.
- `cargo-deny` 0.20.2 checked RustSec advisories, yanked releases, licenses, and
  package sources for Apple silicon and Intel macOS targets.
- Result: **no known vulnerabilities, yanked releases, disallowed licenses, or
  unknown dependency sources were found**.
- Every external Rust package currently resolves from crates.io and is protected
  by a checksum in the committed `Cargo.lock`; there are no Git dependencies.

This result describes the databases and lockfile at the date above. It is not a
guarantee that a dependency is defect-free, and newly published advisories can
change the result without a code change.

### Open maintenance notices

RustSec reports five informational, transitive “unmaintained” notices. None is a
known vulnerability, none is a direct dependency, and no advisory is suppressed:

| Advisory | Package | Introduced through | Current disposition |
| --- | --- | --- | --- |
| `RUSTSEC-2024-0384` | `instant 0.1.13` | GPUI Base/Component | Monitor GPUI releases for migration to `web-time`. |
| `RUSTSEC-2024-0436` | `paste 1.0.15` | GPUI Component, Metal, and image codecs | Monitor upstream migration to `pastey` or native macros. |
| `RUSTSEC-2025-0134` | `rustls-pemfile 2.2.0` | GPUI Kit assets' HTTP stack | Monitor upstream migration to `rustls-pki-types`. |
| `RUSTSEC-2026-0206` | `rustybuzz 0.20.1` | GPUI SVG/text rendering | Monitor upstream migration to `harfrust`. |
| `RUSTSEC-2026-0192` | `ttf-parser 0.25.1` | GPUI font/SVG rendering | Monitor upstream migration to `skrifa`. |

Replacing these packages locally would require patching or forking GPUI and its
rendering stack. That would create a private dependency fork and a larger review
surface, so the project instead blocks new direct unmaintained dependencies and
tracks compatible upstream releases. A future vulnerability in any of these
packages is a release blocker until upgraded, removed, or isolated with a
documented analysis.

## Supply-chain controls

- `Cargo.lock` is committed and must be reviewed whenever dependencies change.
- `deny.toml` restricts the graph to the supported macOS targets, denies unknown
  registries and Git sources, rejects wildcard versions, and enforces a permissive
  license allowlist.
- Direct unmaintained dependencies, known vulnerabilities, unsoundness advisories,
  and yanked releases fail dependency-policy checks.
- Dependabot checks Cargo packages and GitHub Actions weekly.
- CI and dependency-security definitions are stored in
  `.github/workflows.disabled/` and do not run in GitHub Actions. The retained
  security definition uses checksum-verified scanner binaries and a manual
  trigger if restored to `.github/workflows/`. Run policy scans locally meanwhile.
- Actions in the retained workflow definitions are pinned to immutable commit
  hashes, use read-only permissions, and do not persist checkout credentials.
- New dependencies require justification and review under
  [CONTRIBUTING.md](CONTRIBUTING.md).

Rust build scripts and procedural macros execute during compilation. Crates.io and
lockfile checksums provide provenance and integrity, but not a full code review of
those components. GPUI accounts for most of the graph, including packages for
platforms this macOS app does not execute. Release changes should therefore review
the lockfile diff, upstream ownership changes, newly added build scripts, and any
new registry or Git source before building artifacts.

The repository's own `build.rs` only reads an optional hexadecimal
`GIT_COMMIT_HASH` value and, when needed, invokes
`git rev-parse --short=12 HEAD` to embed non-sensitive build metadata. It rejects
non-hexadecimal values, does not access the network, and does not modify the
working tree.

## Authentication and media uploads

Gatto uses GitHub’s existing issue and pull request attachment mechanism. It
requests the active GitHub CLI token when needed, caches it in memory for the
uploader session, and streams image/video bytes directly to GitHub’s attachment service.
It does not persist tokens in app configuration. Restart after switching CLI
accounts; see [Support](SUPPORT.md#common-problems) for account recovery.

## Release updates

Bundled updater-enabled releases make unauthenticated HTTPS requests to the public
GitHub release feed through Sparkle every four hours while Gatto runs. They send no
GitHub CLI token, images, repository selections, or optional system profile.
A click on Update is required before downloading and installing a release.
Sparkle verifies the Ed25519-signed feed and archive before extraction, then
handles installation and relaunch; the distributed app and DMG are Developer ID
signed and notarized. The release pipeline pins the Sparkle SDK's SHA-256 and
keeps its persistent private signing key in the release Mac's Keychain.
See [Releasing](docs/RELEASING.md#one-time-updater-signing-setup) for key handling.
Rust dependency audits do not audit the separately bundled native Sparkle SDK.

## Local settings and clipboard behavior

- Normal uploads do not change the clipboard. **Copy Markdown**, **Copy all**,
  and **Command+C** explicitly copy completed results. **Quick Copy**, invoked
  through the menu bar, custom URL, or enabled global shortcut, uploads a
  clipboard image or copied Finder media file and automatically copies its Markdown on success.
- Preferences persist repository names and app settings; tokens, image bytes,
  and upload URLs remain in memory. Video staging creates a private 0600
  temporary snapshot with a random name; it is removed when its last draft,
  upload, or external viewer reference is released. Viewer references last until
  the macOS player application exits, and forced termination can leave temporary
  files for operating-system cleanup. Optional compression uses private
  `gatto-compress-*` directories and native hardware H.264/AAC encoding after an
  explicit Compress choice. Failed/canceled attempts are deleted; successful
  output is copied into the staged snapshot only after size validation. Source
  videos are never modified. No screen or microphone capture permission is used.
  Start at Login writes a per-user LaunchAgent containing the executable path.
  See [local data and recovery](SUPPORT.md#local-data-and-recovery) for paths and
  removal instructions.

To repeat the policy scan locally:

```sh
cargo install --locked cargo-deny --version 0.20.2
cargo deny --locked check --hide-inclusion-graph advisories licenses bans sources
```

Do not add an advisory to an ignore list merely to make automation pass. Any
exception must identify affected functionality, explain reachability, name an
owner, include an expiry or removal condition, and be recorded in this file.

## Supported versions

Until the first stable release, security fixes are applied to the latest `0.x`
release and the default development branch. Upgrade to the latest release when
reporting a problem; earlier pre-1.0 releases do not receive separate backports.

| Version | Supported |
| ------- | --------- |
| Latest 0.x release and `main` | Yes |
| Earlier releases | No |

## Reporting a vulnerability

Please do not open a public issue for a suspected vulnerability. Use the
repository's private GitHub Security Advisory reporting feature, if available, or
contact a project maintainer privately.

Include as much of the following as possible:

- A description of the issue and its potential impact.
- The affected version, macOS version, and system architecture.
- Reproduction steps or a minimal proof of concept.
- Relevant logs with tokens, usernames, repository names, and image contents removed.
- Any suggested mitigation.

Maintainers will acknowledge the report as soon as practical, investigate it, and
coordinate disclosure and a fix with the reporter. Please allow time for a patch
before publishing details.

Dependency reports should include the package name and version, advisory or CVE
identifier, dependency path, affected feature or platform, and the first known
fixed version.

## Sensitive data

Never include a GitHub token, private repository content, or confidential image in
an issue, log, screenshot, or sample project. If a credential is exposed, revoke it
through GitHub immediately and authenticate again with GitHub CLI.

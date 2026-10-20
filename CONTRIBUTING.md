# Contributing

Thank you for helping improve Gatto. Contributions of code,
documentation, bug reports, and focused feature proposals are welcome.

By participating, you agree to follow the [Code of Conduct](CODE_OF_CONDUCT.md).

## Before you begin

- Read the [architecture overview](docs/ARCHITECTURE.md). For app configuration,
  use the [setup guide](docs/SETUP.md).
- Search existing issues and pull requests before opening a new one.
- For a substantial change, open an issue first so the approach and scope can be
  discussed before implementation.
- Report security problems privately as described in [SECURITY.md](SECURITY.md).

## Development setup

Build on macOS 13 or newer. Install the Xcode Command Line Tools:

```sh
xcode-select --install
xcode-select -p
```

If the tools are already installed, verify the selected developer directory with
the second command. For linker or SDK problems, see
[Support](SUPPORT.md#development-builds).

Install Rust through the official [rustup installer](https://rustup.rs/):

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Choose the default installation, restart Terminal, and verify it:

```sh
rustc --version
cargo --version
rustup --version
```

Clone and run the project:

```sh
git clone https://github.com/jeremyc2/Gatto.git
cd Gatto
rustup component add rustfmt clippy
cargo fetch --locked
cargo run
```

Entering this repository selects the stable toolchain in `rust-toolchain.toml`,
including formatter and linter components. Follow the
[app setup guide](docs/SETUP.md#2-connect-github-cli) for GitHub CLI authentication
before trying uploads. To create an optimized binary, run `cargo build --release`.
A source build can run on Intel macOS; published bundles currently target Apple Silicon.

## Project layout

| Path | Contents |
| --- | --- |
| `src/` | Rust interface, GitHub uploads, and macOS integration. |
| `web/` | Solid website, Bun build and development tools, and website tests. |
| `docs/` | User guides, architecture, decisions, and research. |
| `scripts/` and `packaging/` | Build, packaging, release tools, and assets. |
| `.github/` | Contribution templates and retained workflow definitions. |

## Making a change

1. Create a focused branch from the default branch.
2. Keep the change small enough to review comfortably.
3. Add or update tests when behavior changes.
4. Update the README, architecture notes, and changelog when relevant.
5. Avoid committing credentials, generated build output, or local editor files.

Keep platform-specific code isolated and preserve the menu bar lifecycle. User
errors should be actionable, and failed uploads should not discard a staged image.
New UI should remain keyboard accessible and usable with macOS accessibility
features.

Clipboard writes must always follow an explicit user action. Changes to settings
storage or Start at Login must preserve the per-user scope documented in
[the architecture notes](docs/ARCHITECTURE.md) and must never persist GitHub
credentials or image content.

## Code quality

Before opening a pull request, run:

```sh
cargo install --locked cargo-deny --version 0.20.2
cargo fmt --all -- --check
cargo check --all-targets
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets
./scripts/test-updater.sh
bun typecheck:scripts
bun test scripts
cargo deny --locked check --hide-inclusion-graph advisories licenses bans sources
```

Apply formatting with `cargo fmt --all`. Please resolve warnings rather than
silencing them unless there is a documented reason.

The native updater tests download the pinned Sparkle SDK and compile a Swift
harness and signed test feed. `bun scripts/test-update-install.ts` also checks
Sparkle’s scheduler and installs/relaunches disposable app bundles. These use
isolated preferences and temporary test signing keys; ordinary
`cargo run` does not load Sparkle or contact the release feed. See
[Releasing](docs/RELEASING.md#one-time-updater-signing-setup) for bundled builds.

Release-workflow tests mock native tools, Git and GitHub; they never publish or
push real code. The normal release script runs these checks before offering to
commit code. See [Releasing](docs/RELEASING.md#release-with-one-command).

Root release scripts and their tests use the root `tsconfig.json`, which loads
Bun's types for editor diagnostics and `bun typecheck:scripts`.

### Website

Use Bun for website tooling. From the repository root:

```sh
bun install --frozen-lockfile
bun build:web
bun lint:web
bun test:web
```

Run `bun dev:web` for a local preview and `bun format:web` to apply formatting.
[Website development](web/README.md) owns the compiler, asset, dependency and
preview details. Preserve the approved design and copy during implementation
changes. Website checks do not replace the Rust checks for app changes.

With the beta `cf` CLI installed and signed in (`cf auth login`), `bun deploy:web`
builds and updates the existing `gatto` Worker. Set `CLOUDFLARE_ACCOUNT_ID` in your
environment or the ignored root `.env` file. Use `bun deploy:web --dry-run` to
validate without uploading.

## Documentation and manual verification

Follow [documentation upkeep](docs/agents/documentation.md) for each guide’s scope,
release-size updates, and link checks. Record user-visible changes under
**Unreleased** in [CHANGELOG.md](CHANGELOG.md).

### Walkthrough screenshots

Run `./scripts/update-walkthrough-screenshots.sh` on macOS and inspect the PNGs.
The script renders the production GPUI interface through its off-screen Metal
renderer with fixed in-memory examples. It does not contact GitHub, read or
modify saved preferences, use UI automation, or require Screen Recording
permission. It overwrites `docs/images/walkthrough/`, including the Rooftops game,
results, and exported scorecard images. Game captures show an early coin jump,
a mid-level owl encounter, and the results screen in `rooftops-1.png` through
`rooftops-3.png`; the scorecard is `rooftops-score.png`.

For behavior changes, manually exercise the affected workflow with non-sensitive
images and an accessible test repository. Relevant paths include first-run setup,
paste/drop/file selection, a partially failed batch and retry, Markdown copying,
tray and global shortcut actions, and closing/reopening utility windows. Check
bundled builds for window lifecycle or custom URL changes; `cargo run` does not
register the packaged URL scheme. Reset and login-item checks modify your local
preferences, so use a disposable macOS user if those settings need to be preserved.

### Automation status

The CI and dependency-security workflow definitions currently live in
`.github/workflows.disabled/`. They are inactive, including manual security runs;
contributors must perform the checks above locally. Dependabot's weekly update
configuration remains in `.github/dependabot.yml`. Restoring workflow files to
`.github/workflows/` is a separate repository change.

## Dependencies

The app uses Rust, GPUI, and GPUI Kit. GPUI Kit is pinned to `0.7.3`, which pins
and re-exports its compatible GPUI `0.3.9` snapshot. Import GPUI through that
facade; upgrade the pair together. Check `Cargo.toml` for the minimum Rust version
and `rust-toolchain.toml` for the selected toolchain.

`tests/fixtures/sample.mp4` is a locally generated, two-second H.264 fixture
with ten 64 × 64 solid-color frames and no audio or external artwork.
`sample-audio.mp4` adds a locally generated 440 Hz test tone through a native
AVFoundation export. Tests check actual native H.264/AAC output, dimensions,
duration, cancellation, and oversized-import choices without contacting GitHub
or opening a player.

Media uploads use Tokio and futures-util for streamed async HTTP and cancellation
inside background workers; `tempfile` provides automatically removed, privately
permissioned video snapshots. These were already transitive dependencies; the
new direct requirements enable ownership and cancellation that a blocking body
clone could not provide. Compression uses narrowly enabled `objc2` AVFoundation, AVFAudio, CoreMedia,
CoreVideo, and VideoToolbox bindings. They link macOS's existing decoders and
hardware encoder; no software codec, capture framework, or external encoder
executable is bundled. The first four additions are framework glue; the
VideoToolbox binding supplies the hardware-requirement key.

Keep new dependencies to a minimum. In a dependency change, explain:

- Why the dependency is needed.
- Why an existing dependency or the standard library is insufficient.
- Any security, licensing, binary-size, or platform implications.

When updating GPUI Kit, confirm its published package metadata pins a compatible
GPUI release. Update the compatibility note above in the same pull request.
Commit the resulting `Cargo.lock` change, review every newly introduced package and
source, and do not bypass an advisory without a documented, time-bounded rationale.

## Pull requests

A good pull request includes:

- A concise description of the problem and the chosen solution.
- Links to related issues.
- Screenshots or a short recording for visible UI changes.
- A description of manual verification and automated checks performed.
- Any remaining risks or follow-up work.

Maintainers may ask for a change to be split when unrelated concerns are combined.
Small, descriptive commits are appreciated, but a perfectly polished commit
history is not required.

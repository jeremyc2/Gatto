# Contributing

Thank you for helping improve GitHub Image Upload. Contributions of code,
documentation, bug reports, and focused feature proposals are welcome.

By participating, you agree to follow the [Code of Conduct](CODE_OF_CONDUCT.md).

## Before you begin

- Read the setup instructions in [README.md](README.md).
- Search existing issues and pull requests before opening a new one.
- For a substantial change, open an issue first so the approach and scope can be
  discussed before implementation.
- Report security problems privately as described in [SECURITY.md](SECURITY.md).

## Development setup

The project currently targets macOS. Install the Xcode Command Line Tools, Rust
through `rustup`, and GitHub CLI as described in the README. Then fetch the Rust
dependencies:

```sh
cargo fetch
```

The repository uses the stable Rust toolchain selected by `rust-toolchain.toml`.
GPUI Kit is pinned deliberately because it re-exports a matching GPUI version; do
not upgrade GPUI independently.

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
cargo fmt --all -- --check
cargo check --all-targets
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets
cargo deny --locked check --hide-inclusion-graph advisories licenses bans sources
```

Apply formatting with `cargo fmt --all`. Please resolve warnings rather than
silencing them unless there is a documented reason.

## Dependencies

Keep new dependencies to a minimum. In a dependency change, explain:

- Why the dependency is needed.
- Why an existing dependency or the standard library is insufficient.
- Any security, licensing, binary-size, or platform implications.

When updating GPUI Kit, confirm its published package metadata pins a compatible
GPUI release. Update the compatibility note in the README in the same pull request.
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

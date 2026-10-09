---
name: changelog
description: Update CHANGELOG.md and make the next semantic version release-ready.
disable-model-invocation: true
---

Prepare the next release from every user-facing change since the changelog was last published.

## Steps

1. Establish the **published boundary** before editing anything.

   - Fetch the default remote branch when available.
   - Find the newest GitHub release tag. Its target commit is the boundary.
   - When GitHub cannot be queried, use the newest reachable `v*` tag that matches the latest released version in `CHANGELOG.md`.
   - If neither source identifies one unambiguous boundary, stop and ask the user; do not guess which changes were already released.

2. Inventory the release scope: every committed change after the boundary through `HEAD`, plus all staged and unstaged working-tree changes. Read the affected code, tests, and documentation as needed to describe behaviour rather than commit messages. Exclude the existing unreleased changelog text from the evidence, but preserve it if it is still accurate.

3. Choose the smallest SemVer bump that covers the release:

   - **major**: a user-facing compatibility break, removed supported capability, or required migration;
   - **minor**: a backward-compatible user-facing capability;
   - **patch**: fixes, security or dependency updates, documentation-only changes, and internal changes with no new user-facing capability.

   When changes span categories, use the highest applicable bump. State the choice and the evidence in the final report. If compatibility is ambiguous, ask the user before editing the version.

4. Update `CHANGELOG.md` in Keep a Changelog format.

   - Replace the `Unreleased` heading with a dated `## [x.y.z] - YYYY-MM-DD` release heading and insert a fresh empty `## [Unreleased]` directly above it.
   - Preserve the established category order and use only categories with entries: `Added`, `Changed`, `Deprecated`, `Removed`, `Fixed`, `Security`.
   - Write concise, user-visible bullets. Group one behavioural change once; omit mechanical churn and commit identifiers.
   - If the range has no releasable change, leave files untouched and report that result.

5. Apply the new version consistently.

   - Treat `Cargo.toml` as the release source of truth; update its package version.
   - Refresh `Cargo.lock` using Cargo so the root package version matches.
   - Update `packaging/Info.plist`'s `CFBundleShortVersionString` to the same release version, because the distributed macOS bundle exposes that value to users.
   - Search tracked release and packaging files for the prior version and update only values that identify this application release. Do not alter dependency version constraints.

6. Inspect the final diff and run the narrowest relevant checks. At minimum, verify the manifest, lockfile, changelog heading, and bundle version all agree; run `plutil -lint packaging/Info.plist` when that file exists. Report the boundary, version transition, changelog categories, and checks run.

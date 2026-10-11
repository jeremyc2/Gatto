# Releasing Gatto

Releases are built locally on macOS with `scripts/release.sh`. The script produces
a Developer ID signed and notarized Apple Silicon app, then publishes its ZIP and
checksum to GitHub Releases. There is no active release workflow in GitHub Actions.

## Prerequisites

- The [development tools](../README.md#prerequisites) and an authenticated `gh`
  account with permission to create releases in this repository.
- Apple signing/notarization tools available through `security`, `codesign`,
  `xcrun notarytool`, `xcrun stapler`, `spctl`, `plutil`, and `ditto`.
- A **Developer ID Application** certificate and its private key in the login
  keychain. Check with `security find-identity -v -p codesigning`.
- Notarization credentials stored in the `gatto-notary` keychain profile. Follow
  the [one-time setup](../README.md#releasing); do not commit credentials.

| Environment variable | Purpose |
| --- | --- |
| `SIGN_IDENTITY` | Signing certificate name. Required if more than one Developer ID Application identity is present. |
| `NOTARY_PROFILE` | Existing notarization keychain profile; defaults to `gatto-notary`. |
| `GIT_COMMIT_HASH` | Build metadata override for ordinary Cargo builds. The release script sets this to the full release commit itself. |

## Prepare the release

1. Add release notes to [CHANGELOG.md](../CHANGELOG.md), moving the relevant
   Unreleased entries into the new version/date section.
2. Set the same version in `Cargo.toml` and
   `packaging/Info.plist` (`CFBundleShortVersionString`). Update `Cargo.lock` to
   reflect the package version with `cargo check`, and review the resulting diff.
   The release script copies the plist as-is; it does not synchronize these versions.
3. Run the [contributor checks](../CONTRIBUTING.md#code-quality). Manually verify
   changed workflows, regenerate affected screenshots, and review documentation.
4. Commit and push to `main`. The working tree must be clean and HEAD must equal
   `origin/main` after fetching. A release named `v<version>` must not already exist.

## Build and publish

From the repository root:

```sh
./scripts/release.sh
```

The script installs the `aarch64-apple-darwin` Rust target, performs a locked release
build, and creates `dist/release/Gatto.app` with the plist, binary, and icon assets.
It signs with hardened runtime and a timestamp, verifies the signature, submits
a temporary ZIP for notarization, staples and validates the ticket, and runs
Gatekeeper assessment. It then creates:

- `dist/release/gatto-<version>-aarch64-apple-darwin.zip`
- `dist/release/SHA256SUMS`

Inspect and launch the resulting bundle to verify the release behavior before
answering the final **Publish** prompt. Confirm that App Preferences shows the
expected version and commit, and check custom URLs and window closing/reopening.
Answering `y` creates the GitHub release/tag at the exact commit and uploads the
ZIP and checksum, with GitHub-generated release notes.

The script currently builds only Apple Silicon. Intel users need a local build;
changing the release targets requires a script change and separate verification.

## Interrupted or cancelled releases

Cancelling the publication prompt leaves the artifacts in `dist/release` and does
not publish a release. A rerun deletes and rebuilds that directory, including any
previous artifacts; copy anything you need to retain elsewhere first.

If signing or notarization fails, fix the certificate/profile issue and rerun.
If publication fails, inspect `gh release view v<version>` and its assets before
retrying: GitHub may have created the release even if an asset upload failed.
The script refuses to rerun against an existing release. Repair missing assets
on that release after checking its tag/commit rather than deleting it blindly.

## Verify the download

Download the ZIP and `SHA256SUMS` from the published release into the same directory:

```sh
shasum -a 256 -c SHA256SUMS
```

Unzip the archive, launch the app, and verify its version and commit. Move it to
`/Applications` before enabling Start at Login. Do not ask users to bypass a
failed signature or Gatekeeper assessment; investigate and replace the artifact.

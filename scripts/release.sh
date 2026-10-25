#!/usr/bin/env bash
# One-command app release: prepare, check, commit/push, sign, publish, and sync metadata.
# One-time credentials and recovery: docs/RELEASING.md
set -euo pipefail
cd "$(dirname "$0")/.."

fail() { echo "error: $*" >&2; exit 1; }
# Builds get no inherited publishing/signing tokens. This is environment hygiene,
# not a sandbox: use a separate credentialless build user for strong isolation.
build() (
  unset GH_TOKEN GITHUB_TOKEN GH_ENTERPRISE_TOKEN GITHUB_ENTERPRISE_TOKEN
  unset CLOUDFLARE_API_TOKEN CLOUDFLARE_API_KEY CF_API_TOKEN CF_API_KEY
  unset APPLE_ID APPLE_PASSWORD APPLE_APP_SPECIFIC_PASSWORD ASC_KEY_ID ASC_ISSUER_ID
  unset SIGN_IDENTITY NOTARY_PROFILE SPARKLE_ACCOUNT
  "$@"
)
mode=release
case "${1:-}" in
  "") ;;
  --resume) mode=resume ;;
  -h|--help)
    echo "Usage: ./scripts/release.sh [--resume]"
    echo "Guide a release from main, or finish an existing draft/published release."
    exit 0 ;;
  *) fail "usage: ./scripts/release.sh [--resume]" ;;
esac
[[ $# -le 1 ]] || fail "usage: ./scripts/release.sh [--resume]"
[[ "$(uname -s)" == Darwin ]] || fail "releases require macOS"
for tool in bun gh git cargo plutil hdiutil diskutil; do
  command -v "$tool" >/dev/null || fail "missing required tool: $tool; see docs/RELEASING.md"
done
[[ "$(git branch --show-current)" == main ]] || fail "merge your work into main, then run this command there"
git fetch --quiet origin main --tags
git merge-base --is-ancestor origin/main HEAD || fail "main is behind or diverged from origin/main; reconcile it first"
[[ -z "$(git diff --name-only --diff-filter=U)" ]] || fail "resolve merge conflicts first"

TARGET=aarch64-apple-darwin
APP_NAME=Gatto.app
NOTARY_PROFILE="${NOTARY_PROFILE:-gatto-notary}"
SPARKLE_ACCOUNT="${SPARKLE_ACCOUNT:-gatto-updates}"
dist=dist/release

refresh_readme() {
  if [[ -f "$dist/$archive" ]]; then
    bun scripts/update-readme-size.ts "$@" --tag "$tag" --dmg "$dist/$archive"
  else
    bun scripts/update-readme-size.ts "$@" --tag "$tag"
  fi
}

# Both ordinary publication and recovery use the version's channel.
set_channel() {
  channel=stable
  publish_flags=(--latest)
  create_flags=()
  feed_flags=()
  if [[ "$version" =~ -beta\.[1-9][0-9]*$ ]]; then
    channel=beta
    publish_flags=(--prerelease --latest=false)
    create_flags=(--prerelease --latest=false)
    feed_flags=(--channel beta)
  fi
}

sync_metadata() {
  ./scripts/publish-beta-feed.sh "$tag" \
    || fail "$tag is published; beta feed refresh failed; run ./scripts/release.sh --resume"
  if [[ "$channel" == beta ]]; then
    echo "Published $tag as a prerelease. Beta feed verified; stable README and Homebrew cask remain unchanged."
    return
  fi
  refresh_readme \
    || fail "$tag is published; fix the error and run ./scripts/release.sh --resume"
  refresh_readme --check \
    || fail "$tag is published; README verification failed; run ./scripts/release.sh --resume"
  bun scripts/update-homebrew-cask.ts --tag "$tag" \
    || fail "$tag is published; cask refresh failed; run ./scripts/release.sh --resume"
  bun scripts/update-homebrew-cask.ts --check --tag "$tag" \
    || fail "$tag is published; cask verification failed; run ./scripts/release.sh --resume"
  git add -- README.md Casks/gatto.rb
  if ! git diff --cached --quiet; then
    git commit -m "chore: refresh release metadata for $tag"
  fi
  git push origin HEAD:main \
    || fail "$tag is published and metadata is committed locally; run ./scripts/release.sh --resume to retry the push"
  echo "Published $tag. README and Homebrew cask are verified, committed and pushed."
}

if [[ "$mode" == resume ]]; then
  gh auth status >/dev/null 2>&1 || fail "gh is not authenticated; run gh auth login"
  version="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)"
  set_channel
  tag="v$version"
  archive="gatto-$version-$TARGET.dmg"
  draft="$(gh release view "$tag" --json isDraft --jq .isDraft)" \
    || fail "no release exists for $tag; run ./scripts/release.sh normally"
  release_base="$tag"
  if [[ "$draft" == true ]]; then
    release_base="$(gh release view "$tag" --json targetCommitish --jq .targetCommitish)"
    [[ "$release_base" =~ ^[0-9a-f]{40}$ ]] || fail "draft target must identify the exact release commit"
  fi
  git rev-parse --verify "$release_base^{commit}" >/dev/null || fail "the release commit cannot be resolved"
  git diff --quiet "$release_base" -- . ':!README.md' ':!Casks/gatto.rb' \
    || fail "code differs from $tag; resume using its release checkout"
  [[ -z "$(git ls-files --others --exclude-standard)" ]] || fail "untracked files are present; preserve or commit them before resuming"
  if [[ "$draft" == true ]]; then
    for asset in "$archive" appcast.xml SHA256SUMS; do
      [[ -f "$dist/$asset" ]] || fail "draft recovery needs the original $dist/$asset"
    done
    gh release upload "$tag" "$dist/$archive" "$dist/appcast.xml" "$dist/SHA256SUMS" --clobber
    bun scripts/verify-release.ts --tag "$tag" --directory "$dist"
    read -r -p "Publish verified draft $tag? [y/N] " answer
    [[ "$answer" == y || "$answer" == Y ]] || fail "cancelled; verified draft retained"
    gh release edit "$tag" --draft=false "${publish_flags[@]}"
  else
    bun scripts/verify-release.ts --tag "$tag"
  fi
  sync_metadata
  exit 0
fi

for tool in rustup security codesign xcrun spctl ditto ruby; do
  command -v "$tool" >/dev/null || fail "missing required tool: $tool"
done
cargo deny --version >/dev/null 2>&1 \
  || fail "install the one-time audit tool: cargo install --locked cargo-deny --version 0.20.2"
build ./scripts/prepare-updater.sh

bun scripts/prepare-release.ts
version="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)"
set_channel
tag="v$version"
if gh release view "$tag" >/dev/null 2>&1; then
  fail "$tag already has a release; use --resume to finish it"
fi

# Run required checks here so releasing does not rely on a remembered checklist.
build bun --no-env-file install --frozen-lockfile --ignore-scripts
build bun --no-env-file audit:dependencies
build cargo check --locked --all-targets
bun scripts/prepare-release.ts --check
plutil -lint packaging/Info.plist
build cargo fmt --all -- --check
build cargo clippy --locked --all-targets --all-features -- -D warnings
build cargo test --locked --all-targets
build cargo deny --locked check --hide-inclusion-graph advisories licenses bans sources
build bun --no-env-file typecheck:scripts
build bun --no-env-file lint:scripts
build bun --no-env-file test:scripts
build ./scripts/test-updater.sh
build ./scripts/test-platform.sh
build bun --no-env-file scripts/test-update-install.ts
ruby -c Casks/gatto.rb
build bun --no-env-file build:web
build bun --no-env-file lint:web
build bun --no-env-file typecheck:web
build bun --no-env-file test:web

# Refresh documentation from this checkout before reviewing the release commit.
build ./scripts/update-walkthrough-screenshots.sh

git status --short
git diff --stat HEAD
read -r -p "Commit and push these changes for $tag, then build the release? [y/N] " answer
[[ "$answer" == y || "$answer" == Y ]] || fail "cancelled; prepared version and notes remain for review"
git add --all
if ! git diff --cached --quiet; then git commit -m "Release $tag"; fi
[[ -z "$(git status --porcelain)" ]] || fail "commit hooks or another process left changes; review them before publishing"
git push origin HEAD:main
release_commit="$(git rev-parse HEAD)"
if git rev-parse --verify "refs/tags/$tag" >/dev/null 2>&1; then
  [[ "$(git rev-parse "$tag^{commit}")" == "$release_commit" ]] || fail "$tag already points to another commit"
fi

build rustup target add "$TARGET" > /dev/null
GIT_COMMIT_HASH="$(git rev-parse HEAD)" build cargo build --locked --release --target "$TARGET"

# No third-party compilation or install steps follow credential preflight.
gh auth status >/dev/null 2>&1 || fail "gh is not authenticated; run gh auth login"
if [[ -z "${SIGN_IDENTITY:-}" ]]; then
  identities="$(security find-identity -v -p codesigning | grep 'Developer ID Application' || true)"
  [[ -n "$identities" ]] || fail "no Developer ID Application certificate found in the keychain"
  [[ "$(wc -l <<< "$identities")" -eq 1 ]] || fail "several identities found; set SIGN_IDENTITY"
  SIGN_IDENTITY="$(sed -E 's/.*"(.*)".*/\1/' <<< "$identities")"
fi
xcrun notarytool history --keychain-profile "$NOTARY_PROFILE" >/dev/null 2>&1 \
  || fail "notarytool profile '$NOTARY_PROFILE' is missing or invalid"
sparkle_key="$(dist/sparkle/bin/generate_keys --account "$SPARKLE_ACCOUNT" -p)" \
  || fail "create the one-time Sparkle signing key; see docs/RELEASING.md"
[[ -n "$sparkle_key" ]] || fail "Sparkle signing key is empty"
echo "Releasing $tag signed as: $SIGN_IDENTITY"

dist="dist/release"
bundle="$dist/$APP_NAME"
rm -rf "$dist"
mkdir -p "$bundle/Contents/MacOS" "$bundle/Contents/Resources"
plutil -lint packaging/Info.plist
cp packaging/Info.plist "$bundle/Contents/Info.plist"
plutil -insert SUPublicEDKey -string "$sparkle_key" "$bundle/Contents/Info.plist"
mkdir -p "$bundle/Contents/Frameworks"
ditto dist/sparkle/Sparkle.framework "$bundle/Contents/Frameworks/Sparkle.framework"
cp dist/sparkle/libGattoUpdater.dylib "$bundle/Contents/Frameworks/"
# Sign nested executables from the inside out; never use codesign --deep to sign.
framework="$bundle/Contents/Frameworks/Sparkle.framework/Versions/B"
for nested in "$framework/XPCServices/Downloader.xpc" \
              "$framework/XPCServices/Installer.xpc" \
              "$framework/Autoupdate" "$framework/Updater.app" \
              "$bundle/Contents/Frameworks/Sparkle.framework" \
              "$bundle/Contents/Frameworks/libGattoUpdater.dylib"; do
  codesign --force --options runtime --timestamp --sign "$SIGN_IDENTITY" "$nested"
done
cp packaging/AppIcon.icns packaging/Assets.car "$bundle/Contents/Resources/"
cp dist/sparkle/LICENSE "$bundle/Contents/Resources/Sparkle-LICENSE"
cp "target/$TARGET/release/gatto" "$bundle/Contents/MacOS/"

# Pair the app with its installer before signing. Compression preserves this
# volume's UUID, letting the installed app identify it without reading its files.
installer_template="$dist/installer-template.dmg"
./scripts/build-dmg.sh --prepare "$bundle" "$installer_template"

# The hardened runtime and a secure timestamp are required for notarization.
codesign --force --options runtime --timestamp --sign "$SIGN_IDENTITY" "$bundle"
codesign --verify --deep --strict --verbose=2 "$bundle"

notarize_zip="$dist/notarize.zip"
ditto -c -k --keepParent "$bundle" "$notarize_zip"
xcrun notarytool submit "$notarize_zip" --keychain-profile "$NOTARY_PROFILE" --wait
rm "$notarize_zip"

xcrun stapler staple "$bundle"
xcrun stapler validate "$bundle"
spctl --assess --type execute --verbose=2 "$bundle"

archive="gatto-$version-$TARGET.dmg"
./scripts/build-dmg.sh "$bundle" "$dist/$archive" "$installer_template"
rm "$installer_template"
codesign --timestamp --sign "$SIGN_IDENTITY" "$dist/$archive"
codesign --verify --strict --verbose=2 "$dist/$archive"
xcrun notarytool submit "$dist/$archive" --keychain-profile "$NOTARY_PROFILE" --wait
xcrun stapler staple "$dist/$archive"
xcrun stapler validate "$dist/$archive"
spctl --assess --type open --context context:primary-signature --verbose=2 "$dist/$archive"
dist/sparkle/bin/generate_appcast --account "$SPARKLE_ACCOUNT" \
  --download-url-prefix "https://github.com/jeremyc2/Gatto/releases/download/$tag/" \
  --link "https://github.com/jeremyc2/Gatto/releases/tag/$tag" \
  --maximum-deltas 0 ${feed_flags[@]+"${feed_flags[@]}"} "$dist"
dist/sparkle/bin/sign_update --account "$SPARKLE_ACCOUNT" --verify "$dist/appcast.xml"
(cd "$dist" && shasum -a 256 "$archive" appcast.xml > SHA256SUMS)

read -r -p "Publish $tag to GitHub Releases? [y/N] " answer
[[ "$answer" == y || "$answer" == Y ]] || fail "cancelled; signed artifacts are in $dist"

notes_file="$(mktemp "${TMPDIR:-/tmp}/gatto-release-notes.XXXXXX")"
trap 'rm -f "$notes_file"' EXIT
bun scripts/prepare-release.ts --print-notes > "$notes_file"
# Keep incomplete uploads in a draft until their bytes have been verified.
gh release create "$tag" "$dist/$archive" "$dist/appcast.xml" "$dist/SHA256SUMS" \
  --target "$release_commit" --title "$tag" --notes-file "$notes_file" --draft ${create_flags[@]+"${create_flags[@]}"}
bun scripts/verify-release.ts --tag "$tag" --directory "$dist" \
  || fail "draft upload verification failed; run ./scripts/release.sh --resume"
gh release edit "$tag" --draft=false "${publish_flags[@]}"
sync_metadata

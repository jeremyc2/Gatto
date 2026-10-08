#!/usr/bin/env bash
# Builds, signs, notarizes, and publishes a GitHub release from this Mac.
#
# Requires (see README, "Releasing"):
#   - A "Developer ID Application" certificate in the login keychain
#   - A notarytool keychain profile (xcrun notarytool store-credentials)
#   - gh authenticated with permission to create releases
#
# Optional environment:
#   SIGN_IDENTITY   Certificate name; auto-detected when exactly one exists
#   NOTARY_PROFILE  notarytool keychain profile (default: gatto-notary)
set -euo pipefail

cd "$(dirname "$0")/.."

TARGET="aarch64-apple-darwin"
APP_NAME="Gatto.app"
NOTARY_PROFILE="${NOTARY_PROFILE:-gatto-notary}"

fail() {
  echo "error: $*" >&2
  exit 1
}

version="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)"
tag="v$version"

[[ "$(git branch --show-current)" == "main" ]] || fail "run releases from the main branch"
[[ -z "$(git status --porcelain)" ]] || fail "the working tree has uncommitted changes"
git fetch --quiet origin main
[[ "$(git rev-parse HEAD)" == "$(git rev-parse origin/main)" ]] || fail "HEAD is not pushed to origin/main"
gh auth status > /dev/null 2>&1 || fail "gh is not authenticated; run gh auth login"
if gh release view "$tag" > /dev/null 2>&1; then
  fail "release $tag already exists; bump the version in Cargo.toml first"
fi

if [[ -z "${SIGN_IDENTITY:-}" ]]; then
  identities="$(security find-identity -v -p codesigning | grep "Developer ID Application" || true)"
  [[ -n "$identities" ]] || fail "no Developer ID Application certificate found in the keychain"
  [[ "$(wc -l <<< "$identities")" -eq 1 ]] || fail "several identities found; set SIGN_IDENTITY"
  SIGN_IDENTITY="$(sed -E 's/.*"(.*)".*/\1/' <<< "$identities")"
fi
xcrun notarytool history --keychain-profile "$NOTARY_PROFILE" > /dev/null 2>&1 \
  || fail "notarytool profile '$NOTARY_PROFILE' is missing or invalid"

echo "Releasing $tag signed as: $SIGN_IDENTITY"

rustup target add "$TARGET" > /dev/null
GIT_COMMIT_HASH="$(git rev-parse HEAD)" cargo build --locked --release --target "$TARGET"

dist="dist/release"
bundle="$dist/$APP_NAME"
rm -rf "$dist"
mkdir -p "$bundle/Contents/MacOS" "$bundle/Contents/Resources"
plutil -lint packaging/Info.plist
cp packaging/Info.plist "$bundle/Contents/Info.plist"
cp packaging/AppIcon.icns packaging/Assets.car "$bundle/Contents/Resources/"
cp "target/$TARGET/release/gatto" "$bundle/Contents/MacOS/"

# The hardened runtime and a secure timestamp are required for notarization.
codesign --force --options runtime --timestamp --sign "$SIGN_IDENTITY" "$bundle"
codesign --verify --strict --verbose=2 "$bundle"

notarize_zip="$dist/notarize.zip"
ditto -c -k --keepParent "$bundle" "$notarize_zip"
xcrun notarytool submit "$notarize_zip" --keychain-profile "$NOTARY_PROFILE" --wait
rm "$notarize_zip"

xcrun stapler staple "$bundle"
xcrun stapler validate "$bundle"
spctl --assess --type execute --verbose=2 "$bundle"

archive="gatto-$version-$TARGET.zip"
ditto -c -k --keepParent "$bundle" "$dist/$archive"
(cd "$dist" && shasum -a 256 "$archive" > SHA256SUMS)

read -r -p "Publish $tag to GitHub Releases? [y/N] " answer
[[ "$answer" == "y" || "$answer" == "Y" ]] || fail "cancelled; artifacts are in $dist"

gh release create "$tag" "$dist/$archive" "$dist/SHA256SUMS" \
  --target "$(git rev-parse HEAD)" \
  --title "$tag" \
  --generate-notes

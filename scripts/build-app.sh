#!/usr/bin/env bash
# Compile and package a local app. No publishing, notarization, or signing credentials.
set -euo pipefail
cd "$(dirname "$0")/.."

fail() { echo "error: $*" >&2; exit 1; }
profile=debug
build_flags=(--locked --bin gatto --target-dir target)
case "${1:-}" in
  "") ;;
  --release) profile=release; build_flags+=(--release) ;;
  -h|--help)
    echo "Usage: ./scripts/build-app.sh [--release]"
    echo "Compile a fresh local Gatto.app in dist/local (debug by default)."
    exit 0 ;;
  *) fail "usage: $0 [--release]" ;;
esac
[[ $# -le 1 ]] || fail "usage: $0 [--release]"
[[ "$(uname -s)" == Darwin ]] || fail "app bundles require macOS"

GIT_COMMIT_HASH="$(git rev-parse HEAD)" cargo build "${build_flags[@]}"
plutil -lint packaging/Info.plist
mkdir -p dist/local
work="$(mktemp -d "$PWD/dist/local/.gatto-app.XXXXXX")"
trap 'rm -rf "$work"' EXIT
bundle="$work/Gatto.app"
mkdir -p "$bundle/Contents/MacOS" "$bundle/Contents/Resources"
cp packaging/Info.plist "$bundle/Contents/Info.plist"
cp packaging/AppIcon.icns packaging/Assets.car "$bundle/Contents/Resources/"
cp packaging/licenses/* "$bundle/Contents/Resources/"
cp "target/$profile/gatto" "$bundle/Contents/MacOS/"
# Local apps do not bundle Sparkle or contact its release feed.
plutil -remove SUFeedURL "$bundle/Contents/Info.plist"
codesign --force --sign - "$bundle"
codesign --verify --deep --strict "$bundle"
rm -rf dist/local/Gatto.app
mv "$bundle" dist/local/Gatto.app
echo "Created $PWD/dist/local/Gatto.app"
echo "Run: open \"$PWD/dist/local/Gatto.app\""
echo "To package a disk image: ./scripts/build-dmg.sh"

#!/usr/bin/env bash
# Package an existing Gatto.app in a branded drag-to-Applications disk image.
# Usage: ./scripts/build-dmg.sh /path/to/Gatto.app /path/to/output.dmg
# Signing and notarization belong to release.sh; this script is also a local preview tool.
set -euo pipefail

fail() { echo "error: $*" >&2; exit 1; }
[[ "$(uname -s)" == "Darwin" ]] || fail "disk images must be built on macOS"
[[ $# -eq 2 ]] || fail "usage: $0 /path/to/Gatto.app /path/to/output.dmg"
repo_root="$(cd "$(dirname "$0")/.." && pwd)"
bundle="$(cd "$(dirname "$1")" && pwd)/$(basename "$1")"
[[ "$(basename "$bundle")" == "Gatto.app" && -f "$bundle/Contents/Info.plist" ]] \
  || fail "expected a Gatto.app bundle with Contents/Info.plist"
mkdir -p "$(dirname "$2")"
output="$(cd "$(dirname "$2")" && pwd)/$(basename "$2")"
[[ "$output" == *.dmg ]] || fail "output must end in .dmg"
[[ ! -e "$output" ]] || fail "output already exists: $output"

work="$(mktemp -d "${TMPDIR:-/tmp}/gatto-dmg.XXXXXX")"
mount="$work/volume"
mounted=false
cleanup() {
  if [[ "$mounted" == true ]]; then
    if ! hdiutil detach "$mount" -quiet; then
      echo "Could not eject $mount; temporary files retained in $work" >&2
      return
    fi
  fi
  rm -rf "$work"
}
trap cleanup EXIT

mkdir -p "$work/staging/.background"
ditto "$bundle" "$work/staging/Gatto.app"
ln -s /Applications "$work/staging/Applications"
swift "$repo_root/packaging/render-dmg-background.swift" "$work/artwork"
# A multi-resolution TIFF preserves the same point size on Retina and non-Retina Macs.
tiffutil -cathidpicheck "$work/artwork/background.tiff" "$work/artwork/background@2x.tiff" \
  -out "$work/staging/.background/background.tiff" > /dev/null

hdiutil create -quiet -volname "Install Gatto" -fs HFS+ -format UDRW \
  -srcfolder "$work/staging" "$work/writable.dmg"
hdiutil attach -quiet -noautoopen -mountpoint "$mount" "$work/writable.dmg"
mounted=true
osascript "$repo_root/packaging/configure-dmg.applescript" "$mount"
[[ -s "$mount/.DS_Store" ]] || fail "Finder did not save the installation window layout"
sync
hdiutil detach -quiet "$mount"
mounted=false
hdiutil convert -quiet "$work/writable.dmg" -format UDZO -imagekey zlib-level=9 -o "$output"
hdiutil verify -quiet "$output"
echo "Created $output"

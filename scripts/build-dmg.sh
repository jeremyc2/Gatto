#!/usr/bin/env bash
# Package an existing Gatto.app in a branded drag-to-Applications disk image.
# Usage: ./scripts/build-dmg.sh [/path/to/Gatto.app [/path/to/output.dmg]]
# Defaults: dist/release/Gatto.app and dist/gatto-preview.dmg in this repository.
# Release preparation: --prepare <unsigned-app> <writable-template.dmg>
# Release completion: <signed-app> <output.dmg> <writable-template.dmg>
# Signing and notarization belong to release.sh; this script is also a local preview tool.
set -euo pipefail

fail() { echo "error: $*" >&2; exit 1; }
[[ "$(uname -s)" == "Darwin" ]] || fail "disk images must be built on macOS"
mode=build
if [[ "${1:-}" == --prepare ]]; then
  mode=prepare
  shift
fi
[[ ( "$mode" == prepare && $# -eq 2 ) || ( "$mode" == build && $# -le 3 ) ]] \
  || fail "usage: $0 [app-bundle [output.dmg [template.dmg]]] or $0 --prepare app-bundle output.dmg"
repo_root="$(cd "$(dirname "$0")/.." && pwd)"
bundle_path="${1:-$repo_root/dist/release/Gatto.app}"
output_path="${2:-$repo_root/dist/gatto-preview.dmg}"
bundle="$(cd "$(dirname "$bundle_path")" && pwd)/$(basename "$bundle_path")"
[[ "$(basename "$bundle")" == "Gatto.app" && -f "$bundle/Contents/Info.plist" ]] \
  || fail "expected a Gatto.app bundle with Contents/Info.plist"
mkdir -p "$(dirname "$output_path")"
output="$(cd "$(dirname "$output_path")" && pwd)/$(basename "$output_path")"
[[ "$output" == *.dmg ]] || fail "output must end in .dmg"
[[ ! -e "$output" ]] || fail "output already exists: $output"
template="${3:-}"
if [[ -n "$template" ]]; then
  template="$(cd "$(dirname "$template")" && pwd)/$(basename "$template")"
  [[ -f "$template" ]] || fail "missing installer template: $template"
fi
if [[ "$mode" == prepare ]]; then
  # Mutating a signed bundle would invalidate its signature. Release preparation
  # must happen before signing and notarization, never on an existing release.
  # Rust's linker gives the executable an ad-hoc signature even before bundle
  # signing. The bundle resource seal is what protects Info.plist here.
  if [[ -e "$bundle/Contents/_CodeSignature/CodeResources" ]]; then
    fail "installer preparation requires an unsigned app"
  fi
fi

work="$(mktemp -d "${TMPDIR:-/tmp}/gatto-dmg.XXXXXX")"
mount="$work/volume"
mounted=false
cleanup() {
  if [[ "$mounted" == true ]]; then
    if ! diskutil eject "$mount" > /dev/null; then
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

if [[ -n "$template" ]]; then
  cp "$template" "$work/writable.dmg"
else
  hdiutil create -quiet -volname "Install Gatto" -fs HFS+ -format UDRW \
    -srcfolder "$work/staging" "$work/writable.dmg"
fi
diskutil image attach --mountPoint "$mount" --mountOptions nobrowse "$work/writable.dmg" > /dev/null
mounted=true
volume_uuid="$(diskutil info -plist "$mount" | plutil -extract VolumeUUID raw -o - -)"
[[ "$volume_uuid" =~ ^[[:xdigit:]]{8}-[[:xdigit:]]{4}-[[:xdigit:]]{4}-[[:xdigit:]]{4}-[[:xdigit:]]{12}$ ]] \
  || fail "could not identify installer volume UUID"
if [[ "$mode" == prepare ]]; then
  plutil -insert GattoInstallerVolumeUUID -string "$volume_uuid" "$bundle/Contents/Info.plist"
  diskutil eject "$mount" > /dev/null
  mounted=false
  mv "$work/writable.dmg" "$output"
  echo "Prepared installer template $output"
  exit 0
fi
if [[ -n "$template" ]]; then
  expected_uuid="$(plutil -extract GattoInstallerVolumeUUID raw -o - "$bundle/Contents/Info.plist")"
  [[ "$expected_uuid" == "$volume_uuid" ]] || fail "app does not match installer template"
  rm -rf "$mount/Gatto.app"
  ditto "$work/staging/Gatto.app" "$mount/Gatto.app"
fi
osascript "$repo_root/packaging/configure-dmg.applescript" "$mount"
[[ -s "$mount/.DS_Store" ]] || fail "Finder did not save the installation window layout"
sync
diskutil eject "$mount" > /dev/null
mounted=false
hdiutil convert -quiet "$work/writable.dmg" -format UDZO -imagekey zlib-level=9 -o "$output"
hdiutil verify -quiet "$output"
echo "Created $output"

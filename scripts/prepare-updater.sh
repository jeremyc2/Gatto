#!/usr/bin/env bash
# Fetch a pinned Sparkle SDK and compile Gatto's native update driver.
set -euo pipefail
cd "$(dirname "$0")/.."
version=2.10.0
checksum=c2bf58aa8387266ac179357b1415d6f2635f044da8be41042af32425dae6da0c
sdk="dist/sparkle"
mkdir -p dist
archive="dist/Sparkle-$version.tar.xz"
if [[ ! -f "$archive" ]]; then
  curl --fail --location --proto '=https' --tlsv1.2 \
    "https://github.com/sparkle-project/Sparkle/releases/download/$version/Sparkle-$version.tar.xz" \
    --output "$archive"
fi
[[ "$(shasum -a 256 "$archive" | awk '{print $1}')" == "$checksum" ]] \
  || { echo "error: Sparkle SDK checksum mismatch" >&2; exit 1; }
# Always start with a clean SDK so no stale nested code can enter a release.
rm -rf "$sdk"
mkdir -p "$sdk"
tar -xf "$archive" -C "$sdk"
xcrun swiftc -emit-library -O -warnings-as-errors -target arm64-apple-macos13 \
  -F "$sdk" -framework Sparkle -Xlinker -rpath -Xlinker @loader_path \
  native/Updater.swift -o "$sdk/libGattoUpdater.dylib"

#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
./scripts/prepare-updater.sh
xcrun swiftc -warnings-as-errors -target arm64-apple-macos13 \
  -F dist/sparkle -framework Sparkle -Xlinker -rpath -Xlinker "$PWD/dist/sparkle" \
  native/Updater.swift native/UpdaterTests.swift -o dist/sparkle/updater-tests
dist/sparkle/updater-tests

./scripts/test-update-feed.sh

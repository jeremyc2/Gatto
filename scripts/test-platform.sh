#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
work="$(mktemp -d "${TMPDIR:-/tmp}/gatto-platform-tests.XXXXXX")"
trap 'rm -rf "$work"' EXIT
xcrun clang -fobjc-arc -fblocks -Wall -Wextra -Werror -Wno-unused-parameter \
  -Wno-deprecated-declarations -mmacosx-version-min=13.0 \
  native/DesktopTests.m -framework AppKit -framework Carbon -framework CoreGraphics \
  -o "$work/desktop-tests"
"$work/desktop-tests" "$PWD/packaging/menu-bar-icon.png"
xcrun clang -c -fobjc-arc -fblocks -Wall -Wextra -Werror -Wno-unused-parameter \
  -Wno-deprecated-declarations -mmacosx-version-min=13.0 native/Desktop.m -o "$work/desktop.o"
xcrun swiftc native/FinderIntegrationTests.swift "$work/desktop.o" \
  -framework AppKit -framework Carbon -framework CoreGraphics -o "$work/finder-integration-tests"
"$work/finder-integration-tests" "$PWD/packaging/Info.plist"
cargo run --locked --features walkthrough-screenshots --bin platform-tests

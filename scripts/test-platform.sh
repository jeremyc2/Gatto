#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
work="$(mktemp -d "${TMPDIR:-/tmp}/gatto-platform-tests.XXXXXX")"
trap 'rm -rf "$work"' EXIT
xcrun clang -fobjc-arc -fblocks -Wall -Wextra -Werror -Wno-unused-parameter \
  -Wno-deprecated-declarations -mmacosx-version-min=13.0 \
  native/HotkeyTests.m -framework AppKit -framework Carbon -framework CoreGraphics \
  -o "$work/desktop-tests"
"$work/desktop-tests"
cargo build --locked --features walkthrough-screenshots --bin platform-tests
./target/debug/platform-tests
xcrun swiftc native/FinderIntegrationTests.swift -framework AppKit -framework CoreServices \
  -o "$work/finder-integration-tests"
"$work/finder-integration-tests" "$PWD/packaging/Info.plist" "$PWD/target/debug/platform-tests"

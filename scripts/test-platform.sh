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
xcrun swiftc native/FinderActionTests.swift -o "$work/finder-action-tests"
"$work/finder-action-tests" "$PWD/packaging/Send to Gatto.workflow"
cargo run --locked --features walkthrough-screenshots --bin platform-tests

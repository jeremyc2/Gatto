#!/usr/bin/env bash
# Verify real Sparkle feed generation/signing using a disposable app and key.
set -euo pipefail
cd "$(dirname "$0")/.."
./scripts/prepare-updater.sh
work="$(mktemp -d "${TMPDIR:-/tmp}/gatto-feed-test.XXXXXX")"
trap 'rm -rf "$work"' EXIT
mkdir -p "$work/Gatto.app/Contents/MacOS" "$work/archives"
# Temporary test keys never touch the Keychain or real release credentials.
openssl rand -base64 32 > "$work/test-key"
chmod 600 "$work/test-key"
cat > "$work/key.swift" <<'SWIFT'
import Foundation
import CryptoKit
let seed = try Data(contentsOf: URL(fileURLWithPath: CommandLine.arguments[1]))
let key = try Curve25519.Signing.PrivateKey(rawRepresentation: Data(base64Encoded: String(decoding: seed, as: UTF8.self).trimmingCharacters(in: .whitespacesAndNewlines))!)
print(key.publicKey.rawRepresentation.base64EncodedString())
SWIFT
public_key="$(xcrun swift "$work/key.swift" "$work/test-key")"
cp packaging/Info.plist "$work/Gatto.app/Contents/Info.plist"
plutil -replace CFBundleIdentifier -string com.jeremy-chandler.gatto.feed-test "$work/Gatto.app/Contents/Info.plist"
plutil -insert SUPublicEDKey -string "$public_key" "$work/Gatto.app/Contents/Info.plist"
printf 'int main(void) { return 0; }\n' > "$work/main.c"
xcrun clang -arch arm64 -mmacosx-version-min=13.0 "$work/main.c" -o "$work/Gatto.app/Contents/MacOS/gatto"
codesign --force --sign - "$work/Gatto.app"
ditto -c -k --keepParent "$work/Gatto.app" "$work/archives/gatto-test.zip"
dist/sparkle/bin/generate_appcast --ed-key-file "$work/test-key" \
  --download-url-prefix https://example.com/releases/v0.14.0/ \
  --maximum-deltas 0 "$work/archives"
dist/sparkle/bin/sign_update --ed-key-file "$work/test-key" --verify "$work/archives/appcast.xml"
# A change to signed metadata must be rejected.
sed 's/0.14.0/0.15.0/g' "$work/archives/appcast.xml" > "$work/tampered.xml"
if dist/sparkle/bin/sign_update --ed-key-file "$work/test-key" --verify "$work/tampered.xml" > /dev/null 2>&1; then
  echo "error: accepted a tampered update feed" >&2
  exit 1
fi
echo "Passed: generated signed feed verifies; tampered feed rejected"

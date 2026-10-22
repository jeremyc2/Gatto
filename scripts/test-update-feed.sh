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

# Build two channel feeds, merge only their newest items, then re-sign. Inputs
# retain archive signatures; XML edits require a fresh feed signature.
for release_channel in stable beta; do
  directory="$work/$release_channel"
  mkdir -p "$directory"
  if [[ "$release_channel" == stable ]]; then
    version=0.16.3; build_version=0.16.3
    channel_args=()
  else
    version=0.17.0-beta.1; build_version=0.17.0b1
    channel_args=(--channel beta)
  fi
  plutil -replace CFBundleVersion -string "$build_version" "$work/Gatto.app/Contents/Info.plist"
  plutil -replace CFBundleShortVersionString -string "$version" "$work/Gatto.app/Contents/Info.plist"
  codesign --force --sign - "$work/Gatto.app"
  mkdir -p "$work/payload"
  ditto "$work/Gatto.app" "$work/payload/Gatto.app"
  hdiutil create -quiet -fs HFS+ -volname 'Gatto feed test' -srcfolder "$work/payload" "$directory/gatto-$version-aarch64-apple-darwin.dmg"
  dist/sparkle/bin/generate_appcast --ed-key-file "$work/test-key" \
    --download-url-prefix "https://github.com/jeremyc2/Gatto/releases/download/v$version/" \
    --maximum-deltas 0 ${channel_args[@]+"${channel_args[@]}"} "$directory"
done
xcrun swift scripts/merge-update-feed.swift "$work/combined.xml" "$work/stable/appcast.xml" "$work/beta/appcast.xml"
dist/sparkle/bin/sign_update --ed-key-file "$work/test-key" "$work/combined.xml"
dist/sparkle/bin/sign_update --ed-key-file "$work/test-key" --verify "$work/combined.xml"
[[ "$(grep -c '<item>' "$work/combined.xml")" == 2 ]]
grep -q '<sparkle:channel>beta</sparkle:channel>' "$work/combined.xml"
# Stable promotion must win over its beta while the preview remains separately tagged.
sed 's/0.16.3/0.17.0/g' "$work/stable/appcast.xml" > "$work/promoted.xml"
xcrun swift scripts/merge-update-feed.swift "$work/promotion.xml" "$work/combined.xml" "$work/promoted.xml"
grep -q '<sparkle:version>0.17.0</sparkle:version>' "$work/promotion.xml"
if grep -q '<sparkle:version>0.16.3</sparkle:version>' "$work/promotion.xml"; then exit 1; fi
# An untagged beta and an enclosure from another origin both fail closed.
sed '/<sparkle:channel>beta<\/sparkle:channel>/d' "$work/beta/appcast.xml" > "$work/untagged.xml"
if xcrun swift scripts/merge-update-feed.swift "$work/invalid.xml" "$work/stable/appcast.xml" "$work/untagged.xml" >/dev/null 2>&1; then exit 1; fi
sed 's@https://github.com/jeremyc2/Gatto/releases/download/@https://example.com/@g' "$work/stable/appcast.xml" > "$work/foreign.xml"
if xcrun swift scripts/merge-update-feed.swift "$work/invalid.xml" "$work/foreign.xml" >/dev/null 2>&1; then exit 1; fi
echo 'Passed: signed Beta/Stable feed merge, stable promotion, channel metadata and enclosure isolation'

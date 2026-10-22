#!/usr/bin/env bash
# Refresh the signed beta discovery feed after either channel is published.
set -euo pipefail
cd "$(dirname "$0")/.."
[[ $# == 1 ]] || { echo 'Usage: publish-beta-feed.sh vX.Y.Z[-beta.N]' >&2; exit 1; }
tag="$1"
[[ "$tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-beta\.[1-9][0-9]*)?$ ]] || exit 1
account="${SPARKLE_ACCOUNT:-gatto-updates}"
work="$(mktemp -d "${TMPDIR:-/tmp}/gatto-beta-feed.XXXXXX")"
trap 'rm -rf "$work"' EXIT
inputs=()
fetch_feed() {
  local release="$1" directory="$2"
  mkdir -p "$directory"
  gh release download "$release" --pattern appcast.xml --dir "$directory"
  dist/sparkle/bin/sign_update --account "$account" --verify "$directory/appcast.xml"
  inputs+=("$directory/appcast.xml")
}
exists=false
if gh release view beta --json isDraft,isPrerelease > "$work/beta.json" 2> "$work/beta-error"; then
  # Refuse to overwrite an unrelated release or an unpublished recovery draft.
  bun --no-env-file -e 'const r = await Bun.file(process.argv[1]).json(); if (r.isDraft || !r.isPrerelease) process.exit(1)' "$work/beta.json"
  exists=true
  fetch_feed beta "$work/previous"
else
  [[ "$(cat "$work/beta-error")" == 'release not found' ]] || { cat "$work/beta-error" >&2; exit 1; }
fi
stable="$(gh release view --json tagName --jq .tagName)"
[[ "$stable" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo 'Latest release must remain stable' >&2; exit 1; }
fetch_feed "$stable" "$work/stable"
fetch_feed "$tag" "$work/current"
xcrun swift scripts/merge-update-feed.swift "$work/appcast.xml" "${inputs[@]}"
dist/sparkle/bin/sign_update --account "$account" "$work/appcast.xml"
dist/sparkle/bin/sign_update --account "$account" --verify "$work/appcast.xml"
if [[ "$exists" == true ]]; then
  gh release upload beta "$work/appcast.xml" --clobber
else
  gh release create beta "$work/appcast.xml" --target "$(git rev-parse HEAD)" \
    --title 'Beta channel feed' --notes 'Signed update discovery feed for Beta subscribers. Versioned prereleases contain the installers and release notes.' \
    --prerelease --latest=false
fi
# Verify exact uploaded bytes, not only the local signature.
mkdir -p "$work/uploaded"
gh release download beta --pattern appcast.xml --dir "$work/uploaded"
cmp "$work/appcast.xml" "$work/uploaded/appcast.xml"
dist/sparkle/bin/sign_update --account "$account" --verify "$work/uploaded/appcast.xml"
echo "Updated signed Beta feed after $tag"

#!/usr/bin/env bash
# Writes SHA256SUMS.txt for a GitHub Release's assets and attaches it.
# Usage: scripts/release-checksums.sh v<version>
#
# CI (Linux and Windows installers) and scripts/macos/release.sh (the DMG)
# attach assets at different times, so both run this when they finish;
# whichever runs last leaves the complete list. The hashes are GitHub's own
# asset digests, so nothing is downloaded. Check a download with
# `sha256sum -c SHA256SUMS.txt --ignore-missing` (macOS: `shasum -a 256 -c`,
# Windows: compare with `Get-FileHash`).
set -euo pipefail

tag=${1:?usage: release-checksums.sh v<version>}
repo=${GH_REPO:-harpchad/knobset}
out=$(mktemp -d)/SHA256SUMS.txt

gh api "repos/$repo/releases/tags/$tag" --jq '
  [.assets[] | select(.name != "SHA256SUMS.txt")] | sort_by(.name)[] |
  if (.digest // "" | startswith("sha256:")) then "\(.digest[7:])  \(.name)"
  else error("no sha256 digest for \(.name)") end' >"$out"

cat "$out"
gh release upload "$tag" "$out" --clobber -R "$repo"

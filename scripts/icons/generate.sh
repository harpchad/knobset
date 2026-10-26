#!/usr/bin/env bash
# Regenerate every app icon from the SVG master, app/src-tauri/icons/app-icon.svg.
#
#   - tauri icon expands the master into the desktop and iOS sizes.
#   - The iOS AppIcon PNGs lose their alpha channel; App Store Connect rejects
#     icons with one. The art is opaque, so no pixels change.
#   - icon.icns comes from a derived macOS variant (app-icon-macos.svg) instead,
#     because macOS doesn't mask icons the way iOS does.
#
# Needs Python 3 with Pillow.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
icons="$root/app/src-tauri/icons"
appicon="$root/app/src-tauri/gen/apple/Assets.xcassets/AppIcon.appiconset"

(cd "$root/app" && npx tauri icon src-tauri/icons/app-icon.svg)
# tauri icon also writes Android sizes; there's no Android target.
rm -rf "$icons/android" "$icons/ios"

python3 - "$appicon" <<'PY'
import pathlib, sys
from PIL import Image
for p in pathlib.Path(sys.argv[1]).glob("*.png"):
    Image.open(p).convert("RGB").save(p)
PY

python3 "$root/scripts/icons/macos_icon.py" "$icons/app-icon.svg" "$icons/app-icon-macos.svg"
tmp=$(mktemp -d)
(cd "$root/app" && npx tauri icon src-tauri/icons/app-icon-macos.svg -o "$tmp" >/dev/null)
cp "$tmp/icon.icns" "$icons/icon.icns"
rm -rf "$tmp"

#!/usr/bin/env bash
# Build the Mac app as a universal (Apple silicon + Intel) DMG, sign it with
# Developer ID, notarize and staple it, and attach it to the GitHub Release.
#
# Usage: ASC_KEY_ID=... ASC_ISSUER_ID=... scripts/macos/release.sh
#   SKIP_UPLOAD=1     stop after the notarized DMG is built
#   ALLOW_UNTAGGED=1  skip the clean-tree and release-tag checks
#
# One-time setup per Mac:
#   - "Developer ID Application" certificate in the login keychain (Xcode >
#     Settings > Accounts > Manage Certificates > + > Developer ID Application;
#     only the team's Account Holder can create one), with codesign set to
#     "Always Allow" on the first keychain prompt.
#   - The App Store Connect API key at
#     ~/.appstoreconnect/private_keys/AuthKey_$ASC_KEY_ID.p8 (notarization
#     uses it too).
#   - rustup target add x86_64-apple-darwin
#   - gh signed in, and the GitHub Release for the tag already created.
# The first DMG build asks to let Terminal control Finder; that lays out the
# DMG window.
set -euo pipefail

die() { echo "error: $*" >&2; exit 1; }
: "${ASC_KEY_ID:?set ASC_KEY_ID}" "${ASC_ISSUER_ID:?set ASC_ISSUER_ID}"
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../.." && pwd)

key="$HOME/.appstoreconnect/private_keys/AuthKey_$ASC_KEY_ID.p8"
[[ -f $key ]] || die "no API key AuthKey_$ASC_KEY_ID.p8 in ~/.appstoreconnect/private_keys"
identity=$(security find-identity -v -p codesigning | sed -n 's/.*"\(Developer ID Application: [^"]*\)".*/\1/p' | head -1)
[[ -n $identity ]] || die "no Developer ID Application certificate in the keychain"
rustup target list --installed | grep -qx x86_64-apple-darwin || die "run: rustup target add x86_64-apple-darwin"

version=$(python3 -c 'import json, sys; print(json.load(open(sys.argv[1]))["version"])' "$root/app/src-tauri/tauri.conf.json")
if [[ ${ALLOW_UNTAGGED:-} != 1 ]]; then
  [[ -z $(git -C "$root" status --porcelain) ]] || die "the working tree has changes"
  git -C "$root" tag --points-at HEAD | grep -qx "v$version" || die "HEAD isn't tagged v$version"
fi
if [[ ${SKIP_UPLOAD:-} != 1 ]]; then
  gh release view "v$version" -R harpchad/knobset >/dev/null || die "no GitHub Release v$version yet"
fi

# Tauri signs the app with hardened runtime, notarizes and staples it, then
# builds and signs the DMG.
export APPLE_SIGNING_IDENTITY=$identity APPLE_API_KEY=$ASC_KEY_ID APPLE_API_ISSUER=$ASC_ISSUER_ID APPLE_API_KEY_PATH=$key
(cd "$root/app" && npx tauri build --target universal-apple-darwin --bundles app,dmg)

bundle="$root/target/universal-apple-darwin/release/bundle"
dmg="$bundle/dmg/Knobset_${version}_universal.dmg"
[[ -f $dmg ]] || die "expected $dmg"

# Notarize the DMG as well, so Gatekeeper accepts the download itself offline.
xcrun notarytool submit "$dmg" --key "$key" --key-id "$ASC_KEY_ID" --issuer "$ASC_ISSUER_ID" --wait
xcrun stapler staple "$dmg"

codesign --verify --deep --strict "$bundle/macos/Knobset.app"
spctl --assess --type execute "$bundle/macos/Knobset.app"
spctl --assess --type open --context context:primary-signature "$dmg"
xcrun stapler validate "$dmg"
echo "Notarized $dmg"
[[ ${SKIP_UPLOAD:-} == 1 ]] && exit 0

gh release upload "v$version" "$dmg" --clobber -R harpchad/knobset
echo "Attached $(basename "$dmg") to Knobset $version."
"$root/scripts/release-checksums.sh" "v$version"

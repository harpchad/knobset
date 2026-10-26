#!/usr/bin/env bash
# Build the iPad app, sign it for the App Store, upload it to App Store
# Connect and wait until TestFlight has processed it.
#
# Usage: ASC_KEY_ID=... ASC_ISSUER_ID=... scripts/ios/release.sh [BUILD]
#   BUILD  optional build number appended to the version (0.4.0 -> 0.4.0.BUILD);
#          needed to upload the same version a second time.
#   SKIP_UPLOAD=1     stop after exporting the .ipa
#   ALLOW_UNTAGGED=1  skip the clean-tree and release-tag checks
#
# One-time setup per Mac:
#   - "Apple Distribution" certificate in the login keychain (Xcode > Settings >
#     Accounts > Manage Certificates > + > Apple Distribution), with codesign
#     set to "Always Allow" on the first keychain prompt.
#   - The "Knobset App Store" provisioning profile for that certificate,
#     installed under ~/Library/Developer/Xcode/UserData/Provisioning Profiles.
#   - The App Store Connect API key at
#     ~/.appstoreconnect/private_keys/AuthKey_$ASC_KEY_ID.p8.
# No Xcode sign-in is needed: signing is manual and uploads use the API key.
set -euo pipefail

die() { echo "error: $*" >&2; exit 1; }
: "${ASC_KEY_ID:?set ASC_KEY_ID}" "${ASC_ISSUER_ID:?set ASC_ISSUER_ID}"
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../.." && pwd)

[[ -f "$HOME/.appstoreconnect/private_keys/AuthKey_$ASC_KEY_ID.p8" ]] || die "no API key AuthKey_$ASC_KEY_ID.p8 in ~/.appstoreconnect/private_keys"
security find-identity -v -p codesigning | grep -q '"Apple Distribution: ' || die "no Apple Distribution certificate in the keychain"
grep -lqs "Knobset App Store" "$HOME/Library/Developer/Xcode/UserData/Provisioning Profiles/"*.mobileprovision || die "the \"Knobset App Store\" profile isn't installed"

version=$(python3 -c 'import json, sys; print(json.load(open(sys.argv[1]))["version"])' "$root/app/src-tauri/tauri.conf.json")
if [[ ${ALLOW_UNTAGGED:-} != 1 ]]; then
  [[ -z $(git -C "$root" status --porcelain) ]] || die "the working tree has changes"
  git -C "$root" tag --points-at HEAD | grep -qx "v$version" || die "HEAD isn't tagged v$version"
fi

# Xcode's IPA packaging fails ("Copy failed") with Homebrew's rsync ahead of
# the system one.
export PATH="/usr/bin:$PATH"

build_args=()
[[ -n ${1:-} ]] && build_args=(--build-number "$1")
(cd "$root/app" && npx tauri ios build --archive-only ${build_args[@]+"${build_args[@]}"})
# The build writes BUILD into the tracked Info.plist; the archive keeps it.
[[ -n ${1:-} ]] && git -C "$root" checkout -- app/src-tauri/gen/apple/knobset_iOS/Info.plist

archive="$root/app/src-tauri/gen/apple/build/knobset_iOS.xcarchive"
bundle_version=$(/usr/libexec/PlistBuddy -c 'Print :CFBundleVersion' "$archive/Products/Applications/Knobset.app/Info.plist")
out=$(mktemp -d)
xcodebuild -exportArchive -archivePath "$archive" -exportPath "$out" -exportOptionsPlist "$here/ExportOptions-AppStore.plist"
echo "Exported $out/Knobset.ipa ($version, build $bundle_version)"
[[ ${SKIP_UPLOAD:-} == 1 ]] && exit 0

xcrun altool --upload-app -f "$out/Knobset.ipa" -t ios --apiKey "$ASC_KEY_ID" --apiIssuer "$ASC_ISSUER_ID"

api() { curl -fsS --max-time 60 -H "Authorization: Bearer $(swift "$here/asc_token.swift")" "https://api.appstoreconnect.apple.com$1"; }
app=$(api '/v1/apps?filter%5BbundleId%5D=com.harpchad.knobset' | python3 -c 'import json, sys; print(json.load(sys.stdin)["data"][0]["id"])')
echo "Waiting for App Store Connect to process build $bundle_version..."
for _ in $(seq 60); do
  state=$(api "/v1/builds?filter%5Bapp%5D=$app&filter%5Bversion%5D=$bundle_version&limit=1" |
    python3 -c 'import json, sys; d = json.load(sys.stdin)["data"]; print(d[0]["attributes"]["processingState"] if d else "NOT_LISTED")')
  case $state in
    VALID) echo "Build $bundle_version is ready in TestFlight."; exit 0 ;;
    INVALID | FAILED) die "processing ended as $state; check App Store Connect and the account email" ;;
  esac
  sleep 60
done
die "build $bundle_version is still processing after an hour"

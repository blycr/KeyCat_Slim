#!/bin/sh
# Builds the universal native Metal KeyCat.app and packs dist/KeyCat-macOS.zip + .sha256 + latest-mac.json.
# Usage: sh apps/macos/package.sh
set -eu
cd "$(dirname "$0")/../.."
sh apps/macos/build.sh
rm -rf dist
mkdir -p dist
ditto -c -k --keepParent build/KeyCat.app dist/KeyCat-macOS.zip
(cd dist && shasum -a 256 KeyCat-macOS.zip | tee KeyCat-macOS.zip.sha256)
VERSION="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' apps/macos/Info.plist)"
BUILD="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleVersion' apps/macos/Info.plist)"
SHA="$(cut -d' ' -f1 dist/KeyCat-macOS.zip.sha256)"
REPO="${GITHUB_REPOSITORY:-blycr/KeyCat_Slim}"
cat > dist/latest-mac.json <<EOF
{
  "version": "$VERSION",
  "build": $BUILD,
  "url": "https://github.com/$REPO/releases/download/v$VERSION/KeyCat-macOS.zip",
  "sha256": "$SHA"
}
EOF
cat dist/latest-mac.json
ls -lh dist

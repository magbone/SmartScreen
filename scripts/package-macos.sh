#!/usr/bin/env bash

set -euo pipefail

root_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
binary="${1:-$root_dir/target/release/smart-screen}"
arch_label="${2:-$(uname -m)}"
version="${VERSION:-$(sed -n 's/^version = "\(.*\)"/\1/p' "$root_dir/Cargo.toml" | head -1)}"
build_number="${BUILD_NUMBER:-1}"
dist_dir="$root_dir/dist"
app_dir="$dist_dir/SmartScreen.app"
dmg_root="$dist_dir/dmg-root"
dmg_path="$dist_dir/SmartScreen-${version}-macos-${arch_label}.dmg"

if [[ ! -f "$binary" ]]; then
    echo "Binary not found: $binary" >&2
    exit 1
fi

rm -rf "$app_dir" "$dmg_root"
mkdir -p "$app_dir/Contents/MacOS" "$app_dir/Contents/Resources"

cp "$binary" "$app_dir/Contents/MacOS/SmartScreen"
chmod +x "$app_dir/Contents/MacOS/SmartScreen"
cp "$root_dir/packaging/Info.plist" "$app_dir/Contents/Info.plist"

plutil -replace CFBundleShortVersionString -string "$version" "$app_dir/Contents/Info.plist"
plutil -replace CFBundleVersion -string "$build_number" "$app_dir/Contents/Info.plist"
plutil -lint "$app_dir/Contents/Info.plist"

codesign --force --deep --sign - "$app_dir"

if [[ "${SKIP_DMG:-0}" == "1" ]]; then
    echo "App bundle: $app_dir"
    exit 0
fi

mkdir -p "$dmg_root"
cp -R "$app_dir" "$dmg_root/SmartScreen.app"
ln -s /Applications "$dmg_root/Applications"

hdiutil create \
    -volname "Smart Screen" \
    -srcfolder "$dmg_root" \
    -ov \
    -format UDZO \
    "$dmg_path"

shasum -a 256 "$dmg_path" > "$dmg_path.sha256"

echo "DMG: $dmg_path"
echo "SHA256: $dmg_path.sha256"

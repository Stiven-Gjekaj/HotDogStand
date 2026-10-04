#!/bin/sh
# Builds HotDogStand.app, the form of the application that macOS shows in the
# Dock and in Finder with its icon and its name.
#
# The bundle goes to target/release/HotDogStand.app. Copy it to
# /Applications to install it.
set -eu

cd "$(dirname "$0")/.."
version=$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -n 1)

cargo build --release -p hotdogstand

app=target/release/HotDogStand.app
rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp target/release/hotdogstand "$app/Contents/MacOS/hotdogstand"
cp assets/icons/hotdogstand.icns "$app/Contents/Resources/hotdogstand.icns"
sed "s/@VERSION@/$version/g" packaging/macos/Info.plist > "$app/Contents/Info.plist"
plutil -lint "$app/Contents/Info.plist" >/dev/null

echo "Built $app (version $version)."

#!/bin/sh
# Installs HotDogStand for the person who runs it on Linux: the program, its
# icons, and the entry in the menu of applications.
#
# Run it from the repository after `cargo build --release`.
set -eu

cd "$(dirname "$0")/.."
data=${XDG_DATA_HOME:-$HOME/.local/share}
bin=$HOME/.local/bin

install -Dm755 target/release/hotdogstand "$bin/hotdogstand"
for size in 16 24 32 48 64 128 256 512; do
    install -Dm644 "assets/icons/png/$size.png" \
        "$data/icons/hicolor/${size}x${size}/apps/hotdogstand.png"
done
install -Dm644 assets/hotdogstand.svg "$data/icons/hicolor/scalable/apps/hotdogstand.svg"
install -Dm644 packaging/linux/hotdogstand.desktop "$data/applications/hotdogstand.desktop"

# The menu reads the new entry sooner when these tools are there.
command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$data/applications" || true
command -v gtk-update-icon-cache >/dev/null 2>&1 && gtk-update-icon-cache -q "$data/icons/hicolor" || true

echo "Installed HotDogStand. Make sure that $bin is on your PATH."

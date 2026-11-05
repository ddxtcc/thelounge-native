#!/bin/sh
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
prefix=${PREFIX:-$HOME/.local}
data=${XDG_DATA_HOME:-$prefix/share}

if [ "$#" -ge 1 ]; then
    binary=$1
elif [ -x "$root/lounge-native" ]; then
    binary=$root/lounge-native
else
    binary=$root/../target/release/lounge-native
fi

if [ -f "$root/lounge-native.desktop" ]; then
    desktop=$root/lounge-native.desktop
else
    desktop=$root/../packaging/lounge-native.desktop
fi

if [ -d "$root/icons/hicolor" ]; then
    icons=$root/icons/hicolor
else
    icons=$root/../assets/icons/hicolor
fi

if [ ! -x "$binary" ]; then
    echo "lounge-native binary not found: $binary" >&2
    echo "Build it with: cargo build --release -p lounge-native" >&2
    exit 1
fi

install -Dm755 "$binary" "$prefix/bin/lounge-native"
install -Dm644 "$desktop" "$data/applications/lounge-native.desktop"
install -Dm644 "$icons/scalable/apps/lounge-native.svg" "$data/icons/hicolor/scalable/apps/lounge-native.svg"

for size in 16x16 24x24 32x32 48x48 64x64 128x128 256x256 512x512; do
    install -Dm644 "$icons/$size/apps/lounge-native.png" "$data/icons/hicolor/$size/apps/lounge-native.png"
done

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$data/applications" >/dev/null 2>&1 || true
fi

if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -q "$data/icons/hicolor" >/dev/null 2>&1 || true
fi

echo "Installed lounge-native to $prefix/bin/lounge-native"

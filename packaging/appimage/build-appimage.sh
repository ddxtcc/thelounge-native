#!/bin/sh
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
version=${VERSION:-$(sed -n 's/^version = "\(.*\)"/\1/p' "$root/Cargo.toml" | head -n 1)}
binary=${1:-"$root/target/release/lounge-native"}
output=${2:-"$root/dist/lounge-native_${version}_x86_64.AppImage"}
tools=${TOOLS_DIR:-"$root/.appimage-tools"}
appdir=${APPDIR:-"$root/target/appimage/AppDir"}
update_info=${UPDATE_INFO:-"gh-releases-zsync|ddxtcc|thelounge-native|latest|*x86_64.AppImage.zsync"}

if [ -z "$version" ]; then
    echo "Could not determine package version." >&2
    exit 1
fi
if [ ! -x "$binary" ]; then
    echo "lounge-native binary not found: $binary" >&2
    exit 1
fi

mkdir -p "$tools" "$(dirname -- "$output")"
linuxdeploy="$tools/linuxdeploy"
appimagetool="$tools/appimagetool"
plugin="$tools/linuxdeploy-plugin-gstreamer"
if [ ! -x "$linuxdeploy" ]; then
    curl -fL --retry 5 -o "$linuxdeploy" \
        https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-x86_64.AppImage
    chmod 755 "$linuxdeploy"
fi
if [ ! -x "$appimagetool" ]; then
    curl -fL --retry 5 -o "$appimagetool" \
        https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage
    chmod 755 "$appimagetool"
fi
if [ ! -x "$plugin" ]; then
    curl -fL --retry 5 -o "$plugin" \
        https://raw.githubusercontent.com/linuxdeploy/linuxdeploy-plugin-gstreamer/master/linuxdeploy-plugin-gstreamer.sh
    chmod 755 "$plugin"
fi

rm -rf "$appdir"
mkdir -p "$appdir/usr/bin" "$appdir/usr/share/icons/hicolor" "$appdir/usr/share/metainfo"
PATH="$tools:$PATH" APPIMAGE_EXTRACT_AND_RUN=1 GSTREAMER_INCLUDE_BAD_PLUGINS=1 \
    "$linuxdeploy" \
    --appdir "$appdir" \
    --executable "$binary" \
    --desktop-file "$root/packaging/lounge-native.desktop" \
    --icon-file "$root/assets/icons/hicolor/512x512/apps/lounge-native.png" \
    --plugin gstreamer

cp -R "$root/assets/icons/hicolor/." "$appdir/usr/share/icons/hicolor/"
install -Dm644 "$root/packaging/linux/lounge-native.metainfo.xml" \
    "$appdir/usr/share/metainfo/dev.lounge.native.appdata.xml"
install -Dm644 "$root/packaging/lounge-native.desktop" \
    "$appdir/usr/share/applications/lounge-native.desktop"
if command -v strip >/dev/null 2>&1; then
    strip --strip-unneeded "$appdir/usr/bin/lounge-native" || true
fi

output_dir=$(dirname -- "$output")
output_name=$(basename -- "$output")
set -- -u "$update_info" "$appdir" "$output_name"
if [ -n "${GPG_KEY:-}" ]; then
    set -- -u "$update_info" --sign --sign-key "$GPG_KEY" "$appdir" "$output_name"
fi
(cd "$output_dir" && ARCH=x86_64 APPIMAGE_EXTRACT_AND_RUN=1 "$appimagetool" "$@")
echo "Built $output"

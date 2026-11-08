#!/bin/sh
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
version=${VERSION:-$(sed -n 's/^version = "\(.*\)"/\1/p' "$root/Cargo.toml" | head -n 1)}
binary=${1:-"$root/target/release/lounge-native"}
output=${2:-"$root/dist/lounge-native_${version}_amd64.deb"}
arch=amd64

if [ -z "$version" ]; then
    echo "Could not determine package version." >&2
    exit 1
fi
if [ ! -x "$binary" ]; then
    echo "lounge-native binary not found: $binary" >&2
    exit 1
fi

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT INT TERM
stage="$work/data"
control="$work/control"
mkdir -p "$stage/usr/bin" \
    "$stage/usr/share/applications" \
    "$stage/usr/share/icons" \
    "$stage/usr/share/metainfo" \
    "$stage/usr/share/doc/lounge-native" \
    "$control"

install -Dm755 "$binary" "$stage/usr/bin/lounge-native"
install -Dm644 "$root/packaging/lounge-native.desktop" \
    "$stage/usr/share/applications/lounge-native.desktop"
install -Dm644 "$root/packaging/linux/lounge-native.metainfo.xml" \
    "$stage/usr/share/metainfo/lounge-native.metainfo.xml"
install -Dm644 "$root/README.md" "$stage/usr/share/doc/lounge-native/README.md"
install -Dm644 "$root/LICENSE" "$stage/usr/share/doc/lounge-native/LICENSE"
cp -R "$root/assets/icons/hicolor/." "$stage/usr/share/icons/hicolor/"

cat > "$control/control" <<EOF
Package: lounge-native
Version: $version
Architecture: $arch
Maintainer: Lounge Native contributors <noreply@example.com>
Installed-Size: $(du -sk "$stage" | cut -f1)
Section: net
Priority: optional
Homepage: https://github.com/ddxtcc/thelounge-native
Depends: libc6 (>= 2.39), libgcc-s1, libglib2.0-0t64 | libglib2.0-0, libgstreamer1.0-0, libgstreamer-plugins-base1.0-0, gstreamer1.0-plugins-base, gstreamer1.0-plugins-good, gstreamer1.0-plugins-bad, libxkbcommon0, libxkbcommon-x11-0, libxcb1, libxcb-xkb1, libx11-6, libwayland-client0, libvulkan1, libfontconfig1, libfreetype6, hicolor-icon-theme, desktop-file-utils
Recommends: libnotify-bin, xdg-utils
Description: Native IRC client for The Lounge
 A native desktop client for The Lounge IRC servers, written in Rust
 with GPUI. It connects directly to an existing The Lounge server over
 the same socket.io API as the web client.
EOF

cat > "$control/postinst" <<EOF
#!/bin/sh
set -e
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database -q /usr/share/applications || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -q -t /usr/share/icons/hicolor || true
fi
exit 0
EOF
chmod 755 "$control/postinst"

mkdir -p "$(dirname -- "$output")"
printf '2.0\n' > "$work/debian-binary"
tar --owner=0 --group=0 --numeric-owner -C "$control" -czf "$work/control.tar.gz" .
tar --owner=0 --group=0 --numeric-owner -C "$stage" -czf "$work/data.tar.gz" .
(cd "$work" && ar rcs "$output" debian-binary control.tar.gz data.tar.gz)
echo "Built $output"

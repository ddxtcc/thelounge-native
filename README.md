# Lounge Native

An unofficial native client for [The Lounge](https://github.com/thelounge/thelounge), written in Rust with [GPUI](https://gpui.rs).

It connects directly to an existing The Lounge server over the same socket.io API the web client uses, so networks, channels, history and settings stay in sync with the server.

## Build and run

```sh
cargo run --release -p lounge-native
```

At the login screen enter the server URL, for example:

```text
https://lounge.example.com
```

## Install

```sh
cargo build --release -p lounge-native
sh packaging/install.sh
```

The installer writes to `$HOME/.local` by default. Set `PREFIX` or `XDG_DATA_HOME` to change the target. Release archives include the binary, `lounge-native.desktop`, `install.sh`, and hicolor icons; run `./install.sh` from the unpacked archive.

Ubuntu 24.04, Debian trixie, and compatible distributions can install the Debian package instead:

```sh
sudo apt install ./lounge-native_0.9.9_amd64.deb
```

The package installs the binary, desktop entry, AppStream metadata, and hicolor icons under `/usr`. Update it by installing a newer package or rerunning `install.sh` from a newer archive. Settings > About > Check for updates compares the app against the latest GitHub release.

A portable AppImage is also published with each release:

```sh
chmod +x lounge-native_0.9.9_x86_64.AppImage
./lounge-native_0.9.9_x86_64.AppImage
```

Release assets include `SHA256SUMS` and GitHub build provenance attestations. Verify a downloaded asset with GitHub CLI:

```sh
gh attestation verify lounge-native_0.9.9_amd64.deb --repo ddxtcc/thelounge-native
```

Arch Linux users can build the package in `packaging/aur`:

```sh
cd packaging/aur
makepkg -si
```

The directory also includes a `build-repo.sh` helper for publishing a signed pacman repository.

## Media playback

The build requires Rust 1.96 or newer and GStreamer with its base, good, and bad
plugins. On Arch Linux:

```sh
sudo pacman -S gstreamer gst-plugins-base gst-plugins-good gst-plugins-bad
```

On Debian and Ubuntu:

```sh
sudo apt install libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev \
  gstreamer1.0-plugins-base gstreamer1.0-plugins-good gstreamer1.0-plugins-bad
```

macOS builds require GStreamer from Homebrew or the official GStreamer packages.
Windows builds require the GStreamer SDK and a working `pkg-config` setup. Windows
and macOS release archives bundle the GStreamer runtime. Use `--no-default-features`
to build without media support on other platforms.

## Development

```sh
cargo test --workspace --locked
cargo build -p lounge-native
```

## License

MIT.

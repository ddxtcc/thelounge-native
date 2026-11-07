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

## Media playback

Media playback is enabled by default. Video cards show a still poster and play in
place. Audio cards show progress, time, seek, and volume controls before and during
playback. Both support expanding to the media overlay and saving the original file
to `$XDG_DOWNLOAD_DIR` or `~/Downloads`.

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
Windows builds require the GStreamer SDK and a working `pkg-config` setup. Release
archives are not bundled with GStreamer, so Windows and macOS users need the GStreamer
runtime installed for media playback. Use `--no-default-features` to build without
media support.

## Development

```sh
cargo test --workspace --locked
cargo build -p lounge-native
```

## License

MIT.

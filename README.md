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

## Optional media playback

Direct video and audio links can play inside the message lightbox when built with the
`media-video` feature:

```sh
cargo run --release -p lounge-native --features media-video
```

This build requires Rust 1.96 or newer and GStreamer with its base, good, and bad
plugins. The default build does not link GStreamer.

## Development

```sh
cargo test -p lounge-core
cargo build -p lounge-native
```

## License

MIT.

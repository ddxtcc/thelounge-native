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

If The Lounge is hosted under a subpath, include it:

```text
https://example.com/lounge
```

## Install

```sh
cargo build --release -p lounge-native
sh packaging/install.sh
```

The installer writes to `$HOME/.local` by default. Set `PREFIX` or `XDG_DATA_HOME` to change the target. Release archives include the binary, `lounge-native.desktop`, `install.sh`, and hicolor icons; run `./install.sh` from the unpacked archive.

## Development

```sh
cargo test -p lounge-core
cargo build -p lounge-native
```

## License

MIT.

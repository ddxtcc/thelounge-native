# Lounge Native

<img src="assets/icon.svg" width="96" height="96" alt="Lounge Native icon">

A native client for [The Lounge](https://github.com/thelounge/thelounge), written in Rust with [GPUI](https://gpui.rs).

It connects directly to an existing The Lounge server over the same socket.io API the web client uses, so networks, channels, history and settings stay in sync with the server. All IRC work still happens on the server.

## What works

- Private-server login with username/password, token login, and public servers (no credentials)
- Automatic reconnection with exponential backoff; session tokens survive restarts
- Multiple IRC networks, lobby/channel/query sidebar, server-provided ordering, unread and highlight badges
- Messages, notices, actions, joins/parts/quits/modes/nick changes/errors, unread markers, date separators
- mIRC formatting: bold, italic, underline, strikethrough, monospace, 16-colour foreground/background, hex colours
- Clickable links, reply-to-message support, condensed status messages (expandable), link previews sent by the server
- Channel topic display (click the topic to edit it), join/leave, mute/unmute
- User list with mode prefixes (`@`, `+`), away and bot markers; click a nick to insert it into the composer, right-click for query/whois/op/voice/kick actions
- Composer history (Up/Down), nick/slash-command/emoji completion (Tab in the composer), multi-line messages (Shift+Enter)
- Normal form navigation: Tab/Shift+Tab between inputs, word-wise cursor movement/deletion, Shift+Home/End selection, double-click word selection, click-to-focus
- Per-channel drafts, unread/highlight clearing, active-channel sync with other Lounge clients
- Paginated history (`Show older messages`), search in the current channel, recent mentions
- Server-side highlight words / exceptions / away message settings
- Add and edit IRC networks, defaults and network locking from the server configuration; per-network reconnect/disconnect/remove actions
- Channel and network reordering through action menus, plus drag-and-drop
- Read-only table views for `/list`, `/banlist`, `/invitelist` and `/ignorelist`
- Emoji picker with search, and `:shortcode` completion
- Password change and active-session management (list and sign out other sessions)
- Server link previews with text and thumbnail images, plus inline rendering for direct image URLs and image uploads
- File uploads through the system file chooser or drag-and-drop, inserted as a URL
- Desktop notifications for highlights while the window is not focused (via `notify-send`)
- Built-in Dark, Light, Nord, Dracula, Gruvbox, Tokyo Night, Solarized Light and pure-black OLED themes
- Hideable network/channel sidebar with a header menu button, and an aligned nickname column for messages
- Font picker for the message/UI font and monospace font, font size, line height, message spacing, timestamps, 12/24-hour clock, seconds, nick colours, inline-image/preview toggles and user-list toggle
- Config and session token stored in `~/.config/lounge-native/config.toml` with mode `0600`

## What is intentionally different

The web client themes and some web-only integrations are not used. Themes are bundled locally, and the following features are not implemented yet:

- Inline playback of previewed video/audio (links and thumbnail images work; media opens externally)
- The browser push-notification service worker (native desktop notifications are supported)
- Browser-specific integrations such as `irc://` deep links and custom CSS

## Requirements

- Rust 1.89+ (edition 2024)
- Linux with Vulkan and Wayland or X11 (GPUI's current Linux backends); macOS/Windows should build with GPUI's platform support but are not the primary target
- An existing The Lounge server (the client speaks the v4 socket.io protocol used by The Lounge 4.x)

The web client must be built on the server, but the native client only needs the socket.io endpoint and optional uploads. Server-generated link preview cards require The Lounge's `prefetch` setting; direct image URLs and uploaded images are rendered inline regardless (images are fetched by the native client).

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

Leave username and password empty for a public server.

The first successful login stores the server, username and a session token. Subsequent launches sign in automatically. Use **Sign out** in the sidebar to remove the local token.

## Install

Build a release binary and install it with the desktop entry and icons:

```sh
cargo build --release -p lounge-native
sh packaging/install.sh
```

The installer writes to `$HOME/.local` by default. Set `PREFIX` or `XDG_DATA_HOME` to change the target. Release archives include the binary, `lounge-native.desktop`, `install.sh`, and hicolor icons; run `./install.sh` from the unpacked archive.

## Development

```sh
cargo test -p lounge-core        # protocol, state reducer and formatting tests
cargo build -p lounge-native     # GPUI application
```

CI and releases:

`.gitlab-ci.yml` runs formatting, clippy, and tests on merge requests and the default branch. Tags build a release tarball and attach it to a GitLab Release.

Workspace layout:

```text
crates/lounge-core   Engine.IO/socket.io v4 client, wire types, state reducer, IRC formatting, uploads
crates/lounge-app    GPUI application, text input widget, theme, persisted configuration
```

### Protocol notes

- The client implements the small Engine.IO v4 subset needed by socket.io: WebSocket transport, `40` namespace connect, `42` events, `2`/`3` ping/pong.
- Authentication starts with the server's `auth:start`, then sends `auth:perform` with either `{user, password}` or a fast-auth token payload including `lastMessage`, `openChannel` and `hasConfig`.
- On reconnect the client re-authenticates with its token and the last seen message id, so the server only sends new messages.
- `init` merges into the existing state instead of replacing it, preserving loaded history. Older history is fetched with `more` and merged by message id.
- The GPUI application owns presentation only; `ClientState::apply` mirrors the web client's event reducer and is unit tested.

### Notes on dependencies

GPUI 0.2.2 pulls in `xattr 0.2.3` transitively. That old crate references `libc::ENOATTR`, which was removed from recent `libc` releases on Linux, so the workspace pins `libc = 0.2.186`. The UI also uses GPUI's native file chooser, which goes through the desktop portal on Wayland.

## License

MIT.

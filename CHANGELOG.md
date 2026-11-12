# Changelog

## 1.1.7 - 2026-11-12

### Added

- A Features section in settings with individual toggles for unread jump, recent emoji, pinned messages, emoji suggestions, reply jumping, typing indicators, clickable timestamps, clipboard image paste, video speed, large emoji, Channel List tools, and draft indicators.
- New toggles for hiding muted channels, channel topic tooltips, full timestamp tooltips, wheel channel switching, pin markers, hiding date separators, unread-only sidebars, and dimming own messages.
- Pinned messages with a per-channel pin list and jump-to-message support.
- Recent emoji, live emoji suggestions in the composer, and unread jump navigation.
- Channel List filtering and sortable Channel, Users, and Topic headers.
- Sidebar typing and draft indicators.
- Clickable reply previews and click-to-copy timestamps.
- Global clipboard image paste, including from non-composer areas.
- Video playback speed controls.

### Fixed

- Windows executables now embed the application icon, and the launcher suppresses child console windows.
- Friend context menus use network-wide actions instead of channel-only op/voice/kick actions.
- Image lightbox uses a solid backdrop, a framed panel, and safer zoom/pan behavior.

## 1.1.6 - 2026-11-10

### Changed

- Windows releases no longer bundle GStreamer. The zip and installer contain only the application, launcher, and documentation.
- Windows playback uses the installed GStreamer 1.26 MSVC x86_64 runtime. The launcher finds it, configures plugin paths, and starts the application.

### Fixed

- Windows builds now ship an NSIS installer with a Start Menu shortcut and uninstaller instead of only a zip.
- Windows release builds no longer open a console window.

## 1.1.5 - 2026-11-10

### Fixed

- Friend and favourite right-click menus now reuse the standard user, channel, and network menus, with working close buttons and every normal action.
- Query channel menus now include Add friend and Remove friend.

### Added

- Video playback now shows elapsed and total time with a progress bar, inline and in the lightbox.
- Channel List requests its data when opened and shows a spinner until results arrive.

### Changed

- The sidebar Networks section now has a heading matching Friends and Favorites.

## 1.1.4 - 2026-11-10

### Fixed

- Friend and favourite right-click menus now open at the pointer like other context menus.

## 1.1.3 - 2026-11-10

### Changed

- Sidebar channel rows no longer repeat channel prefixes.
- All sidebar row markers now share the same column width and vertical alignment.

## 1.1.2 - 2026-11-10

### Changed

- Sidebar sections now share consistent row widths, heights, padding, and marker sizes.
- Network favorites show connection status like normal network rows.

## 1.1.1 - 2026-11-09

### Fixed

- Friends and favourites can be removed from their row context menus.

## 1.1.0 - 2026-11-09

### Added

- Friends list with online presence indicators.
- Favourite networks and channels pinned at the top of the sidebar.
- Friend and favourite actions in user, network, and channel context menus.

## 1.0.0 - 2026-11-09

### Added

- Keyboard section navigation, context menus, message selection, and Home/End navigation.
- Optional user mode prefixes and mode-based user sorting.
- Optional media playback position persistence across restarts.
- Reconnect recovery after system suspend and server restarts.
- Release checksums and GitHub build provenance attestations.

### Changed

- Default keyboard focus now stays on the composer without a navigation highlight.
- Media, thumbnail, clipboard, poster, and config temporary files use safe unique names.
- Link and thumbnail fetches reject private, loopback, link-local, mapped IPv6, and unsafe DNS targets.
- Settings split security and session controls from general rendering.

### Fixed

- Typing from an unfocused input no longer causes a reentrant crash.
- Message `account` values from newer The Lounge servers deserialize correctly.
- User mode signs use the server-provided symbols rather than mode letters.
- Account changes preserve existing data and tokens until authentication succeeds.
- Reconnect backoff buffers commands and replays them after init.
- WebSocket writes, uploads, and connects have bounded timeouts.
- Empty init data clears stale state; highlighted messages are not double-counted.
- Drafts are saved on shutdown and multiline selection no longer underflows.

### Security

- Added URL safety checks and disabled redirects for automatic link and thumbnail fetches.
- Redacted upload tokens from error messages.
- Blocked keyring migration failures from overwriting the only token copy.

### Known limitations

- DNS resolution is validated before the HTTP client resolves the host, leaving a narrow DNS rebinding window.
- Windows and macOS packages are built by CI but not runtime-tested for this release.
- AUR publishing is unavailable while package registration is closed.

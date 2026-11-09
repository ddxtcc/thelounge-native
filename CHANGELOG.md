# Changelog

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

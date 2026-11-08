# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2026-11-08

First public release of Kestrel Download Manager for Windows.

### Added

- Desktop front end for HTTP(S), FTP, magnet links and BitTorrent, with pause, resume, rename, remove, and a compact or advanced layout.
- **Kestrel Engine** in `engine/`, a lightly modified aria2 1.37.0 (GPL-2.0-or-later). It allows up to 64 connections per server instead of 16, identifies itself while crediting aria2, and links newer releases of zlib, Expat, SQLite, c-ares and libssh2. The Docker build exports the complete corresponding source with every binary.
- **Connections per server** in Preferences. The limit is 64 with Kestrel Engine and 16 with stock aria2, and the value is saved to `aria2.conf`.
- Kestrel starts `kestrel-engine.exe` when it sits next to the app, and otherwise falls back to `aria2c`.
- **Translucent window** in Preferences. Acrylic on Windows 11 22H2 and later, Mica on earlier Windows 11 builds, and blur on Windows 10. The background-opacity slider defaults to the lowest level that keeps WCAG AA contrast and warns below that level.
- **About** section in Preferences, including the running engine version and links to aria2 and the Kestrel Engine source.
- `THIRD_PARTY_NOTICES.md` for aria2, the libraries in the engine build, the Rust dependencies, icons and fonts.
- Browser companion extension for Chromium. It reads the RPC secret from Kestrel automatically while the app is running.
- A current-user Windows installer with Kestrel artwork. It installs Kestrel Engine, the licence notices and the browser extension beside the app. The portable zip remains available alongside it.
- **Folder** opens the directory that contains the download. A torrent is labelled Seeding only while it is actively seeding, and Stop Seed reports a failure if the engine rejects the pause.

### Fixed

- The tray menu shows an icon beside each command.
- Kestrel Engine's version banner links to this repository.

### Accessibility

- Targets WCAG 2.2 AA. See [ACCESSIBILITY.md](ACCESSIBILITY.md).
- Full keyboard operation: arrow keys between transfer cards, <kbd>Enter</kbd> for the main action, <kbd>Shift</kbd>+<kbd>F10</kbd> or <kbd>Menu</kbd> for the actions menu, and <kbd>Delete</kbd> to remove a transfer. A skip link jumps to the transfer list.
- Dialogs trap focus, make the rest of the window inert, and return focus to the control that opened them. After a transfer is removed, focus moves to the next one.
- Focus is preserved while the transfer list updates live.
- The card actions menu and the sort menu follow the ARIA menu pattern, with arrow keys, type-ahead and <kbd>Esc</kbd>.
- Accessible names on every control, `aria-pressed` on toggles, `role="switch"` on switches, and progress bars with readable values.
- Status messages are announced through a live region. Form errors appear next to the field and are linked to it.
- Contrast meets AA in all four themes, for text (4.5:1) and for borders and focus rings (3:1).
- Reflow down to 320 CSS px. The window is resizable, and zoom (<kbd>Ctrl</kbd>+<kbd>+</kbd> / <kbd>-</kbd> / <kbd>0</kbd>) is enabled.
- Reduced-motion and Windows High Contrast (forced colours) support.
- Toasts last longer, pause while hovered, and no longer rely on an icon or colour alone.
- The extension popup has labels, a focusable switch, a status region, and inline results instead of `alert()`.

### Security

- The download engine's RPC interface is protected by a secret. Kestrel generates one, or uses `rpc-secret` from `aria2.conf`.
- File names, paths and messages from the engine and from remote servers are escaped before rendering.
- The browser extension forwards only the cookies the browser would send to the download URL.
- The RPC address in Preferences must be an `http` or `https` URL.
- While Kestrel is open, the extension can read the secret from `127.0.0.1:6801`. That port is loopback only and sends no CORS header. See [SECURITY.md](SECURITY.md).

[1.0.0]: https://github.com/awm404/kestrel-download-manager/releases/tag/v1.0.0

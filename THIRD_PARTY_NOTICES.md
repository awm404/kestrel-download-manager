# Third-Party Notices

Kestrel Download Manager is released under the [MIT License](LICENSE). It builds on, bundles or loads the third-party software listed below. Each component remains under its own licence, and all copyright notices belong to their respective authors.

## aria2: the download engine

Kestrel is a graphical front end. **All downloading (HTTP/HTTPS, FTP, SFTP, BitTorrent, Metalink and magnet links) is done by [aria2](https://aria2.github.io/).**

| | |
| --- | --- |
| Project | aria2, <https://aria2.github.io/> |
| Source | <https://github.com/aria2/aria2> |
| Version tested | 1.37.0 |
| Copyright | Copyright © 2006, 2019 Tatsuhiro Tsujikawa, with Ross Smith II (Windows port), Nils Maier and [the aria2 contributors](https://github.com/aria2/aria2/blob/master/AUTHORS) |
| Licence | [GNU General Public License v2.0 or later](https://github.com/aria2/aria2/blob/master/COPYING), with a [linking exception for OpenSSL](https://github.com/aria2/aria2/blob/master/LICENSE.OpenSSL) |

**How Kestrel uses aria2.** Kestrel starts the engine as a separate process and controls it over aria2's JSON-RPC interface. Kestrel's own code contains no aria2 source and does not link against it. Under the GPL this is "mere aggregation", which is why Kestrel itself can be MIT-licensed.

### Kestrel Engine (modified aria2)

Portable builds ship **Kestrel Engine** (`kestrel-engine.exe`), a modified version of aria2 1.37.0 maintained in this repository's [`engine/`](engine/) directory.

- **Licence:** like aria2, it is licensed under GPL-2.0-or-later with the OpenSSL exception.
- **Changes from aria2:** the connection cap is raised to 64, and the program identifies itself as Kestrel Engine and credits aria2. The full list is in [engine/README.md](engine/README.md).
- **Files shipped with it:**
  - `engine-COPYING.txt`: the GPL text.
  - `aria2-AUTHORS.txt`: the aria2 authors.
  - `engine-LICENSE.OpenSSL.txt`: the OpenSSL exception.
- **Source code:** the complete corresponding source is attached to every release that contains the binary. This covers the patched engine tree, the patch itself, and every library archive.

Installer builds do not include an engine. Kestrel uses `kestrel-engine.exe` if it sits next to `Kestrel.exe`, and otherwise an aria2 you install yourself, for example with `winget install aria2.aria2`.

Kestrel Engine statically includes these libraries. It uses Windows' built-in TLS (SChannel), not OpenSSL.

| Library | Version | Licence |
| --- | --- | --- |
| [zlib](https://zlib.net/) | 1.3.1 | zlib License |
| [Expat](https://libexpat.github.io/) | 2.7.1 | MIT |
| [SQLite](https://sqlite.org/) | 3.46.1 | Public domain |
| [GMP](https://gmplib.org/) | 6.3.0 | LGPL-3.0-or-later or GPL-2.0-or-later |
| [c-ares](https://c-ares.org/) | 1.34.5 | MIT |
| [libssh2](https://libssh2.org/) | 1.11.1 | BSD-3-Clause |
| [mingw-w64](https://www.mingw-w64.org/) runtime | 8.0.0 | ZPL 2.1 and public domain |

## Desktop application

| Component | Licence |
| --- | --- |
| [Tauri](https://tauri.app/) and `tauri-plugin-opener` | MIT or Apache-2.0 |
| [serde](https://serde.rs/), [serde_json](https://github.com/serde-rs/json) | MIT or Apache-2.0 |
| [ureq](https://github.com/algesten/ureq) | MIT or Apache-2.0 |
| [getrandom](https://github.com/rust-random/getrandom) | MIT or Apache-2.0 |
| [windows-sys](https://github.com/microsoft/windows-rs) | MIT or Apache-2.0 |
| [open](https://github.com/Byron/open-rs) | MIT |
| [rfd](https://github.com/PolyMeilex/rfd) | MIT |
| [Feather Icons](https://feathericons.com/) (the interface icons are based on them) | MIT, Copyright © 2013–2023 Cole Bemis |

The table lists direct dependencies. Their own dependencies, with licences, can be listed with [`cargo about`](https://github.com/EmbarkStudios/cargo-about) or `cargo tree --manifest-path src-tauri/Cargo.toml`.

**Loaded at runtime, not redistributed:**

- **Fonts:** [Plus Jakarta Sans](https://github.com/tokotype/PlusJakartaSans) and [JetBrains Mono](https://github.com/JetBrains/JetBrainsMono), both under the SIL Open Font License 1.1, are served by Google Fonts.
- **Rendering:** the [Microsoft Edge WebView2 Runtime](https://developer.microsoft.com/microsoft-edge/webview2/) renders the interface and is provided by Windows.

## Trademarks

"aria2" is the name of the aria2 project. Kestrel and Kestrel Engine are independent projects and are not affiliated with or endorsed by the aria2 authors.

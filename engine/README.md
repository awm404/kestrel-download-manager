# Kestrel Engine

Kestrel Engine is the download engine bundled with Kestrel Download Manager. It is a lightly modified build of **[aria2](https://aria2.github.io/) 1.37.0** by Tatsuhiro Tsujikawa and the aria2 contributors.

> [!IMPORTANT]
> Everything in this `engine/` directory, and every `kestrel-engine` binary built from it, is licensed under the **GNU General Public License version 2 or later**, with aria2's OpenSSL linking exception. See [COPYING](COPYING) and [LICENSE.OpenSSL](LICENSE.OpenSSL).
>
> The rest of this repository is under the MIT License. Kestrel starts the engine as a separate program and talks to it over JSON-RPC. It does not link against it.

## Changes from aria2 1.37.0

A side-by-side comparison of the app and of this engine is in the [project README](../README.md#kestrel-and-aria2). All engine changes are in [`patches/0001-kestrel-engine.patch`](patches/0001-kestrel-engine.patch), and each modified file carries a dated notice.

| Change | Files |
| --- | --- |
| `--max-connection-per-server` (`-x`) accepts up to **64** instead of 16. | `src/OptionHandlerFactory.cc` |
| `--version` and `--help` identify the program as Kestrel Engine, name aria2 as its upstream, keep the original copyright, and send bug reports to Kestrel instead of aria2. | `src/version_usage.cc`, `src/kestrel_engine.h`, `src/Makefile.am` |
| `aria2.getVersion` also returns `engine` and `engineVersion`. The `version` field still reports the aria2 version, so existing RPC clients keep working. | `src/RpcMethodImpl.cc` |
| The Windows build links newer releases of zlib, Expat, SQLite, c-ares and libssh2 that fix known vulnerabilities, including CVE-2023-48795 ("Terrapin") in libssh2 1.11.0. | [`Dockerfile`](Dockerfile) |

**Kept the same on purpose:**

- The default `User-Agent` (`aria2/1.37.0`) and the BitTorrent peer ID prefix (`A2-1-37-0-`). Servers and private trackers recognise these, and changing them can get downloads blocked.
- All other options, defaults and behaviour.

**A note on more connections.** Opening many connections to one server can be treated as abuse. Some hosts throttle or ban clients that do it. Kestrel's default is still set by `aria2.conf`, and the cap only lets you choose a higher value.

## Building

The build cross-compiles a static 64-bit Windows executable inside Docker. It is based on aria2's own `Dockerfile.mingw`. From the repository root:

```sh
docker build -f engine/Dockerfile --target artifacts --output type=local,dest=engine/dist engine
```

This produces the following in `engine/dist/`:

| File | Contents |
| --- | --- |
| `kestrel-engine.exe` | The engine. |
| `kestrel-engine.exe.sha256` | Its checksum. |
| `sources/kestrel-engine-1.37.0-k1.tar.xz` | The patched engine source tree. |
| `sources/*.tar.*` | The exact source archives of every library linked into the binary. |
| `sources/0001-kestrel-engine.patch`, `sources/SHA256SUMS` | The patch and checksums of all source archives. |

The aria2 source archive is checked against a pinned SHA-256 before it is used. Library versions are build arguments at the top of the `Dockerfile`.

## Releasing

The GPL requires that anyone who receives the binary can get the complete corresponding source. When you publish a Kestrel release that includes `kestrel-engine.exe`, **attach everything in `engine/dist/sources/` to the same GitHub release.** Offering the source from the same place as the binary satisfies GPLv2 section 3.

## Reporting problems

- Problems you can reproduce with official aria2 belong upstream, in the [aria2 issue tracker](https://github.com/aria2/aria2/issues).
- Problems specific to Kestrel Engine belong in the Kestrel issue tracker.

## Credits

aria2 is Copyright © 2006, 2019 Tatsuhiro Tsujikawa. See [AUTHORS](AUTHORS) for the full list of contributors. Kestrel Engine modifications are Copyright © 2026 Kestrel contributors.

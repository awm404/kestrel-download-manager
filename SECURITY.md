# Security Policy

## Supported versions

Only the latest release of Kestrel receives security fixes.

| Version | Supported |
| --- | --- |
| 1.0.x | Yes |
| < 1.0 | No |

## Reporting a vulnerability

**Please do not report security vulnerabilities in public issues, discussions or pull requests.**

Report them privately in one of these ways:

- Use GitHub's [private vulnerability reporting](https://github.com/awm404/kestrel-download-manager/security/advisories/new) (**Security → Report a vulnerability**).
- Email **SECURITY_CONTACT_EMAIL**.

Please include:

- the Kestrel version, Windows version, and aria2 version (`aria2c --version`)
- the steps to reproduce, or a proof of concept
- the impact you believe it has, for example what an attacker could read, write or run

What to expect:

- We aim to acknowledge reports within **5 working days** and to send an assessment within **14 days**.
- We will keep you updated while we work on a fix, and agree a disclosure date with you. Coordinated disclosure normally happens within 90 days.
- With your permission, we will credit you in the release notes and the advisory.

Vulnerabilities in aria2 itself should be reported to the [aria2 project](https://github.com/aria2/aria2). If you are unsure which project is affected, report it to us and we will help route it.

## Security model

### Components and trust boundaries

Kestrel has three parts:

1. **aria2c**, a separate process that does the downloading. It exposes a JSON-RPC server on `127.0.0.1:6800`.
2. **The desktop app.** A WebView2 page (`src/index.html`) talks to aria2 over HTTP, and a Rust backend provides Tauri commands for file and OS operations (open, reveal in Explorer, rename, delete, folder picker, tray, notifications).
3. **The browser extension** (optional). It sends URLs and the matching cookies to aria2 over the same RPC interface.

### The aria2 RPC interface is protected by a secret

Anything that can reach aria2's RPC port can make it download any URL to any folder the user can write to. That includes overwriting files or dropping executables into the Startup folder. aria2 also accepts RPC calls as plain HTTP `GET` requests, so a malicious **web page** could send such commands simply by loading an image URL, without the user doing anything else.

Kestrel therefore never runs aria2 without authentication:

- If `%USERPROFILE%\.aria2\aria2.conf` sets `rpc-secret`, that value is used.
- Otherwise Kestrel generates a random 192-bit secret using the operating system's random number generator. It stores the secret in `%USERPROFILE%\.aria2\kestrel-rpc-secret` and starts aria2 with `--rpc-secret`.
- aria2 rejects requests without the correct `token:` with `Unauthorized`.
- aria2 listens only on the loopback interface (`--rpc-listen-all=false`).

If you start aria2 yourself, **always set `rpc-secret`**. Never expose port 6800 to your network without TLS (`rpc-secure`) and a strong secret.

### Known limitations and residual risks

We document these so that users and reviewers can make informed decisions. Contributions that reduce them are very welcome.

| Area | Detail |
| --- | --- |
| **Secret visible to local processes** | When Kestrel generates the secret, it is passed on aria2's command line. Other processes running as the same Windows user can read it. Users with access to your account can already read the secret file, so this does not cross a privilege boundary. To avoid it, put `rpc-secret` in `aria2.conf`. |
| **Secret stored in plain text** | `kestrel-rpc-secret`, `aria2.conf` and the extension's `chrome.storage.local` hold the secret unencrypted, protected only by your Windows account. |
| **Pairing port 6801** | While Kestrel is open it answers `GET http://127.0.0.1:6801/kestrel-rpc-secret` with the RPC secret, so the browser extension can pair without a manual paste. It listens on loopback only and sends no CORS header, so a web page cannot read the response. A program running as your Windows user can. |
| **`--rpc-allow-origin-all`** | Required for the WebView and the extension to call aria2 with `fetch`. It allows any web origin to send RPC requests, but they still fail without the secret. |
| **No Content Security Policy** | `tauri.conf.json` sets `"csp": null` because the UI still uses inline event handlers. Instead, all text from aria2 and remote servers (file names, paths, URLs, errors) is escaped before rendering. A strict CSP is planned. |
| **Tauri commands accept arbitrary paths** | The commands for opening, renaming, deleting and revealing files take paths from the UI. Any script injection in the UI would therefore give access to local files. This makes the escaping rules in [CONTRIBUTING.md](CONTRIBUTING.md) security-critical. |
| **Cookies forwarded to aria2** | The extension and the "paste a curl command" feature pass cookies to aria2 as a `Cookie` header. The extension only sends cookies the browser itself would send to that exact URL. aria2 may still send that header to a different host if the server redirects. Only download from sites you trust with your session. |
| **Third-party fonts** | The UI loads its fonts from Google Fonts, which makes a network request to Google on startup. If the request fails, the UI falls back to Segoe UI. Bundling the fonts locally is planned. |
| **Unsigned builds** | Release binaries are currently not code-signed, so Windows SmartScreen may warn about them. Verify that downloads come from the official Releases page. |

### Things that are *not* considered vulnerabilities

- A user downloading a malicious file they chose to download.
- Attacks that require an attacker who can already run code as the same Windows user. They can read the secret file and control aria2 directly.
- Issues in a self-managed aria2 instance that was started without `rpc-secret`.

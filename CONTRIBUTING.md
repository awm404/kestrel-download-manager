# Contributing to Kestrel

Thanks for helping improve Kestrel. This guide covers how to report problems, set up a development environment, and get a pull request merged.

By participating you agree to follow the [Code of Conduct](CODE_OF_CONDUCT.md).

## Ways to contribute

- **Report a bug** with the [bug report template](https://github.com/awm404/kestrel-download-manager/issues/new?template=bug_report.yml).
- **Report an accessibility barrier** with the [accessibility template](https://github.com/awm404/kestrel-download-manager/issues/new?template=accessibility.yml). You do not need to know which WCAG criterion applies; describing what you tried and what happened is enough.
- **Suggest a feature** with the [feature request template](https://github.com/awm404/kestrel-download-manager/issues/new?template=feature_request.yml).
- **Report a security vulnerability privately**, following [SECURITY.md](SECURITY.md). Please do not open a public issue for it.
- **Improve the documentation**, including fixing typos.

## Development setup

1. Install the prerequisites listed in the README under [Building from source](README.md#building-from-source), plus aria2 (`winget install aria2.aria2`).
2. Clone the repository and install the Tauri CLI:

   ```powershell
   git clone https://github.com/awm404/kestrel-download-manager.git
   cd kestrel-download-manager
   npm install
   ```

3. Start the app in development mode:

   ```powershell
   npm run dev
   ```

   Changes to `src/index.html` reload the window. Changes to Rust code in `src-tauri/` trigger a rebuild.

### Useful checks before opening a pull request

```powershell
# Rust: compile, lint and format
cargo check  --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo fmt    --manifest-path src-tauri/Cargo.toml --check

# Extension scripts: syntax check
node --check kestrel-extension/background.js
node --check kestrel-extension/popup.js
```

## Code guidelines

### Front end (`src/index.html`)

- **Never put untrusted text into `innerHTML` without `escapeHtml()`.** File names, paths, URLs and error messages from aria2 or remote servers are all untrusted. Prefer `textContent` where you can. Pass values to event handlers with `data-*` attributes and delegated listeners, not by interpolating them into inline `onclick` strings.
- Reuse the shared helpers instead of re-implementing them:
  - `openModal` / `closeModal` for dialogs. They handle `inert`, the focus trap and returning focus to where it was.
  - `setFieldError` for form validation messages.
  - `announce` for screen-reader status messages.
  - `showToast` for visible notifications.
  - `copyText` for clipboard access.
  - `withPreservedFocus` when re-rendering a region that may contain keyboard focus.
- Colours must meet WCAG AA contrast against every theme: 4.5:1 for text, 3:1 for icons, borders and focus indicators. Use the CSS custom properties (`--text-muted`, `--accent-fill`, `--status-danger-text`, `--border-control`, `--focus-ring`, …) rather than new hex values.

### Rust (`src-tauri/`)

- Keep Tauri commands small, and return `Result<T, String>` with a human-readable error so the UI can show it.
- Do not hardcode user-specific paths. Use `aria2_home()`, `configured_download_dir()` and friends.
- Anything passed to a shell or PowerShell must be escaped for that context.

## Accessibility checklist

Kestrel targets [WCAG 2.2 AA](ACCESSIBILITY.md). For any UI change, please confirm:

- [ ] Everything works with the keyboard alone. Focus is always visible and moves in a logical order.
- [ ] Opening a dialog or menu moves focus into it. Closing it, including with <kbd>Esc</kbd>, returns focus to the control that opened it.
- [ ] Every interactive element has an accessible name. Icon-only buttons have an `aria-label`. When a control shows visible text, its accessible name starts with that text.
- [ ] State is exposed programmatically (`aria-pressed`, `aria-expanded`, `aria-checked`, `aria-invalid`), not only through colour.
- [ ] Status changes that are not visible where focus is are announced (`announce()` or a live region).
- [ ] The layout still works at 200% zoom and in a 320 px wide window without horizontal scrolling.
- [ ] Animations respect `prefers-reduced-motion`, and the UI is usable with Windows High Contrast themes.
- [ ] Interactive targets are at least 24 × 24 CSS px.

If you can, test with a screen reader: [NVDA](https://www.nvaccess.org/) is free, and Windows Narrator is built in.

## Commit and pull request process

1. Fork the repository and create a branch from `main` (for example `fix/rename-focus` or `feat/speed-limit`).
2. Keep each pull request focused on one change. Large refactors are easier to review when split up.
3. Write commit messages in the imperative mood ("Fix focus loss after deleting a transfer"). [Conventional Commits](https://www.conventionalcommits.org/) prefixes such as `fix:` and `feat:` are welcome but not required.
4. Update `CHANGELOG.md` under **Unreleased** when your change affects users.
5. Fill in the pull request template, including how you tested it. Screenshots or short recordings help a lot for UI changes.
6. A maintainer will review your pull request. Please respond to feedback, or say so if you no longer have time; someone else can pick it up.

## License of contributions

Kestrel is MIT licensed. By submitting a contribution, you agree that it is licensed under the [MIT License](LICENSE) and that you have the right to submit it.

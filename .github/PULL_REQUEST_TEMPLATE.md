## Summary

<!-- What does this change, and why? Link the issue it addresses, e.g. "Fixes #123". -->

## Type of change

- [ ] Bug fix
- [ ] New feature
- [ ] Accessibility improvement
- [ ] Security fix
- [ ] Documentation
- [ ] Refactor / maintenance

## How was this tested?

<!-- Describe what you ran and clicked. Screenshots or short recordings help for UI changes. -->

## Checklist

- [ ] `cargo check`, `cargo clippy -- -D warnings` and `cargo fmt --check` pass (if Rust changed)
- [ ] `node --check` passes for changed extension scripts
- [ ] Text from aria2 or remote servers is escaped (`escapeHtml` / `textContent`) and not interpolated into inline handlers
- [ ] `CHANGELOG.md` is updated under **Unreleased** (if users will notice the change)
- [ ] Documentation is updated (if behaviour or settings changed)

### Accessibility (for UI changes)

- [ ] Works with the keyboard alone, with a visible focus indicator and a logical focus order
- [ ] Dialogs and menus move focus in, and return it on close or <kbd>Esc</kbd>
- [ ] New controls have accessible names, and their state is exposed (`aria-pressed`, `aria-expanded`, …)
- [ ] Status changes are announced (`announce()` or a live region)
- [ ] Colours meet 4.5:1 for text and 3:1 for non-text in all four themes
- [ ] Usable at 200% zoom and 320 px width; respects reduced motion and forced colours
- [ ] Tested with a screen reader (NVDA / Narrator), or noted below that it was not

<!-- Accessibility notes, e.g. "Tested with NVDA 2024.4" or "Not tested with a screen reader". -->

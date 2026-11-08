# Accessibility Statement

Kestrel aims to be usable by everyone, whether they use a mouse, only a keyboard, a screen reader, magnification, high-contrast themes, or have a preference for reduced motion.

## Conformance target

Kestrel targets the [Web Content Accessibility Guidelines (WCAG) 2.2](https://www.w3.org/TR/WCAG22/) at **level AA**, applied to desktop software as described in [WCAG2ICT](https://www.w3.org/TR/wcag2ict-22/).

**Status: partially conformant.** We believe the desktop UI and the browser extension popup meet the level AA success criteria. However, they have not yet been formally audited or tested end to end with every major screen reader; see [Known limitations](#known-limitations). Treat this statement as our commitment, and please [tell us](#feedback) where we fall short.

This statement was last reviewed for version **1.0.0**.

## What is supported

### Keyboard (2.1.1, 2.1.2, 2.4.3, 2.4.7, 2.4.11)

- Every feature works without a mouse. See [Keyboard shortcuts](README.md#keyboard-shortcuts).
- A **Skip to transfers** link is the first thing you reach with <kbd>Tab</kbd>.
- Transfer cards are a list you move through with the arrow keys, <kbd>Home</kbd> and <kbd>End</kbd>.
  - <kbd>Enter</kbd> runs the card's main action.
  - <kbd>Shift</kbd>+<kbd>F10</kbd> or the <kbd>Menu</kbd> key opens its actions menu.
  - <kbd>Delete</kbd> removes it after you confirm.
- Dialogs trap focus while they are open and make the rest of the window inert. When they close, focus returns to the control that opened them.
- After a transfer is removed, focus moves to the next transfer instead of being lost.
- When the list re-renders with live progress, focus stays on the same control.
- Menus (the card actions menu and the sort menu) follow the ARIA menu pattern: arrow keys, <kbd>Home</kbd> and <kbd>End</kbd>, type-ahead, and <kbd>Esc</kbd> to close. The current sort order is exposed as `menuitemradio` with `aria-checked`.
- Destructive confirmations focus the safe choice (**Cancel**) first.
- Focus indicators are at least 2 px thick, and have at least 3:1 contrast in every theme.
- Shortcuts that use a single printable character are not used outside menus (2.1.4).

### Screen readers (1.3.1, 2.5.3, 4.1.2, 4.1.3)

- Landmarks (`header`, `main`, `nav`) and a logical heading structure.
- Every button has an accessible name. When a button shows visible text, its accessible name starts with that text. Icon-only buttons include the file name, for example "Pause ubuntu.iso".
- Toggle buttons use `aria-pressed`, switches use `role="switch"`, disclosure buttons use `aria-expanded`, and invalid fields use `aria-invalid` with the error message linked through `aria-describedby`.
- Each transfer card is named by its file name and status. Its progress is a `progressbar` with a readable `aria-valuetext`, for example "42.0%", or "Resolving metadata" while a magnet link starts. Speed and time remaining are plain text in the card.
- Status changes, such as a transfer being added, paused or finished, a setting being saved, or an error, are announced through a polite live region without moving focus.
- The bandwidth chart in Advanced mode is a labelled image. The current and peak speeds it shows are also available as text next to it.
- Decorative icons, emoji and separators are hidden from assistive technology.

### Vision (1.4.1, 1.4.3, 1.4.4, 1.4.10, 1.4.11)

- All four themes meet AA contrast. Each figure is the lowest ratio against the background, surface and raised-surface colours of all four themes:

  | Element | Lowest contrast | Required |
  | --- | --- | --- |
  | Secondary text (`#94a3b8`) | 5.90 : 1 | 4.5 : 1 |
  | Muted text (`#8b9bb0`) | 5.34 : 1 | 4.5 : 1 |
  | Error text (`#f87171`) | 5.47 : 1 | 4.5 : 1 |
  | White text on primary buttons (all themes) | 5.36 : 1 | 4.5 : 1 |
  | White text on danger buttons (`#dc2626`) | 4.83 : 1 | 4.5 : 1 |
  | Control borders (`#64748b`) | 3.18 : 1 | 3 : 1 |
  | Focus indicators | 5.73 : 1 | 3 : 1 |

- State is never shown by colour alone. Status labels use text, and the compact status badge also differs in shape.
- Text can be zoomed to 200% (<kbd>Ctrl</kbd>+<kbd>+</kbd>). The layout reflows without horizontal scrolling down to a width of 320 CSS px, for example a 1280 px window at 400% zoom.
- **Translucent window (optional, off by default):**
  - On Windows 11 22H2 and later it uses Acrylic, and a brighter text palette.
  - At the default background opacity (55%), every text colour still meets AA and borders still meet 3:1, even with a white window behind Kestrel. This was measured against dark Acrylic over white, about `rgb(160, 163, 170)`.
  - Lower opacities are allowed, but the setting then shows and announces a contrast warning.
  - Translucency is switched off automatically when Windows transparency effects are off or a contrast theme is active.
- **Windows High Contrast / forced colours:** there are dedicated `forced-colors` styles, so borders, focus rings, selected states and progress bars use system colours.

### Motion and timing (2.2.1, 2.2.2, 2.3.3)

- With **Reduce motion** turned on in Windows (*Settings → Accessibility → Visual effects → Animation effects*), animations, the scrolling of long file names and transitions are switched off.
- Notification toasts stay for 5 seconds, pause while the pointer is over them, and their content is also announced to screen readers. No action requires you to respond to a toast.
- There are no time limits on any task.

### Input (2.5.1, 2.5.2, 2.5.7, 2.5.8)

- Interactive targets are at least 24 × 24 CSS px, or have enough spacing around them. Checkboxes and switches can be activated anywhere on their label.
- No feature needs a path-based gesture or dragging. The window can be maximised and restored with the title bar button, and snapped with <kbd>Win</kbd>+arrow keys.
- Actions run when the button is released, not when it is pressed.

### Forms and errors (3.3.1, 3.3.2, 3.3.3, 3.3.4)

- Every field has a visible label, and hints are linked with `aria-describedby`.
- Validation errors appear next to the field and are announced (`role="alert"`). Each error says how to fix the problem, for example "File name cannot contain any of these characters: \ / : * ? " < > |". Focus moves back to the invalid field.
- Deleting files from disk always asks for confirmation first.

### Browser extension popup

The extension popup follows the same rules. It has a labelled switch with a visible focus ring, labelled fields, the engine status as a status region, inline (not `alert()`) results for the send form, and reduced-motion and forced-colours styles.

## Known limitations

| Limitation | Impact | Plan |
| --- | --- | --- |
| Not yet tested end to end with NVDA, JAWS and Narrator | Some announcements may be verbose or missing in a particular screen reader. | Manual screen-reader testing before 1.1. Reports are very welcome. |
| Text spacing (1.4.12) and Windows High Contrast themes not yet verified by hand | The layout uses flexible sizing and has forced-colours styles, but it has not been checked with text-spacing overrides or each High Contrast theme. | Manual check before 1.1. |
| Custom title bar | The window uses a custom drawn title bar. Its minimise, maximise and close buttons are keyboard and screen-reader accessible. Snap layouts from hovering the maximise button are not available; use <kbd>Win</kbd>+<kbd>Z</kbd> instead. | Track Tauri support for native caption buttons. |
| Fonts are loaded from Google Fonts | Offline, the UI falls back to Segoe UI. This does not affect accessibility. | Bundle fonts locally. |
| Piece map in Advanced mode | The visual piece map has no detailed text equivalent. The overall percentage and piece counts are available as text. | Consider a text summary of missing ranges. |
| Not tested on macOS or Linux | Platform screen readers (VoiceOver, Orca) have not been tested. | Depends on cross-platform support. |

## How we test

- Keyboard-only walkthroughs of every flow: adding (including validation errors), pause and resume, renaming, removing with and without deleting the file, the sort and filter controls, preferences, and the feature tour.
- Automated checks that every `aria-labelledby`, `aria-describedby` and `aria-controls` reference points to an existing element, and that inline handlers exist.
- A mock aria2 server that serves hostile file names (HTML and script payloads), to verify that nothing from the network is rendered as markup.
- Layout checks at widths from 320 to 1700 CSS px, looking for overlapping content and horizontal scrolling. 320 px is the width a 1280 px window has at 400% zoom.
- Contrast ratios calculated for every theme colour pair.

Contributors are asked to run through the [accessibility checklist](CONTRIBUTING.md#accessibility-checklist) for UI changes.

## Feedback

If anything in Kestrel is hard or impossible for you to use, please tell us. That includes problems that are not covered by WCAG.

- Open an [accessibility issue](https://github.com/awm404/kestrel-download-manager/issues/new?template=accessibility.yml).
- Or email **ACCESSIBILITY_CONTACT_EMAIL** if you prefer not to use GitHub.

We aim to reply within 5 working days. Accessibility bugs are treated with the same priority as functional bugs.

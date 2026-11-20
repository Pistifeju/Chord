# Settings

- A panel centered over the window, which blurs behind it as behind a sheet: `canvas`, Card radius, sheet shadow, 960 × 760px at most and 28px clear of the window edges.
  It rises in like a sheet.
  Esc, the close icon button or a click outside it closes it; a sheet opened over it closes first.
  The window behind it is inert.
- **Section list:** 220px on the left, `panel` with a 1px `line` right edge, 12px 8px padding, rows 2px apart: General, Agents (a chat icon), Appearance, Shortcuts, and Simulators once a worktree has an Xcode project (a phone icon).
  A row is 30px, 10px padding, Control radius, a 16px `ink-muted` icon and the name in `ink-secondary` 8px after it.
  Hover takes `raised` and `ink` over 120ms, pressed takes `selected`; the shown section takes `selected`, `ink` at 500 and an `ink-secondary` icon.
  The focus ring sits inside the row.
  It opens on General.
- **Header:** 48px with a 1px `line` bottom rule, 28px left padding: "Settings / General" at Heading size, "Settings" in `ink-muted`, the slash at 400 in `ink-disabled`, the section in `ink`.
  A close icon button sits at the right.
- **Content:** scrolls inside; 640px at most, centered, 28px side padding.
  Each group takes a Heading (28px above, 8px below, a status notice in meta text at its right), then a `panel` card with a 1px `line` border and the Card radius.
- **Row:** 44px at least, 10px 16px padding, a 1px `line` rule above every row but the first.
  The name in body `ink` on the left, a one-line meta description in `ink-muted` under it (an error takes `danger`; Settings › Simulators rows wrap it), the control on the right.
- **Agents:** one group, "Agent for new sessions", with Check Again (`design/onboarding.md`) at the heading's right.
  The harness rows draw their own card, git's row with them while git is missing.
  Under the card, 8px down and 4px in, a meta line: "New sessions use the checked agent." while several take the pick, or "Run the command, then switch back. Chord checks again." while none is the default or git is missing.
  The check runs when the section shows and each time the window comes forward.

# Editor

- Fills the work area left of the side panel while a file tab is open; the panes stay mounted behind it.
  `canvas` fill, Code text at the editor font size (Settings › Appearance › Editor, 9 to 24px, 12px by default; diffs take it too), Body line height, a line number gutter in `ink-disabled` (`ink-secondary` on the cursor's line), no ring on focus, `editor-selection` for the selection, `editor-match` on the matching bracket and the `syn-*` colors for keywords, strings, numbers, types and functions; comments are `ink-muted` italic.
  It saves 1s after the last edit and when its tab goes.
- **Editing:** brackets and quotes close as you type them.
  ⌘-click adds a cursor and ⌥-drag a column of them, the cursor's line takes `editor-line`, see-through so the selection shows under it, and the other copies of the selected text take `editor-match`.
  A fold gutter right of the line numbers shows a 12px `ink-muted` chevron on every block that folds (⌥⌘[ and ⌥⌘]); a folded block reads as a `raised` placeholder with the Inset radius.
- **Completion:** typing opens a list under the cursor of the file's own words and the language's keywords: `sheet`, a 1px `line` border, the Control radius, `--shadow-popover`, 4px padding, 240px tall at most.
  Rows are 24px in Code text, `ink-secondary`, 8px side padding, the Inset radius, no icons; the typed part takes 600 and the highlighted row `selected` and `ink`.
  ↑ and ↓ move the highlight, Return picks, Esc closes.
- **Find:** ⌘F in the editor opens the chat's find bar at the editor's top right.
  Matches take `editor-match`; the current one adds a 2px `accent-ink` underline and scrolls to the middle.
  A new query starts at the first match after the caret.
  Esc closes the bar, selects the current match and returns the focus to the editor.
- **Image view:** an image file (PNG, JPEG, GIF, WebP) opens in its file tab as the image, not the editor: centered on `canvas` with 24px padding, scaled down to fit and never up.
  It reads the file again within a second of a change on disk; a file gone from disk closes its tab.

# Sheets

- A `sheet` card centered over the window, which blurs 12px behind it and keeps its brightness.
  A click outside the sheet cancels.
  While a sheet is open the window behind it is inert: Tab and clicks stay in the sheet.
- **Form sheet:** a header row with the title on the left and a close icon button (X) on the right that cancels.
  The fields follow, then a footer with the primary button on the right.
  Return triggers the primary button, Escape cancels.
- **Focus after a sheet:** when a sheet or Settings closes, the focus goes back to the control that held it before, unless the sheet's action moved it or that control is gone.
- **Help sheet:** 440px wide, 20px between blocks.
  A form sheet header titled "Help", then three blocks, each under a field label: Basics (project, worktree, session: the name at 500, a line in `ink-secondary` under it), Statuses (each `StatusMark` with its word in the status color, the line under it) and Shortcuts (a command title on the left, its keys in `ink-muted` on the right; a command with no key is left out).
  A secondary "All Shortcuts" button in the footer opens Settings on Shortcuts.
- **Confirm sheet:** title, one centered `ink-secondary` line (`.sheet-description`) and an optional detail in meta text, 4px apart as one block; 20px below it two equal 32px buttons: secondary "Cancel" and the destructive or primary action.
  Focus starts on the action, Escape cancels, and a long name or path wraps inside the sheet.
  The detail is selectable text.

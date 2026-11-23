# Preview view

- **Header:** a row over the screen, left of the list, as tall as the list's filter row so the model button lines up with the field: 30px with 4px top and bottom and 12px side padding.
  Centered in it, over the screen it names, the model button: an `ink-muted` phone icon and the model name in 12px `ink-secondary`, without a pick the simulator a run without one uses, as the toolbar shows it (tooltip "Preview Device"); 24px tall, 8px side padding, 6px gaps, Inset radius, `raised` on hover, `selected` when pressed.
  On hover or keyboard focus a 12px `ink-muted` chevron opens in after the name over 120ms with `--ease-out` (at once under Reduce Motion).
  It opens a native menu of the simulator models as the run menu lists them, the project's pick with a check mark.
  While a rebuild runs, a 12px spinner and "Rebuilding" in meta text sit right of it; a hot reload shows nothing.
- **List:** a column right of the screen, on the panel's outer edge, 200px wide by default, with a 1px `line` left edge that drags it between 140px and 320px, as the side panel's edge, and never past half the view's width, so a narrowed side panel keeps room for the screen; the width lasts until the app quits.
  It holds the Files filter field (placeholder "Filter"), then the worktree's `#Preview`s as Files tree rows.
  A file's previews drop the name prefix all of them share up to " — " and start with a capital ("Symptoms — first use" reads "First use"); the tooltip keeps the full name and the file.
  The list runs the view's full height, so its edge meets the panel's top bar.
  A file reads as a folder row with a file icon and its relative path; its previews sit one level in, each with its name and no icon, an unnamed one by its index ("Preview 2").
  A file with one preview reads as a single row with the file icon and path and no chevron, and a click shows that preview.
  Files keep path order.
  The shown preview's row takes `selected` and `ink`.
  ↑ and ↓ go to the previous or next preview, past file rows and the previews of a closed file, from a row or from the filter field; ⌥↑ and ⌥↓ go to the first preview of the previous or next file.
  From a row the focus moves with them; the field keeps the focus, so typing goes on.
  The preview takes the selection at once and shows once the keys rest for 150ms.
  → and ← open and close a file, as in the Files tree.
  A click shows a preview or opens and closes a file.
  The pick stays the worktree's, whatever file the editor shows.
  Typing filters on name and file and opens every file.
  No match reads "No matches".
  The list reads again when the window takes the focus.
  Showing the view widens the side panel to 560px at least; the next view gets back the width the panel had before.
- **Empty:** a worktree without a `#Preview` shows no header, screen or list.
  In their place sits the Simulator view's centered meta line, 40% down: "No previews in this worktree." in `ink-secondary`, then "Add a `#Preview` to a SwiftUI view to see it here." in `ink-muted`, 4px apart, the `#Preview` as inline code on `raised` with the Inset radius.
  The list reads again when the window takes the focus, and the view fills once a `#Preview` shows up.
  While the first list loads, the view stays blank.
- **Screen:** the Simulator view's screen, bezel and input, without its button row.
- **Compile error:** the last frame stays, and a bar on `raised` with a 1px `line` top edge sits along the view's bottom: a 16px `danger` error icon, then the first error, `file:line` in 12px `danger` and the message in 12px `ink`, which truncates, its full text as the tooltip, 10px 12px padding.
  A click opens the file at that line.
  The bar goes when a build succeeds.
- **Off:** the Simulator view's centered meta line until the preview app first runs on the booted preview simulator: "Pick a preview." with no pick, "Starting the preview…" while the preview simulator boots, "Waiting for the Run build…" while ⌘R builds, "Building the preview…" while the preview builds, the failure's message when the build or launch fails, and "Two previews already run: <worktree>, <worktree>." when the limit holds.
  A rebuild after that keeps the screen.

# Status bar

A 36px strip under the main area, on `canvas` with a 1px `line` top edge.
The sidebar reaches the window's bottom edge, and its 36px footer row shares the strip's top line.
Its items sit on the strip's center line.
The 28px gear icon button that opens Settings (tooltip "Settings (⌘,)") sits in a 36px footer row at the sidebar's bottom, 8px in, under a 1px `line` divider.
At the row's right end, 8px in, a question mark icon button (tooltip "Help") opens the Help sheet (see Sheets).
Both hide with the sidebar; ⌘, and the app menu still open Settings.
The strip's items sit at the right with 12px padding, in 11px `ink-muted`, tabular figures.
The strip always shows.
Subscription usage shows one group per harness that reported it in this run, in the harness list's order (pi, Codex, Claude Code), 16px apart; a group starts with the harness's 12px logo, 8px before its bar, and reads `42% 2h 48m  ·  59% 4d 6h`: each window's percentage and the time left to its reset in its two largest units, windows joined by a middle dot with two spaces each side, refreshed each minute.
Before it, 8px away, a 32 × 4px bar (2px radius, `line-strong` track, `accent-ink` fill; above 70% `status-waiting`, above 85% `status-error`) shows the shortest window's percentage.
A group's tooltip names the harness, then lists each window as `5h 42% (resets 14:30)`; a reset a day or more away carries its weekday.
A window past its reset is hidden, and a group with no window left goes.
A group is a button: a click opens the Usage panel (see `design/usage-panel.md`), and hover turns its text to `ink` over 120ms.
A login without a subscription (an API key) reports no window, so its harness shows no group.
In a worktree with an Xcode project the simulators button sits before the usage (see Build and Run).
Once a newer release exists, an update button sits at the strip's left end, 12px in: a 12px down arrow, 6px before `Update to 0.2.1` at 500 in `ink-secondary`, tabular figures; hover turns its text to `ink` over 120ms.
Chord looks for a release at launch and every six hours; a dev build never looks.
A click downloads the release and restarts Chord into it; while an agent works it first opens a confirm sheet titled "Update Chord?" with a destructive "Update" action.
During the download the button takes no click and reads `Downloading 42%` in `ink-muted`, after the usage bar's 32 × 4px bar, which fills with the download.
Before the server tells the download's size it reads `Downloading` with no bar; once the download ends it reads `Restarting`.
A failed download shows a failure toast and the button comes back.

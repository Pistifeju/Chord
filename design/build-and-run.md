# Build and Run

Every part shows only in a worktree with an Xcode project.
A failure is `danger` text and the toolbar's Run and Stop icons take `success` and `danger`; the rest is neutral, and the status colors stay with the agents.

- **Toolbar:** the title bar row over the panes and the side panel, as in Xcode: 40px on `canvas` with a 1px `line` bottom edge, dragging the window between its controls, 4px left and 8px right padding; with the sidebar hidden it starts after the sidebar toggle.
  Centered as one group between the bar's left edge and the view buttons: Run, a `success` play triangle (tooltip "Run (⌘R)"), which builds and runs, in place of a running app too; Stop, a `danger` stop square (tooltip "Stop (⌘.)"), disabled with the `ink-disabled` glyph while nothing builds or runs, its color changing over 120ms; both plain icon buttons 2px apart with no capsule, as the side panel's view buttons that close the row.
  After Stop, 8px away, the activity field: 28px, 880px at most, `raised` fill, Control radius, 12px text on one line.
  Its left part holds the run target, as in Xcode, a fixed 55% of the field so a new pick or a loading listing never moves the rule or the status, then a 1px `line-strong` rule: two buttons, the scheme button as wide as its content and the destination button taking the rest, with a 12px `ink-disabled` chevron between them, 12px/500 `ink`, square corners, `selected` on hover and when pressed, their parts 6px apart, the focus ring inside.
  The scheme button (tooltip "Scheme and Build Configuration", 10px left and 4px right padding): an `ink-muted` scheme icon, the scheme's name, then the configuration at 400 in `ink-secondary` after an `ink-muted` "·".
  The name and configuration always show: the project's pick at once, else what a build used at the project's last listing, then what it uses once the new listing came.
  A click opens the scheme menu under it.
  The destination button (tooltip "Run Destination", 4px left and 8px right padding): an `ink-muted` phone icon (a Mac icon for My Mac), the destination's name (without a pick, the simulator a run uses: the newest iOS version's iPhone Pro that is not a Max, or My Mac for a project that builds for macOS alone; nothing until the listing comes) and a simulator's iOS version at 400 in `ink-muted`.
  A click opens the run menu under it.
  A long scheme or destination name truncates.
  The rest of the field holds the status, left aligned with 10px on each side, truncating, its full text as the tooltip: a lead at 500 in `ink`, then a detail in `ink-secondary` that carries its own lead-in.
  "Building Chord · Compiling ChordKit" (the scheme, then the last step the log names with its target: Compiling, Linking, Signing, Compiling assets of; Rust reports no file counts), "Building Chord · Waiting for another build", "Installing Chord on iPhone 17 Pro", "Launching Chord on …" and "Running Chord on …"; "Build succeeded · today at 10:32" with the lead in `success`; "Build failed · today at 10:32" with the lead in `danger` and, at the field's right end, 4px in, the error count as a 22px button (a 12px warning icon and the count at 11px/500 tabular, `danger`, Inset radius, `selected` on hover and while its list shows, tooltip "Show Build Errors (⌘')") that opens the error list; "Build stopped · …" and "Chord exited · …" with the lead at 400; "No build yet" at 400.
  An end's time reads as a message's time does, with "today at" before today's clock.
  While it builds, installs or launches, a 2px line runs along the field's bottom edge where a 30% `accent-ink` segment slides from left to right every 1.4s, easing in and out; the line shows that work goes on, not how far.
  Without the scheme's listing the lead drops the scheme's name.
  A build beside the running app speaks first.
  Under a 900px toolbar the iOS version hides, under 640px the scheme's name and configuration, and the scheme button shrinks to its icon.
- **Error list:** a popover under the error count, its right edge on the count's, 6px below it: 480px wide, 320px tall at most, `sheet`, Control radius, the Popover shadow.
  It opens out of the count's corner, fading in, dropping 4px and growing from 96% over 200ms with `--ease-out`; under Reduce Motion it only fades.
  The failed build's errors, 30 at most, grouped by file in the order they came: a 26px heading per file, sticky, with the file name at 11px/600 in `ink-secondary` and its error count at 500 in `ink-muted`, tabular; errors without a file group under "Build".
  Each error is a 26px row, Inset radius, 20px in: a 12px `danger` warning icon, the message at 12px in `ink`, truncating with the full text as its tooltip, and the line at 11px in `ink-muted`, tabular, at the right.
  The list takes the keys when it opens: ↑ ↓ move the highlight, which wraps and takes `raised` (the line steps up to `ink-secondary` on it), the pointer moves it too, Return or a click opens the file at that line in the focused group, as the Build view's rows do, and closes the list; an error without a file in the worktree opens nothing and reads in `ink-secondary`.
  More than 30 end with "and 12 more in the build log" in meta text.
  A footer under a 1px `line` rule holds "↑↓ to move · ↩ to open" at 11px in `ink-muted` and a "Show Build Log" text button at 12px/500 in `ink-secondary` (24px, Inset radius, `raised` on hover, `selected` when pressed) that shows the Build view.
  Esc closes it and puts the focus back on the count; a click outside, the count, ⌘' again, a resize and a new build close it too.
  ⌘' (Show Build Errors in the Product menu) opens it while the worktree's last build failed.
- **Run menu:** a native menu, as the worktree card's.
  Run (⌘R), Build (⌘B) and Stop (⌘., disabled while nothing builds or runs), a separator, then the run destinations: My Mac and a separator when the project builds for macOS, connected devices under a disabled "Devices" line, then the simulator models under a disabled line per iOS version ("iOS 27.0"), newest first, a separator before each group after the first, iPhones before iPads.
  The project's pick carries a check mark.
  Without a simulator the list is one disabled "No Simulators" line.
  A pick that is gone falls back to the first simulator, and a notice toast names both.
- **Scheme menu:** a native menu, as the run menu.
  The worktree's schemes, a separator, then its project's configurations under a disabled "Build Configuration" line.
  Each checks the project's pick, else the scheme a build without a pick uses and the configuration that scheme's Run action names.
  ⌘R, ⌘B and the agent's builds use the picks; the preview builds the default.
- **Product menu:** a menu bar menu after View, as Xcode's: Run (⌘R) and Build (⌘B), a separator, Clean Build Folder (⌘⇧K), a separator, Stop (⌘.), a separator, Show Build Errors (⌘', enabled while the last build failed with errors).
  Each item is disabled while its command cannot run: outside a worktree with an Xcode project, and Stop while nothing builds or runs.
- **Run marker:** on the worktree's card, at the meta line's right end, as a session row's status sits, 8px from the git counts at least, 11px/500, tabular: "Building" while it compiles and "Running" in `accent-ink`, "Build succeeded" in `success` once the build is done (a run's too, while the app installs and launches) and "Build stopped" in `ink-muted` until the user goes to the worktree, or for 5 seconds on the open worktree's card, "Build failed" in `danger` until the worktree's next build starts.
  It fades in and rises 4px as the session row's status does.
  Every build of the worktree sets it, the agent's too.
- **Bottom panel views:** the log half: one column right of the Terminal that shows Build or Console, split from the Terminal by a 1px `line` rule.
  Dragging that rule resizes the log half, 480px wide by default, at least 200px, and it leaves the Terminal 200px at least; the rule has the bottom panel edge's grab area and hover.
  The width is saved per worktree.
  Each half shows on its own, and alone it takes the panel's width; the panel hides when neither shows.
  The log half has the Terminal's 28px header: two tabs, Build and Console, side by side from the header's left edge with no gap, in the place of a name, then a Hide button at the right end that hides that half only.
  A tab is the tab bar's tab at the header's height: the header's full height above its bottom rule, square corners, 10px side padding, 12px/500 `ink-muted`.
  Hover and the shown view's tab take `raised` and `ink` over 120ms with `--ease-out`; under Reduce Motion they change at once.
  The focus ring sits inside the tab.
  A click switches the view; the Terminal's header keeps its name at 12px/500 in `ink-secondary`, focused or not.
  Console has no clear button: no control in this document clears a view, and a build clears it.
  ⌘T shows or hides the Terminal; ⌘⇧Y shows or hides the log half.
  Both are saved per worktree.
- **Which view a run shows:** ⌘R and the play button open the side panel on the Simulator view when the run goes to a simulator.
  An agent's run opens it too: an agent runs only when the user asks, and always on the worktree's simulator.
  A build the user starts (⌘R, ⌘B, the play button) shows the log half on Build and leaves the Terminal as it is; when the app launches the half switches to Console, if it still shows Build; a failed build stays on Build.
  A build the agent started moves nothing else; the run marker reports it.
- **Build view:** the build's output as Code text in `ink-secondary` on `canvas`, 8px padding, selectable, one scroll area that stays at the end while output arrives unless the reader scrolled up.
  Only the lines in view are mounted; the log keeps 10,000.
  After a failed build its errors come first in the scroll area, 30 at most, and the view scrolls to them: 26px rows with the Control radius, a 16px `danger` warning icon, then the file name and line ("LoginView.swift:42") in `ink-secondary`, then the message in `ink`, which truncates and holds the full text as its tooltip.
  Hover takes `raised`; a click opens the file at that line in the focused group, as the Files tree does.
  An error without a file shows its message only and takes no click.
  More than 30 end with "and 12 more" in meta text.
  Before the first build the view reads "No build yet" in meta text.
- **Console view:** the running app's output in the Build view's text style, cleared when a build starts, 10,000 lines.
  An OSLog error or fault line takes `danger`; Chord's own line ("Process exited with code 1") takes `ink-muted`.
  Until the app prints, the view reads "No app output yet" in meta text.
  ⌘⇧Y with neither column showing opens Console once the app has run, else Build.
- **Find:** ⌘F in the Build or Console view opens the chat's find bar under the header, as in the Terminal.
- **Simulators:** a text button at the status bar's right, 16px before the usage, shown while a simulator Chord created is booted, in any worktree: a 16px phone icon and the count in 11px `ink-muted`, tabular, 4px apart; 20px tall, 8px side padding, Inset radius, `raised` on hover, `selected` when pressed; tooltip "2 Simulators Running".
  A click opens a native menu: one disabled line per booted simulator ("feature-login · iPhone 17 Pro", a preview simulator "feature-login · Preview · iPhone 17 Pro"), a separator, then Shut Down Others, which keeps the open worktree's run and preview simulators and is disabled when no other is booted.
- **Build data:** a row in Settings › General under a "Storage" heading, shown once a worktree has an Xcode project: "Build Data", the description "DerivedData folders and simulators of your worktrees. Deleting a worktree deletes its share.", and the size at the right in `ink-secondary`, tabular ("12.4 GB").
  No button: Chord removes the data of a worktree that is gone by itself.
- **Screenshots:** Settings › Simulators, shown once a worktree has an Xcode project, holds a group "Screenshots and recordings" above "Background services": a row "Save to" with the folder's path as its description, "Desktop" until the user chooses one, and a secondary "Choose…" button at the right that opens the folder dialog.
  Cancelling keeps the folder.
  Screenshots and recordings both go in it.
- **Simulator services:** Settings › Simulators, shown once a worktree has an Xcode project, holds one group "Background services" with a meta notice at its right ("Applies at the next boot · iOS 18.5 and newer").
  A row per service group: its name, a meta description that wraps (what stops working while it is off, then "Frees about 265 MB."), and an On / Off segmented control, labeled "Keep <name> on", at the right.
  The description sits under the name, aligned to it.
  A group with several subgroups has a chevron before its name, and a click on its name or description opens the row, the chevron turning as a folder's does, to one row per subgroup under it, indented to the group's name, with the same parts: name, effect, an estimated memory and its own switch.
  The group's switch sets all its subgroups; it shows On when all are on, Off when all are off and neither selected when they differ, and the group's description then ends with "<n> of <m> on." A group with one function has no chevron and no subgroups; its name keeps the chevron's space, so every name lines up.
  App Store, Push & Media and Safari Sync & Web Services start On with all their subgroups, every other group Off.

# Onboarding

- **Window:** while Chord has no project the window shows no sidebar, no sidebar toggle and no status bar; the traffic lights stay.
  Settings stays on ⌘, and the app menu.
  The content sits in a centered column, 520px at most, 28px side padding, centered in the height.
  A folder dropped on the window adds it, as always.
- **Welcome:** three steps take the column in turn, once: after the first project the window never shows them again.
  A track of three 3px bars sits 28px above each step: 20px each, 6px apart, the current one 28px in `ink-secondary`, the ones before it `ink-muted`, the ones after it `line-strong`, changing over 200ms.
  A step change slides the new step in from 16px to the side it comes from, from 2px blur and transparent, over 280ms with `--ease-out`; the first step shows still.
  Each step's primary button takes the focus, so Return moves on.
- **Step 1, Welcome:** "Welcome to Chord" at 22px/600, -0.015em, then one `ink-muted` line.
  Under it, a 340px `panel` card with a `line` border and the Card radius that draws a sidebar: the project name "harbor" in the sidebar's project style and three worktree cards, each a title at 500 in `ink`, the branch at 12px in the code font, and a session row with its `StatusMark`, "Session 1" and the status word at 11px/500 in its status color.
  Every 1.8s the statuses step through a fixed loop, and the open card (`sheet` with a 1px `line` ring) moves to the agent that waits.
  A changed status word fades in and rises 4px over 200ms.
  Under Reduce Motion the loop stays on its first beat.
  Then the primary "Get Started".
- **Step 2, Agent:** the title says what the check found: "pi is ready" with one ready agent, "Pick your agent" when several take the pick, "Install an agent" with none, "Your agent" while the first check runs.
  Then the harness rows, a line under them in meta text with Check Again at its right, and a footer: a plain "Back" on the left (28px, no fill, `ink-secondary` at 500; `raised` and `ink` on hover, `selected` pressed), and on the right the primary "Continue" while a default exists or no check has ended, else a secondary "Skip for Now".
- **Step 3, Project:** "Add your first project", then a `panel` card with a 1.5px dashed `line-strong` border and the Card radius: a 28px `ink-muted` folder icon, the primary "Add Project" (tooltip with ⌘O) and "or drop a folder anywhere on the window" in meta text, 12px apart.
  The footer holds Back and, on the right, "New sessions run **pi**" in meta text with the name at 500 in `ink-secondary`, or "No agent yet. Chord asks at the first session." A project added while no worktree is open, here or anywhere, opens its first worktree.
- **Empty state:** after the first project, a window whose projects were all removed shows the plain empty state: a folder icon, "No projects yet", "Add a git repository or any folder, or drop it on the window." and the primary "Add Project".
  No harness rows.
- **The check:** the welcome checks every harness and git when it opens, so the agent step shows results, and again each time the window comes forward.
  One check runs at a time.
  The rows keep the last result while a check runs; only Check Again shows the spinner in its place.
  Before the first result one row reads "Agents", "Checking", with a 14px `ink-muted` spinner.
- **Check Again:** a compact secondary button: 24px tall, 8px side padding, the Inset radius, 12px/500.
- **Harness rows:** the welcome, Settings › Agents and a chat pane whose harness cannot start draw the same rows.
  A `panel` card with a 1px `line` border and the Card radius, one 44px row per harness Chord can start, in the list's order, a 1px `line` rule between rows.
  A row is the harness's 16px logo, 12px after it the name in body `ink` and under it the state in meta text, tabular: "Version 1.5.2", "Not installed", "Installed, not signed in" or "Version 1.4.0, Chord needs 1.5.0".
  The state fades in and rises 4px over 200ms when a check changes it, and stays still on a redraw.
  At the right, the default's 16px `ink-muted` check, which draws its stroke over 320ms when it lands on a row; or the command the row needs: the command in 12px code text in `ink-secondary` on a 28px `sheet` chip with a 1px `line` inner border and the Control radius, selectable, its full text as the tooltip, fading out over its last 16px when the row cuts it, and a 24px copy icon button that shows a check for 1.5s.
  A ready or signed-out row that is not the default is a radio: `raised` on hover, `selected` pressed, over 120ms, focus ring inside, tooltip "Use for New Sessions"; a click, Return or Space makes it the default.
  Missing git adds a second card 12px under it with one row: the branch icon, "Git", "Not installed, needed for worktrees" and its install command.
- Under Reduce Motion the step slide, the state lines, the status words and the chips only fade, the check shows drawn, and the spinner runs at a third of its speed.

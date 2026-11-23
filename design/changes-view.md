# Changes view

- **Branch line:** the view's first line, over the scope control, 8px in: the branch button (tooltip "Switch Branch"), a 28px button with the 16px branch icon and a 12px chevron in `ink-muted`, 6px apart around the worktree's branch at 13px/500 `ink-secondary` ("Detached HEAD" without one), which truncates in a narrow panel; 8px padding, Control radius, no fill, `raised` with `ink` on hover over 120ms with `--ease-out`, `selected` with `ink` when pressed and while its menu is open, pressing it scales it to 97%.
  While any agent in the worktree runs it takes `ink-disabled`, no click and the tooltip "Switch branches after the agent stops", and an open branch menu closes.
  At the line's right end, 4px after the branch button, which truncates to leave them their room, three icon buttons 2px apart while a push has somewhere to go, so not on a detached HEAD or in a repository without a remote: Fetch (the reload icon), Pull (the down arrow) and Push (the up arrow).
  Pull and Push show the commits waiting 2px after their arrow, at 11px/500, tabular, as a card's git counts, with 6px side padding; a count of 0 shows no number.
  The tooltips read "Fetch (fetched 2m ago)" ("Fetch" before the worktree's first fetch), "Pull 3 Commits" ("Pull" with none) and "Push 2 Commits" ("Push to origin/feat" before the upstream exists, "Push" with none).
  Pull is disabled on a branch without an upstream, Push while its upstream has every commit.
  A click runs at once and takes no second click until it ends: the Fetch icon turns once every 800ms (once every 2.4s under Reduce Motion), and Pull and Push show the loading spinner in `ink-secondary` in their arrow's place, Push also while a Commit & Push pushes.
  Each ends in a notice toast: "Fetched origin", "Pulled from origin/main" or "Already up to date", "Pushed 2 commits to origin/main" ("Pushed to origin/feat" the first time).
  Pull joins branches that diverged the way the user's git config says, and by a merge commit while it says nothing.
  A pull git refuses shows its reason in a failure toast.
  A click opens the branch menu under it, 2px down and 8px in from both panel edges: the picker menu's card (see `design/chat.md` › Picker menu), fading in and dropping 4px from 96% scale over 200ms with `--ease-out` (a fade under Reduce Motion).
  It lists the worktree's local branches, newest commit first, each a commit mode row: a 14px check column marking the current branch, then the name in `ink`, which truncates, over a 12px `ink-muted` line, "Last commit 3h ago" ("Last commit just now" in the first minute), or "In harbor-flaky · 5h" for a branch another worktree holds, which shows in `ink-disabled` and takes no highlight or pick; 6px 8px 6px 6px padding, Inset radius, the highlighted row `selected` with its line in `ink-secondary`.
  Up to four branches it moves the highlight by typing, as the model picker; from five it starts with a Field (placeholder "Filter", 30px tall) that takes the focus and narrows the rows, reading "No matches" when none is left.
  ↑ and ↓ or the pointer move the highlight, Return or a click picks; Esc, a click outside the card or a second click on the button closes it.
  A pick switches the worktree to the branch; git's refusal shows in a failure toast.
  Right-click on a branch row opens Rename Branch and, for a branch no worktree or the main checkout holds, Delete Branch; either one closes the branch menu.
  A branch row ends with a 20px More icon button (three dots, Inset radius, tooltip "More") that shows while the row is highlighted, under the pointer or holds the focus, fading in over 120ms with `--ease-out`; `raised` on hover, `control-selected` on the highlighted row.
  It turns the card into that branch's actions: a 30px back row with a 16px `ink-muted` left arrow 8px before the branch at 500 in `ink`, which truncates (tooltip "Back (←)", Inset radius, `raised` on hover, `selected` pressed), then, 2px apart, commit mode rows without the check column: Switch, "Merge into main", Rename Branch and, in `danger`, Delete Branch.
  Switch and Delete Branch are disabled in `ink-disabled` on the current branch and on one another worktree holds, which Switch names in its hint ("In harbor-flaky"); Merge is disabled on the current branch.
  The back row takes the focus; ↑ and ↓ move between the rows, ← or the back row brings the list back, and a pick closes the menu.
  Merge merges the branch into the worktree's own at once, then shows a notice toast ("Merged fix/a into main", or "Already up to date with fix/a").
  A merge that stops on conflicts shows git's report in a failure toast and leaves the conflicted files in the Changes group.
  Under the branch rows, 8px down and past a 1px `line` rule across the card, a New Branch row with the back row's look and a 16px `ink-muted` plus, in `ink-secondary` (`ink` on hover).
  New Branch opens a sheet with a Branch name field and a primary Create button; the field hint reads "Starts from main", then the typed name or, in `danger`, the live check's or git's reason, which disables Create and keeps the sheet open.
  Create makes the branch at HEAD and switches the worktree to it, changes and all.
  Rename Branch opens a sheet with a Branch name field, the branch selected in it, and a primary Rename button; the field hint shows the typed name or, in `danger`, the live check's or git's reason, which disables Rename and keeps the sheet open.
  Delete Branch asks with a confirm sheet ("Delete Branch?", the destructive button "Delete"); when git calls the branch unmerged, a second one asks again ("Delete Unmerged Branch?") before it deletes with force.
- **Scope:** while the worktree has a base, a header over the list, 4px 8px 0 padding: a Segmented control as wide as the panel with Uncommitted and Branch, each segment half of it, its label truncating in a narrow panel.
  Under Branch, 4px under the control, a 28px row 8px in: "Compared to" in 12px `ink-muted`, which never wraps, then the base as a 24px button (12px/500 `ink-secondary`, 6px padding, Inset radius, `raised` and `ink` on hover over 120ms with `--ease-out`, `selected` pressed, tooltip "Base Branch") with a 12px `ink-muted` down chevron 4px after the name, which truncates.
  It opens a native menu under itself, as the run menu: the default base, then under disabled "Local" and "Remote" lines, each after a separator, the other branches; the base in use carries the check mark.
  Branch has no checkboxes, More buttons or commit area.
- **Branch:** two groups with the Staged and Changes headings' look, without checkboxes, their names 8px in: Commits, then Files, each with its count; empty they read "No commits" and "No changes".
  A commit row is a Files tree row: its 12px chevron 8px in (turns like a folder's), 4px after it the subject in `ink-secondary`, which truncates, then the short hash at 11px in the code font and the age ("3h", as a session row's) in 12px, both `ink-muted`; its tooltip is the subject.
  A click, → and ← open and close it to the files it changed against its first parent, 24px in, on the subject's edge.
  A file row is the Changes row without its checkbox and More button, 8px in under Files; a click opens its diff in the preview tab, for the branch since the merge base or for its commit, read-only without hunk buttons; the tab's tooltip adds "(Compared to main)" or "(Commit a41c9e2)".
  ↑ and ↓ move the focus between rows and open a file's diff, as under Uncommitted.
- **Commit:** under the groups: a 1px `line` top edge and 12px padding.
  The message card, as the composer in small: `sheet` fill, a 1px `line-strong` border, Control radius.
  Inside it the message field with no border of its own, 8px 10px 4px padding, Body line height, placeholder "Message", three lines tall empty and growing with the text to eight, then scrolling; Return adds a line and ⌘Return commits; its tooltip is the author git records.
  The card's foot, 4px padding: the first line's length as "34/50" at 11px tabular `ink-muted`, 6px in, `git-changed` past 50, empty while the field is; then at the right end Commit, a primary button at 24px with the Inset radius, tooltip "Commit (⌘Return)", enabled while the field holds text and a file is staged or the mode amends.
  Its label names the mode: "Commit", "Commit & Push", "Amend" or "Amend & Push".
  Joined to its right edge by a 1px `accent-pressed` line, a 22px chevron (tooltip "Commit Mode", turns 180° over 120ms while open) opens the mode menu above it, right-aligned, 6px up: `sheet`, 1px `line` border, Control radius, the popover shadow, 4px padding, 248px wide, fading in and rising 4px from 96% scale over 200ms with `--ease-out` (a fade under Reduce Motion).
  Each mode is a row with a 14px check column, its label in `ink` and under it a 12px `ink-muted` hint that truncates: "Record the staged files", "Then push to origin/main", "Rewrite the last commit", "Rewrite it, then push to origin/main"; `raised` on hover.
  The Amend modes show while HEAD has a commit, the Push modes while the branch has a remote; with one mode left, no chevron shows.
  Opening focuses the checked mode; ↑ and ↓ move between modes, Esc and a press outside close it, a pick closes it.
  While Commit cannot run, the chevron takes `raised` and `ink-secondary` and still opens.
  The box opens in Commit or Commit & Push, whichever the last commit without Amend ran, saved in `state.json`; an Amend mode never carries over.
  Picking an Amend mode fills an empty field with the last commit's message, and leaving Amend empties the field while that message stands unchanged.
  A running commit shows the loading spinner in Commit, which takes no click until the commit and its push end.
  A commit empties the field and puts the box back in its saved mode; in a Push mode, the push follows it.
  Both read the view and the cards' git counts again when they end.
- **Groups:** the worktree's changed files from `git status`, under Staged (the index against HEAD), then Changes (the file on disk against the index); both groups always show, so an empty one reads its 0 count; under its heading it reads "No staged files" or "No changes" in meta text on the rows' name edge, its checkbox is disabled (a `line` border, no fill).
  A group's heading is a 28px band across the list's full width: `raised` laid over `canvas`, a 1px `line` hairline along its top and bottom edges, 16px left and 12px right padding, its name at 12px/600 `ink`, then 6px after it the file count at 500 in `ink-muted`, tabular.
  The group's rows start 4px under it.
  The heading starts with its checkbox, 8px in, the name 8px after it on the rows' name edge: checked on Staged, where a click unstages every file (tooltip "Unstage All"), empty on Changes, where it stages every file ("Stage All").
  Groups sit 8px apart.
  A heading sticks to the list's top on `canvas` while its group's rows scroll under it, until the next heading pushes it out, as in VS Code.
  The list scrolls inside with the Files tree's padding, its hover and its focus ring inside the row; ↑ and ↓ move the focus between rows and open each one's diff as a click does, across both groups, Return too, and the selected row scrolls into view.
- **Checkbox:** 14px, Check radius, a 1px `line-strong` border on `sheet`, `ink-muted` border on hover; checked takes an `ink` fill and border with a 12px check in `canvas`, neutral, since a blue box on every row would break the One Blue Rule.
  Fill and border change over 120ms with `--ease-out`.
- **Row:** a Files tree row in path order: its checkbox 8px in, checked under Staged and empty under Changes, where a click stages or unstages the file and Space does the same on the focused row, then moves the focus on and opens the diff of the next row in the group (the one before when it was last, the file itself in its new group when the group is left empty); the checkbox takes no Tab stop of its own.
  Then, 30px in, the file name in `ink-secondary` (struck through for a deleted file; it truncates past 70% of the row), 6px after it the folder in 12px `ink-muted` (`ink-secondary` on the selected row), which truncates, then at the right end the lines added ("+12", `success`) and removed ("−3", `danger`) at 11px/500, tabular, each only when not zero, and the status letter at 11px/600 in a 12px column: M modified and R renamed in `git-changed`, A added and U untracked in `success`, D deleted and ! conflicted in `danger`; its tooltip is the status word.
  The row's tooltip is its path, a rename's as "old → new".
  The row whose diff the focused tab shows takes `selected` and `ink`.
  Right of the status letter, 4px from the row's end, a 20px More icon button (three dots, Inset radius, tooltip "More") shows while the pointer or the focus is on the row and takes no room when hidden, so the status letter ends 8px from the row's end and moves in to make room for it; `selected` on hover, as a tab's close button.
  It opens the row's menu, as right-click does: Stage and Discard under Changes, Unstage and Discard under Staged; a conflicted row, or a pick that holds one, has no Discard, since git cannot restore it.
- **Pick:** a shift-click, or ↑ and ↓ with shift, picks the rows from the last plainly clicked or arrowed row to this one, within its group; a shift-click opens no diff.
  The picked rows take `selected` and `ink`, in place of the row whose diff the focused tab shows.
  A picked row's checkbox, Space, More button and right-click act on every picked row ("Stage 3 files" as the checkbox's label).
  A plain click or arrow picks one row again, Esc drops the pick, and a row that moves to the other group or goes leaves it.
- **Discard:** Discard and a hunk's Discard ask with a confirm sheet first ("Discard Changes?" for one file, "Discard 3 Files?" for more, "Discard Hunk?"), naming the file and its path; then Chord restores the file or the hunk from the index, a staged file or hunk from the last commit, and moves a file git does not track or a staged new file to the Trash (the button reads "Move to Trash").
- **Stashes:** at the Changes heading's right end, a 20px Stash Changes icon button (a box, Inset radius, tooltip "Stash Changes") stashes the staged, unstaged and untracked changes, then shows a notice toast ("Stashed 3 files", a file both staged and changed counting once); it is disabled while both groups are empty.
  Under Changes, while the repository has stashes, a Stashes group: its heading looks like the others, with a 12px `ink-muted` chevron in the checkbox's place, so the name sits on the same edge, and the whole band is a button (`selected` over `canvas` on hover, focus ring inside) that opens and closes the group, as → and ← do; the group starts closed and holds its state per worktree until the app quits, so 30 stashes cost one row.
  Open, it lists the stashes newest first, each a Branch commit row with the message in place of the subject, then the branch it came from in 12px `ink-muted` (40% of the row at most, truncating), then the age; git's "WIP on main:" leaves the message.
  A stash opens like a commit to the files it holds, the untracked ones with U; a click opens the file's diff read-only, its tab's tooltip adding "(Stash a41c9e2)".
  Right-click on a stash: Apply, Pop, a separator, Drop.
  Drop asks with a confirm sheet ("Drop Stash?", the message as its detail, the destructive button "Drop").
  A failed or conflicting Apply or Pop shows git's message in a failure toast and leaves the stash in the list.
- **Reads:** git is read when the view shows and with each git counts refresh while it shows, so on window focus and every 10 seconds.
  A failed read shows git's message in a failure toast and keeps the list.

# Chord: features

One line per feature. A ticked box means the feature works. Section 15 lists what Chord leaves out.

Rule: build nothing before a feature needs it. A feature brings its own buttons, panel views, status bar items, menu items, shortcuts, card details and Settings rows. Each line below lists those with the feature that owns them.

Tags:

- **core**: needs no harness and no Xcode.
- **harness(x)**: needs capability `x` from the harness adapter. Section 14 lists the capabilities.
- **hard**: the riskiest tickets; give them the strongest agent.
- **iOS**: goes into the `chord-ios` crate; its UI shows only in a worktree with an Xcode project.

"Edge cases" lines are details a feature must keep.

## 1. Window and navigation (core)

- [x] A1 Sidebar: one flat list of worktree cards across projects (project, title, branch), show/hide remembered.
- [x] A2 Worktree card: branch name, ⌘1-9 hint. Other features add to it: git counts (B6), status (D2).
- [x] A3 One commands table drives shortcuts, menu accelerators and tooltips.
- [x] A5 Light / dark / system theme; text size (⌘=, ⌘-, ⌘0). Settings rows for both.
- [x] A6 Restores the open project and worktree on launch.
- [x] A7 Settings row: editor font size, for the editor and diffs.
- [x] A8 Navigator at the top of the sidebar, as in Xcode: Workspaces (⌃1), Files (⌃2, ⌘⇧E) and Changes (⌃3, ⌘⇧G). Files and Changes move out of the side panel. Workspaces carries a dot while an agent awaits input.

Containers. The first feature that puts something in one builds it:

| Container | What it is | Features that fill it |
|-----------|------------|-----------------------|
| Tab bar | One strip over each tab group; the ones along the top are the window drag region; shows the project name or "Settings" as a title when it has no tabs | Tabs: C3, F2, M1. Buttons: F1, H1 (its menu holds H2 and L4), I1, M1 |
| Side panel | One panel right of the panes, one view at a time; resizable; width, open state and shown view remembered per worktree | F1 Files, I1 Simulator, J1 Preview, M1 Changes |
| Bottom panel | Resizable, the terminal with the run's view in a column beside it; open state and shown view remembered per worktree | G1 terminal, H3 build log and console |
| Status bar | Strip at the bottom of the window | E11 usage, H5 running simulators |
| Settings panel (⌘,) | A panel over the blurred window: a section list (General, Appearance, Shortcuts) and grouped rows | A5, A7, D4, D5, G1, H5, H11, K1 |

Edge cases:
- The window opens centered on the screen under the cursor.
- Fullscreen drops the traffic-light gap.
- Panels never slide off screen when focus reaches a clipped child (`overflow: clip`).
- The app's webview never navigates away; a web link opens in a browser tab, or in the default browser when Settings says so (C7).
- Dividers have wider hit areas than they look.
- Selectable text shows the I-beam; buttons inside it keep the arrow.
- Devtools and the browser context menu stay off in release builds.

## 2. Projects and worktrees (core)

- [x] B1 Add a project (dialog or folder drop), remove it, switch between projects.
- [x] B2 New worktree popup (⌘N): branch name with the live `git check-ref-format` check, project picker; create opens the worktree.
- [x] B3 Delete worktree: the card goes at once, git and cleanup run in the background; force prompt; a merged branch goes with it, and Chord says when it keeps one.
- [x] B5 A new worktree gets a copy of the main checkout's gitignored files (up to 5 MB each, no ignored folders).
- [x] B6 Git counts on the worktree card: ↑ahead and ↓behind against the branch's upstream, or against the main checkout's branch without one (nothing on a detached HEAD), and changed files from `git status --porcelain`; zeros show nothing. Refreshes on window focus, then every 10 seconds while the window is visible, only for worktrees of expanded projects and the selected one; a refresh still running skips the next. No file watcher.
- [x] B7 Extends B5: a new worktree also gets the main checkout's gitignored folders (`node_modules`, `Pods`, `.build`) and files of any size as APFS clones (`clonefile`), which take no extra disk space until one side changes.
- [x] B8 Rename a worktree: Rename in the card's right-click menu, or a double click on the branch name, turns the name into a field with the live `git check-ref-format` check of B2; Return renames the branch (`git branch -m`), Esc cancels. The folder keeps its name.
- [x] B9 Drag a project's header to reorder the projects, and a worktree card to reorder the cards within its project. The order is saved through Rust; a new worktree goes to the end of its project.

Edge cases:
- Worktree folder names never collide across projects.
- Deleting the open worktree opens the one below it, or above if it was last.
- A gitignored file that fails to copy does not strand the new worktree.
- On a volume without clone support, or when the worktree sits on another volume than the main checkout, B7 skips ignored folders and keeps B5's 5 MB file cap.
- B7 never clones DerivedData.
- Worktrees deleted outside Chord lose their caches on app start and window focus.
- Counts work in reftable repos; a branch without an upstream counts against the main checkout's current branch.
- An unchanged worktree list skips the git refresh.
- Project switch shows cached worktrees at once.
- `await` the confirm dialog before deleting.
- B8: a name another branch holds, or a detached HEAD, keeps the field open with the reason; the main checkout's card has no Rename.
- B8: a rename while the agent runs leaves the agent and its session running.
- B9: a card never drags into another project.

## 3. Panes and tabs (core, needs E1)

- [x] C1 Split right (⌘D), split down (⌘⇧D): a new tab group with a new session beside the focused one; close tab (⌘⇧W); every chat tab runs its own agent in the worktree.
- [x] C2 Drag dividers to resize, double-click evens out, sizes persist.
- [x] C3 Tab groups: each pane of the split is a group with its own tab bar over it, one tab per chat session or file; a + after a group's tabs opens a new session there (⌘⇧T in the focused group).
- [x] C4 Drag a tab onto another group's tab bar to move it there, or onto a pane edge to open a new group there.
- [x] C5 harness(sessions): agents that ran at quit come back on launch in their exact session.
- [x] C6 Quit asks while agents run.
- [x] C7 Browser tabs: a web link from a chat, the terminal or the app's own webview opens as a tab of the focused group with back, forward, reload, an address field and Open in Default Browser; the page keeps its history while hidden and its address and title persist. Settings › General › Links sends links to the default browser instead.

Edge cases:
- Holding ⌘D splits once.
- A stale exit event does not kill a restarted pane.
- Unsent composer text survives splits and worktree switches.
- A divider drag updates the layout once per frame and saves once.

## 4. Agent status and notifications (harness(status))

- [x] D1 Statuses: running, waiting for input (a dialog is open, or the last message ends with a question), done, error, exited with code. Rust decides each one.
- [x] D2 Under its worktree card, each chat session has a sidebar row with its name, status, last message and time; a click opens its tab. A tab its group does not show carries its status dot.
- [x] D3 A finished pane shows a marker until you open it; its session row too.
- [x] D4 macOS notification for done and waiting, naming project and branch; a click opens the worktree and focuses the pane. Settings row: notifications on or off.
- [x] D5 Settings row: keep the Mac awake while an agent runs (`caffeinate`).
- [x] D6 Next Agent Needing You (⌘J) opens the next agent in the attention queue: Awaiting input first, then a Failed, an Exited or a Done you have not opened, the longest wait first within each. It opens the one after the focused pane and wraps; with none it says "No agent needs you", or "No other agent needs you" when the focused pane is the only one. An attention dock over the sidebar's foot, in every navigator view, folds the running agents into a Working row and the queue's others into a Needs you row, each with its count, and shows a card for the agent ⌘J opens; the dock hides while no agent runs or needs you.

Edge cases:
- A failure in status handling never breaks the agent.
- Opening a worktree clears its finished markers.
- D6: opening a Failed, Exited or Done agent takes it out of the queue; an agent that awaits input stays until it runs again.
- A running card shows "Working" in its first minute.

## 5. Chat (harness(chat) and others)

- [x] E1 harness(chat): streaming replies, composer, steer while running, Esc stops. A new worktree starts a chat; an empty pane shows a Start button.
- [x] E2 harness(chat): tool calls and thinking folded into step groups; a group stays folded to its count while it runs; read-only shell commands show as Read, Search or List.
- [x] E3 core: Markdown (lists, tasks, headings, quotes, tables, links), code blocks with a copy button, ⌘F in the transcript.
- [x] E4 harness(attachments): attach button, dropped files, screenshots; sent images as thumbnails with a lightbox (← → steps through).
- [x] E5 harness(tool-images): images a tool returns show in one thumbnail row below its step group, folded or open, and open in the E4 lightbox.
- [x] E6 harness(dialogs): extension dialogs (select, confirm, input) over the composer; a select marks the current option and shows its filter row.
- [x] E7 harness(commands): slash menu with the typed part marked; a bubble shows `/skill:name`, not the expanded skill.
- [x] E8 **hard** harness(sessions): resume, fork, clone, tree, export, new; `/new` mid-run kills the busy agent and starts a new one at once; the session's name on its tab; history on restart.
- [x] E16 harness(sessions): `/reload` reloads extensions, skills, prompts and context files in the running agent, without a restart; the slash menu reads the commands again.
- [x] E19 harness(sessions): `/compact` in the slash menu summarizes the session to free the context; text after it says what the summary keeps. The chat shows Working while it runs, Esc stops it, and the transcript then holds what the agent still sees: a folded row with the summary, then the newest messages.
- [x] E9 harness(models): model picker with search; thinking level picker.
- [x] E10 harness(context): context meter with a limit warning; `/new` resets it.
- [x] E11 harness(usage): subscription usage (5h, weekly) in the status bar as a bar, the percentage and the time left to the reset; stale windows hidden. The windows come from the pi-powerbar extension; without it the bar shows no usage.
- [x] E15 harness(chat): a slow (5 s or more) or failed call shows its run time ("55.4 s"); a step group shows its total. Live runs only; pi saves no run time in the session.
- [x] E13 harness(subagents): background subagent runs as a lineage line in the transcript, where the first run showed up. The pi adapter reads the snapshot a pi subagent extension writes. Without such an extension the line never shows.
- [x] E17 harness(chat): hovering a message shows a copy button and the time it was sent under it; history keeps the saved times.
- [x] E18 harness(chat): a step group names what it did by kind ("2 searches, edited 2 files, ran 4 commands") and the lines its finished edits and writes added and removed ("+43 −5"); history keeps them.
- [x] E20 core: a user message longer than 12 lines folds with Show more / Show less; ⌘F unfolds it.
- [x] E14 core: branch button in the composer: local branches newest first, switch, a Filter field from five branches; branches another worktree holds stay disabled; locked while the agent runs.

Edge cases:
- Split harness output on LF only; JSON strings may hold U+2028.
- A grep with no matches is not a failure.
- Tool rows show MCP and unknown tools with a readable name and argument values.
- The chat hides harness noise lines (pi's image resize note, the MCP server count). The adapter filters them.
- Every picker shows the filter row.

## 6. Files and editor (core)

- [x] F1 Files view in the side panel (⌘⇧E), with its tab bar button: a tree of the worktree folder on disk, without `.git`; a folder reads its children when you open it; folders first, natural sort (`file2` before `file10`); a chain of single-child folders shows as one row (`src/app/views`). Header: filter field (⌘F while the focus is in the view), Collapse All, Reload. The open file's row stays selected and scrolls into view.
- [x] F2 CodeMirror editor with syntax highlighting; a file opens in a tab of the focused group; saves 1 s after you stop typing and on file switch.
- [x] F3 Saving never overwrites an agent's change on disk; a conflict stops the save.
- [x] F4 Image view, reloads on change.
- [x] F5 Right click: New File, New Folder, Rename (inline, the name without its extension selected), Copy Path, Copy Relative Path, Reveal in Finder, Move to Trash (⌘⌫) after asking.
- [x] F6 Needs C1: a file tab drags like a session tab (C4); ⌘D on a file tab puts an agent beside it.
- [x] F7 Open file by name (⌘⇧O).
- [x] F8 Keyboard in the tree: ↑ ↓ move, → opens a folder, ← closes it or jumps to the parent, Return opens the file.
- [x] F9 Drag a file from the tree into the composer to put its relative path in the draft.
- [x] F10 Preview tab: a click in the tree, ⌘⇧O and a build error open the file in the focused group's preview tab (italic name), which the next file replaces; a double click on the row or the tab, or an edit, keeps it.
- [x] F11 ⌘F searches the open file in the editor with the chat's find bar: previous and next match, match count; Esc closes it and selects the current match.
- [x] F12 Editing in the editor: brackets and quotes close themselves, several cursors (⌘-click, ⌥-drag), the cursor's line and the selection's other copies marked, code folding in a gutter, and completion from the file's words and the language's keywords.

Edge cases:
- The tree refreshes on window focus and on Reload; open folders and the selection survive a refresh.
- A file an agent deletes leaves the tree and its tab closes; a tab with unsaved edits stays open, and saving it asks first.
- CRLF files stay CRLF.
- A folded block stays folded when an agent changes the file outside it.
- Switching files mid-autosave saves to the old file.
- A draft reopens as a conflict when the disk changed; saving a deleted file asks first.
- Closing a tab with unsaved edits asks.

## 7. Terminal and logs (core)

- [x] G1 One login shell per worktree (⌘T) in the bottom panel, shown or hidden per worktree. Settings row: terminal font size.
- [x] G2 ⌘F searches the terminal or log that has focus.
- [x] G3 Drop files on the terminal to paste their paths; links open in the browser.

Edge cases:
- The shell is the user's `$SHELL` as a login shell, so PATH and API keys match their terminal.
- A new worktree's terminal opens in the new worktree.
- Only on-screen terminals hold a WebGL context.

## 8. Build and Run (iOS)

- [x] H1 **hard** ⌘R builds into the worktree's own DerivedData and runs on its own simulator; ⌘B builds only; ⌘. stops. Play / stop button in the tab bar. The worktree card shows Building, Running, Build succeeded, Build stopped or Build failed.
- [x] H2 Run destination, per project, in the menu beside the play button: connected devices above simulators.
- [x] H3 Build log and app console (stdout, stderr, OSLog) as views of the bottom panel beside the terminal, 10,000 lines, ⌘F; ⌘⇧Y shows or hides them. The status bar shows no run status.
- [x] H4 **hard** One build per worktree: ⌘R and ⌘B take over the worktree's running build.
- [x] H5 Caches: DerivedData and simulators per worktree; one forget, one prune; stop other worktrees' simulators. Running simulators in the status bar, with Shut Down Others. Settings row: the size of the build data.
- [x] H7 a missing `xcodebuild` or `xcrun` shows the install step.
- [x] H8 At most `max(1, RAM in GB / 6)` of Chord's run simulators stay booted; one more shuts down the one whose worktree was opened longest ago and has no run in progress.
- [x] H9 harness(tools): `build` builds the worktree like ⌘B, in the same slot. The agent gets no log; the Build view shows it. The tool returns the error lines as `file:line: message`, 30 at most; a failed build with no such line returns the log's last 20 lines. A build that ⌘R or ⌘B takes over returns "Stopped".
- [x] H10 harness(tools): `run` builds and runs like ⌘R on the worktree's simulator and returns "Launched", "Stopped" or what `build` returns. Both tools exist only in a worktree with an `.xcodeproj` or `.xcworkspace`.
- [x] H11 Slim simulators: before a boot, Chord writes launchd disable overrides into its own simulators, so the background services a run does not need (Siri, Spotlight, widgets, iCloud and more) never start. A Rust port of simslim's service list, with its MIT notice. Settings section Simulators: one row per service group with its name, what stops working and its memory, and a switch to keep it on.
- [x] H12 Product menu in the menu bar after View, as Xcode's: Run (⌘R), Build (⌘B), Clean Build Folder (⌘⇧K) and Stop (⌘.), each disabled while it cannot run. Clean Build Folder stops the worktree's build and run, then removes the `Build` folder of its DerivedData and its hot reload folder; the build log says Clean Finished. Take Screenshot moves to ⌘S.
- [ ] H15 **hard** Project view: opens the worktree's `.xcodeproj` as a tab, lists its targets, and shows each target's build settings per configuration and its Info.plist keys. A value edits in place and writes back to `project.pbxproj`.
- [x] H16 Toolbar in a worktree with an Xcode project, as in Xcode: Run, Stop, the scheme and run destination buttons, which open the scheme menu and the run menu, the activity field with the build's status and error count, and the Simulator and Preview buttons. The tab bars sit under it.

Edge cases:
- ⌘R opens the Simulator view in the side panel; when the private API fails to load, the selected Xcode's simulator app (DeviceHub from Xcode 27, Simulator.app before).
- The console reads output lossily; a failed launch fails the run; multi-line OSLog messages stay whole.
- A fresh simulator warms up during the build.
- The build log rewrites once per 100 KB past its cap.
- Chord's build, an agent's `xcodebuild` and Xcode share one DerivedData folder per worktree: Chord passes no `-derivedDataPath` and no build setting.
- A build waits while a build Chord did not start holds the worktree's build database, and says so in the log.
- After a stop, a takeover or a quit, no process of the build holds the build database.
- A failed build lists its errors above the log, 30 at most; an error with a path opens the file at that line.
- A build the user starts opens the build log; a build the agent started opens nothing.
- The build log and the console show beside the terminal and never open a hidden one.
- A run destination is a model and iOS version that exist together among the user's simulators.
- A simulator that does not report Booted after its boot fails the run with that reason.
- Chord never stops or deletes a simulator or a DerivedData folder it does not own.
- H11 slims only simulators Chord owns, on iOS 18.5 or newer; an older runtime boots stock.
- H11: a switch change reaches each Chord simulator at its next boot.
- H11 keeps App Store, Push & Media and Safari Sync & Web Services on by default, and never lists a service the simulator needs to boot.
- H11: every service group with several functions opens to its subgroups, each with its own name, effect, memory and On / Off switch; the group row's switch sets them all, and a service stays on while any subgroup that lists it is kept.
- H11: Chord reads the overrides back after the boot; a lost override leaves the simulator stock and the run goes on.
- H9 and H10: each tool's description tells the agent to build and run through the tools and never to use `simctl … booted`; a call that runs long reports every 30 s that it still runs.
- H4: ⌘B while the app runs builds beside the run that holds the app, as Xcode does: the app, its console and the Running marker stay (the marker says Building while it builds), ⌘. stops the app and the build, and a ⌘R takeover ends both and launches with `--terminate-running-process`.
- H15: a write keeps the file's order, quoting and untouched entries as they were; a change an agent or Xcode made on disk since the view opened stops the write, as F3 does.
- H15: a setting inherited from an xcconfig file shows read only with the file's name.
- H4: only a stop (⌘. or quit) ends the app, through the run that launched it; a takeover never runs `simctl terminate`, so a stale run never ends an app a newer ⌘R launched.

## 9. Simulator (iOS, private API)

- [x] I1 **hard** Simulator view in the side panel (⌘⇧S), with its tab bar button: live frames from the IOSurface, touch, pinch and rotate, Home, Lock, Shake, keyboard.
- [x] I2 Fallback message and "Open in Simulator / DeviceHub" when the private API fails to load.
- [x] I3 Screenshot: thumbnail at the bottom left of the simulator; drag it into Finder or any app. With harness(attachments): drag it into a pane, or click to paste it into the focused agent. Settings row: the folder screenshots and recordings go in, the Desktop by default.
- [x] I4 No frame callbacks while the window is hidden.

Edge cases:
- Input goes over DTUHID, one XPC connection to the guest's digitizer service, from CoreSimulator 1155.4 (Xcode 27) on, and over Indigo before; Home there relaunches SpringBoard. Shake posts `com.apple.UIKit.SimulatorShake` on both.
- The view follows boots and shutdowns through CoreSimulator's notifications, without polling. A boot that shows no screen restarts once; a second one shows the error.
- An agent's run neither opens the panel nor the simulator app.
- Lock reads Unlock while SpringBoard is locked; Unlock presses Home, and a second Home when the first only woke the screen.

## 10. SwiftUI previews (iOS, private API)

- [x] J1 **hard** Preview view in the side panel (⌘⇧P), with its tab bar button, on a second simulator in Chord's own device set; at most two preview simulators; a preview build waits for the worktree's ⌘R build, then follows.
- [x] J2 **hard** Hot reload of the shown preview's file and the view files between it and the saved file; any other change rebuilds, and the view's header says so while it runs. Its hot reload folders go with the worktree: one forget, one prune, counted in the build data size.
- [x] J3 Preview list beside the Preview view's screen: every `#Preview` by file as Files tree rows, with a filter; ↑↓ show the next preview at once. The pick stays per worktree and does not follow the editor. Model picker per project in the header.
- [x] J4 Compile error keeps the last frame under a red bar with the first error as `file:line: message`; a click opens the file at that line.

Edge cases:
- Rebuild on stale file lists, stored property changes and ⌘R builds; modules of 128 files and more work.
- No restart loop after saves.
- A preview switch that failed during launch is sent again.
- A third worktree's Preview view says that two previews already run and names their worktrees.
- Preview simulators show in the status bar's Simulators menu, and Shut Down Others stops them too; H8's cap counts only run simulators.
- The preview build writes no line into the H3 build log.
- A pick whose `#Preview` is gone shows the picker's empty state until the user picks again.

## 11. Settings and onboarding

- [x] K1 shortcut editor in Settings: every command's shortcut; the recorder says which command lost its key; bad saved shortcuts load as defaults.
- [x] K4 A question mark button at the right end of the sidebar footer opens a Help sheet: what a project, a worktree and a session are, what each status means, the shortcuts used most, and a button to all shortcuts.
- [ ] K2 onboarding: find the harness, show its install step when it is missing, check its version, add the first project. A missing `git` shows its install step.

## 12. Simulator tools (iOS)

- [x] L1 Appearance menu on the Simulator view: light / dark, Dynamic Type size, language and region of the worktree's simulator.
- [x] L4 Scheme and build configuration (Debug / Release), per project, in the menu of the toolbar's scheme button, which shows the scheme and the configuration; ⌘R and ⌘B use them.
- [x] L5 Screen recording: record / stop button next to the I3 screenshot button (`simctl io recordVideo`); the video gets the I3 thumbnail and drag.

Edge cases:
- L1 acts on the worktree's own simulator and is disabled while it is not booted. Light / dark and the text size apply at once; a language or region is written to the simulator's preferences and reaches an app the next time it launches (⌘R). The languages and regions are the ones the Mac's locales list.
- A window close or stop of the simulator ends a recording and keeps the file.

## 13. Source control (core)

- [x] M1 Changes view in the side panel (⌘⇧G), with its tab bar button: the worktree's changed files from `git status`, the staged ones under Staged and the others under Changes, each row with the file's name, its folder, its status (modified, added, deleted, renamed, untracked, conflicted) and the lines it added and removed. A click opens the file's diff in the focused group's preview tab (F10): a read-only unified diff with the editor's syntax highlighting, of the index against HEAD for a row under Staged and of the file on disk against the index for a row under Changes.
- [x] M2 Needs M1. Stage, unstage and discard at three levels. Group: a checkbox on its heading for Stage All on Changes and Unstage All on Staged. Row: a checkbox that stages or unstages it, and in its More button and right-click menu Stage and Discard under Changes, Unstage and Discard under Staged. Hunk of an open diff: Stage and Discard in a Changes diff, Unstage and Discard in a Staged diff. Discard asks with a confirm sheet first, then restores the file or the hunk from the index, a staged file or hunk from HEAD; an untracked or a staged new file moves to the Trash as in F5.
- [x] M3 Needs M2. Commit and push: under the groups a message field, with the author a commit records as its tooltip, and a Commit button (⌘Return in the field), enabled while the field has text and a file is staged; it commits the staged files and empties the field. Two toggles beside the button: Push After, to the upstream (or the remote and branch a first push sets), which pushes right after the commit, and Amend, which fills an empty field with the last commit's message and lets Commit replace that commit, with or without staged files. A push line with the count of unpushed commits and a Push button shows while the branch has commits its upstream lacks or has no upstream; it runs `git push` and sets the upstream on the first push. The card's git counts (B6) refresh after both.
- [x] M4 Needs M1. Scope control in the view's header: Uncommitted (M1's groups) or Branch. Branch lists the files that differ between HEAD and its merge base with the main checkout's branch, with their statuses and line counts, and over them the branch's commits since that merge base, newest first: subject, short hash and age. A commit opens like a folder to the files it changed. A click on a file opens its diff for the branch or for that commit. Under the control, Compared to names the base; its menu lists the default base, then the local and the remote branches, and a pick holds until the app quits.
- [x] M5 Needs M1. Stashes: Stash Changes on the Changes group stashes the worktree's changes with its untracked files. A Stashes group under the others lists the repository's stashes, newest first: message, the branch it came from and age; a stash opens like a folder to the files it holds, and a click on one opens its diff. Right-click: Apply, Pop, Drop; Drop asks first.

Edge cases:
- The view reads git when it shows, on window focus, after each of its own actions and on B6's refresh while it shows; an open diff tab reads again with it. No file watcher.
- A read takes no index lock (`--no-optional-locks`), so an agent's own git command never meets it.
- A git failure (an agent's index lock, a hook that rejects the commit, a rejected push, a stash that does not apply) shows git's own message in a failure toast and leaves the view as it was.
- An untracked file counts every line as added; a binary file shows no counts and its diff tab says "Binary file".
- A renamed file is one row with the old and the new name.
- A file with staged and unstaged changes has a row in both groups.
- A diff tab whose file has no change left in its scope closes.
- M2: Stage on a conflicted file marks it resolved; its diff has no hunk buttons.
- M3: Commit and Push show a busy state and take no second click while they run.
- M3: Push never asks in a terminal (`GIT_TERMINAL_PROMPT=0`); missing credentials fail with git's reason. The remote is `origin`, else the repository's only remote; without a remote Push does not show.
- M4: on the main checkout Branch compares against the branch's upstream; without an upstream, on a detached HEAD and on a branch that shares no history with its base, the view shows Uncommitted without the control.
- M4: a picked base that is gone falls back to the default.
- M5: stashes belong to the repository, so every worktree lists the same ones.
- M5: an Apply or Pop that conflicts leaves the stash in the list.
- M6: branches belong to the repository, so every worktree lists the same ones.
- M6: a branch a worktree or the main checkout holds has no Delete; B3 deletes it with its worktree. A rename of a worktree's branch updates its card as B8 does.
- M6: a name another branch holds keeps the Rename field open with the reason.

## 14. Harness capabilities these features need

A tag with a capability is a `Capability` variant: the UI hides the feature when the adapter's list lacks it. A tag with an event shows its feature once the adapter sends that `HarnessEvent`; an adapter that never sends it hides the feature.

| Tag | Used by | Gate | pi |
|-----|---------|------|----|
| status | D1-D6 | events: `state`, `dialog`, `message_end`, `exited` | RPC records on stdout |
| chat | E1, E2 | capability `chat` | `pi --mode rpc` records on stdout, commands on stdin |
| attachments | E4, I3 | capability `attachments` | RPC |
| tool-images | E5 | event: images in `tool_end` | RPC |
| dialogs | E6 | event: `dialog` | RPC extension UI requests |
| commands | E7 | event: `commands` | RPC |
| sessions | E8, E16, E19, C5 | event: `session_info`; the session commands come in `commands` | RPC plus three commands from the extension Chord ships |
| models | E9 | capability `models` | RPC |
| context | E10 | capability `context` | RPC |
| usage | E11 | event: `usage` | a pi extension that reports it |
| subagents | E13 | capability `subagents` | a pi subagent extension |
| tools | H9, H10 | the adapter starts the agent with the tools in a worktree with an Xcode project | the extension Chord ships registers them and sends each call to Chord's socket; ACP agents will get an MCP server in `session/new` that does the same (`docs/ios.md`, 3.6) |

Steering while a run goes (E1) waits for capability `steer`.

## 15. Left out

| Feature | What it would be |
|---------|------------------|
| Profiles | Work / Home profiles, each with its own projects, model, agents and layouts |
| Source control beyond section 13 | Pull and fetch, pull requests with their checks, merge, a conflict resolver, the commit graph, a diff comment sent to the agent, the diff of an agent's last turn |
| Side chat | A read-only chat about the focused chat |
| Dictation | On-device speech to text in the composer |
| Swift check for agents | A type-check command agents can call |
| Setup script | A per-project script that runs in each new worktree |
| Command palette | Worktrees and actions with fuzzy search |
| Extension status texts | The lines pi extensions set with `setStatus` (a mode, an MCP server count), anywhere in the window; only the usage status is read |
| Status for an agent started by hand in the terminal | The terminal is a shell; chat panes report status |

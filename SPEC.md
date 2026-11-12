co# Chord: features

One line per feature. A ticked box means the feature works. Section 14 lists what Chord leaves out.

Rule: build nothing before a feature needs it. A feature brings its own buttons, panel views, status bar items, menu items, shortcuts, card details and Settings rows. Each line below lists those with the feature that owns them.

Tags:

- **core**: needs no harness and no Xcode.
- **harness(x)**: needs capability `x` from the harness adapter. Section 13 lists the capabilities.
- **hard**: the riskiest tickets; give them the strongest agent.
- **iOS**: goes into the `chord-ios` crate; its UI shows only in a worktree with an Xcode project.

"Edge cases" lines are details a feature must keep.

## 1. Window and navigation (core)

- [x] A1 Sidebar: one flat list of worktree cards across projects (project, title, branch), show/hide remembered.
- [x] A2 Worktree card: branch name, ⌘1-9 hint. Other features add to it: git counts (B6), status (D2).
- [x] A3 One commands table drives shortcuts, menu accelerators and tooltips.
- [x] A5 Light / dark / system theme; text size (⌘=, ⌘-, ⌘0). Settings rows for both.
- [x] A6 Restores the open project and worktree on launch.

Containers. The first feature that puts something in one builds it:

| Container | What it is | Features that fill it |
|-----------|------------|-----------------------|
| Tab bar | Strip above the panes; window drag region; shows the project name or "Settings" as a title when it has no tabs | Tabs: C3, F2. Buttons: F1, H1, H2, I1, L4 |
| Side panel | One panel right of the panes, one view at a time; resizable; width, open state and shown view remembered per worktree | F1 Files, I1 Simulator, J1 Preview |
| Bottom panel | Resizable; open state remembered per worktree | G1 terminal, H3 build log and console |
| Status bar | Strip at the bottom of the window | E11 usage, H3 build log toggle, H5 cache size and running simulators |
| Settings panel (⌘,) | A panel over the blurred window: a section list (General, Appearance, Shortcuts) and grouped rows | A5, D5, G1, H2, K1, L4 |

Edge cases:
- The window opens centered on the screen under the cursor.
- Fullscreen drops the traffic-light gap.
- Panels never slide off screen when focus reaches a clipped child (`overflow: clip`).
- The webview never navigates away; web links open in the default browser.
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

## 3. Panes and tabs (core, needs E1)

- [x] C1 Split right (⌘D), split down (⌘⇧D), close pane (⌘⇧W); every pane runs its own agent in the worktree.
- [x] C2 Drag dividers to resize, double-click evens out, sizes persist.
- [x] C3 Session tabs in the tab bar: one per chat session of the worktree, each its own split layout; a + after them opens a new one (⌘⇧T).
- [x] C4 Drag a session tab onto a pane edge to split there.
- [x] C5 harness(sessions): agents that ran at quit come back on launch in their exact session.
- [x] C6 Quit asks while agents run.

Edge cases:
- Holding ⌘D splits once.
- A stale exit event does not kill a restarted pane.
- Unsent composer text survives splits and worktree switches.
- A divider drag updates the layout once per frame and saves once.

## 4. Agent status and notifications (harness(status))

- [x] D1 Statuses: running, waiting for input (a dialog is open, or the last message ends with a question), done, error, exited with code. Rust decides each one.
- [x] D2 The worktree card shows status, last message and time. A session tab shows the status dot of its most urgent pane.
- [x] D3 A finished pane shows a marker until you open it; its card too.
- [x] D4 macOS notification for done and waiting, naming project and branch; a click opens the worktree and focuses the pane.
- [x] D5 Settings row: keep the Mac awake while an agent runs (`caffeinate`).

Edge cases:
- A failure in status handling never breaks the agent.
- Opening a worktree clears its finished markers.
- A running card shows "Working" in its first minute.

## 5. Chat (harness(chat) and others)

- [x] E1 harness(chat): streaming replies, composer, steer while running, Esc stops. A new worktree starts a chat; an empty pane shows a Start button.
- [x] E2 harness(chat): tool calls and thinking folded into step groups; a running group keeps its last thought and last three calls open; read-only shell commands show as Read, Search or List.
- [x] E3 core: Markdown (lists, tasks, headings, quotes, tables, links), code blocks with a copy button, ⌘F in the transcript.
- [x] E4 harness(attachments): attach button, dropped files, screenshots; sent images as thumbnails with a lightbox (← → steps through).
- [x] E5 harness(tool-images): images a tool returns show in one thumbnail row below its step group, folded or open, and open in the E4 lightbox.
- [x] E6 harness(dialogs): extension dialogs (select, confirm, input) over the composer; a select marks the current option and shows its filter row.
- [x] E7 harness(commands): slash menu with the typed part marked; a bubble shows `/skill:name`, not the expanded skill.
- [x] E8 **hard** harness(sessions): resume, fork, clone, tree, export, new; the session's name on its tab; history on restart.
- [x] E16 harness(sessions): `/reload` reloads extensions, skills, prompts and context files in the running agent, without a restart; the slash menu reads the commands again.
- [x] E9 harness(models): model picker with search; thinking level picker.
- [x] E10 harness(context): context meter with a limit warning; `/new` resets it.
- [x] E11 harness(usage): subscription usage (5h, weekly) in the status bar, with the reset as a clock time; stale windows hidden. The windows come from the pi-powerbar extension; without it the bar never shows.
- [x] E15 harness(chat): a slow (5 s or more) or failed call shows its run time ("55.4 s"); a step group shows its total. Live runs only; pi saves no run time in the session.
- [x] E12 harness(status-line): extension status texts in the composer bar.
- [x] E13 harness(widgets): background subagent runs as a lineage line at the end of the transcript. The pi adapter reads the snapshot a pi subagent extension writes. Without such an extension the line never shows.
- [ ] E14 core: branch button in the composer: local branches newest first, switch, New Branch; branches another worktree holds stay disabled; locked while the agent runs.

Edge cases:
- Split harness output on LF only; JSON strings may hold U+2028.
- A grep with no matches is not a failure.
- Tool rows show MCP and unknown tools with a readable name and argument values.
- The chat hides harness noise lines (pi's image resize note, the MCP server count). The adapter filters them.
- Every picker shows the filter row.

## 6. Files and editor (core)

- [ ] F1 Files view in the side panel (⌘⇧E), with its tab bar button: file tree from git's file list (tracked and untracked, minus ignored); search; Collapse All, Reload.
- [ ] F2 CodeMirror editor with syntax highlighting; a file opens in a file tab of the tab bar; saves 1 s after you stop typing and on file switch.
- [ ] F3 Saving never overwrites an agent's change on disk; a conflict stops the save.
- [ ] F4 Image view, reloads on change.
- [ ] F5 Right click: reveal in Finder, move to Trash after asking.
- [ ] F6 Needs C1: drag a file tab onto a pane edge to show the file beside a chat; ⌘D on a file tab puts an agent beside it.
- [ ] F7 Open file by name (⌘⇧O).

Edge cases:
- CRLF files stay CRLF.
- Switching files mid-autosave saves to the old file.
- A draft reopens as a conflict when the disk changed; saving a deleted file asks first.
- Closing a tab with unsaved edits asks.

## 7. Terminal and logs (core)

- [ ] G1 One login shell per worktree (⌘T) in the bottom panel, shown or hidden per worktree. Settings row: terminal font size.
- [ ] G2 ⌘F searches the terminal or log that has focus.
- [ ] G3 Drop files on the terminal to paste their paths; links open in the browser.

Edge cases:
- The shell is the user's `$SHELL` as a login shell, so PATH and API keys match their terminal.
- A new worktree's terminal opens in the new worktree.
- Only on-screen terminals hold a WebGL context.

## 8. Build and Run (iOS)

- [ ] H1 **hard** ⌘R builds into the worktree's own DerivedData and runs on its own simulator; ⌘B builds only; ⌘. stops. Play / stop button in the tab bar.
- [ ] H2 Run destination picker in the tab bar, per project: connected devices above simulators. Settings row: the project's run destination.
- [ ] H3 Build log and app console (stdout, stderr, OSLog) in the bottom panel, 10,000 lines, ⌘F. Build log toggle in the status bar.
- [ ] H4 **hard** One build per worktree: ⌘R takes over; a preview build waits, then follows.
- [ ] H5 Caches: DerivedData, hot reload folders and simulators per worktree; one forget, one prune; stop other worktrees' simulators. Cache size and running simulators in the status bar.
- [ ] H7 a missing `xcodebuild` or `xcrun` shows the install step.

Edge cases:
- ⌘R opens the selected Xcode's simulator app (DeviceHub from Xcode 27, Simulator.app before).
- The console reads output lossily; a failed launch fails the run; multi-line OSLog messages stay whole.
- A fresh simulator warms up during the build.
- The build log rewrites once per 100 KB past its cap.

## 9. Simulator (iOS, private API)

- [ ] I1 **hard** Simulator view in the side panel (⌘⇧S), with its tab bar button: live frames from the IOSurface, touch, pinch and rotate, Home, Lock, Shake, keyboard.
- [ ] I2 Fallback message and "Open in Simulator / DeviceHub" when the private API fails to load.
- [ ] I3 Screenshot: thumbnail at the bottom right; drag it into Finder or any app. With harness(attachments): drag it into a pane, or click to paste it into the focused agent. Settings row, off by default: a folder to save screenshots in.
- [ ] I4 No frame callbacks while the window is hidden.

## 10. SwiftUI previews (iOS, private API)

- [ ] J1 **hard** Preview view in the side panel (⌘⇧P), on a second simulator in Chord's own device set; model picker per project; at most two preview simulators.
- [ ] J2 **hard** Hot reload of the shown preview's file and the view files between it and the saved file; any other change rebuilds.
- [ ] J3 Previews tab in the sidebar: every `#Preview` by file, search, ↑↓, search text and scroll kept per worktree.
- [ ] J4 Compile error keeps the last frame under a red bar that opens the build log.

Edge cases:
- Rebuild on stale file lists, stored property changes and ⌘R builds; modules of 128 files and more work.
- No restart loop after saves.
- A preview switch that failed during launch is sent again.

## 11. Settings and onboarding

- [x] K1 shortcut editor in Settings: every command's shortcut; the recorder says which command lost its key; bad saved shortcuts load as defaults.
- [ ] K2 onboarding: find the harness, show its install step when it is missing, check its version, add the first project. A missing `git` shows its install step.
- [ ] K3 Copy diagnostics (versions of Chord, macOS, Xcode, harness) and a log file.

## 12. Simulator tools (iOS)

- [ ] L1 Appearance menu on the Simulator view: light / dark, Dynamic Type size, language and region of the worktree's simulator.
- [ ] L2 Open URL: a field on the Simulator view sends a deep link or web URL to the worktree's simulator (`simctl openurl`); recent URLs per project.
- [ ] L3 Push notification: drop a JSON payload on the Simulator view, or pick a recent one; `simctl push` sends it to the running app's bundle id.
- [ ] L4 Scheme and build configuration picker (Debug / Release) in the tab bar, next to the run destination, per project; ⌘R and ⌘B use it. Settings row: the project's scheme and configuration.
- [ ] L5 Screen recording: record / stop button next to the I3 screenshot button (`simctl io recordVideo`); the video gets the I3 thumbnail and drag.

Edge cases:
- L1 to L3 act on the worktree's own simulator and are disabled while it is not booted.
- A payload without `Simulator Target Bundle` goes to the last built app.
- A window close or stop of the simulator ends a recording and keeps the file.

## 13. Harness capabilities these features need

A harness that lacks one hides the features tagged with it.

| Capability | Used by | pi |
|------------|---------|----------|
| status | D1-D6 | RPC records on stdout |
| chat | E1, E2 | `pi --mode rpc` records on stdout, commands on stdin |
| attachments, tool-images | E4, E5, I3 | RPC |
| dialogs | E6 | RPC extension UI requests |
| commands | E7 | RPC |
| sessions | E8, E16, C5 | RPC plus three commands from the extension Chord ships |
| models | E9 | RPC |
| context, usage, status-line | E10-E12 | RPC; usage comes from a pi extension that reports it |
| widgets | E13 | a pi subagent extension |

## 14. Left out

| Feature | What it would be |
|---------|------------------|
| Profiles | Work / Home profiles, each with its own projects, model, agents and layouts |
| Source control | Changes view with diffs and staging, commit, sync, stash, pull requests, merge |
| Side chat | A read-only chat about the focused chat |
| Dictation | On-device speech to text in the composer |
| Swift check for agents | A type-check command agents can call |
| Setup script | A per-project script that runs in each new worktree |
| Next waiting agent | A shortcut that jumps to the next pane waiting for input |
| Command palette | Worktrees and actions with fuzzy search |
| Status for an agent started by hand in the terminal | The terminal is a shell; chat panes report status |

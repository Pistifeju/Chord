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
| Tab bar | One strip over each tab group; the ones along the top are the window drag region; shows the project name or "Settings" as a title when it has no tabs | Tabs: C3, F2. Buttons: F1, H1 (its menu holds H2 and L4), I1 |
| Side panel | One panel right of the panes, one view at a time; resizable; width, open state and shown view remembered per worktree | F1 Files, I1 Simulator, J1 Preview |
| Bottom panel | Resizable, the terminal with the run's view in a column beside it; open state and shown view remembered per worktree | G1 terminal, H3 build log and console |
| Status bar | Strip at the bottom of the window | E11 usage, H5 running simulators |
| Settings panel (⌘,) | A panel over the blurred window: a section list (General, Appearance, Shortcuts) and grouped rows | A5, D5, G1, H5, H11, K1 |

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

- [x] C1 Split right (⌘D), split down (⌘⇧D): a new tab group with a new session beside the focused one; close tab (⌘⇧W); every chat tab runs its own agent in the worktree.
- [x] C2 Drag dividers to resize, double-click evens out, sizes persist.
- [x] C3 Tab groups: each pane of the split is a group with its own tab bar over it, one tab per chat session or file; a + after a group's tabs opens a new session there (⌘⇧T in the focused group). A saved layout from before tab groups keeps its open session's split and adds the other sessions as tabs of the focused group.
- [x] C4 Drag a tab onto another group's tab bar to move it there, or onto a pane edge to open a new group there.
- [x] C5 harness(sessions): agents that ran at quit come back on launch in their exact session.
- [x] C6 Quit asks while agents run.

Edge cases:
- Holding ⌘D splits once.
- A stale exit event does not kill a restarted pane.
- Unsent composer text survives splits and worktree switches.
- A divider drag updates the layout once per frame and saves once.

## 4. Agent status and notifications (harness(status))

- [x] D1 Statuses: running, waiting for input (a dialog is open, or the last message ends with a question), done, error, exited with code. Rust decides each one.
- [x] D2 Under its worktree card, each chat session has a sidebar row with its name, status, last message and time; a click opens its tab. A tab its group does not show carries its status dot.
- [x] D3 A finished pane shows a marker until you open it; its session row too.
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
- [x] E19 harness(sessions): `/compact` in the slash menu summarizes the session to free the context; text after it says what the summary keeps. The chat shows Working while it runs, Esc stops it, and the transcript then holds what the agent still sees: a folded row with the summary, then the newest messages.
- [x] E9 harness(models): model picker with search; thinking level picker.
- [x] E10 harness(context): context meter with a limit warning; `/new` resets it.
- [x] E11 harness(usage): subscription usage (5h, weekly) in the status bar as a bar, the percentage and the time left to the reset; stale windows hidden. The windows come from the pi-powerbar extension; without it the bar shows no usage.
- [x] E15 harness(chat): a slow (5 s or more) or failed call shows its run time ("55.4 s"); a step group shows its total. Live runs only; pi saves no run time in the session.
- [x] E13 harness(subagents): background subagent runs as a lineage line at the end of the transcript. The pi adapter reads the snapshot a pi subagent extension writes. Without such an extension the line never shows.
- [x] E17 harness(chat): hovering a message shows a copy button and the time it was sent under it; history keeps the saved times.
- [x] E18 harness(chat): a step group names what it did by kind ("2 searches, edited 2 files, ran 4 commands") and the lines its finished edits and writes added and removed ("+43 −5"); history keeps them.
- [ ] E14 core: branch button in the composer: local branches newest first, switch, New Branch; branches another worktree holds stay disabled; locked while the agent runs.

Edge cases:
- Split harness output on LF only; JSON strings may hold U+2028.
- A grep with no matches is not a failure.
- Tool rows show MCP and unknown tools with a readable name and argument values.
- The chat hides harness noise lines (pi's image resize note, the MCP server count). The adapter filters them.
- Every picker shows the filter row.

## 6. Files and editor (core)

- [x] F1 Files view in the side panel (⌘⇧E), with its tab bar button: a tree of the worktree folder on disk, without `.git`; a folder reads its children when you open it; folders first, natural sort (`file2` before `file10`); a chain of single-child folders shows as one row (`src/app/views`). Header: filter field, Collapse All, Reload. The open file's row stays selected and scrolls into view.
- [x] F2 CodeMirror editor with syntax highlighting; a file opens in a tab of the focused group; saves 1 s after you stop typing and on file switch.
- [x] F3 Saving never overwrites an agent's change on disk; a conflict stops the save.
- [x] F4 Image view, reloads on change.
- [x] F5 Right click: New File, New Folder, Rename (inline, the name without its extension selected), Copy Path, Copy Relative Path, Reveal in Finder, Move to Trash (⌘⌫) after asking.
- [x] F6 Needs C1: a file tab drags like a session tab (C4); ⌘D on a file tab puts an agent beside it.
- [x] F7 Open file by name (⌘⇧O).
- [x] F8 Keyboard in the tree: ↑ ↓ move, → opens a folder, ← closes it or jumps to the parent, Return opens the file.
- [x] F9 Drag a file from the tree into the composer to put its relative path in the draft.

Edge cases:
- The tree refreshes on window focus and on Reload; open folders and the selection survive a refresh.
- A file an agent deletes leaves the tree and its tab closes; a tab with unsaved edits stays open, and saving it asks first.
- CRLF files stay CRLF.
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

- [ ] H1 **hard** ⌘R builds into the worktree's own DerivedData and runs on its own simulator; ⌘B builds only; ⌘. stops. Play / stop button in the tab bar. The worktree card shows Building, Running, Build succeeded, Build stopped or Build failed.
- [ ] H2 Run destination, per project, in the menu beside the play button: connected devices above simulators.
- [ ] H3 Build log and app console (stdout, stderr, OSLog) as views of the bottom panel beside the terminal, 10,000 lines, ⌘F; ⌘⇧Y shows or hides them. The status bar shows no run status.
- [ ] H4 **hard** One build per worktree: ⌘R and ⌘B take over the worktree's running build.
- [ ] H5 Caches: DerivedData, hot reload folders and simulators per worktree; one forget, one prune; stop other worktrees' simulators. Running simulators in the status bar, with Shut Down Others. Settings row: the size of the build data.
- [x] H7 a missing `xcodebuild` or `xcrun` shows the install step.
- [ ] H8 At most `max(1, RAM in GB / 6)` of Chord's run simulators stay booted; one more shuts down the one whose worktree was opened longest ago and has no run in progress.
- [ ] H9 harness(tools): `build` builds the worktree like ⌘B and returns the error lines, 30 at most, and no log.
- [ ] H10 harness(tools): `run` builds and runs like ⌘R on the worktree's simulator and returns "Launched" or the error lines.
- [ ] H11 Slim simulators: before a boot, Chord writes launchd disable overrides into its own simulators, so the background services a run does not need (Siri, Spotlight, widgets, iCloud and more) never start. A Rust port of simslim's service list, with its MIT notice. Settings section Simulators: one row per service group with its name, what stops working and its memory, and a switch to keep it on.

Edge cases:
- ⌘R opens the selected Xcode's simulator app (DeviceHub from Xcode 27, Simulator.app before).
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
- H11 keeps App Store, Push & Media (L3) and Safari Sync & Web Services (L2 universal links) on by default, and never lists a service the simulator needs to boot.
- H11: Chord reads the overrides back after the boot; a lost override leaves the simulator stock and the run goes on.
- H9 and H10: each tool's description tells the agent to build and run through the tools and never to use `simctl … booted`; a call that runs long reports every 30 s that it still runs.

## 9. Simulator (iOS, private API)

- [ ] I1 **hard** Simulator view in the side panel (⌘⇧S), with its tab bar button: live frames from the IOSurface, touch, pinch and rotate, Home, Lock, Shake, keyboard.
- [ ] I2 Fallback message and "Open in Simulator / DeviceHub" when the private API fails to load.
- [ ] I3 Screenshot: thumbnail at the bottom right; drag it into Finder or any app. With harness(attachments): drag it into a pane, or click to paste it into the focused agent. Settings row, off by default: a folder to save screenshots in.
- [ ] I4 No frame callbacks while the window is hidden.
- [ ] I5 **hard** harness(tools): screenshot, tap, swipe and type on the worktree's simulator; an action returns the settled screen.

## 10. SwiftUI previews (iOS, private API)

- [ ] J1 **hard** Preview view in the side panel (⌘⇧P), on a second simulator in Chord's own device set; model picker per project; at most two preview simulators; a preview build waits for the worktree's ⌘R build, then follows.
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
- [ ] L4 Scheme and build configuration (Debug / Release), per project, as submenus of the play button's menu under the destinations; ⌘R and ⌘B use them.
- [ ] L5 Screen recording: record / stop button next to the I3 screenshot button (`simctl io recordVideo`); the video gets the I3 thumbnail and drag.

Edge cases:
- L1 to L3 act on the worktree's own simulator and are disabled while it is not booted.
- A payload without `Simulator Target Bundle` goes to the last built app.
- A window close or stop of the simulator ends a recording and keeps the file.

## 13. Harness capabilities these features need

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
| tools | H9, H10, I5 | capability `tools` | tools the extension Chord ships registers; its channel back to Chord is open (`docs/ios.md`, 3.6) |

Steering while a run goes (E1) waits for capability `steer`.

## 14. Left out

| Feature | What it would be |
|---------|------------------|
| Profiles | Work / Home profiles, each with its own projects, model, agents and layouts |
| Source control | Changes view with diffs and staging, commit, sync, stash, pull requests, merge |
| Side chat | A read-only chat about the focused chat |
| Dictation | On-device speech to text in the composer |
| Swift check for agents | A type-check command agents can call |
| Tests | ⌘U, and a test tool for agents on the worktree's simulator |
| Setup script | A per-project script that runs in each new worktree |
| Next waiting agent | A shortcut that jumps to the next pane waiting for input |
| Command palette | Worktrees and actions with fuzzy search |
| Extension status texts | The lines pi extensions set with `setStatus` (a mode, an MCP server count), anywhere in the window; only the usage status is read |
| Status for an agent started by hand in the terminal | The terminal is a shell; chat panes report status |

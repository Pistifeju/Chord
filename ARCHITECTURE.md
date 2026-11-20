# Chord: architecture

## 1. Layers

```
React views
stores outside React           one per feature, rules in a pure module
generated IPC bindings         src/ipc/bindings.ts, written by the Rust build
------------------------------ Tauri commands, events, channels
src-tauri (app crate)          thin commands, menu, window, notifications
------------------------------ plain Rust calls
chord-core                    projects, worktrees, git, state file,
                               harness trait, pi adapter
------------------------------ stdin / stdout
pi --mode rpc                  one process per pane
```

## 2. Rust

1. **`chord-core` has no `tauri` dependency.** Its functions take paths and values and return values, so `cargo test` reaches all of it.
2. **Commands are thin.** A Tauri command in `src-tauri` runs the core call on `spawn_blocking` and maps the result. No git, filesystem or process work on the main thread.
3. **One error type.** `chord_core::Error` has a `MissingTool { tool, install }` case and a message case. The UI shows the install step for the first and the text for the second.
4. **No `unwrap` outside tests.** Clippy denies it.

## 3. Harness

The trait, events, commands and capabilities: `docs/harness-contract.md`, section 3. How they sit in the code:

1. **One trait, one folder per harness.** `Sessions` holds each pane's agent as a `Box<dyn Agent>` and starts it through `harness::start`, which picks the adapter by the harness name the pane saved (`PaneAgent` in `state.json`). The session key is the adapter's own string, from `SessionInfo`.
2. **Records stop at the adapter.** `harness/pi/` turns each pi record into Chord events. The frontend's chat reducer handles every `HarnessEvent` and never sees a pi record.
3. **One path for history.** On resume the adapter reads the saved messages and sends them as the same events a live reply produces.
4. **Status without a socket.** pi's stdout carries what the status needs: `agent_start`, `agent_settled`, `message_end`, `extension_ui_request` for dialogs, and `get_state` for the session file. `status_for` in `harness/mod.rs` folds the events into a status; the app crate emits it to the sidebar and to notifications. No socket and no reporting extension.
5. **The extension Chord ships adds three commands**: `/chord-resume`, `/chord-tree`, `/chord-reload`. RPC has no session list, tree navigation or reload. It also reports the subscription usage windows as a status text. In a worktree with an Xcode project it also adds the `build` and `run` tools, which send each call to Chord's socket (`docs/ios.md`, 3.6). Codex and Claude Code get the same tools from Chord's own binary, which `main.rs` starts as the stdio MCP server in `harness/agent_tools.rs` on `agent-tools <socket> <worktree>`.
6. **Process.** One `pi --mode rpc` per pane, on pipes, started through the user's login shell so PATH and API keys match their terminal; `harness/process.rs` holds that part for every adapter. The adapter calls `installed::require` first, which runs `pi --version` and returns `MissingTool`, or for a version older than the tested one, the upgrade step the chat shows as a notice. `harness/installed.rs` holds the one version check and parser for every harness.
7. **Capabilities.** The adapter returns its list when a session starts. A view checks the list before it shows a control.
8. **Codex.** One `codex app-server` per pane, JSON-RPC lines on the same pipes. `harness/codex/reader.rs` holds the thread, the running turn and the requests waiting for an answer, behind one mutex the stdout thread and the `Agent` methods share. `items.rs` turns a thread item into events, for a live turn and for a resumed thread's turns alike. `asks.rs` turns Codex's approval, question and form requests into dialogs, one open at a time, and answers with Codex's own decision ids. `commands.rs` holds the models, the skills, the picker menus and the picks every later turn carries; `permissions.rs` names a policy pair as a mode. A start with a saved thread id resumes the thread and falls back to a new one when Codex cannot open it.
9. **Claude Code.** One `claude -p` per pane, stream-json lines on the same pipes. `harness/claude/reader.rs` holds the session, whether a turn runs, the control requests waiting for an answer and the messages waiting for their echo, behind one mutex. `blocks.rs` turns content blocks into events, for live records and transcript lines alike. `asks.rs` turns `can_use_tool` and `elicitation` into dialogs through the shared `harness/asks.rs`; `commands.rs` holds the models, the commands and the picker menus; `permissions.rs` names a mode. Claude Code replays nothing on `--resume`, so `transcript.rs` reads the session's file for the history and the worktree's other files for `/resume`, which starts a new process on the pick.

## 4. IPC

1. `tauri-specta` writes `src/ipc/bindings.ts` from the Rust command and event types. The frontend calls `commands.listWorktrees()`; no `invoke("...")` strings and no hand-written copies of Rust types.
2. Chat events go over one Tauri `Channel` per pane. Status and worktree changes go over app events.
3. `npm run check` fails when the committed bindings differ from the generated ones.

## 5. Frontend

1. **Stores outside React**, read with `useSyncExternalStore`: a pure rules module and a thin module that connects it to IPC.
2. **`App.tsx` is layout.** It places the sidebar, tab bar, panes and panels, and lists each feature's exported button, view and commands. No registry.
3. **One commands table** in `app/commands.ts`. A feature exports its entries; shortcuts, menu accelerators and tooltips read the table.
4. **A core feature never imports from `features/ios`.** iOS views render only in a worktree with an Xcode project.
5. **Markdown renders to React elements**, never to an HTML string.

## 6. State file

1. One file, `state.json`, in the app data folder. Rust reads it at launch and writes it with a rename, so a crash leaves the old file whole.
2. One Rust struct, `State`, with `#[serde(default)]` on every field. A feature adds its fields; an older file still loads. A file that does not parse stays beside the new one as `state.json.bad`. A breaking change adds a `version` field and one migration function.
3. The frontend gets the state at launch and sends changes back; Rust writes the file on each change that alters the state. Nothing goes to `localStorage`.

## 7. Security

1. **CSP on.** Scripts from the bundle only; styles from the bundle and inline, since xterm.js writes its renderer's `<style>` elements at run time (Tauri's style hashes stay off, or they would void `'unsafe-inline'`); images from the bundle, the asset protocol and `data:`; connections to Tauri IPC only.
2. **Path check in one place.** A command that takes a path gets a worktree and a relative path. `files.rs` (`inside`) refuses a relative path that is absolute or climbs out with `..`. The worktree itself is not checked against the ones Chord knows.
3. **No shell strings from the UI.** Arguments travel as process arguments or env vars.

## 8. iOS

`chord-ios` depends on `chord-core`; `chord-core` cannot name `chord-ios`, so a leak fails to compile. `src-tauri` wires both and registers the iOS commands. The worktree record carries `xcode: bool`; that flag is the only iOS fact core holds.

How worktrees stay apart (DerivedData, build slot, simulator), how ⌘R runs and the order of the iOS rows: `docs/ios.md`.

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

`chord-ios` (build, simulator, previews) joins beside `chord-core` with the first iOS feature.

## 2. Repo layout

```
Cargo.toml                     workspace: src-tauri, crates/*
crates/chord-core/src/
  projects.rs  worktrees.rs  git.rs  state.rs  paths.rs  error.rs
  harness/mod.rs               trait, Event, Command, Capability, status_for
  harness/pi/                  the only module that knows pi's records
src-tauri/src/
  lib.rs                       plugins, the command list, menu
  commands/                    one file per feature area
src/
  app/                         App.tsx (layout), commands.ts (the commands table)
  features/<name>/             view, store, pure rules
  features/ios/<name>/         with the first iOS feature
  ipc/bindings.ts              generated, committed
  ui/                          icons, shared controls, tokens.css
```

Feature folders follow the areas of `SPEC.md`: `sidebar`, `worktrees`, `panes`, `status`, `chat`, `files`, `terminal`, `settings`, `onboarding`. A folder appears with its first feature.

## 3. Rust

1. **`chord-core` has no `tauri` dependency.** Its functions take paths and values and return values, so `cargo test` reaches all of it.
2. **Commands are thin.** A Tauri command in `src-tauri` runs the core call on `spawn_blocking` and maps the result. No git, filesystem or process work on the main thread.
3. **One error type.** `chord_core::Error` has a `MissingTool { tool, install }` case and a message case. The UI shows the install step for the first and the text for the second.
4. **No `unwrap` outside tests.** Clippy denies it.

## 4. Harness

The trait, events, commands and capabilities: `docs/harness-contract.md`, section 3. How they sit in the code:

1. **Records stop at the adapter.** `harness/pi/` turns each pi record into Chord events. The frontend's chat reducer handles every `HarnessEvent` and never sees a pi record.
2. **One path for history.** On resume the adapter reads the saved messages and sends them as the same events a live reply produces.
3. **Status without a socket.** pi's stdout carries what the status needs: `agent_start`, `agent_settled`, `message_end`, `extension_ui_request` for dialogs, and `get_state` for the session file. `status_for` in `harness/mod.rs` folds the events into a status; the app crate emits it to the sidebar and to notifications. No socket and no reporting extension.
4. **The extension Chord ships adds two commands**: `/chord-resume`, `/chord-tree`. RPC has no session list or tree navigation.
5. **Process.** One `pi --mode rpc` per pane, on pipes, started through the user's login shell so PATH and API keys match their terminal. The binary name is a parameter with the default `pi`. The adapter runs `pi --version` first and returns `MissingTool` or the upgrade step.
6. **Capabilities.** The adapter returns its list when a session starts. A view checks the list before it shows a control.

## 5. IPC

1. `tauri-specta` writes `src/ipc/bindings.ts` from the Rust command and event types. The frontend calls `commands.listWorktrees()`; no `invoke("...")` strings and no hand-written copies of Rust types.
2. Chat events go over one Tauri `Channel` per pane. Status and worktree changes go over app events.
3. `npm run check` fails when the committed bindings differ from the generated ones. The first Tauri command brings `tauri-specta` and this check.

## 6. Frontend

1. **Stores outside React**, read with `useSyncExternalStore`: a pure rules module and a thin module that connects it to IPC.
2. **`App.tsx` is layout.** It places the sidebar, tab bar, panes and panels, and lists each feature's exported button, view and commands. No registry.
3. **One commands table** in `app/commands.ts`. A feature exports its entries; shortcuts, menu accelerators and tooltips read the table.
4. **A core feature never imports from `features/ios`.** iOS views render only in a worktree with an Xcode project.
5. **Markdown renders to React elements**, never to an HTML string.

## 7. State file

1. One file, `state.json`, in the app data folder. Rust reads it at launch and writes it with a rename, so a crash leaves the old file whole.
2. One Rust struct, `State`, with `version` and `#[serde(default)]` on every field. A feature adds its fields; an older file still loads. A breaking change bumps `version` and adds one migration function.
3. The frontend gets the state at launch and sends changes back; Rust debounces the write. Nothing goes to `localStorage`.

## 8. Security

1. **CSP on.** Scripts and styles from the bundle only; images from the bundle, the asset protocol and `data:`; connections to Tauri IPC only.
2. **Path check in one place.** A command that takes a path gets a worktree and a relative path. `paths.rs` accepts it when the worktree is one Chord knows and the resolved path stays inside it.
3. **No shell strings from the UI.** Arguments travel as process arguments or env vars.

## 9. iOS

`chord-ios` depends on `chord-core`; `chord-core` cannot name `chord-ios`, so a leak fails to compile. `src-tauri` wires both and registers the iOS commands. The worktree record carries `xcode: bool`; that flag is the only iOS fact core holds.

## 10. What enforces each rule

| Rule | Enforced by |
|------|-------------|
| Only the adapter knows the wire format | pi record types are private to `harness/pi/` |
| Core never imports iOS | crate dependency direction; `scripts/check-rules.sh` in `npm run check` for `features/ios` imports |
| Rust decides every status | `status_for` is the only function that returns `Status`; TS gets the type from the bindings |
| The UI never waits on git or a process | commands are `async` and call core on `spawn_blocking`; review |
| Missing tool shows the install step | `Error::MissingTool` |
| No raw hex in components | `scripts/check-rules.sh` in `npm run check` over `src/features` and `src/app` |

## 11. Not verified

- `tauri-specta` for Tauri 2 is a release candidate (2.0.0-rc.28) and its docs pin exact versions. Fallback: `ts-rs` for the types and a hand-written `commands` object.
- pi's own prompts (not from an extension) arriving as `extension_ui_request` in RPC mode. pi's docs describe the extension case.
- The CSP against xterm.js (WebGL, workers) and CodeMirror (inline styles). Tune it when the terminal and the editor land.
- Chat streaming speed with the extra parse in Rust, against sending pi's bytes to the webview untouched.

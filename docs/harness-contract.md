# Harness contract

## Decision

Chord's own adapter trait in Rust, with an event schema that borrows ACP's names. The pi adapter speaks pi RPC. Codex and Claude Code each get an adapter that speaks the CLI's own protocol: `codex app-server`, and `claude -p` with stream-json in and out. OpenCode's adapter speaks ACP v1 to `opencode acp`, plus OpenCode's own extensions for steering and subagents.

ACP as the contract fails for pi, Codex and Claude Code: section 2 lists five features that would disappear, and both ACP adapters need Node and an npm package on top of the CLI. OpenCode ships `opencode acp` in its own binary and adds steering and subagents through extensions, so its adapter speaks ACP: `docs/features/opencode-adapter.md`. The feature list and the edge cases: `docs/features/onboarding-and-harnesses.md`.

## 1. What Chord needs from pi

- 21 RPC commands: `prompt`, `abort`, `clear_queue`, `new_session`, `get_state`, `get_messages`, `get_commands`, `get_available_models`, `set_model`, `get_available_thinking_levels`, `set_thinking_level`, `get_tree`, `fork`, `clone`, `get_fork_messages`, `set_session_name`, `get_session_stats`, `export_html`, `compact`, `extension_ui_response`, plus image content in `prompt`.
- 37 record kinds: run events (`agent_start`, `agent_settled`), message streaming (`message_start`, `message_update` with text, thinking and tool call deltas, `message_end`), tool events (`tool_execution_start`, `_update`, `_end`), `queue_update`, compaction, retry, and nine extension UI methods (`select`, `confirm`, `input`, `editor`, `notify`, `setStatus`, `setWidget`, `setTitle`, `set_editor_text`).
- Three commands RPC lacks, added by the extension Chord ships: `/chord-resume`, `/chord-tree`, `/chord-reload`.
- Status facts from pi's stdout.

## 2. ACP against that list

ACP v1 is the stable version; v2 is a published draft. Rust and TypeScript SDKs are at 1.0.

| Chord capability | pi RPC | ACP v1 stable |
|-------------------|--------|---------------|
| chat: streaming text, thinking | `message_update` deltas | `agent_message_chunk`, `agent_thought_chunk` |
| chat: tool calls | `tool_execution_*` | `tool_call`, `tool_call_update`, with kind, status, diffs |
| status: running, done | `agent_start`, `agent_settled` | stop reason on the prompt response |
| status: waiting for input | extension dialog open | `session/request_permission` or `elicitation/create` pending |
| stop | `abort` | `session/cancel` |
| attachments | image content | image content blocks |
| commands | `get_commands` | available commands update |
| models, thinking level | `set_model`, `set_thinking_level` | session config options (model category is stable) |
| sessions: new, list, resume, name | RPC plus `/chord-resume` | `session/new`, `list`, `resume`, `session_info_update` |
| context meter | `get_session_stats` | `usage_update` (used, size, cost) |
| dialogs: select, confirm, input | extension UI requests | elicitation, form mode |
| **steer while running** | `prompt` with a streaming behaviour, `queue_update` | No. The v2 draft lets a client prompt during a run and leaves the meaning to the agent |
| **session tree, fork at an entry, clone** | `get_tree`, `fork`, `clone` | No. Fork is an RFD, not stable; no tree |
| **status line texts** | `setStatus` | No |
| **widgets** (subagent card) | `setWidget` | No. Subagent sessions are an RFD |
| **subscription usage** (5h, weekly) | pi-powerbar event | No |
| compact, export | `compact`, `export_html` | Compaction is in the v2 draft; no export |

The pi side of ACP:

- pi's maintainer declined ACP in core and pointed to an adapter on top of RPC.
- At least four community adapters exist. The most complete one forwards no extension dialogs, no `setStatus`, no `setWidget`, no steering and no tree. It needs Node 24.
- A user of Chord would install pi and a third-party adapter, and Chord's bugs would depend on that adapter's release cycle.

## 3. Contract

The `Agent` trait and `start` in `crates/chord-core/src/harness/mod.rs`, one implementation per harness in `harness/<name>/`. The names and types of the trait and of `HarnessEvent` are the reference; this section holds what they do not say.

- **Events** (adapter to UI): the `HarnessEvent` variants, named after ACP's session updates where ACP has one. The frontend gets these and never a harness record. Core adds `status`; an adapter never emits it.
- **Sessions**: `session_info` carries the adapter's own session key as a string (a pi file path, a Codex thread id, a Claude Code session id); `Sessions` saves it in `PaneAgent` and hands it back to `start`. On open, the adapter replays the saved messages as live events between `history_start` and `history_end`.
- **Commands** (UI to adapter): the trait's methods, and dropping the agent to end it. Session commands (`/new`, `/resume`, `/fork`, `/model`, `/permissions`) arrive as text through `send`: the adapter lists them in `commands` with kind `session` and runs them itself. A picker it needs opens as a `dialog`, and the answer comes back through `answer_dialog`. `harness/earlier.rs` holds the select of `/fork` and `/rewind` for Codex and Claude Code; pi keeps its own fork select and `/tree`.
- **Dialogs**: the adapter emits `dialog_closed` when the agent goes on past its open dialog: after the user's answer, when the harness withdraws the ask, and when the run ends. An adapter holds one open dialog per pane; asks that arrive meanwhile wait in it and open oldest first. An approval answers with the option id the harness sent, never the label.
- **Models**: an adapter with the models capability sends `models` with every model it offers, each with the id a pick sends back and its thinking levels, once it lists them, and `model` with the model's id after each change. `pick_model` takes an id from that list and an optional level; it may arrive before the harness answered its handshake, as when the agent menu starts another harness on a picked model. `Sessions` saves each harness's last list in `State.models`, so the agent menu shows it while another harness runs. At launch `harness/models.rs` reads every harness's list at once, each from a short process that opens no session (pi with `--no-session`, Codex's app server without a thread, Claude Code's handshake with `--no-session-persistence`), and saves the ones that came back.
- **Permissions**: an adapter whose harness has a permission mode sends `permissions` with the mode's name and whether it still asks, when a session starts, when the harness changes it and after a pick. The composer bar shows its button only once one arrives.
- **Capabilities**: `capabilities()` returns the list at start; the UI hides a control whose capability is missing.
- **Status**: `status_for` folds the events into the card status; an adapter only emits events. The pi, Codex and Claude Code adapters read the facts from their harness's stdout (`ARCHITECTURE.md`, section 3).
- **Process**: `harness/process.rs` starts the binary through the user's login shell on pipes, hands each stdout line to the adapter and reports the exit. An adapter that speaks over stdio builds on it. `harness/relaunch.rs` adds the swap for an adapter whose session commands start a new process (Codex, Claude Code): the old process's events go out before the new one's, and its late output goes nowhere.
- **Harness list**: `HARNESSES` in `harness/mod.rs` describes each harness: name, title, binary, install, sign-in and upgrade step, oldest tested version, and whether Chord has its adapter. The order decides a new pane's default.
- **Version check**: `harness/installed.rs` runs each binary's `--version` and sign-in check on the login shell and opens no session. An adapter calls `installed::require` at start, which refuses a missing harness with its install command. One older than the tested version starts, and the adapter emits a `Notice` with the upgrade command.
- **Spend**: `harness/spend.rs` reads the tokens and cost every session file on the Mac records, for the Usage panel. Each harness has a `spend.rs` in its folder with two functions: `folder`, where its session files live (pi: `PI_CODING_AGENT_SESSION_DIR`, else `PI_CODING_AGENT_DIR` plus `sessions`, else `.pi/agent/sessions`; Codex: `codexHome` from its app server's `initialize` answer plus `sessions`; Claude Code: its projects folder), and `replies`, one file's text in, its replies out, each keyed by the harness's own reply id. Only pi records a cost. `spend.rs` counts a reply once per harness, skips one with no tokens, and keeps each file's replies in memory until its modified time or size changes. A missing folder or an unreadable line gives no rows, never an error.
- **Default harness**: a pane saves its harness. A pane without one takes the default at its first start: `State.default_harness`, the user's pick, while it is installed, else the first signed-in harness in the list, older than the tested version or not. `start_chat` returns the harness with its capabilities. `start_chat` with a harness and a model pick starts a pane on that harness at that model; the agent menu uses it when the user picks another agent's model: in the same pane while its chat is empty, else in a new chat tab.

## 4. Adapter order

1. pi over RPC. First release. Full capability list.
2. Codex over `codex app-server` (`docs/features/onboarding-and-harnesses.md`, N2). Its protocol has a published schema, so it goes first.
3. Claude Code over `claude -p` with stream-json (N3).
4. OpenCode over `opencode acp` (`docs/features/opencode-adapter.md`, N8).
5. Terminal pane for any other CLI. Status from process exit only.

## 5. Not verified

- The record-by-record mappings live in `docs/features/codex-adapter.md` and `docs/features/claude-adapter.md`; section 6 of each lists the records nobody has seen live.
- The ACP column comes from the docs index and the v2 pages.
- Three of the four pi ACP adapters.
- Status from pi's stdout: a recorded pi 1.5.0 RPC run had `agent_start` and `agent_settled`; the session file and the dialog records come from pi's docs only.

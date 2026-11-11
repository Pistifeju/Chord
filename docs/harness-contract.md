# Harness contract

## Decision

Chord's own adapter trait in Rust, with an event schema that borrows ACP's names. The pi adapter speaks pi RPC. A generic ACP adapter comes second and brings Claude Code, Codex and every other ACP agent at once.

ACP as the only contract fails for pi: section 2 lists five features that would disappear.

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

- pi's maintainer declined ACP in core (issue 175, closed 2025-12-13) and pointed to an adapter on top of RPC.
- At least four community adapters exist. `victor-software-house/pi-acp`, the most complete one, forwards no extension dialogs, no `setStatus`, no `setWidget`, no steering and no tree. It needs Node 24.
- A user of Chord would install pi and a third-party adapter, and Chord's bugs would depend on that adapter's release cycle.

## 3. Contract

One trait in `chord-core`, one implementation per harness. The frontend gets Chord events and never a harness record.

- **Events** (adapter to UI), named after ACP's session updates so the ACP adapter maps one to one: `message_chunk`, `thought_chunk`, `tool_call`, `tool_call_update`, `state` (running, waiting, idle with a stop reason), `commands`, `config_options`, `session_info`, `usage`, `notice`, `dialog`.
- **Chord-only events**, behind capabilities: `status_line`, `widget`, `queue`, `plan_usage`.
- **Commands** (UI to adapter): `prompt`, `cancel`, `answer_dialog`, `set_config_option`, `new_session`, `list_sessions`, `resume_session`; behind capabilities: `steer`, `fork`, `tree`, `compact`, `export`.
- **Capabilities**: the adapter returns the list at start. The UI hides a control whose capability is missing. The names are the table in `SPEC.md` section 12.
- **Status**: the adapter emits `state`; `status_for` and `exit_status` in Rust keep deciding the card status from it. The pi adapter reads the facts from pi's stdout (`ARCHITECTURE.md`, section 4).
- **Version check**: the pi adapter runs `pi --version` at start and refuses a pi older than the tested one, with the upgrade command.

## 4. Adapter order

1. pi over RPC. First release. Full capability list.
2. ACP, one adapter for every ACP agent. Claude Code and Codex both have adapters in the `agentclientprotocol` GitHub organisation. Capabilities: chat, tools, dialogs, models, sessions, context. No steer, tree, widgets or status line.
3. Terminal pane for any other CLI. Status from process exit only.

## 5. Not verified

- Claude Code's stream JSON and Codex's app-server protocol have no record-by-record mapping here. The plan reaches both through ACP, so the mapping matters only if their ACP adapters fall short.
- The ACP column comes from the docs index and the v2 pages. Before writing the ACP adapter, read the v1 schema for each row.
- Three of the four pi ACP adapters.
- Status from pi's stdout: a recorded pi 1.5.0 RPC run had `agent_start` and `agent_settled`; the session file and the dialog records come from pi's docs only.

## Sources

- pi: `docs/rpc.md`, `rpc-commands.md`, `rpc-extension-ui.md`, `json.md` under the installed package
- https://agentclientprotocol.com/protocol/v2/overview and `/prompt-lifecycle`, `/elicitation`, `/llms.txt`
- https://github.com/earendil-works/pi/issues/175
- https://github.com/victor-software-house/pi-acp
- https://github.com/agentclientprotocol/codex-acp

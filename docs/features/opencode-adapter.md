# The OpenCode adapter (N8)

Chord runs `opencode acp`, one process per pane, as `harness/opencode/` next to pi, Codex and Claude Code. The adapter speaks ACP v1 as JSON-RPC lines on stdio, plus three extensions OpenCode adds to it. This document holds the feature list, the record mapping, the contract changes, what nobody has checked yet, and the tickets (section 8).

Written against OpenCode 2.4.1, from live runs of `opencode acp` on 2026-11-24, OpenCode's ACP reference (`opencode.ai/docs/acp`) and the ACP v1 pages (`agentclientprotocol.com/protocol`). Section 7 says which claims come from a live process and which from the docs alone. To see a record live, pipe JSON lines into `opencode acp` from a temp folder. Never commit a recorded run.

## 1. Features and limits

N8 OpenCode over `opencode acp`: chat, tool calls, thinking, stop, steer, attachments, approvals and questions as dialogs, model and thinking level, context meter, slash menu, resume with history, subagent rows, compaction, and the build and run tools.

OpenCode's ACP server leaves out three things the other harnesses have:

- **No permission mode.** OpenCode has no flag that skips its asks, and its `mode` option picks a primary agent (`build`, `plan`). The adapter sends no `Permissions`, so the composer bar shows no button, as for pi.
- **No fork at a message and no rewind.** `session/fork` copies the whole session.
- **No subscription usage.** ACP reports the context fill and the session's cost. The adapter sends no `Usage`.

Why ACP here when `docs/harness-contract.md` refuses it: that refusal rests on pi, whose ACP adapters come from third parties and need Node. OpenCode ships `opencode acp` in its own binary. OpenCode's other protocol, `opencode serve` over HTTP and SSE, needs an HTTP client `chord-core` does not have, and OpenCode's docs call it experimental.

The three extensions:

| Extension | Chord advertises | OpenCode advertises |
|---|---|---|
| Steer: the request `_session/steering` | nothing | `_meta.steering.supported` on the `initialize` answer |
| Subagents: the notification `opencode/session/child_update` | `clientCapabilities._meta["opencode/child-session-updates"]: true` | the same key in `agentCapabilities._meta` |
| Questions as forms: `elicitation/create` | `clientCapabilities.elicitation.form` | nothing |

Edge cases:

- Chord runs the user's own binary under their own login. OpenCode signs in per provider, so the harness has no sign-in check, as pi.
- OpenCode's default policy allows every action except a read of a `.env` file. A user sees an approval only for that read and for the `ask` rules in their own `opencode.jsonc`.
- An approval's answer goes back as the `optionId` OpenCode sent, never as its label.
- Chord advertises no `fs` and no `terminal` capability. OpenCode reads, writes and runs commands itself.
- A subagent runs in the background and outlives the turn that started it: its updates and its end arrive after the prompt's answer.
- OpenCode echoes no live user message. The adapter sends `UserMessageChunk` itself.

## 2. Chat

### Files

`crates/chord-core/src/harness/opencode/`:

| File | Holds |
|---|---|
| `mod.rs` | The adapter: `HARNESS`, the process, the `Agent` implementation through `Relaunchable`. The only module that knows OpenCode's records. |
| `requests.rs` | The JSON-RPC lines the adapter writes: `initialize`, the session requests, the prompt, the steer, the config picks, and the answers to OpenCode's own requests. |
| `reader.rs` | One stdout line in; the harness events it means and the lines to write next out. Holds the session id, whether a prompt runs, the pending requests by id, the open reply and thought, the open tool calls and the config options. |
| `updates.rs` | What a `session/update` means as harness events. Live updates and a loaded session's replay share it. |
| `tests.rs` | Hand-written records fed to the reader, and the lines written to OpenCode. |

Sections 3 and 4 cover `asks.rs` and `commands.rs`.

The reader sits behind one mutex and implements `relaunch::Reader`, as Codex's does. It reads `serde_json::Value` and skips a record it does not know.

`HARNESS`:

| Field | Value |
|---|---|
| `name` | `opencode` |
| `title` | `OpenCode` |
| `binary` | `opencode` |
| `install` | `npm install -g @opencode/cli` |
| `sign_in` | `None` |
| `upgrade` | `opencode upgrade` |
| `oldest_tested` | `(2, 4, 1)` |
| `starts` | `true` once the `start` arm lands |

Capabilities: `Chat`, `Attachments`, `Steer`, `Models`, `Context`, `Subagents`, the list `Relaunchable` returns today.

### Handshake

1. `initialize` with `protocolVersion: 1`, `clientInfo` (`chord`, Chord's version) and `clientCapabilities: { elicitation: { form: {} }, session: { compaction: {} }, _meta: { "opencode/child-session-updates": true } }`.
2. On its answer: an answer whose `protocolVersion` is not 1 sends a `Notice` with the upgrade step and `HistoryEnd`, and opens no session. A send then gets the same text as its error.
3. `session/new { cwd, mcpServers }`. With a saved session id: `session/load` (section 5).
4. On the session's answer: `SessionInfo` with `sessionId`, `Models` and `Model` from `configOptions`, then `HistoryStart` and `HistoryEnd`. `startChat` in `chat/store.ts` waits for `history_end`, so a new session sends the pair with nothing between.

`cwd` is the worktree's absolute path. OpenCode rejects a relative one.

On 2.4.1 `initialize` answers in under 0.3 s and `session/new` in under 1.5 s.

Build and run tools (N4): `mcpServers` holds `{ name: "chord", command, args, env: [] }` from `agent_tools::command`, and is empty when that returns none. The first prompt of each session on a process carries `agent_tools::instructions()` as a `text` block with `annotations: { audience: ["assistant"] }`, which OpenCode gives the model outside the user's message.

### Updates

Every `session/update` carries the session id. The reader drops one for a session the pane left.

| `sessionUpdate` | Events |
|---|---|
| `agent_message_chunk` | `MessageChunk` with the block's text |
| the first record after a reply's chunks that is not a chunk with the same `messageId` | `MessageEnd` for the open reply |
| `agent_thought_chunk` | `ThoughtStart` before the first chunk of a `messageId`, then `ThoughtChunk`; a thought ends as a reply does |
| `user_message_chunk`, in a replay only | `UserMessageChunk`: the `text` blocks joined, each `image` block as an `Image` |
| `tool_call`, `tool_call_update` | `ToolStart` and `ToolEnd`, below |
| `usage_update` | `Context { tokens: used, window: size, estimated: false }` |
| `config_option_update` | `Models` and `Model` from the new `configOptions` |
| `available_commands_update` | `Commands` (section 4) |
| `compaction_update`, `in_progress` | `Compacting { active: true }` |
| `compaction_summary_chunk` | the text joins the held summary of its `compactionId` |
| `compaction_update`, `completed` | `Compacting { active: false }`, `Compaction` with the update's `summary`, else the held chunks, then `Context { tokens: None }` |
| `compaction_update`, `failed` or `cancelled` | `Compacting { active: false }`; `failed` adds a `Notice` with `error` |
| stdout closes | `Exited`, as pi |

Skipped: `plan`, `current_mode_update` and `session_info_update`.

A thought's `messageId` reads `<message id>:reasoning:<n>`. A reply's `messageId` changes with each model call.

ACP has no record for a turn's start. When the adapter writes `session/prompt` it sends `UserMessageChunk` with the message and `State { running: true }`.

OpenCode sends `usage_update` before each prompt's answer, a cancelled one too. The answer to `session/prompt` ends the turn:

| Answer | Events |
|---|---|
| `stopReason: "end_turn"` | `MessageEnd` for an open reply, open tool calls end as `Stopped` through `harness/calls.rs`, `State { running: false }` |
| `stopReason: "cancelled"` | the same, then `Aborted` |
| `stopReason` `max_tokens`, `max_turn_requests` or `refusal` | `MessageEnd { failed: true }` and a `Notice` that names the reason, then `State { running: false }` |
| a JSON-RPC error | `MessageEnd { failed: true }` and a `Notice` with the error's `message`, then `State { running: false }`; an `auth_required` error names `opencode auth login` |

Every ending also sends `DialogClosed` for an open ask and drops the waiting ones.

### Tool calls

A call arrives in three steps: `tool_call` at `pending` with an empty `rawInput`, then `tool_call_update` at `in_progress` with the `title`, the `locations` and the full `rawInput`, then an ending status. OpenCode may send the `in_progress` update twice. A replay sends `tool_call` at `pending` with the full input, then the ending status.

The reader holds a call from its `tool_call` and sends `ToolStart` at its first `in_progress` update, or at its ending status when none came. A second `in_progress` sends nothing. The prompt's answer drops the held calls, so an update that arrives later for one sends nothing.

| `name` (`kind`) | Title | Detail |
|---|---|---|
| `shell` (`execute`) | `read_only_title` of `harness/shell.rs` on the command, else Run | `rawInput.command`, with `command: true` |
| `read` (`read`) | Read | the first `locations` path, relative to the worktree |
| `edit` (`edit`) | Edit | the first `locations` path |
| `write` (`edit`) | Write | the first `locations` path; `changes` is the line count of `rawInput.content` |
| any call of kind `search` | Search | `rawInput.pattern`, else the call's `title` |
| any other | the call's `name` | the call's `title` when it differs from the name |

`rawInput.path` may be relative. `locations` holds the absolute path with its symlinks resolved.

`ToolEnd`: `completed` ends as `Done`, `failed` as `Failed`. A `failed` call whose `rawOutput.metadata.interrupted` is true ends as `Stopped`. An edit's `changes` sum `additions` and `deletions` over `rawOutput.metadata.files`. `images` come from the `image` blocks of `content`. `ms` stays none, since ACP reports no duration.

### Subagents

OpenCode starts a subagent through its `subagent` tool. The tool call ends at once with the child's session id, and the child runs in the background.

The notification `opencode/session/child_update` holds `rootSessionId`, `childSessionId`, `parentSessionId`, `depth`, `title` and a `type`. The reader drops one whose `rootSessionId` is not the pane's session. The rows go through `harness/subagents.rs`:

| `type` | Row |
|---|---|
| `status`, `created` | starts a row named `title` under `parentSessionId` when that is another child |
| `status`, `running` | nothing |
| `update` with a `tool_call_update` at `in_progress` | sets the row's tool to the update's `title` without its `<child title>: ` prefix |
| `status`, `completed` | ends the row Done |

A child's updates never reach the transcript. The prompt's answer leaves the rows as they are.

A permission request from a child arrives on the pane's session with the child's key in `toolCall._meta["opencode/child-session"]`. It opens a dialog as any other does.

### What `Agent` writes

| Call | Line |
|---|---|
| `send`, no run | `session/prompt { sessionId, prompt }`: one `text` block, then each image |
| `send`, mid-run | `_session/steering { sessionId, prompt }`. The answer `{ outcome: "injected" }` sends `UserMessageChunk`. The answer `{ outcome: "promptRequired" }` means the turn ended first: the message goes out again as `session/prompt`. While the prompt's answer is still on its way, the adapter holds each such message: at the answer the first goes out as the next prompt and the rest steer it. `abort` drops the held messages, and a refused steer before the next send goes nowhere. |
| an image | `{ type: "image", mimeType, data }` |
| `abort` | the notification `session/cancel { sessionId }`, and the `cancelled` outcome to each pending permission request, as ACP requires |
| `pick_model` | `session/set_config_option` (section 4) |

The adapter never writes a second `session/prompt` while one runs. OpenCode answers it with the error -32603, "Session already has an active ACP prompt".

The adapter sends no `Queue`: OpenCode answers a steer within milliseconds and says nothing when the model takes it in.

### Tab logo

- `src/ui/icons.tsx`: an `OpenCodeIcon`, drawn to `design/icons.md`, with its `--brand-*` token in `src/ui/tokens.css`.
- `src/features/harnesses/logos.tsx`: the `opencode` entry.

## 3. Asks

`harness/opencode/asks.rs` turns OpenCode's requests into dialogs through the shared `harness/asks.rs`: one open dialog per pane, the rest waiting, oldest first.

| Request | Dialog | Answer |
|---|---|---|
| `session/request_permission` | `Select` with each option's `name`; `detail` is `toolCall.rawInput.command`, else the `toolCall.locations` paths | `{ outcome: { outcome: "selected", optionId } }` |
| `elicitation/create`, mode `form` | one dialog per field through `asks::form_steps` | `{ action: "accept", content }` after the last |
| Any other request | none; a `Notice` names it | a JSON-RPC error, code -32601 |

OpenCode offers three permission options: `once` (`allow_once`, "Allow once"), `always` (`allow_always`, "Always allow"), `reject` (`reject_once`, "Reject"). Esc answers with the option whose `kind` is `reject_once`. OpenCode then rejects every other pending request of the session too, fails the call with "The user declined this tool call", and answers the prompt `cancelled`. The adapter keeps each open ask's options next to their ids.

A question arrives as one field per question: `q0` is a string with `title` the question's header, `description` the question, and `oneOf` the options as `{ const, title, description }`. Its sibling `q0_custom` is a plain string for a typed answer. `required` is empty. For each field `<key>` with a sibling `<key>_custom`, the adapter sets `other` on the `Choice` step, sends the typed text as `<key>_custom`, and opens no dialog for `<key>_custom` itself.

Esc on a form answers `{ action: "cancel" }`. OpenCode fails the `question` call with "The user dismissed this question" and answers the prompt `cancelled`.
A question with several answers arrives as an `array` field whose `items.anyOf` holds the options. Its `<key>_custom` sibling opens its own text dialog, and an empty answer goes out as no field. A form whose schema the adapter cannot read gets `cancel` and a `Notice`.

`DialogClosed` goes out after the user's answer and at the prompt's answer.

## 4. Commands and models

`Commands` lists `model`, `thinking`, `new`, `resume`, `clone`, `reload` and `compact` as `Session`, then each entry of `available_commands_update` as `Prompt`, without OpenCode's own `compact`. OpenCode 2.4.1 lists `init`, `review`, `goal`, `automation` and `compact` in an empty folder.

| Typed | The adapter |
|---|---|
| `/<command> rest` | sends the text as the prompt; OpenCode runs the command |
| `/model` | opens a `Menu` titled "Model" from the `model` option's values; the pick sends `session/set_config_option { sessionId, configId: "model", value }` |
| `/thinking` | opens a `Menu` titled "Thinking Level" from the `effort` option's values; refuses with a notice when the model has none |
| `/new` | starts a new process on a new session; works mid-turn and ends the turn |
| `/reload` | starts a new process on the same session with `session/load` |
| `/resume` | section 5 |
| `/clone` | sends `session/fork { sessionId, cwd, mcpServers }`; `SessionInfo` goes out with the answer's `sessionId`, and the transcript stays, since a fork replays nothing |
| `/compact` | sends `/compact` as the prompt; its answer is `end_turn` |

`/resume`, `/clone` and `/compact` refuse while a turn runs. Every session command refuses with images attached.

### Models

OpenCode reports its settings as `configOptions` on each session answer, on each `session/set_config_option` answer and in `config_option_update`. Each time holds the whole list. OpenCode also sends `config_option_update` on its own when its model list changes, as it does about a second after a session opens.

| Option | Category | Becomes |
|---|---|---|
| `model` | `model` | `Models`: one `ModelOption` per value, `id` the value (`provider/model`), `provider` the part before the first slash, `name` the value's `name` without its `provider/` prefix. `Model.id` is `currentValue`. |
| `effort` | `thought_level` | the `levels` of the current model's `ModelOption`, and `Model.thinking` from `currentValue`. Absent for a model without variants. |
| `mode` | `mode` | skipped |

The reader finds an option by its `category`, never by its `id`, and reads flat values and grouped values alike. 2.4.1 sends the models flat.

OpenCode reports `effort` for the session's model alone, so every other model's `levels` stay empty until a session runs it. The reader keeps the levels it has seen per model for the process's life and sends them again in each `Models`.

`pick_model(id, level)` sends the `model` pick. With a level it sends the `effort` pick after the model's answer, when that answer lists the level.

`harness/models.rs` reads no OpenCode list at launch: the list comes with a session, and that read opens none. The agent menu shows OpenCode's models from `State.models` once a pane has run OpenCode.

## 5. Resume

- A start with a saved session id sends `HistoryStart`, then `session/load { sessionId, cwd, mcpServers }`. OpenCode replays the session as `session/update` lines, which go out through `updates.rs`. `HistoryEnd` follows the answer, which holds the `configOptions`. An error answer falls back to `session/new`.
- A replay holds the user messages, the replies and thoughts whole, and each tool call as its start and its end. It holds no `usage_update`, so the context meter stays empty until the first turn, and no child session.
- `/resume` sends `session/list { cwd }`. The answer holds `sessions`, each with `sessionId`, `cwd`, `title` and `updatedAt`, and a `nextCursor`. The select of `harness/session_dialogs.rs` shows each session's title and age, without the sessions in `held_elsewhere`. The pick sends `session/close` for the pane's session, then `session/load` on the same process.
- A replayed update sends no `State`.

## 6. Contract changes and not built

Changes outside `harness/opencode/`:

- `harness/mod.rs`: `HARNESSES` holds `opencode::HARNESS` last, and `start` gets the `opencode::NAME` arm.
- `harness/installed.rs`: `parse_version` reads a version behind a leading `v`. `opencode --version` prints `opencode v2.4.1`, which it reads as no version today.
- `docs/harness-contract.md`: the decision says which harness speaks ACP and why, and section 4 lists OpenCode. Landed with N8.1.
- `docs/features/onboarding-and-harnesses.md`: N8 in section 1, OpenCode in the tables of sections 2 and 3.
- `design/icons.md`, `design/onboarding.md`, `design/status-bar.md`: the harness list reads pi, Codex, Claude Code, OpenCode.

No new `HarnessEvent`, `DialogAsk` or `Capability`.
`relaunch::Reader::interrupt` returns a `Read`, so a stop writes the cancel and the answers to the open asks, and closes the dialog.
`asks::Field::Choice` names the key a typed Other answer goes under.

Not built:

- The `mode` option as a picker of OpenCode's primary agent, with plan mode.
- `/fork` at a message and `/rewind`.
- OpenCode's models in the agent menu before its first session.
- Spend in the Usage panel: OpenCode keeps its sessions in a SQLite database, and `chord-core` has no SQLite reader.
- The retry status of a failed model call, a session's title in the tab, plans.
- More than the first page of `session/list`.
- Sign-in from Chord through OpenCode's terminal auth method.
- OpenCode 1.x.

## 7. Verified and not

Checked on a live `opencode acp` 2.4.1, on the free `opencode/big-pickle` model and on `openai/gpt-6-luna` at effort `none`:

- The handshake: `protocolVersion` 1, `loadSession`, `sessionCapabilities` with `close`, `delete`, `fork`, `list` and `resume`, both `_meta` keys, and the auth method `opencode-login`.
- `session/new`, `session/load` with its replay, `session/list`, `session/fork` and `session/set_config_option`, each with the answer section 4 or 5 gives.
- A turn: reply and thought chunks with their `messageId`, the three steps of a `shell`, `read`, `edit` and `write` call, `usage_update`, and the `end_turn` answer.
- No `user_message_chunk` for a live prompt or a steer.
- `_session/steering` mid-turn answering `injected`, the model following the steer, and the same request after the turn answering `promptRequired` with `reason: "noRunningTurn"`.
- A second `session/prompt` mid-turn refused with -32603.
- `session/cancel` during a shell command: the call ends `failed` with `interrupted`, and the prompt answers `cancelled`.
- `session/request_permission` for a `shell` command, from the session and from a child, answered with `once`, `reject` and `cancelled`. Its `toolCall` holds `kind: "execute"`, `rawInput.command` and the three options of section 3.
- An image in a prompt: `openai/gpt-6-luna` named the color of a red PNG.
- The `effort` option on `openai/gpt-6-luna`: `none`, `low`, `medium`, `high`, `xhigh`, `max`, then `default`, which is also the `currentValue` before a pick. A `set_config_option` of `none` answers with `currentValue: "none"`, and the turn sends no thought chunks.
- The `model` values carry no `/variant` suffix. The model pick sends `config_option_update` twice before the next prompt's updates.
- A call of kind `search`: the `grep` tool, with `rawInput.pattern`.
- A JSON-RPC error on a prompt: -32602 for a session OpenCode does not know.
- `elicitation/create` for a question with two options, answered with `accept`, with `cancel`, and with a typed `q0_custom`, which OpenCode takes as the answer.
- A question with several answers, answered with an array of `const` values.
- A subagent: `created`, `running`, its updates and `completed`, with the end after the prompt's answer.
- `/compact` as a prompt: `compaction_update` at `in_progress`, summary chunks, `completed` with the `summary`.

From the docs alone, or not seen:

- A permission request for an edit. A rule `{ action: "edit", resource: "*", effect: "ask" }` in the folder's `opencode.jsonc` asked nothing on 2.4.1, and the edit ran.
- An image in a tool's `content`.
- Chord's MCP server in `mcpServers`, and the `audience` annotation on the tool instructions.
- A call of kind `fetch` or `delete`, and the `patch` tool.
- The stop reasons `max_tokens`, `max_turn_requests` and `refusal`, and `auth_required`.
- A child that fails, and a child of a child.
- A turn OpenCode starts on its own. A parent gets a `<background-result>` message when its subagent goes idle; in one run no update followed on the parent's session within 40 s. Updates that arrive with no prompt open would need a `State` the adapter does not send.
- `session/load` on a session id OpenCode no longer has.
- A `compaction_update` that fails, and a compaction OpenCode starts on its own.
- Any OpenCode version other than 2.4.1.

## 8. Tickets

Four tickets, built in order. Each one ends with `npm run check` passing and leaves the app working. Tick a ticket here when it lands.

- [x] N8.1 Chat
- [x] N8.2 Asks
- [ ] N8.3 Models and commands
- [ ] N8.4 Resume, subagents and tools

A ticket that checks an item of section 7 on a live process moves that item to the checked list.

### N8.1 Chat

Needs: nothing.

Build:

- `harness/installed.rs`: `parse_version` reads a version behind a leading `v`.
- `harness/opencode/` with `mod.rs`, `requests.rs`, `reader.rs`, `updates.rs` and `tests.rs`, on `Relaunchable`.
- `HARNESS` as section 2 gives it, `opencode::HARNESS` in `HARNESSES`, and the `opencode::NAME` arm in `harness::start`.
- The handshake of section 2 with `session/new`, without `elicitation` in `clientCapabilities`. `mcpServers` stays empty.
- The whole Updates table, the prompt's answers, and Tool calls.
- What `Agent` writes: the prompt, the steer, images and `abort`.
- `Model` from the `model` option's `currentValue`, so the composer bar names the model.
- The tab logo.

Until later tickets land:

- A request from OpenCode gets the error -32601 and a `Notice` (N8.2).
- `Commands` lists OpenCode's own commands alone, `compact` among them, and `config_option_update` sends `Model` alone (N8.3).
- A typed session command goes to OpenCode as a prompt (N8.3).
- A start with a saved session id opens a new session (N8.4).
- The reader skips `opencode/session/child_update` (N8.4).

Check live: an image in a prompt, a call of kind `search`, and a JSON-RPC error on a prompt.

Done when, in the app:

- Onboarding and Settings show an OpenCode row with version 2.4.1.
- A new pane on OpenCode answers a message with streamed text, thoughts and tool cards.
- A message sent mid-run steers the run.
- Stop ends a running shell command, and the card reads Done after a finished turn.
- The context meter fills after the first turn.

### N8.2 Asks

Needs: N8.1.

Build:

- `harness/opencode/asks.rs` on the shared `harness/asks.rs`: section 3.
- `elicitation: { form: {} }` in `clientCapabilities`.
- `session/request_permission` as a `Select`, answered with the `optionId`.
- `elicitation/create` through `asks::form_steps`, with `other` on a field that has a `<key>_custom` sibling.
- `DialogClosed` after an answer and at the prompt's answer; the `cancelled` outcome to pending requests at `abort`.

Check live: an answer of `cancel` to a form, a question with several answers, a typed `<key>_custom` answer, and a permission request for an edit.

Done when, in the app, with `{ "action": "shell", "resource": "*", "effect": "ask" }` under `permissions` in the worktree's `opencode.jsonc`:

- A shell command opens a dialog with the command and three options, and the card reads Waiting.
- Allow once runs the command; Reject and Esc refuse it, and OpenCode ends the turn as stopped.
- "Ask me a question with your question tool" opens a dialog with the options and Other, and the reply uses the answer.

### N8.3 Models and commands

Needs: N8.1.

Build:

- `harness/opencode/commands.rs`: section 4.
- `Models` from `configOptions`, at each session answer and each `config_option_update`.
- `pick_model`, `/model` and `/thinking` through `session/set_config_option`.
- `Commands` with the session commands and OpenCode's own as `Prompt`.
- `/new`, `/clone` and `/compact`, with their refusals mid-turn and with images.

Done when, in the app:

- The model button opens OpenCode's models, and a pick changes the label and the model of the next turn.
- The slash menu lists the session commands and `init`, `review`, `goal` and `automation`.
- `/new` clears the transcript onto a new session, mid-run too.
- `/compact` shows the compaction with its summary and empties the context meter.

### N8.4 Resume, subagents and tools

Needs: N8.1 and N8.3.

Build:

- `session/load` at a start with a saved session id, between `HistoryStart` and `HistoryEnd`, with the fallback to `session/new`: section 5.
- `/resume` through `session/list` and `harness/session_dialogs.rs`, and `/reload`.
- The subagent rows from `opencode/session/child_update`: section 2, Subagents.
- Chord's MCP server in `mcpServers`, and the tool instructions on a session's first prompt.
- The doc edits of section 6: `docs/features/onboarding-and-harnesses.md` and the three `design/` files. `docs/harness-contract.md` and the harness list in `design/icons.md` landed with N8.1.

Check live: `session/load` on a session id OpenCode no longer has, a child that fails, Chord's MCP server and the `audience` annotation, and a turn OpenCode starts on its own after a subagent ends. If such a turn comes, its `State` handling joins this ticket.

Done when, in the app:

- A pane closed and opened again shows its OpenCode session's messages and tool cards.
- `/resume` lists the worktree's other OpenCode sessions and opens the pick.
- "Launch an explore subagent" shows a subagent line that ends Done.
- In a worktree with an Xcode project, OpenCode builds through Chord's `build` tool.

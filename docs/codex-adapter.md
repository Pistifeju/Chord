# Plan: the Codex adapter (N2)

N2 of `docs/onboarding-and-harnesses.md`: Chord runs `codex app-server`, one process per pane, as `harness/codex/` next to pi. That document holds the feature list, the edge cases and the contract changes (its sections 1, 3 and 7). This one holds the build order, the record mapping and what nobody has checked yet.

Written against codex-cli 0.158.0. `codex app-server generate-ts --out <folder>` writes the protocol's types; generate them into a temp folder to read while you write the adapter, and never commit them.

## 1. Build order

One commit, built in five stages. Each stage leaves the code compiling and pi working, so you can stop after any of them and try it. `npm run check` runs once, after stage 5.

1. **`DialogClosed` moves to the adapters.** No Codex code. `status_for` closes the dialog at `DialogClosed` alone; the pi reader emits it where core adds it today.
2. **Chat.** The process, the handshake, a thread, a turn: stream, thinking, tool calls, stop, steer, attachments, the context meter, the model label. `starts: true`, the `start` arm, the tab logo.
3. **Asks as dialogs.** Approvals, questions, elicitations, the queue of waiting asks, and the dialog contract additions they need.
4. **Commands.** The slash menu with the skills, `/model`, `/thinking`, `/permissions`, the `Permissions` event and the composer bar button.
5. **Resume with history.** A start reopens the pane's thread with its turns; `/resume` picks another one. Then the docs.

After stage 2 and before stage 3 an approval ask fails with a notice in place of a dialog (section 4, "Any other server request"). The turn goes on; nothing hangs. The commit holds all five stages, so no user meets that state.

## 2. Stage 1: `DialogClosed`

| Where | Change |
|---|---|
| `harness/mod.rs`, `status_for` | `Dialog` sets `tracker.dialog`; `DialogClosed` clears it. The arms for `MessageChunk`, `ThoughtStart`, `ThoughtChunk`, `ToolStart`, `ToolEnd` and `State { running: true }` stop clearing it. |
| `harness/mod.rs`, `HarnessEvent::DialogClosed` | The doc comment says an adapter emits it. |
| `harness/sessions.rs:227`, `PaneWatch::deliver` | Stops adding `DialogClosed`. |
| `harness/pi/reader.rs` | Holds whether a dialog is open. Emits `DialogClosed` before the first message, thought, tool or run-start event that follows an open dialog. |
| Tests | `mod.rs`: `an_open_dialog_waits_until_the_agent_goes_on` and `a_dialog_stays_open_across_a_history_until_the_agent_goes_on` close on `DialogClosed`, and a `ToolEnd` leaves the dialog open. `sessions.rs:408` moves to `pi/tests.rs` as a reader test. |

The frontend does not change: `transcript.ts:213` already closes on `dialog_closed`.

## 3. Stage 2: chat

### Files

`crates/chord-core/src/harness/codex/`:

| File | Holds |
|---|---|
| `mod.rs` | The adapter: `HARNESS`, the process, the `Agent` implementation. The only module that knows Codex's records. |
| `requests.rs` | The JSON-RPC requests the adapter writes: `initialize`, the thread, the turn, the lists. |
| `reader.rs` | One stdout line in; the harness events it means and the lines to write next out. Holds the thread id, the running turn id, the pending requests by id and the open items. |
| `items.rs` | What a thread item means as harness events. Live items and a resumed thread's turns share it. |
| `tests.rs` | Hand-written records fed to the reader, and the requests written to Codex. |

The reader sits behind one mutex. The stdout thread and the `Agent` methods both lock it, because a `send` needs the thread id, the running turn id and a fresh request id. The adapter reads `serde_json::Value`, as pi's does, and skips a record it does not know.

`HARNESSES` takes `codex::HARNESS` with `starts: true`, and `harness::start` gets the `codex::NAME` arm. A failed spawn returns `Error::start_failed` with the install step, as pi's does. `Session::start` calls `installed::require`.

Capabilities: `Chat`, `Attachments`, `Steer`, `Models`, `Context`, `Subagents`.

Usage (N6): `account/rateLimits/read` after the handshake and again at each `account/rateLimits/updated`, since an update may leave fields out. `primary` and `secondary` become `Usage` windows, shortest first, named by `windowDurationMins` (`5h`, `week`, `month`); a refusal, as for an API key, sends no windows.

Subagents (N6): Codex 0.158 reports a subagent as `subAgentActivity` items on the thread that started it, with multi-agent v1 and v2 alike; it sends no `thread/started` for the subagent's thread and no `spawnAgent` call. `started` or `interacted` starts a row for `agentThreadId`, named by the last part of `agentPath` (`/root/scout` reads `scout`), under the thread the item came on; `completed` ends it Done and `interrupted` Error. The subagent's own thread then speaks on the same process: its `item/started` sets the row's tool, its `turn/completed` ends it (failed or interrupted as Error).

### Handshake

1. `initialize` with `clientInfo` (`chord`, Chord's version) and `capabilities: null`. Never `experimentalApi`.
2. The `initialized` notification, then `model/list`.
3. On the `model/list` answer, error or not: `thread/start { cwd }`.
4. On the thread's answer: `SessionInfo`, `Model`, `HistoryStart`, `HistoryEnd`. `startChat` in `chat/store.ts` waits for `history_end`, so a new thread sends the pair with nothing between.

### Records

The reader drops a notification whose `threadId` is not the pane's thread. A subagent's thread and a thread the pane left both speak on the same process.

| Record | Events |
|---|---|
| `turn/started` | `State { running: true }`; keep the turn id |
| `item/started`, `userMessage` | `UserMessageChunk`: the `text` inputs joined, each `image` data URL as an `Image` |
| `item/agentMessage/delta` | `MessageChunk` |
| `item/completed`, `agentMessage` | the whole text as one `MessageChunk` when no delta came, then `MessageEnd { at: completedAtMs }` |
| `item/reasoning/summaryTextDelta`, `textDelta`, an item's first | `ThoughtStart` before its `ThoughtChunk`; a reasoning item without deltas sends nothing |
| `item/reasoning/summaryTextDelta`, `textDelta` | `ThoughtChunk`; `summaryPartAdded` sends a blank line |
| `item/started` and `item/completed`, `commandExecution` | `ToolStart` and `ToolEnd`. `shell_script` in `harness/shell.rs` unwraps the `/bin/zsh -lc` script; the title is `read_only_title` on it, else Run; detail is the script. `ms` from `durationMs`. `failed` and `declined` end as `Failed`. |
| same, `fileChange` | Title Write for an added file, Edit for an update, Delete for a delete; detail is the paths; `changes` counted from each `diff` |
| same, `mcpToolCall` | Title is the tool's name, detail the server and the arguments; `images` from the result's image blocks |
| same, `webSearch` | Title Search |
| same, `imageView` | Title Read, detail the path, the image read through `images.rs` |
| `item/completed`, `contextCompaction` | `Compaction` with an empty summary, then `Context { tokens: None }` |
| `thread/tokenUsage/updated` | `Context { tokens: last.totalTokens, window: modelContextWindow }` |
| `thread/settings/updated` | `Model` from `model` and `effort` (and `Permissions` from stage 4 on) |
| `error` | `Notice` with the message, unless `willRetry` |
| `turn/completed` | open tool calls end as `Stopped`; status `failed` sends `MessageEnd { failed: true }` and a `Notice`; then `State { running: false }`; status `interrupted` adds `Aborted` |
| stdout closes | `Exited`, as pi |

Skipped: `dynamicToolCall` (N4 uses MCP), `plan`, `hookPrompt`, `sleep`, review mode, image generation.

`Model.name` is the model's `displayName` from `model/list`, else its id. `thinking` is the effort string.

### What `Agent` writes

| Call | Request |
|---|---|
| `send`, no run | `turn/start { threadId, input }` |
| `send`, mid-run | `turn/steer { threadId, input, expectedTurnId }`. The adapter emits `Queue` with the steers Codex has not echoed yet and drops each one at its `userMessage` item or at the turn's end. A steer Codex refuses because the turn ended goes out again as `turn/start`. |
| an image | `{ type: "image", url: "data:<mime>;base64,<data>" }` |
| `abort` | `turn/interrupt { threadId, turnId }` |

### Tab logo

- `src/ui/icons.tsx`: a `CodexIcon`, drawn to `design/icons.md`.
- The `logos` map leaves `HarnessRows.tsx` for its own file in `src/features/harnesses/`, with the Codex entry.
- `PaneArea.tsx:81`: a chat tab shows the logo of its pane's harness, read from `paneHarnesses`, and the harness's title in the tab's tooltip.

## 4. Stage 3: asks

`harness/codex/asks.rs` holds the server requests as dialogs: the queue, the walk through one request's questions, and the answer that goes back. One dialog is open per pane. A request that arrives meanwhile waits and opens after `DialogClosed` goes out for the open one, oldest first.

`DialogClosed` goes out after the user's answer, at `serverRequest/resolved` for the open request, and at `turn/completed`, which also drops the waiting ones.

| Server request | Dialog | Answer |
|---|---|---|
| `item/commandExecution/requestApproval` | `Select`, `detail` the command, the `reason` in the title | `{ decision }` |
| `item/fileChange/requestApproval` | `Select`, `detail` the paths of the open `fileChange` item with that `itemId` | `{ decision }` |
| `item/permissions/requestApproval` | `Select`: for this turn, for the session, deny; `detail` the paths and the network access asked for | `{ permissions, scope }`; deny grants an empty profile |
| `item/tool/requestUserInput` | one dialog per question: `Select` with `descriptions` for options, plus "Other" when `isOther`; `Input` without options or with `isSecret` | `{ answers: { <question id>: { answers: [..] } } }` after the last |
| `mcpServer/elicitation/request`, `form` | one dialog per field: an enum as `Select`, a multi-select as `Select` with `multi`, a boolean as `Confirm`, a string or number as `Input` | `{ action: "accept", content }` |
| same, `url` | `Confirm` with `url` | accept after Open, decline after Decline |
| `agentMessage.questions` on `item/completed` | one `Select` or `Input` per question | a new `turn/start` whose text holds the answers |
| Any other server request | none; a `Notice` names it | a JSON-RPC error, code -32601 |

The approval options: Allow (`accept`), Allow for This Session (`acceptForSession`), Always Allow with the proposed prefix (`acceptWithExecpolicyAmendment`, shown only when the request carries `proposedExecpolicyAmendment`), Deny (`decline`). Esc answers `cancel`, which also stops the turn. The adapter keeps each open ask's options next to their decisions and answers with the decision, never the label.

A form whose schema the adapter cannot read gets `cancel` and a `Notice`.

Contract additions, each with its frontend half in `DialogPanel.tsx` and `design/chat.md`:

- `DialogAsk::Select` gains `detail: Option<String>`, `descriptions: Vec<String>` and `multi: bool`. pi's selects send none, empty and false.
- `DialogAnswer::Values { values: Vec<String> }` answers a `multi` select.
- `DialogAsk::Confirm` gains `url: Option<String>`. With one, the buttons read Open and Decline, and Open calls `openInDefaultBrowser` before it answers.

## 5. Stage 4: commands

`Commands` lists `model`, `thinking`, `permissions`, `new`, `resume`, `fork`, `clone`, `rewind`, `reload` and `compact` as `Session`, then each enabled skill of `skills/list { cwds: [dir] }` as `Skill`. `skills/changed` reads the list again.

| Typed | The adapter |
|---|---|
| `/<skill> rest` | sends `$<skill> rest` as the text input plus `{ type: "skill", name, path }` |
| `/model` | opens a `Menu` titled "Model" from the held `model/list`; the pick rides the next `turn/start` as `model`, and `Model` goes out at once |
| `/thinking` | opens a `Menu` titled "Thinking Level" from the model's `supportedReasoningEfforts`; the pick rides as `effort` |
| `/permissions` | opens a `Menu` titled "Permissions"; the pick rides as `approvalPolicy` and `sandboxPolicy`, and `Permissions` goes out at once |
| `/new` | starts a new process with the config's model, effort and mode; works mid-turn and ends the turn |
| `/reload` | starts a new process on the same thread with the user's picks; works mid-turn and ends the turn |
| `/fork` | opens a numbered select of the thread's user messages from `thread/turns/list`; the pick sends `thread/fork` with `lastTurnId` the turn before it, or opens a new thread for the first message, and puts the message in the composer |
| `/clone` | sends `thread/fork` without `lastTurnId` |
| `/rewind` | the same select; the pick sends `thread/revert` with `beforeTurnId`, then `thread/resume` for the shortened history, and puts the message in the composer |
| `/compact` | sends `thread/compact/start` |

A picked model that lacks the current effort takes its `defaultReasoningEffort`.

`/resume`, `/fork`, `/clone`, `/rewind` and `/compact` refuse while a turn runs. Every session command refuses with images attached.

### Permission modes

`harness/codex/permissions.rs`, a pure module: the modes, and the label of a policy pair.

| Mode | `approvalPolicy` | `sandboxPolicy` | `asks` |
|---|---|---|---|
| Read Only | `on-request` | `readOnly` | true |
| Workspace | `on-request` | `workspaceWrite` | true |
| Full Access | `never` | `dangerFullAccess` | false |

- `HarnessEvent::Permissions { mode: String, asks: bool }`. It goes out with the thread's answer (`approvalPolicy`, `sandbox`), at `thread/settings/updated`, and after a pick.
- A pair from the user's config that matches no row shows the sandbox and the approval policy by name, such as "Workspace, untrusted". `asks` is false for `never`.
- A pick of Workspace sends the `workspaceWrite` policy the thread's answer held, when it held one, so the user's writable roots and network setting stay. Else the defaults: no extra roots, no network.

### Frontend

- `transcript.ts`: `Chat.permissions: { mode, asks } | null`, set at `permissions`, cleared at `exited`.
- `ChatPane.tsx`: a `chat-picker` button titled "Permissions" before the model button, shown while `chat.permissions` holds a mode, in the warning color while `asks` is false. It calls `togglePicker("/permissions", "Permissions")`.
- `store.ts:157`: `openPicker` takes `"/permissions"`.
- `design/chat.md` gains the button.

pi sends no `Permissions`, so its panes show no button.

## 6. Stage 5: resume

- A start with a saved thread id sends `thread/resume { threadId, cwd }` in place of `thread/start`. An error answer falls back to `thread/start`: a thread that never ran a turn may have no file.
- The answer's `thread.turns` go out between `HistoryStart` and `HistoryEnd` through `items.rs`. A completed item sends its start and its end; an agent message sends its text as one chunk.
- `/resume` sends `thread/list { cwd }`, then opens a `Select` of each thread's `preview` and age, without the threads in `held_elsewhere`. The pick sends `thread/resume`. Mid-run it refuses with pi's line, "Wait for the agent to finish, then change the session".
- Docs: tick N2 and correct section 4's rows in `docs/onboarding-and-harnesses.md`; `docs/harness-contract.md` sections 3 and 5; the Codex paragraph of `ARCHITECTURE.md` section 3.

## 7. Tests

All in `chord-core`, with hand-written records and made-up model names.

- `status_for`: a `ToolEnd` leaves an open dialog open; `DialogClosed` closes it.
- pi reader: a tool record after a dialog emits `DialogClosed` first.
- Codex reader: a streamed turn gives `State`, `MessageChunk`, `MessageEnd`, `State`.
- Codex reader: a notification of another thread gives nothing.
- Codex asks: "Allow for This Session" answers `{"decision":"acceptForSession"}` on the request's id.
- Codex asks: a second request opens its dialog only after the first one's `DialogClosed`.
- Codex permissions: `on-request` with `readOnly` reads "Read Only" and asks; `never` with `dangerFullAccess` reads "Full Access" and does not.

## 8. Not built

A thread's name in the tab, `thread/turns/list` paging for a long history, plan items, review mode, Codex's own permission profiles (`permissionProfile/list`), the `granular` approval policy as a pick, a masked field for a secret answer.

## 9. Not verified

Checked on a live `codex app-server` 0.158.0: JSON lines on stdio, `initialize` with `capabilities: null`, `model/list`, `skills/list`, `thread/list` by `cwd`, `permissionProfile/list`, and the `thread/start` answer's `model`, `approvalPolicy`, `sandbox` and `reasoningEffort`.

Checked by hand during stage 2: `userMessage`, `agentMessage` with deltas, `commandExecution` (its `command` wraps the script in `/bin/zsh -lc`, and `commandActions` drops the parts after a pipe, so the title reads the unwrapped script), `fileChange` with an update diff, `reasoning` (empty summary and content at medium effort, no deltas, so a thought starts at its first delta), `last.totalTokens` as the fill, a steer's `userMessage` arriving before the next model call, a steer after the turn ended refused with -32600 "no active turn to steer", and `turn/interrupt` ending a running command with `turn/completed` status `interrupted` and no `item/completed`.

Still from the generated types alone:

- `mcpToolCall`, `webSearch`, `imageView`, `contextCompaction`, reasoning summary deltas, an added or deleted file's diff, and `thread/settings/updated`.
- Whether `thread/queue/changed` reports steers better than the adapter's own list.
- What Codex does with a JSON-RPC error answer to a server request Chord does not know.
- Every ask of stage 3: the approval and question requests, the elicitation forms and links, `serverRequest/resolved`, and `agentMessage.questions`.
- A `sandboxPolicy` on `turn/start` holding for the turns after it, and `thread/settings/updated` following it.
- `thread/resume` on a thread that never ran a turn.
- `thread/resume` returning the turns of a thread whose `historyMode` is `paginated`; the types call full hydration deprecated there.
- The three mode names against Codex's own `/permissions` menu.

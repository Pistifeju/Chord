# Plan: the Claude Code adapter (N3)

N3 of `docs/onboarding-and-harnesses.md`: Chord runs `claude -p` with stream-json in and out, one process per pane, as `harness/claude/` next to pi and Codex. That document holds the feature list, the edge cases and the contract changes (its sections 1, 3 and 7). This one holds the build order, the record mapping and what nobody has checked yet.

Written against Claude Code 2.1.328. Claude Code has no command that writes its protocol's types. The message types are published with the Claude Agent SDK; the control requests it leaves out were read from the schemas inside the 2.1.328 binary, then tried on a live process (section 9). To see a record live, pipe JSON lines into the start line of section 3 from a temp folder. Never commit a recorded run.

N3 changes no contract: every event and dialog field it needs landed with Codex.

## 1. Build order

One commit, built in five stages. Each stage leaves the code compiling and pi and Codex working, so you can stop after any of them and try it. `npm run check` runs once, after stage 5.

1. **The shared parts leave `codex/`.** No Claude Code. The queue of asks, the elicitation form and the resume select move up to `harness/`, where both adapters call them.
2. **Chat.** The process, the handshake, a turn: stream, thinking, tool calls, stop, steer, attachments, the context meter, the model label. `starts: true`, the `start` arm, the tab logo.
3. **Asks as dialogs.** Tool permissions, `AskUserQuestion`, `ExitPlanMode`, elicitations.
4. **Commands.** The slash menu, `/model`, `/thinking`, `/permissions` and the `Permissions` event.
5. **Resume with history.** A start reopens the pane's session with its turns, read from the transcript file; `/resume` picks another one. Then the docs.

After stage 2 and before stage 3 a permission ask fails with a notice in place of a dialog (section 4, "Any other control request"). The tool call fails, the turn goes on, nothing hangs. The commit holds all five stages, so no user meets that state.

## 2. Stage 1: the shared parts

| Where | Change |
|---|---|
| `harness/asks.rs`, new | From `codex/asks.rs`: `Asks` (one ask open, the rest waiting oldest first, the dialog ids), `Step`, `Field`, the walk through one ask's steps, and the elicitation form (`form_steps`, `enum_options`). An ask carries its adapter's own reply tag. `Asks::answer` returns that tag with what the user settled: a cancel, one option's value, or the answers by key. |
| `harness/codex/asks.rs` | Keeps `parse`, Codex's questions, `permissions_detail`, and the results Codex reads, built from what `Asks::answer` returns. |
| `harness/resume.rs`, new | From `codex/reader.rs`: the numbered select of a worktree's other sessions, the session a pick names, `age`, and the notice for a worktree with no other session. It takes each session's id, label and age in seconds. |
| `harness/mod.rs` | One `capitalized`, which makes a level's menu label. Its two copies (`pi/records.rs:110`, `codex/commands.rs:287`) go. |
| Tests | None new. `codex/tests.rs` feeds the reader and passes unchanged. |

## 3. Stage 2: chat

### Files

`crates/chord-core/src/harness/claude/`:

| File | Holds |
|---|---|
| `mod.rs` | The adapter: `HARNESS`, the process, the `Agent` implementation. The only module that knows Claude Code's records. |
| `requests.rs` | The lines the adapter writes: the user message, the control requests, and the answers to Claude Code's own control requests. |
| `reader.rs` | One stdout line in; the harness events it means and the lines to write next out. Holds the session id, whether a turn runs, the pending control requests by id, the messages waiting for their echo, the steers and the open tool calls. |
| `blocks.rs` | What a content block means as harness events: text, thinking, `tool_use`, `tool_result`. Live records and a transcript's lines share it. |
| `tests.rs` | Hand-written records fed to the reader, and the lines written to Claude Code. |

Stages 3 to 5 add `asks.rs`, `commands.rs`, `permissions.rs` and `transcript.rs`.

The reader sits behind one mutex, as Codex's does: the stdout thread and the `Agent` methods both lock it. The adapter reads `serde_json::Value` and skips a record it does not know.

`HARNESSES` takes `claude::HARNESS`, the entry `harness/mod.rs` holds today, with `starts: true`. `harness::start` gets the `claude::NAME` arm. A failed spawn returns `Error::start_failed` with the install step. `Session::start` calls `installed::require`.

Capabilities: `Chat`, `Attachments`, `Steer`, `Models`, `Context`, `Subagents`.

Usage (N6): the control request `get_usage` after the handshake, after each `result`, and at a `rate_limit_event`, at most once per 30 s, since the event holds one window's state alone. With `rate_limits_available`, `rate_limits.five_hour` and `seven_day` become the windows `5h` and `week` from `utilization` and `resets_at`; an error says nothing, since an API key has no usage.

Subagents (N6): `task_started` with a `task_type` of `local_agent`, `remote_agent`, `in_process_teammate` or `local_workflow` starts a row named `subagent_type`, else `workflow_name`, else `description`, under its `parent_task_id`. `task_progress` sets the tool from `last_tool_name`; `task_notification` `status` or `task_updated` `patch.status` ends it: `completed` Done, `failed`, `killed` or `stopped` Error. A `local_bash` task and a monitor show nothing.

### Start line and handshake

`claude -p --input-format stream-json --output-format stream-json --verbose --include-partial-messages --replay-user-messages --permission-prompt-tool stdio`

One flag is new against section 2 of the onboarding plan: `--replay-user-messages`. Claude Code echoes each user message with its `uuid` and `timestamp` once a turn takes it in. The echo is the chat's user message and the moment a steer leaves the queue.

Chord never passes `--allow-dangerously-skip-permissions`. Without it Claude Code refuses a switch to `bypassPermissions` (`error_code: "bypass_not_launched"`), so the picker does not offer that mode (section 5).

1. The control request `initialize`, with no field. Never `supportedDialogKinds`, so Claude Code sends no `request_user_dialog`.
2. On its answer: keep `commands` and `models`; write `set_max_thinking_tokens { thinking_display: "summarized" }`, `get_settings` and `get_context_usage`; send `HistoryStart` and `HistoryEnd`. `startChat` in `chat/store.ts` waits for `history_end`, so a new session sends the pair with nothing between.
3. On the `get_settings` answer: `Model` from `applied.model` and `applied.effort`.
4. On the `get_context_usage` answer: `Context` from `totalTokens` and `maxTokens`.

Without that `thinking_display` 2.1.328 streams each thought empty, with a token estimate alone, and saves it empty.

A new session has no id before its first message. `SessionInfo` goes out at the first `system` `init` record, with its `session_id`.

A control request is `{ type: "control_request", request_id, request: { subtype, ... } }`. Its answer is a `control_response` whose `response` holds `subtype` (`success` or `error`), the `request_id`, and `response` or `error`.

### Records

The reader drops a `stream_event`, `assistant` or `user` record whose `parent_tool_use_id` is set: a subagent speaks on the same stdout (N6).

| Record | Events |
|---|---|
| `system` `init`, at each turn's start | `State { running: true }`; `SessionInfo` when the `session_id` is new |
| `system` `status` `compacting` | `State { running: true }`: a `/compact` sends its `init` after the compaction |
| `user` with `isReplay` and a `uuid` the adapter sent | `UserMessageChunk`: the text (a string, or the `text` blocks joined), each `image` block as an `Image`, `at` from `timestamp`; a steer with that `uuid` leaves `Queue` |
| same, its text in `<command-name>` markup | `UserMessageChunk` with the command as the user typed it: the name, then its `<command-args>` |
| `stream_event`, `content_block_delta`, `text_delta` | `MessageChunk` |
| same, `thinking_delta` with text, a block's first | `ThoughtStart` before its `ThoughtChunk` |
| same, `thinking_delta` with text | `ThoughtChunk` |
| `assistant`, `text` block | the whole text as one `MessageChunk` when no delta came, then `MessageEnd { at }` from `timestamp`; `failed` when the record holds `error` |
| `assistant` with `local_command_run` | a command's own output, which Claude Code sends before the command's echo. The reader holds its events until that echo, or until `result`. |
| `assistant`, `thinking` block | `ThoughtStart` and its text as one `ThoughtChunk` when no delta came and it has text |
| `assistant`, `tool_use` block | `ToolStart`, by tool (the table below) |
| `user`, `tool_result` block | `ToolEnd` on its `tool_use_id`: `Failed` when `is_error`; `images` from the result's `image` blocks; an Edit's `changes` counted from the `+` and `-` lines of `tool_use_result.structuredPatch` |
| `system` `compact_boundary` | `Compaction` with an empty summary and `tokens_before` from `compact_metadata.pre_tokens`, then `Context { tokens: None }` |
| `system` `status` with `permissionMode` | `Permissions`, from stage 4 on |
| `system` `commands_changed` | `Commands`, from stage 4 on |
| `result` | open tool calls end as `Stopped`; then `State { running: false }`; `terminal_reason` `aborted_streaming` or `aborted_tools` adds `Aborted`; any other result whose `subtype` is not `success` sends `MessageEnd { failed: true }` and a `Notice` with its `errors` first. The adapter then writes `get_context_usage`. |
| `control_response` | what the pending request with that `request_id` waits for; an `error` sends a `Notice` |
| stdout closes | `Exited`, as pi |

Four records look like one of the rows above and send nothing:

- A `user` record with `isReplay` and a `uuid` the adapter did not send. Claude Code echoes its own lines too: `<local-command-stdout>` after a `set_model` mid-turn and after `/compact`.
- A `user` record without `isReplay` and without a `tool_result`: `[Request interrupted by user]`, a compaction's summary.
- A `control_response` with no pending request: Claude Code echoes each answer the adapter writes.
- A `thinking_delta` without text.

An `assistant` record arrives once per content block, before that block's `content_block_stop`, and on an interrupt it holds the text so far.

A reply that fails at the API arrives as an `assistant` record with `error` and no deltas, then a `result` with `is_error: true` and `subtype: "success"`. The row for an `assistant` text block covers it. An interrupt's `result` has `subtype: "error_during_execution"`, so the reader reads `terminal_reason` first.

`at` is epoch milliseconds, and Claude Code writes ISO 8601 in UTC. `blocks.rs` parses it; no date crate.

| Tool | Title | Detail |
|---|---|---|
| `Bash` | `read_only_title` on `command`, else Run | the command |
| `Read` | Read | `file_path`, relative to the worktree |
| `Write` | Write | the path; `changes` holds the line count of `content` |
| `Edit`, `NotebookEdit` | Edit | the path |
| `WebSearch` | Search | `query` |
| `mcp__<server>__<tool>` | the tool's name | the server and the arguments, as Codex |
| `AskUserQuestion` | Ask | the questions, one per line |
| `ExitPlanMode` | no card | `plan` goes out as a reply: `MessageChunk`, `MessageEnd` |
| any other | the tool's name | `input` as JSON, empty for an empty object |

2.1.328 has no `Grep`, `Glob` or `LS` tool: Claude Code searches through `Bash`, and `read_only_title` reads those as Search and List.

Skipped: each record with a `parent_tool_use_id`, whose subagent shows as its lineage row. Skipped for good: the hook records, `thinking_tokens`, `api_retry`, `keep_alive`, `command_lifecycle`, prompt suggestions.

### What `Agent` writes

| Call | Line |
|---|---|
| `send` | `{ type: "user", uuid, message: { role: "user", content }, parent_tool_use_id: null }`, the same idle or mid-run. Claude Code queues a mid-run message and takes it in after the running tool call, in the same turn. Mid-run the adapter emits `Queue` with the steers Claude Code has not echoed yet. |
| an image | `{ type: "image", source: { type: "base64", media_type, data } }` in `content` |
| `abort` | control `interrupt` with `cancel_queued: true`. Its answer lists the dropped steers in `cancelled`, and `Queue` empties. |

`requests.rs` makes each message's `uuid` from the clock and a counter, in UUID form. No new crate.

### Tab logo

- `src/ui/icons.tsx`: a `ClaudeIcon`, drawn to `design/icons.md`.
- `src/features/harnesses/logos.tsx`: the `claude` entry.

`PaneArea.tsx` and the harness rows read `harnessMarks`, so the tab, its tooltip, onboarding and Settings › Agents pick the logo up with no other change.

## 4. Stage 3: asks

`harness/claude/asks.rs` holds Claude Code's control requests as asks for `harness/asks.rs`, and the answer each one gets. One dialog is open per pane; the shared queue holds the rest.

`DialogClosed` goes out after the user's answer, at `control_cancel_request` for the open request, and at `result`, which also drops the waiting ones. `control_cancel_request` is a record type of its own, with the `request_id` it withdraws; an interrupt sends one for an ask nobody answered.

| Control request | Dialog | Answer |
|---|---|---|
| `can_use_tool` | `Select`. Title: the request's `title`, else "Run This Command?" for `Bash`, "Apply This Change?" for a file tool, "Allow <tool>?" for the rest. `detail`: the command, the path, or the `input` as JSON. | `{ behavior, ... }`, the options below |
| `can_use_tool`, tool `AskUserQuestion` | one dialog per question: `Select` with `descriptions`, `multi` when `multiSelect`; a single-choice question gains "Other", which opens an `Input` | `{ behavior: "allow", updatedInput }` after the last: the request's `input` plus `answers`, a map from each `question` text to its label or text; several labels joined with ", " |
| `can_use_tool`, tool `ExitPlanMode` | `Select` titled "Run This Plan?"; the plan is the reply above it | the plan options below |
| `elicitation`, `form` | one dialog per field, through `form_steps` | `{ action: "accept", content }` |
| `elicitation`, `url` | `Confirm` with `url` | `accept` after Open, `decline` after Decline |
| Any other control request | none; a `Notice` names it | an `error` response |

The permission options:

- Allow: `{ behavior: "allow" }`.
- One option per entry of `permission_suggestions` the adapter can name. Each answers `allow` plus `updatedPermissions` with that one entry, as Claude Code sent it.
  - `addRules`: "Always Allow “<ruleContent>”", or the tool's name for a rule without content. Left out when `suppress_always_allow_rule` is true. An entry with `destination: "localSettings"` makes Claude Code write `.claude/settings.local.json` in the worktree.
  - `setMode`: "Allow and Switch to <mode>", with the mode's label from section 5.
  - Any other type, such as `addDirectories`: no option.
- Deny: `deny` with a message.
- Esc: `deny` with `interrupt: true`, which stops the turn.

A file tool's ask suggests `setMode` to `acceptEdits`. A `Bash` ask suggests `addRules` for the command, `addDirectories` and that `setMode`.

The plan options: "Yes, Accept Edits" (`allow` plus a `setMode` to `acceptEdits` with `destination: "session"`), "Yes, Ask Before Edits" (`allow` plus a `setMode` to `default`), "Keep Planning" (`deny` with a message). Esc denies with `interrupt: true`. An `ExitPlanMode` ask carries no suggestions.

The adapter keeps each open ask's options next to their answers and answers with those, never the label.

A form whose schema `form_steps` cannot read gets `cancel` and a `Notice`.

## 5. Stage 4: commands

`harness/claude/commands.rs` holds the models, the model and effort in effect, the commands and the picker menus.

`Commands` lists `model`, `thinking`, `permissions`, `new`, `resume`, `fork`, `clone`, `rewind`, `reload` and `compact` as `Session`, then the `commands` of `initialize`: one with `builtin` as `Session`, the rest as `Skill` with `origin` the part of the name before a colon. Left out: Claude Code's own `model` and `effort`, which Chord's `model` and `thinking` stand in for, its `thinking`, `permissions` and `resume`, which would show twice next to Chord's, and `clear`, which Chord's `new` stands in for. Chord's `new`, `fork`, `clone`, `rewind` and `reload` start a new process; `compact` goes out as the message text. `resume`, `fork`, `clone`, `rewind` and `compact` refuse while a turn runs, and every session command refuses with images attached. Until a fork or rewind's first message, `reload`, `clone`, `fork` and `rewind` start from the session and message that fork or rewind opened, since Claude Code writes the new session or branch only with that message. A `commands_changed` record holds the list again, and `Commands` goes out from it.

| Typed | The adapter |
|---|---|
| `/<command> rest` | sends it as the message text; Claude Code runs it. A name Claude Code does not know reaches the model as a prompt. |
| `/model` | opens a `Menu` titled "Model" of each model's `displayName`; the pick writes `set_model { model }` with the entry's `value` |
| `/thinking` | opens a `Menu` titled "Thinking Level" from the model's `supportedEffortLevels`; the pick writes `apply_flag_settings { settings: { effortLevel } }` |
| `/permissions` | opens a `Menu` titled "Permissions"; the pick writes `set_permission_mode { mode }` |

- After the answer to `set_model` or `apply_flag_settings` the adapter writes `get_settings`, and `Model` goes out from `applied`. One path for the handshake and for each pick, so a model that changes the effort shows it.
- `Model.name` is the picked entry's `displayName`. Before a pick it is the `displayName` of the first entry other than `default` whose `resolvedModel` is `applied.model`, else that id. `thinking` is `applied.effort`, none for a model whose `supportsEffort` is false.
- `/thinking` on a model whose `supportsEffort` is false sends Codex's notice, "The model does not think".
- `apply_flag_settings` takes any string as `effortLevel` without an error, so the adapter sends a value of `supportedEffortLevels` alone.
- Both picks work mid-turn.

### Permission modes

`harness/claude/permissions.rs`, a pure module: the modes, and the label of a mode.

| Mode | `mode` | `asks` | In the picker |
|---|---|---|---|
| Manual | `default` | true | yes |
| Accept Edits | `acceptEdits` | true | yes |
| Plan | `plan` | true | yes |
| Auto | `auto` | true | while the model's `supportsAutoMode` is true |
| Bypass Permissions | `bypassPermissions` | false | no |
| Don't Ask | `dontAsk` | true | no |

- `Permissions` goes out with the `initialize` answer (`current_permission_mode`) and at each `system` `status` record that holds `permissionMode`. One follows a pick, and one follows a change Claude Code makes itself, as after an approved plan.
- `bypassPermissions` and `dontAsk` come from the user's config alone. A session whose config holds `bypassPermissions` starts in it without the launch flag. The picker lists neither, and marks none of its options while one of them is in effect. A pick away from one has no way back in Chord.
- `bypassPermissions` skips each ask, so its button takes the warning color. `dontAsk` denies what it would ask, so its button keeps the plain color.
- A mode the adapter does not know shows by its own name, with `asks` true.

The frontend does not change: the composer's Permissions button, `openPicker("/permissions")` and the warning color landed with Codex.

## 6. Stage 5: resume

`harness/claude/transcript.rs` holds the session files: the folder of a worktree, one session's turns, and the worktree's sessions.

- The projects folder is `projectsDirectory` in the JSON `claude auth status` prints. The adapter reads it on the login shell once per app run.
- A session's file is `<projectsDirectory>/<folder>/<session id>.jsonl`. `<folder>` is the worktree's canonical path with each character outside `A-Z`, `a-z` and `0-9` as `-`. Claude Code cuts a name over 200 characters and adds a hash; for such a path the adapter takes the folder whose name starts with those 200 characters.
- A start with a saved session id whose file exists adds `--resume <id>` to the start line. Without the file it starts a new session: `--resume` on an unknown id prints a `result` with `errors` and exits.
- Claude Code replays nothing on a resume. The turns come from the file, between `HistoryStart` and `HistoryEnd`, after `SessionInfo`.
- The walk starts at the line `--resume-session-at` named after a fork or rewind, else at the file's last line with a `uuid`, and goes back through `parentUuid`, then sends oldest first. It follows each line with a `uuid`: the chain runs through `attachment` and `system` lines too. A branch a rewind left behind is off the chain.
- A `compact_boundary` line has no `parentUuid`. It sends `Compaction` with `tokens_before` from `compactMetadata.preTokens`, and the walk goes on through its `logicalParentUuid`, so the turns before a compaction stay in the history.

| Line on the chain | Events |
|---|---|
| `user` with `promptSource` | `UserMessageChunk`, `at` from `timestamp` |
| `user`, a string in `<command-name>` markup | `UserMessageChunk` with the command as the user typed it |
| `user`, `tool_result` blocks | `ToolEnd`, through `blocks.rs` |
| `assistant` | its blocks, through `blocks.rs` |
| `system` `local_command` | its `content` without the `<local-command-stdout>` wrapper, as a reply |
| any other, `isSidechain`, `isMeta`, `isCompactSummary` | nothing |

- `/resume` lists the folder's `.jsonl` files, newest first by their change time, without the pane's own and those in `held_elsewhere`. Each one's label is the `lastPrompt` of its last `last-prompt` line; a file without one is left out. The select comes from `harness/resume.rs`.
- The pick starts a new process on that session and drops the old one, as pi's `/new` does (`pi/mod.rs`, `restart`). Each process has its own reader. Mid-run it refuses with pi's line, "Wait for the agent to finish, then change the session".
- Docs: tick N3, and correct the start line of section 2 and the Claude Code rows of sections 3 and 4 in `docs/onboarding-and-harnesses.md`; `docs/harness-contract.md` sections 3 and 5; a Claude Code paragraph in `ARCHITECTURE.md` section 3.

## 7. Tests

All in `chord-core`, with hand-written records and made-up model names.

- Claude reader: a streamed turn (`init`, two `text_delta`, the `assistant` text record, `result`) gives `State`, two `MessageChunk`, `MessageEnd`, `State`.
- Claude reader: an `assistant` record with `error` and no deltas gives its text and `MessageEnd { failed: true }`.
- Claude reader: an `isReplay` record with a `uuid` the adapter did not send gives nothing.
- Claude reader: a `result` with `subtype: "error_during_execution"` and `terminal_reason: "aborted_tools"` gives `State` and `Aborted`, and no `Notice`.
- Claude asks: "Always Allow “touch notes.txt”" answers `allow` on the request's id, with that one `addRules` entry as `updatedPermissions`.
- Claude asks: an `AskUserQuestion` with two questions answers once, after the second, with both answers under their question texts.
- Claude transcript: a file with a branch off the chain gives the last line's chain alone, and a `compact_boundary` on the chain keeps the turns before it.
- Claude transcript: `/work/my app/feature.one` has the folder `-work-my-app-feature-one`.

## 8. Not built

A session's name in the tab, a tool card before its input is whole, a tool call's duration (`ms` stays empty), "Other" on a question that takes several answers, the option previews of `AskUserQuestion`, `bypassPermissions` and `dontAsk` as picks, an `addDirectories` suggestion as an option, a command's aliases in the slash menu, fast mode, prompt suggestions, a compaction's summary text, a masked field for a secret answer.

## 9. Not verified

Checked on a live `claude -p` 2.1.328, signed in, with turns driven by hand over stdio:

- The handshake: `initialize` with `commands`, `models` and `current_permission_mode`; `get_settings` with `applied.model` and `applied.effort`; `get_context_usage` with `totalTokens` and `maxTokens`, idle and after a turn; `set_max_thinking_tokens` with `thinking_display: "summarized"` turning a thought's text on.
- A turn's order: `system` `init`, the echo with `isReplay`, the `stream_event` deltas, one `assistant` record per block, `tool_result` records, `result`. `system` `status` `requesting` comes once per model request, so `init` marks the turn's start.
- `tool_use` and `tool_result` for `Write`, `Edit` (with `structuredPatch`), `Bash`, `Read` of an image (an `image` block in the result), `AskUserQuestion` and `ExitPlanMode`.
- A steer: its echo right after the running tool call's result, in the same turn, with one `result`. `interrupt` with `cancel_queued` listing the steer in `cancelled`, and no turn after it.
- An interrupt while text streams (`aborted_streaming`) and while a tool waits (`aborted_tools`), both with `subtype: "error_during_execution"`.
- The asks: `can_use_tool` with its `permission_suggestions`; `allow` without `updatedInput`; `allow` with one `addRules` entry, after which the same command runs without an ask; `deny`; `deny` with `interrupt`; an `error` response, which fails the tool call and leaves the turn running; the `answers` map of `AskUserQuestion`; `ExitPlanMode` with a `setMode`, followed by a `system` `status` with the new mode; `control_cancel_request` after an interrupt.
- `set_model` and `apply_flag_settings` mid-turn. `set_permission_mode` for each mode, `auto` included; `bypassPermissions` refused without `--allow-dangerously-skip-permissions`; a config with `bypassPermissions` starting in it without the flag.
- `/context`, `/color`, `/model` and `/compact` sent as text: the output as an `assistant` record with `local_command_run`, then the echo in `<command-name>` markup. `/compact`: `status` `compacting`, `init`, `compact_boundary`, the summary as a `user` record.
- The signed-out reply: an `assistant` record with `error: "authentication_failed"`, then a `result` with `is_error: true` and `subtype: "success"`.
- `--resume` on an unknown id (a `result` with `errors`, then the exit), on a saved session (no replay), and on a session whose user lines hold `uuid`s in Chord's form.
- The transcript: its folder name, the `parentUuid` chain through `attachment` and `system` lines, `promptSource` on a prompt, the command and `local_command` lines, `compact_boundary` with `logicalParentUuid` and `compactMetadata.preTokens`, the `last-prompt` line.

Still unchecked:

- Both elicitation modes: no MCP server asked for one.
- A compaction Claude Code starts itself, with `trigger: "auto"`.
- A skill or a prompt command sent as text: its echo and the turn after it.
- A prompt typed in Claude Code's own terminal: whether its transcript line holds `promptSource`. The transcripts on the test machine all came from `-p`.
- `title`, `suppress_always_allow_rule` and an `addRules` entry without `ruleContent` on a `can_use_tool`: no ask seen carried them.
- "Keep Planning", "Yes, Ask Before Edits", and free text as an `AskUserQuestion` answer.
- A subagent's records, which the reader drops by `parent_tool_use_id`.
- `NotebookEdit`, `WebSearch`, an MCP tool call and an image in its result.
- Whether the `thinking_display` holds after a `set_model`, an `apply_flag_settings` and a resume.
- Whether `commands_changed` entries carry `builtin`.
- The six mode names against Claude Code's own mode switch.

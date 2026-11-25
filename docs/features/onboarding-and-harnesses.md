# Onboarding and the Claude Code, Codex and OpenCode harnesses

Chord runs Codex over `codex app-server`, Claude Code over `claude -p` with stream-json in and out, and OpenCode over `opencode acp`, each as its own adapter next to pi.

## 1. Features

- N1 core: one list describes every harness: its name, binary, install, sign-in and upgrade step, and oldest tested version. One check reads each harness's version and sign-in state on the login shell. A pane saves its harness; a new pane takes the default: the harness the user picked while it is installed (sign-in is a hint, as K2 says), else the first signed-in one in the list's order, pi, Codex, Claude Code, OpenCode, older than the tested version or not.
- K2 Onboarding: the window with no project lists every harness Chord can start in the list's order (pi, then Codex, then Claude Code, then OpenCode) in one group, each row with its state: ready with its version, not installed, not signed in, or older than the tested version. A row that needs something shows the install, sign-in or upgrade command with a copy button. The checks run when the view shows and on window focus. The first ready harness becomes the default for new sessions; a click on another ready row changes it. Add Project adds the first project. A missing `git` shows its install step.
- K5 Settings section Agents: K2's rows and the default harness for new sessions. A chat pane whose harness is missing shows its row in place of the Start button.
- N2 Codex over `codex app-server`: chat, tool calls, thinking, stop, steer, attachments, approvals and questions as dialogs, model and thinking level, context meter, slash menu, resume with history, and the permission mode picker: a composer bar button with the session's approval and sandbox mode, next to the model picker, and a `/permissions` session command that opens the same picker. A chat tab shows its harness's logo.
- N3 Claude Code over `claude -p` with stream-json in and out: N2's list, with the picker holding Claude Code's permission mode.
- N4 Build and run tools for Codex, Claude Code and OpenCode through a stdio MCP server that Chord's own binary serves.
- N5 Session commands for Codex and Claude Code: new, fork, clone, rewind, reload and compact. OpenCode takes new, clone, resume, reload and compact.
- N6 Usage for Codex and Claude Code, and subagents for Codex, Claude Code and OpenCode, from what each CLI reports itself. The status bar shows the usage of each harness that reported, after its logo.
- N7 The oldest version of each harness Chord runs: its `oldest_tested` is the oldest released version where every feature of Chord's list works. To find it, step the harness back through its released versions and run the list on each one. Section 2's tested versions list each harness's oldest and newest passing versions.
- N8 OpenCode over `opencode acp`: N2's list without the permission mode picker, fork at a message, rewind and usage. `docs/features/opencode-adapter.md` holds its plan.

Edge cases:
- K2: Chord never asks for a credential and never runs a sign-in or an install; it shows the command.
- K2: a harness row shows its maker's logo before its name.
- K2: the sign-in state is a hint: a row that reads not signed in still takes the pick and still starts.
- K2: Add Project works while no harness is ready.
- N1: the check opens no session. It runs `--version` and the sign-in status command only.
- N1: while no harness is ready a new pane has no harness. It shows every row in place of the Start button and takes the default at its first start.
- N2, N3: Chord runs the user's own binary under their own login. One older than the tested version starts, and the chat shows a notice with the upgrade step. A request it refuses shows its error as a notice.
- N2, N3: an approval's answer goes back as the option id the harness sent, never as its label.
- N2, N3: an ask stays open until its answer or until the harness withdraws it. A tool call that ends meanwhile leaves the dialog open and the card Waiting.
- N2, N3: asks that arrive while one is open wait in the adapter and open one at a time, oldest first. An ask that arrives while a picker menu is open takes the menu's place.
- N2, N3: an ask with several questions, or an elicitation form with several fields, opens one dialog per question, in order. A question that takes free text gets an "Other" option that opens an input.
- N2, N3: a question that takes several answers shows a checkbox on each option and a Submit button.
- N2, N3: an MCP server's sign-in link opens a dialog with the server's message and the link, with Open and Decline. Open sends the link to the default browser, whatever the link setting says.
- N2: a Codex question marked `isSecret` opens a plain input.
- N2, N3: the approval and permission mode come from the user's own Codex and Claude Code config until the picker changes it.
- N2, N3: a mode that skips every ask (Codex's full access, Claude Code's `bypassPermissions`) shows its button in the warning color.
- The picker button hides for a harness that never sends a mode, so pi shows none.
- N3: `AskUserQuestion` and `ExitPlanMode` open dialogs.
- N2: a Codex question arrives as a server request or as a notification; a notification gets its answer as a new user message.
- N4: a tool call outlasts a cold build; Codex's 60 s tool timeout is raised for the server.
- N5: rewind drops the picked message and the turns after it; fork keeps both. Both put the picked message in the composer. pi's `/tree` keeps its name.
- N5: `/new` and `/reload` start a new process, so they work mid-run and end the run; fork, clone, rewind and compact wait for the run to end. `/new` takes the model, thinking level and mode from the user's config; `/reload`, fork, clone and rewind keep what the user picked in the session. A pick never changes the config.
- N5: a Claude Code fork takes its new session id at its first message; until then the pane keeps the old session. A rewind before any new message shows the full session again after a restart, since Claude Code writes the branch only with that message.
- N6: usage shows only for a subscription login; an API key has no window.

## 2. How each CLI runs

Tested versions: pi 1.5.2, Claude Code 2.1.328, codex-cli 0.158.0, OpenCode 2.4.1.

| | Claude Code | Codex | OpenCode | pi |
|---|---|---|---|---|
| Start | `claude -p --input-format stream-json --output-format stream-json --verbose --include-partial-messages --replay-user-messages --permission-prompt-tool stdio`, then the control request `initialize` | `codex app-server`, then `initialize` | `opencode acp`, then `initialize` | `pi --mode rpc` |
| Version | `claude --version`: "2.1.328 (Claude Code)" | `codex --version`: "codex-cli 0.158.0" | `opencode --version`: "opencode v2.4.1" | `pi --version` |
| Sign-in check | `claude auth status` (JSON, `loggedIn`) | `codex login status` | none: OpenCode signs in per provider | none: `pi auth check` needs a provider |
| Sign-in step | `claude auth login` | `codex login` | none | none |

- Claude Code: the first message is the control request `initialize`. It returns the slash commands, the models with their effort levels, and the agents. A transcript lives at `<projectsDirectory>/<encoded cwd>/<session id>.jsonl`; `claude auth status` prints `projectsDirectory`.
- Codex: `codex app-server generate-ts` writes the protocol's types for the installed version. The adapter follows them and never sets `experimentalApi`.

## 3. Each feature on each harness

| Feature | Claude Code | Codex | OpenCode |
|---|---|---|---|
| Stream | `stream_event` deltas, `assistant`, `result` | `item/agentMessage/delta`, `turn/completed` | `session/update` chunks, the answer to `session/prompt` |
| Stop | control `interrupt` | `turn/interrupt` | `session/cancel` |
| Steer | a user message on stdin mid-run; Claude Code takes it in after the running tool call, in the same turn, and its `--replay-user-messages` echo takes it off the queue | `turn/steer` | `_session/steering` |
| Tools, thinking | `tool_use` and `tool_result` blocks, thinking deltas after `set_max_thinking_tokens` with `thinking_display: "summarized"` | `item/started` and `item/completed` for `commandExecution`, `fileChange`, `mcpToolCall`, `webSearch`; `item/reasoning/*Delta` | `tool_call`, `tool_call_update`, `agent_thought_chunk` |
| Attachments | base64 image blocks in the user message | `image` input with a data URL | `image` blocks in the prompt |
| Tool images | image blocks in a `tool_result` | `imageView` items, MCP results | `image` blocks in a call's `content` |
| Dialogs, Waiting status | control `can_use_tool` (permission asks and `AskUserQuestion`), `elicitation` | `item/*/requestApproval`, `item/tool/requestUserInput`, `mcpServer/elicitation/request` | `session/request_permission`, `elicitation/create` |
| Slash menu | `initialize` commands, `commands_changed`; a command goes out as the message text | `skills/list`; a pick goes out as a `skill` input | `available_commands_update`; a command goes out as the prompt text |
| Model, thinking level | `initialize` models, `set_model`, `apply_flag_settings` with `effortLevel`, then `get_settings` | `model/list`, `model` and `effort` on `turn/start` | `configOptions`, `session/set_config_option` |
| Context | `get_context_usage` (`totalTokens`, `maxTokens`) | `thread/tokenUsage/updated` (`modelContextWindow`) | `usage_update` |
| Usage | `get_usage`, `rate_limit_event` (`five_hour`, `seven_day`) | `account/rateLimits/read` and `/updated` (`primary`, `secondary`: `usedPercent`, `resetsAt`) | none |
| Subagents | `task_started`, `task_progress`, `task_notification`, `parent_tool_use_id` | `collabAgentToolCall`, `subAgentActivity` items | `opencode/session/child_update` |
| Resume, history | `--resume <id>` on a new process; list and history from the transcript files, since Claude Code replays nothing | `thread/list` by cwd, `thread/resume` returns the turns | `session/list` by cwd, `session/load` replays the session |
| New | a new process | a new process | a new process |
| Fork, clone | `--resume <id> --fork-session`, with `--resume-session-at` at the last reply before the picked message | `thread/fork`, with `lastTurnId` the turn before the picked message | clone only: `session/fork` |
| Rewind | `--resume <id> --resume-session-at` the last reply before the picked message; later turns leave the context | `thread/revert` with `beforeTurnId`, then `thread/resume` for the shortened history | none |
| Reload | restart the process on the same session | restart the process on the same thread | restart the process and `session/load` |
| Compact | `/compact` sent as a message, then `compact_boundary` | `thread/compact/start`, then a `contextCompaction` item | `/compact` sent as the prompt, then `compaction_update` |
| Export | Chord writes the transcript it holds as Markdown | same | same |
| Build, run tools | stdio MCP server through `--mcp-config` | stdio MCP server through a `mcp_servers.chord` config override | stdio MCP server in each session's `mcpServers` |

Permission asks open as dialogs, and the card reports Waiting while one is open. The permission mode picker switches the mode the same way the model picker does: `openPicker(pane, "/permissions")`, a dialog, then the adapter applies the pick. Every adapter takes `/model`, `/thinking` and `/permissions` as text, as the frontend sends them (`openPicker` in `chat/store.ts`).

## 4. Onboarding view

The window with no project, without the sidebar and the status bar. Once, on the first launch: three steps, Welcome, Agent, Project. After the first project, a window whose projects were all removed shows the plain "No projects yet", also without the sidebar and the status bar, with Add Project; the agent rows live in Settings › Agents (K5). `design/onboarding.md` holds the details.

- A centered column. The agent step's title says what the check found; one muted line under it.
- A group "Agent": a `panel` card with one 44px Settings row per harness, each with its logo and name: pi, Codex, Claude Code, OpenCode. A secondary "Check Again" sits in the group's heading.
- A row's meta line is its state. Its right side holds what the state needs:

| State | Meta line | Right side |
|---|---|---|
| Checking | "Checking" | 14px spinner |
| Ready | "Version 2.1.328" | the dialog's check on the default harness; a click on another ready row makes it the default |
| Not installed | "Not installed" | the install command as selectable code with a copy icon button |
| Sign in | "Installed, not signed in" | the sign-in command, same form |
| Outdated | "Version 1.4.0, Chord needs 1.5.0" | the upgrade command, same form |

- A "Git" row shows only when git is missing, with its install step.
- The project step's primary button, "Add Project", always enabled.
- A row's state fades in and rises 4px over 200ms, as a session row's status does.
- One component draws the rows here, in Settings › Agents, and in a chat pane whose harness is missing.

## 5. Contract changes

- `harness::start` gets the arms `claude` and `codex`.
- `DialogAsk::Select` gains `detail: Option<String>`: the command or file a permission ask is about, shown as selectable text under the title. It also gains `descriptions: Vec<String>`: one line per option, shown under its label; empty for a select without them. And `multi: bool`: each option gets a checkbox and the dialog a Submit button; the answer is the new `DialogAnswer::Values { values: Vec<String> }`.
- `DialogAsk::Confirm` gains `url: Option<String>`: a link shown under the message as selectable text. With one, the buttons read Open and Decline, and Open opens it through `openInDefaultBrowser` before it answers.
- `DialogClosed` becomes an adapter event; core stops adding it. `status_for` opens the dialog at `Dialog` and closes it at `DialogClosed` alone. The pi reader emits it before the first message, thought, tool or run-start event that follows an open dialog. The Codex and Claude Code adapters emit it after the user's answer, when the harness withdraws the ask (`serverRequest/resolved`, `control_cancel_request`) and when the run ends.
- The Codex and Claude Code adapters hold one open dialog per pane, since the chat shows one. An ask that arrives meanwhile waits in the adapter until `DialogClosed` goes out for the open one. The composer hides while an ask is open (`chat/composer.ts`), so no picker opens over one; an ask that arrives over an open picker menu takes its place in `chat.dialog`.
- Several questions need no contract change: the adapter opens one dialog per question or form field (a choice as `Select`, several choices as `Select` with `multi`, a boolean as `Confirm`, text or a number as `Input`) and sends one answer after the last. "Other" is an option the adapter adds; picking it opens an `Input`.
- Each ask has a dialog: `AskUserQuestion` with `multiSelect` and an elicitation's multi-select field open a `Select` with `multi`; a URL elicitation opens a `Confirm` with `url`, and the adapter answers accept after Open and decline after Decline; a Codex question with `isSecret` opens an `Input`. An elicitation form whose schema the adapter cannot read gets cancelled with a notice.
- `start_chat` returns `{ harness, capabilities }`.
- `HarnessEvent::Permissions { mode: String, asks: bool }`: the session's permission mode and whether it still asks; sent when a session starts and after each pick, as `Model` is. `openPicker` takes `"/permissions"`.
- `State.default_harness: Option<String>` holds the user's pick, none until a click; `PaneAgent::default` stops naming pi.
- No new `Capability`. Sessions, usage and dialogs are gated by their events.
- Both adapters build on `harness/process.rs` unchanged: one process per pane, JSON lines on pipes, the login shell. The Codex adapter keeps a map from JSON-RPC ids to pending requests.
- Both adapters skip records they do not know. Their tests use hand-written records.
- Tool titles reuse `harness/shell.rs`, so a `grep` reads Search on every harness.

## 6. Not built

An Install button that runs the installer, a harness menu on the tab bar's + button, custom binary paths, a masked field for a secret answer.

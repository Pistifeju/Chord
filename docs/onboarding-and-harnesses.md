# Plan: onboarding and the Claude Code and Codex harnesses

Chord runs Codex over `codex app-server` and Claude Code over `claude -p` with stream-json in and out, each as its own adapter next to pi.

## 1. What to build

- [x] N1 core: one list describes every harness: its name, binary, install, sign-in and upgrade step, and oldest tested version. One check reads each harness's version and sign-in state on the login shell. A pane saves its harness; a new pane takes the default: the harness the user picked while it is installed and new enough (sign-in is a hint, as K2 says), else the first ready one in the list's order, pi, Codex, Claude Code.
- [x] K2 Needs N1. Onboarding: the window with no project lists every harness Chord can start in the list's order (pi, then Codex, then Claude Code, each once its adapter lands) in one group, each row with its state: ready with its version, not installed, not signed in, or older than the tested version. A row that needs something shows the install, sign-in or upgrade command with a copy button. The checks run when the view shows and on window focus. The first ready harness becomes the default for new sessions; a click on another ready row changes it. Add Project adds the first project. A missing `git` shows its install step.
- [x] K5 Needs K2. Settings section Agents: K2's rows and the default harness for new sessions. A chat pane whose harness is missing or too old shows its row in place of the Start button.
- [x] N2 **hard** Needs N1. Codex over `codex app-server`: chat, tool calls, thinking, stop, steer, attachments, approvals and questions as dialogs, model and thinking level, context meter, slash menu, resume with history, and the permission mode picker: a composer bar button with the session's approval and sandbox mode, next to the model picker, and a `/permissions` session command that opens the same picker. A chat tab shows its harness's logo.
- [x] N3 **hard** Needs N1. Claude Code over `claude -p` with stream-json in and out: N2's list, with the picker holding Claude Code's permission mode.
- [x] N4 Needs N2 or N3. Build and run tools for Codex and Claude Code through a stdio MCP server that Chord's own binary serves.
- [x] N5 Needs N2 or N3. Session commands for Codex and Claude Code: new, fork, clone, rewind, reload and compact.
- [ ] N6 Needs N2 or N3. Usage and subagents for Codex and Claude Code, from what each CLI reports itself. The status bar shows the usage of every used harnesses. So we need to use the icons of the harnesses so set them apart.
- [ ] N7 Needs N1. The oldest version of each harness Chord runs: step each harness back through its released versions, run Chord's feature list on each one, and set its `oldest_tested` to the oldest version where every feature works. pi now, Codex after N2, Claude Code after N3. Section 2's tested versions list each harness's oldest and newest passing versions.

Edge cases:
- K2: Chord never asks for a credential and never runs a sign-in or an install; it shows the command.
- K2: a harness row shows its maker's logo before its name.
- K2: the sign-in state is a hint: a row that reads not signed in still takes the pick and still starts.
- K2: Add Project works while no harness is ready.
- N1: the check opens no session. It runs `--version` and the sign-in status command only.
- N1: while no harness is ready a new pane has no harness. It shows every row in place of the Start button and takes the default at its first start.
- N2, N3: Chord runs the user's own binary under their own login, and refuses one older than the tested version with the upgrade step.
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

Tested versions: pi 1.5.2, Claude Code 2.1.328, codex-cli 0.158.0.

| | Claude Code | Codex | pi |
|---|---|---|---|
| Start | `claude -p --input-format stream-json --output-format stream-json --verbose --include-partial-messages --replay-user-messages --permission-prompt-tool stdio`, then the control request `initialize` | `codex app-server`, then `initialize` | as today |
| Version | `claude --version`: "2.1.328 (Claude Code)" | `codex --version`: "codex-cli 0.158.0" | `pi --version` |
| Sign-in check | `claude auth status` (JSON, `loggedIn`) | `codex login status` | none: `pi auth check` needs a provider |
| Sign-in step | `claude auth login` | `codex login` | none |

- Claude Code: the first message is the control request `initialize`. It returns the slash commands, the models with their effort levels, and the agents. A transcript lives at `<projectsDirectory>/<encoded cwd>/<session id>.jsonl`; `claude auth status` prints `projectsDirectory`.
- Codex: `codex app-server generate-ts` writes the protocol's types for the installed version. Write the adapter against them and never set `experimentalApi`.

## 3. Each feature on each harness

| Feature | Claude Code | Codex |
|---|---|---|
| Stream | `stream_event` deltas, `assistant`, `result` | `item/agentMessage/delta`, `turn/completed` |
| Stop | control `interrupt` | `turn/interrupt` |
| Steer | a user message on stdin mid-run; Claude Code takes it in after the running tool call, in the same turn, and its `--replay-user-messages` echo takes it off the queue | `turn/steer` |
| Tools, thinking | `tool_use` and `tool_result` blocks, thinking deltas after `set_max_thinking_tokens` with `thinking_display: "summarized"` | `item/started` and `item/completed` for `commandExecution`, `fileChange`, `mcpToolCall`, `webSearch`; `item/reasoning/*Delta` |
| Attachments | base64 image blocks in the user message | `image` input with a data URL |
| Tool images | image blocks in a `tool_result` | `imageView` items, MCP results |
| Dialogs, Waiting status | control `can_use_tool` (permission asks and `AskUserQuestion`), `elicitation` | `item/*/requestApproval`, `item/tool/requestUserInput`, `mcpServer/elicitation/request` |
| Slash menu | `initialize` commands, `commands_changed`; a command goes out as the message text | `skills/list`; a pick goes out as a `skill` input |
| Model, thinking level | `initialize` models, `set_model`, `apply_flag_settings` with `effortLevel`, then `get_settings` | `model/list`, `model` and `effort` on `turn/start` |
| Context | `get_context_usage` (`totalTokens`, `maxTokens`) | `thread/tokenUsage/updated` (`modelContextWindow`) |
| Usage | `get_usage`, `rate_limit_event` (`five_hour`, `seven_day`) | `account/rateLimits/read` and `/updated` (`primary`, `secondary`: `usedPercent`, `resetsAt`) |
| Subagents | `task_started`, `task_progress`, `task_notification`, `parent_tool_use_id` | `collabAgentToolCall`, `subAgentActivity` items |
| Resume, history | `--resume <id>` on a new process; list and history from the transcript files, since Claude Code replays nothing | `thread/list` by cwd, `thread/resume` returns the turns |
| New | a new process | a new process |
| Fork, clone | `--resume <id> --fork-session`, with `--resume-session-at` at the last reply before the picked message | `thread/fork`, with `lastTurnId` the turn before the picked message |
| Rewind | `--resume <id> --resume-session-at` the last reply before the picked message; later turns leave the context | `thread/revert` with `beforeTurnId`, then `thread/resume` for the shortened history |
| Reload | restart the process on the same session | restart the process on the same thread |
| Compact | `/compact` sent as a message, then `compact_boundary` | `thread/compact/start`, then a `contextCompaction` item |
| Export | Chord writes the transcript it holds as Markdown | same |
| Build, run tools | stdio MCP server through `--mcp-config` | stdio MCP server through a `mcp_servers.chord` config override |

Permission asks open as dialogs, and the card reports Waiting while one is open. The permission mode picker switches the mode the same way the model picker does: `openPicker(pane, "/permissions")`, a dialog, then the adapter applies the pick.

## 4. What changes in the code

| Where | Today | Change | Step |
|---|---|---|---|
| `harness/mod.rs`, `start` | arms for pi, Codex and Claude Code | none | 3, 4 |
| `state.rs:122` | a new pane's harness is `pi` | the default harness: the user's pick while it is ready, else the first ready one | 1 |
| `harness/mod.rs`, `status_for` | closes the open dialog at `DialogClosed` alone, which the pi reader and the Codex and Claude Code adapters emit; Claude Code's after an answer, at `control_cancel_request` and at the run's end | none | 4 |
| `harness/pi/mod.rs:29` | install step is `npm install -g` | `curl -fsSL https://pi.dev/install.sh \| sh` | 1 |
| `harness/pi/mod.rs:332` | the version check and its parser are private to pi | one check and one parser for every harness | 1 |
| `panes/PaneArea.tsx` | a chat tab shows its pane's harness logo from `harnesses/logos.tsx`, the harness name in its tooltip; `logos.tsx` has pi, Codex and Claude Code | none | 4 |
| `usage/store.ts` | one usage list for the whole app | one per harness; the status bar shows the focused pane's | 6 |
| `harness/pi/extension.ts` | pi gets the `build` and `run` tools from the extension; Codex and Claude Code from `chord agent-tools`, a stdio MCP server in Chord's own binary | none | 5 |
| `AGENTS.md` "New harness", `harness-contract.md` 3 "Version check", `ARCHITECTURE.md` 3.6 | a new harness is one `start` arm; pi checks its own version | a new harness is a list entry and a `start` arm; one check for every harness | 1 |
| `PRODUCT.md`, `README.md:13`, `ARCHITECTURE.md` 1 and 3 | name pi as the one harness | name the three | 7 |

Every adapter takes `/model`, `/thinking` and `/permissions` as text, as the frontend sends them (`openPicker` in `chat/store.ts`).

## 5. Onboarding view

The window with no project, without the sidebar and the status bar. Once, on the first launch: three steps, Welcome, Agent, Project. After the first project, a window whose projects were all removed shows the plain "No projects yet", also without the sidebar and the status bar, with Add Project; the agent rows live in Settings › Agents (K5). `design/onboarding.md` holds the details.

- A centered column. The agent step's title says what the check found; one muted line under it.
- A group "Agent": a `panel` card with one 44px Settings row per harness, each with its logo and name: pi, Codex, Claude Code. A secondary "Check Again" sits in the group's heading.
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
- One component draws the rows here, in Settings › Agents, and in a chat pane whose harness is missing (in place of today's one-line notice).

Run this step's UI through the impeccable skill, and add an "Onboarding" entry and the Agents section to `DESIGN.md` in the same commit.

## 6. Build order

Each step ships alone and leaves pi working.

1. **N1 Harness list and detection** (core). `harness/mod.rs` describes each harness: name, title, binary, install step, sign-in step, oldest tested version. A new `harness/installed.rs` checks all of them and git on the login shell, in parallel, with one version parser. `State` gains `default_harness`, the user's pick. One pure function gives the default from the pick and the checked states; onboarding, Settings and a new pane's first start call it. `start_chat` returns the pane's harness with its capabilities. One Tauri command lists the states. Test: the version parser on the three version strings of section 2. The docs of section 4's step 1 row change in the same commit.
2. **K2 Onboarding and K5 Settings › Agents** (section 5). It lists pi alone; steps 3 and 4 add their rows.
3. **N2 Codex adapter**, `harness/codex/`. First `DialogClosed` moves to the adapters (section 7), with pi's rule in the pi reader. Then: chat, tools, thinking, stop, steer, attachments, dialogs for approvals, models, context, slash menu, resume with history, the permission mode picker. With it: the tab logo.
4. **N3 Claude Code adapter**, `harness/claude/`: the same list.
5. **N4 Build and run tools over MCP**. `chord agent-tools <socket> <worktree>` answers `initialize`, `tools/list` and `tools/call` on stdio and forwards each call to the socket `agent_tools.rs` already serves. `main.rs` branches to it before Tauri starts. Codex gets `tool_timeout_sec` raised past a cold build.
6. **N5 Session commands**, then **N6 usage and subagents** with the usage store of section 4, then Markdown export.
7. **Docs.** `ARCHITECTURE.md`, `PRODUCT.md` and `README.md` name the three harnesses.

Not built: an Install button that runs the installer, a harness menu on the tab bar's + button, custom binary paths, a masked field for a secret answer.

## 7. Contract changes

- `harness::start` gets the arms `claude` and `codex`.
- `DialogAsk::Select` gains `detail: Option<String>`: the command or file a permission ask is about, shown as selectable text under the title. It also gains `descriptions: Vec<String>`: one line per option, shown under its label; empty for a select without them. And `multi: bool`: each option gets a checkbox and the dialog a Submit button; the answer is the new `DialogAnswer::Values { values: Vec<String> }`.
- `DialogAsk::Confirm` gains `url: Option<String>`: a link shown under the message as selectable text. With one, the buttons read Open and Decline, and Open opens it through `openInDefaultBrowser` before it answers.
- `DialogClosed` becomes an adapter event; core stops adding it. `status_for` opens the dialog at `Dialog` and closes it at `DialogClosed` alone. The pi reader emits it at the events that close a dialog today. The Codex and Claude Code adapters emit it after the user's answer, when the harness withdraws the ask (`serverRequest/resolved`, `control_cancel_request`) and when the run ends.
- The Codex and Claude Code adapters hold one open dialog per pane, since the chat shows one. An ask that arrives meanwhile waits in the adapter until `DialogClosed` goes out for the open one. The composer hides while an ask is open (`chat/composer.ts:17`), so no picker opens over one; an ask that arrives over an open picker menu takes its place in `chat.dialog`.
- Several questions need no contract change: the adapter opens one dialog per question or form field (a choice as `Select`, several choices as `Select` with `multi`, a boolean as `Confirm`, text or a number as `Input`) and sends one answer after the last. "Other" is an option the adapter adds; picking it opens an `Input`.
- Each ask has a dialog: `AskUserQuestion` with `multiSelect` and an elicitation's multi-select field open a `Select` with `multi`; a URL elicitation opens a `Confirm` with `url`, and the adapter answers accept after Open and decline after Decline; a Codex question with `isSecret` opens an `Input`. An elicitation form whose schema the adapter cannot read gets cancelled with a notice.
- `start_chat` returns `{ harness, capabilities }`.
- `HarnessEvent::Permissions { mode: String, asks: bool }`: the session's permission mode and whether it still asks; sent when a session starts and after each pick, as `Model` is. `openPicker` takes `"/permissions"`.
- `State.default_harness: Option<String>` holds the user's pick, none until a click; `PaneAgent::default` stops naming pi.
- No new `Capability`. Sessions, usage and dialogs are gated by their events.
- Both adapters build on `harness/process.rs` unchanged: one process per pane, JSON lines on pipes, the login shell. The Codex adapter keeps a map from JSON-RPC ids to pending requests.
- Both adapters skip records they do not know. Their tests use hand-written records.
- Tool titles reuse `harness/shell.rs`, so a `grep` reads Search on every harness.

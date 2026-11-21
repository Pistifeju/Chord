# The Usage panel

The panel shows the tokens and cost of every pi, Codex and Claude Code session on this Mac: a ranked list of every harness and its models on the left, and on the right an inspector for the picked row: its total, a chart over time, its tokens by type, five figures and its busiest days. The layout is version 1, "Inspector", of four variants. The panel's rules: `design/usage-panel.md`.

## 1. Decisions

1. **Panel, not a full window.** It opens over the blurred window as Settings does: 960 × 760px at most, 28px clear of the window edges, same backdrop, same close rules.
2. **Cost comes from the harness's own records.** pi writes a cost on each reply. Codex and Claude Code write none, so their rows show tokens and the text "No cost recorded". Chord holds no price table: a table per model name breaks the "no fixed model name" rule.
3. **Colors come from existing tokens.** The harness with the largest share in the shown range takes `accent-ink` with an `accent-wash` fill under its line; the second takes `ink-secondary`, the third `ink-disabled`. The token type bar uses the gray ladder. No new hue.
4. **One new type step.** The total at the top left takes 28px at 600, tabular figures. `DESIGN.md` had nothing above 16px before it.
5. **Memory cache only.** Rust keeps each file's parsed replies in memory, so the first open per app run reads every file and later reads parse changed files alone. Section 4 holds the measured first read; a disk cache follows only if that number turns bad.
6. **pi's default folder sits in code.** pi's RPC has no call that reports the sessions folder, so the reader names `.pi/agent/sessions` after pi's two environment variables. The other way, Chord's extension reporting `getAgentDir()`, needs the launch-time models read to load the extension; rejected as a second mechanism for the same result.

## 2. Out of scope

- A filter per project ("All environments" in the reference screenshot).
- A Limits tab. The status bar shows the subscription windows.
- A cost for Codex and Claude Code.
- pi's `sessionDir` setting in its settings file. The environment variables and the default folder cover the rest.

## 3. What the session files hold

Measured on one Mac, pi 1.5.2:

| Harness | Files | Record with the tokens | Cost | Repeats |
|---|---|---|---|---|
| pi | 3,154 files, 2.4 GB | `message` with role `assistant`: `usage.input`, `output`, `cacheRead`, `cacheWrite`, `usage.cost.*`, `message.timestamp` in ms, `provider`, `model`, `responseId` | yes | 1,437 response ids sit in more than one file: a fork copies history |
| Codex | 19 files, 15 with usage | `token_usage_record`: `payload.usage.input_tokens`, `cached_input_tokens`, `cache_write_input_tokens`, `output_tokens`, `payload.response_id`, the record's ISO `timestamp` | no | none seen |
| Claude Code | 7 files | `assistant`: `message.usage.input_tokens`, `cache_read_input_tokens`, `cache_creation_input_tokens`, `output_tokens`, `message.id`, `message.model`, ISO `timestamp` | no | one message id sits on several lines |

Rules that follow:

- **Count a reply once per harness.** The key is pi's `responseId` (the entry `id` when it is missing), Codex's `response_id`, Claude Code's `message.id`. For Claude Code the last line of an id holds the final counts.
- **Codex's input holds the cached parts.** Uncached input is `input_tokens − cached_input_tokens − cache_write_input_tokens`. All 53 local records satisfy `input ≥ cached + cache write`.
- **Codex's model** comes from the record when it names one, else from the last `turn_context` before it.
- **pi's model name** reads `provider/model`.
- **pi's cache saving** per reply is `cacheRead × (cost.input ÷ input) − cost.cacheRead`. A reply with `input` 0 gives no rate, so it adds no saving; a `shortcut:` comment in `pi/spend.rs` marks that.
- `rg` searched the whole pi folder in 0.98 s, so a line filter before the JSON parse keeps the read short.

Where the files live:

- **pi:** `PI_CODING_AGENT_SESSION_DIR`, else `PI_CODING_AGENT_DIR` plus `sessions`, else `.pi/agent/sessions` under `HOME`. Files nest up to four folders deep.
- **Codex:** the `codexHome` that `codex app-server` returns to `initialize`, plus `sessions`. Codex 0.158.0 answered in 0.25 s. `codex/spend.rs` runs that handshake and keeps the folder once Codex answers.
- **Claude Code:** `transcript::projects()`.
- `pi/spend.rs` reads pi's variables through `login_shell`, once per app run, as `projects()` does. An app started from the Finder lacks the shell's variables.
- A folder that does not exist gives that harness no rows. No error reaches the user.

## 4. Risks

- **pi's default folder.** A later pi that moves its default folder shows no pi usage until the reader follows (decision 6).
- **Record formats change.** `token_usage_record` appeared in a recent Codex; an older file with `token_count` alone gives no rows. The reader skips what it cannot parse, so a format change shows as missing usage, never as an error.
- **The first read.** Measured on one Mac, release build: 3.2 s for the first read of 2.4 GB of pi files plus Codex's and Claude Code's, which includes the three folder lookups; 44 ms for the second read from the memory cache. The panel shows "Loading" for those seconds once per app run.

# Plan: the Usage panel

Built. The panel shows the tokens and cost of every pi, Codex and Claude Code session on this Mac: a ranked list of every harness and its models on the left, and on the right an inspector for the picked row: its total, a chart over time, its tokens by type, five figures and its busiest days. The layout is version 1, "Inspector", of four variants. The built panel's rules: `design/usage-panel.md`.

## 1. Decisions

1. **Panel, not a full window.** It opens over the blurred window as Settings does: 960 × 760px at most, 28px clear of the window edges, same backdrop, same close rules.
2. **Cost comes from the harness's own records.** pi writes a cost on each reply. Codex and Claude Code write none, so their rows show tokens and the text "No cost recorded". Chord holds no price table: a table per model name breaks the "no fixed model name" rule.
3. **Colors come from existing tokens.** The harness with the largest share in the shown range takes `accent-ink` with an `accent-wash` fill under its line; the second takes `ink-secondary`, the third `ink-disabled`. The token type bar uses the gray ladder. No new hue.
4. **One new type step.** The total at the top left takes 28px at 600, tabular figures. `DESIGN.md` has nothing above 16px today.
5. **Memory cache only.** Rust keeps each file's parsed replies in memory, so the first open per app run reads every file and later reads parse changed files alone. Step 1 measures the first read; a disk cache follows only if that number is bad.
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
- **pi's cache saving** per reply is `cacheRead × (cost.input ÷ input) − cost.cacheRead`. A reply with `input` 0 gives no rate, so it adds no saving; mark that with a `shortcut:` comment.
- `rg` searched the whole pi folder in 0.98 s, so a line filter before the JSON parse keeps the read short.

Where the files live:

- **pi:** `PI_CODING_AGENT_SESSION_DIR`, else `PI_CODING_AGENT_DIR` plus `sessions`, else `.pi/agent/sessions` under `HOME`. Files nest up to four folders deep.
- **Codex:** the `codexHome` that `codex app-server` returns to `initialize`, plus `sessions`. Codex 0.158.0 answered in 0.25 s. `codex/mod.rs` runs that handshake to list the models; read the folder the same way, and keep it once Codex answers.
- **Claude Code:** `transcript::projects()`, which exists.
- Read pi's variables through `login_shell`, once per app run, as `projects()` does. An app started from the Finder lacks the shell's variables.
- A folder that does not exist gives that harness no rows. No error reaches the user.

## 4. Steps

### Step 1: the reader in `chord-core`

New files, each with a line 1 that says what it holds:

- `crates/chord-core/src/harness/spend.rs`: the walk, the cache, the dedupe and the sums.
- `crates/chord-core/src/harness/pi/spend.rs`, `codex/spend.rs`, `claude/spend.rs`: one function each, a session file's lines in, its replies out. Each knows its own harness's records and nothing else.

Shapes:

```rust
// One reply as its harness recorded it. Stays inside chord-core.
struct Reply { id: String, at: f64, model: String, input: f64, cache_read: f64, cache_write: f64, output: f64, cost: Option<f64>, saved: Option<f64> }

// One hour of one model of one harness. `hour` is the start of the UTC hour in ms.
pub struct SpendRow { hour: f64, harness: String, model: String, input: f64, cache_read: f64, cache_write: f64, output: f64, cost: Option<f64>, saved: Option<f64> }

pub struct Spend { rows: Vec<SpendRow>, sessions: BTreeMap<String, u32> }

pub fn read(since: f64) -> Spend
```

- Token counts travel as `f64`, as the timestamps do: a 90-day sum passes `u32`.
- `read` skips a file whose modified time is before `since`, and a reply before `since`.
- A session is one file with a reply at or after `since`.
- The cache is a process-wide map from path to modified time, size and replies.
- Each adapter skips a line that lacks its marker text (`"usage"`, `"token_usage_record"`, `"assistant"`) before it parses, and parses into a typed struct so serde drops the message content.
- A line or a file that fails to parse is skipped.
- Move `timestamp` from `claude/blocks.rs` to `harness/mod.rs`; Codex's records need it too.
- Time the first full read once on the 2.4 GB folder and report the number. Decision 5 depends on it.

Tests, in `chord-core`, on hand-written records with hardcoded expected values. Never a recorded session.

1. pi: one `responseId` in two files counts once.
2. pi: `input` 100 at cost 0.0005 and `cacheRead` 1000 at cost 0.0005 give a saving of 0.0045.
3. Codex: `input_tokens` 1000, `cached_input_tokens` 600, `cache_write_input_tokens` 300 give an uncached input of 100.
4. Claude Code: one `message.id` on two lines counts once, with the last line's counts.

### Step 2: the command

- `read_spend(since: f64) -> Result<Spend, String>` in `src-tauri/src/commands/harnesses.rs`, on `blocking`.
- One line in the command list in `lib.rs`.
- `npm run bindings`.

### Step 3: the rules in TypeScript

`src/features/usage/spend.ts`, a pure module:

- The four ranges (Past 24h, 7 days, 30 days, 90 days) and the `since` each one gives.
- The list for a range: All agents, then each harness with rows, ranked by the shown value (cost or tokens, ties by tokens), each followed by its models in the same order. Each row carries its value and its share of the range's total; a row without a cost has neither in Cost mode.
- A fold of the rows for a range and a pick (all, a harness, or one model of a harness) into: the total, the session count, the chart's points per harness (local days; hours for Past 24h), the four token type sums, the five figures, and the three busiest days or hours by the shown value. Uncached input is input plus cache write.
- The number texts through `Intl.NumberFormat` with the system's locale: compact tokens ("6.92B", "123M"), currency, and a share that reads "<0.1%" under 0.1.

`src/features/usage/store.ts` gains the panel's open flag and the last `Spend` read; update its line 1.

One test: rows of two harnesses and two models, hardcoded, give the expected list order, totals and shares, and the expected total for a pick of one model.

### Step 4: the view

`src/features/usage/UsagePanel.tsx` and `usage.css`.

- **Header**, 48px with the Settings header's rule: "Usage" at Heading size, then two `Segmented` controls (range: "24h", "7d", "30d", "90d", each with a tooltip such as "Past 7 days"; Cost or Tokens), a refresh icon button whose icon turns while a read runs, and a close icon button.
- **Under the header**, the panel splits in two: the list on the left takes the rest of the width, the inspector sits on the right.
- **List:** scrolls, 12px padding. One `role="listbox"` that takes the focus as a single tab stop and names the picked row with `aria-activedescendant`. Rows are 36px with 12px side padding and the Control radius:
  - "All agents" first, 40px, at 600 in `ink`.
  - Each harness with 12px above it: its rank color as an 8px dot, its logo from `harnessMarks`, its title at 500, its share, its value.
  - Its models under it, 48px in: the model name with a 2px `accent-ink` share bar under it, the share in meta text, the value at the right in a 96px tabular column.
  - Hover takes `raised` over 120ms with `--ease-out`. The picked row takes `selected` and `ink`, and its share steps up to `ink-secondary`. While the list holds the focus, the picked row carries a 2px inset focus ring.
  - ↑ ↓ Home End move the pick and scroll it into view; a click picks. It opens on All agents.
- **Inspector:** 340px, `panel`, a 1px `line` left edge, 20px 28px 28px 20px padding, 20px between its parts, scrolls. For the picked row:
  - The title: the logo and the name at Heading size. A model name wraps and is selectable text. It fades in and rises 4px over 200ms with `--ease-out` when the pick changes.
  - The total at 28px, 600, tabular figures, with the session count and the range in meta text under it; a model shows the range alone, since the session counts are per harness. Cost mode for a pick with no cost shows "No cost recorded" at 16px in `ink-muted`.
  - The chart, 280 × 140: an inline SVG, one path per harness in the pick, four horizontal grid lines in `line` with their values in meta text, three date labels under the axis. `role="img"` with a label that states the range and the total. No chart dependency.
  - Tokens by type: one 8px bar in four gray segments 2px apart, and a legend under it: Input, Cache read, Cache write, Output, each with its count.
  - Five figures as rows split by `line` rules, the name in `ink-muted` and the value at 500: Processed tokens, Cached input, Uncached input, Output, Cache savings.
  - "Busiest days" ("Busiest hours" for Past 24h) as a field label, then the three top rows by the shown value.
- **States:** "Loading" in meta text, centered, on the first read; "No usage in this range" at Empty title size for an empty one.
- The panel reads on open, on a range change and on refresh.
- It is a `role="dialog"` with `aria-modal`, inert window behind it, focus back to the control that held it on close. Opening it closes Settings and the reverse.
- Motion is the sheet's rise and the inspector title's fade. Under Reduce Motion both only fade and the refresh icon turns at a third of its speed. The chart and the bars do not animate.

Wiring:

- Commands `usage.open` (title "Usage", ⌘⇧U, free today) and `usage.close` (Esc while the panel is open and no sheet is) in `src/features/usage/commands.ts`, listed in `src/app/commands.ts`.
- A "Usage" item under Settings in the Chord menu, in `src/app/appMenu.ts`.
- `App.tsx` renders the panel beside `SettingsPage`.
- A click on a usage group in `StatusBar.tsx` opens the panel; the group becomes a button with the pointing-hand cursor.

### Step 5: docs and the check

- `design/usage-panel.md`: the panel's sizes, colors, texts and states as built.
- `DESIGN.md`: the panel in Components and Layout, the 28px step in Typography, the chart line among the uses of `accent-ink`.
- `design/status-bar.md`: the click on a usage group.
- `docs/harness-contract.md`: one row for the spend a harness's session files record.
- `npm run fix`, then `npm run check` once.

## 5. Risks

- **pi's default folder.** A later pi that moves its default folder shows no pi usage until the reader follows (decision 6).
- **Record formats change.** `token_usage_record` appeared in a recent Codex; an older file with `token_count` alone gives no rows. The reader skips what it cannot parse, so a format change shows as missing usage, never as an error.
- **The first read.** Measured after step 1 on this Mac, release build: 3.2 s for the first read of 2.4 GB of pi files plus Codex's and Claude Code's, which includes the three folder lookups; 44 ms for the second read from the memory cache. The panel shows "Loading" for those seconds once per app run.

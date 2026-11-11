# Chord

A macOS workspace for coding agents: one git worktree per task, each with its own agent chats. Tauri 2, Rust, React/TS.

- Scope: `SPEC.md`. Architecture: `ARCHITECTURE.md`. UI rules: `DESIGN.md` and `design/`.
- Module map: `find src crates src-tauri \( -name '*.ts*' -o -name '*.rs' \) | xargs head -1`. Line 1 of every source file says what the file holds.

## Boundaries

- Only `crates/chord-core/src/harness/pi.rs` knows pi's records. The frontend gets Chord events.
- `chord-core` has no `tauri` dependency and never names `chord-ios`.
- Rust decides every status (`status_for`). TS stores it.
- A Tauri command runs git, filesystem and process work on `spawn_blocking`.
- A tool Chord calls (`git`, `pi`, `xcodebuild`, `xcrun`) can be missing. Return `Error::MissingTool` with the install step.
- `src/ipc/bindings.ts` is generated. Never edit it.
- Saved state goes through Rust into `state.json`. No `localStorage`.
- Status reporting never breaks the agent.

## Rules

- Build nothing before a feature needs it: no empty container, no option nobody asked for.
- Before you write new logic at any scale, search the repo for the same thing. Extend or generalize it instead of building a parallel version.
- State rules live in a pure module outside React.
- Every UI change follows `DESIGN.md` and the `design/<group>.md` of its component group.
- CSS variables from `src/ui/tokens.css`. No raw hex in components.
- Name a file, folder or module after the domain concept it holds. Never `utils`, `helpers`, `common`, `misc` or `shared`. Reaching for one of those means the file holds two jobs: split it, or find the name in what its functions operate on.
- Write a comment only where the code cannot say it: one line, the why, never the how.

## Tests

Do not add tests by default. Add one only when all three hold:

- it fails when you revert your change;
- it hardcodes its expected values, never computed by production code, fixtures or mocks;
- it runs at the one layer that owns the behavior: a pure module or `chord-core`, never the UI.

Never test copy, CSS classes, constants, third-party libraries, private state, timing or work counts. A test you touch that breaks these rules gets deleted, not updated. Fix or delete a flaky test.

## A stranger's machine

- No absolute path, user name or home folder layout in code or fixtures.
- No fixed language, model name or pi extension name. A missing optional extension hides its UI.
- Never commit a recorded pi RPC run. It holds the system prompt and the user's commands.

## How to

- New Tauri command: a function in `src-tauri/src/commands/<area>.rs`, one line in the command list in `lib.rs`, then regenerate the bindings.
- New shortcut or menu item: one entry in the feature's commands, listed in `src/app/commands.ts`.
- New harness capability: a variant in `Capability`, emit it in `harness/pi.rs`, check it in the view, add a row to `SPEC.md`.

## Done

- `npm run check` passes. Never report a check you did not run.
- `SPEC.md` ticks the feature in the same commit.
- Commit messages: `type(area): what changed`, lower case. Types: `feat`, `fix`, `perf`, `refactor`, `test`, `docs`, `chore`. The area is a feature folder (`sidebar`, `chat`, `ios`, ...) and may be left out: `docs: add the harness contract`.

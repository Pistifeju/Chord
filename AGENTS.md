# Chord

A macOS workspace for coding agents: one git worktree per task, each with its own agent chats. Tauri 2, Rust, React/TS.

- Architecture: `ARCHITECTURE.md`. UI rules: `DESIGN.md`.
- Module map: `npm run map` prints every source folder with its file count. `npm run map <folder>` prints that folder's files, each with its line 1, which says what the file holds. When you look for code, run the first, then the second on the folders the task names, and read only the files you need, by name.
- ripgrep has no `tsx` type: `rg -t ts` searches `.ts` and `.tsx`.

## Boundaries

- Only `crates/chord-core/src/harness/<name>/` knows that harness's records. The frontend gets Chord events.
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
- Every UI change follows `DESIGN.md` and the `design/` file of each component you touch. `DESIGN.md` lists the files under `Components`.
- In a `design/` file, start each sentence on its own line, so a diff shows the one sentence that changed.
- CSS variables from `src/ui/tokens.css`. No raw hex in components.
- Name a file, folder or module after the domain concept it holds. Never `utils`, `helpers`, `common`, `misc` or `shared`. Reaching for one of those means the file holds two jobs: split it, or find the name in what its functions operate on.
- Line 1 of every source file is a comment that says what the file holds. `npm run map` prints it.
- A `///` doc comment states an item's contract. Write any other comment only where the code cannot say it: one line, the why, never the how.

## Tests

Do not add tests by default. Add one only when all three hold:

- it fails when you revert your change;
- it hardcodes its expected values, never computed by production code, fixtures or mocks;
- it runs at the one layer that owns the behavior: a pure module or `chord-core`, never a React component.

Never test copy, CSS classes, constants, third-party libraries, private state, timing or work counts. A test you touch that breaks these rules gets deleted, not updated. Fix or delete a flaky test.

## A stranger's machine

- No absolute path, user name or home folder layout in code or fixtures.
- No fixed language, model name or pi extension name. A missing optional extension hides its UI.
- Never commit a recorded pi RPC run. It holds the system prompt and the user's commands.

## How to

- New Tauri command: a function in `src-tauri/src/commands/<area>.rs`, one line in the command list in `lib.rs`, then regenerate the bindings with `npm run bindings`.
- New shortcut or menu item: one entry in the feature's commands, listed in `src/app/commands.ts`.
- New harness: an entry in `HARNESSES` in `harness/mod.rs`, `Agent` implemented in `crates/chord-core/src/harness/<name>/`, and one arm in `harness::start`. Set the entry's `starts` once the arm lands.
- New harness capability: a variant in `Capability`, add it to `capabilities()` for each harness that has it (`harness/pi/mod.rs` for pi, `harness/relaunch.rs` for Codex and Claude Code), check it in the view.

## Done

- `npm run fix` formats the Rust code. Run it before `npm run check`.
- `npm run check` passes. It prints one `step=<name> exit=<code> secs=<n>` line per step, the last 20 lines of each failing step, and the path of its full log. `exit=skipped` means your change touched no Rust. Never report a check you did not run.
- Run `npm run check` once, after your last code edit. A commit request alone needs no check.
- Skip visual checks: no screenshots, no launching the app to look at the UI. The user checks it.
- Commit messages: `type(area): what changed`, lower case. Types: `feat`, `fix`, `perf`, `refactor`, `test`, `docs`, `chore`. The area is a feature folder (`sidebar`, `chat`, `ios`, ...) and may be left out: `docs: add the harness contract`.

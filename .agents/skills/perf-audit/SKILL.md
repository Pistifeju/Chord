---
name: perf-audit
description: Find the free speedups in Chord, hot code that can do less work with the same observable behavior, measure them before and after, and report them. Leaves the working tree as it found it.
disable-model-invocation: true
---

An audit. You hunt **free speedups**, measure them where a run can, and write them up. The report in step 6 is your only lasting output: the working tree ends as you found it.

A speedup is free when both hold:

- **Hot**: the code runs per stream event, per keystroke, per render, per timer tick or per file event, or its work grows with a count (messages, worktrees, panes, lines of a file). Launch code is hot where it spawns a process or reads a file that grows. One click over a few items is cold.
- **Equivalent**: for every input the faster version gives the same return values, the same side effects in the same order (files, processes, events, IPC calls), the same errors, the same collection order, and the same timing as far as the user or another process can observe.

## 1. Pin the target

- The user named files, a folder or a feature: that is the target.
- Nothing named: `src`, `src-tauri/src` and `crates`.

Leave out `src/ipc/bindings.ts`, test files, and the cost of transitions and keyframes (`improve-animations` owns it).

Done when: you hold the list of files.

## 2. Map the hot paths

List every **trigger** in the target: Tauri commands, harness events on a `Channel`, app events, store subscriptions and the components that read them, timers, DOM listeners, spawned processes and threads.

Write one row per trigger: what fires it, how often, the count its work grows with, and each function it reaches down to the leaf. Follow the row across the IPC boundary: a stream event runs from pi's stdout line through the adapter, the channel, the reducer and the store into each component that renders again.

Done when: every trigger has a row, marked hot or cold with the frequency that decides it.

## 3. Sweep the hot rows

Read each function on a hot row in full, then check it against each line below. A hit becomes a candidate with its `file:line`.

**Render**

- A component renders on a store change it does not show: it calls `use()` where `usePart` returns its slice.
- A `usePart` selector builds a new object or array on each call.
- One row changed and the whole list renders again (each message on each stream chunk).
- A render repeats work whose inputs stayed the same: sort, filter, parse, format, regex.
- A fresh object, array or function on each render reaches a memoized child or an effect's dependency list.
- An effect unsubscribes and subscribes again on each render.
- An index or unstable `key` remounts rows.
- Layout reads (`getBoundingClientRect`, `offsetHeight`, `scrollHeight`) alternate with writes in a loop or a listener.

When `vite.config.ts` enables the React Compiler, drop the memoization candidates.

**State**

- A reducer walks or copies a whole collection for an event that changes one element, so a reply of n chunks costs n².
- `set` runs with the value the store holds and wakes each subscriber.
- Several subscribers derive the same value from the same store value.

**IPC**

- The frontend calls a command again for data it holds unchanged, or calls several commands where one existing command returns all of it.
- Rust emits an event whose value equals the last one it sent.
- A payload crosses the boundary through an extra round trip: a `serde_json::Value` in between, a string parsed and printed again.

**Rust**

- Two process spawns where one invocation prints both answers; a spawn inside a loop.
- `clone()`, `to_string()`, `to_owned()` or `collect()` in a loop where a borrow or an iterator serves.
- A whole file read for one part of it; the same file read or parsed twice in one call; an unbuffered read or write; a flush per line.
- A regex or other setup rebuilt on each call.
- A lock held across I/O or a process call.
- A linear search inside a loop over the same collection.
- Git, filesystem or process work on the main thread instead of `spawn_blocking`.

Done when: you read every function on every hot row and checked each line above against it.

## 4. Vet: prove each candidate equivalent

Read the code at the candidate's `file:line` again, and each caller. The candidate stays when you can write its **proof**: one or two sentences on why each observable stays the same, naming the edge inputs you checked (empty, one item, the error path, non-ASCII text, a second call while the first runs).

Move a candidate to the **not free** list when its change:

- adds a cache, a debounce, a throttle, a lazy load, a batch across ticks, a thread or other concurrency: freshness or timing changes;
- touches a type in the IPC bindings, a field of `state.json`, the harness trait or the order of events;
- changes which error surfaces, or whether a side effect runs on a failing path;
- changes the order of a collection (a `HashMap` for a `Vec`, an unstable sort);
- needs a new dependency;
- breaks a rule in `AGENTS.md`, or replaces code that a comment or doc marks as decided (a `shortcut:` line counts);
- rests on a caller you did not read.

Drop a candidate when you cannot state its **gain** as a work count read from the code: "3 `git` spawns per worktree per tick, 2 after", "copies n messages per chunk, 1 after". Give a time only when you measured it.

Done when: each candidate carries a proof, a not-free reason, or is dropped.

## 5. Measure

Run what you need: tests, scripts, benchmarks, builds, `git`. A run settles the two things reading leaves open, the gain and the proof.

1. Before the first run, record `git status --porcelain` and `git diff HEAD | shasum`.
2. Build the experiment in a `mktemp -d` folder: the before and the after version side by side, fed the same inputs. Size the inputs like the hot row (a reply of thousands of chunks, dozens of worktrees) and add the edge inputs of the proof.
3. Drive it below the UI: a pure module under `node`, a `chord-core` function under `cargo`, a command line under the shell.
4. When the experiment needs the change inside the repo, copy the file to the scratch folder first, edit, run the experiment and the tests that cover the file, then copy the file back.
5. Run each side at least 5 times and keep the median. Compare the outputs first, then the counts or times. Outputs differ: the candidate moves to not free. The gain stays inside the spread of the runs: drop it.

Done when: each free candidate is measured or names the reason no run reaches it (it needs the app window), and both recordings from line 1 match the working tree again.

## 6. Report

Rank the free speedups by how often the trigger fires times the work saved.

```
## Free speedups (count)

### 1. <what gets faster, in the user's terms>
- Where: `path:line`
- Now: <the wasted work, and how often its trigger fires>
- Change: <the edit, three lines at most>
- Gain: <work count before, work count after>
- Measured: <the command, the median before and after>, or the reason no run reaches it
- Proof: <why it is equivalent, and the edge inputs checked>

## Not free (count)
- `path:line`: <the speedup>. Would change: <the observable>.

## Checked
<hot rows, cold rows, files read>
```

"No free speedup in <target>" is a valid result: say it and list the hot rows you checked.

Close with one offer: apply finding 1.

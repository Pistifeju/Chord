# iOS: isolation, build and run


## 1. The promise

You and several agents work on one Xcode project, each task in its own worktree. A build or run in one worktree never breaks another, and never repeats work the same worktree has done.

| Shared thing | What goes wrong | Chord's answer | Decision |
|--------------|-----------------|-------------------|----------|
| Build database (`build.db`) | A second build in one DerivedData folder fails after 10 s with "database is locked" | One build per worktree: a slot for Chord's builds, a wait for builds it did not start | 3.3 |
| DerivedData | Two tools build one worktree into two folders, or with two sets of settings, and each recompiles what the other built | One folder per worktree for Chord, the agent and Xcode. Chord adds no flag and no build setting | 3.2 |
| Simulator | Two worktrees install the same bundle id on one simulator | One simulator per worktree. Agents reach it through Chord's tools | 3.4, 3.6 |

## 2. Measurements


### Build

| Case | Time | Notes |
|------|------|-------|
| Cold, own DerivedData, compilation cache off | 48.3 s | 1.1 GB, 13,599 log lines |
| Cold, compilation cache on, filling a new cache | 59.6 s | |
| Second checkout, same cache, prefix mapping on, shared module caches | 49.2 s | the log holds 21 cache hit remarks and 600 cache miss remarks |
| Second checkout, shared module caches, compilation cache off | 56.8 s | |
| Nothing changed | 4.9 s | |
| One file changed in the app target | 6.7 s | |
| One file changed in a local package | 5.5 s | |
| Build settings changed since the last build | 41.1 s | against 6.7 s for the same edit with the same settings; the failing build of the row below ran beside it for 10 s |
| Two cold builds at once, two checkouts | 87.5 s each | 48.3 s alone |
| Two cold builds at once, two checkouts, one empty module cache, SDK module folder and compilation cache between them | 105.9 s each | both succeed, no error line |
| Two builds at once in one DerivedData | the later one fails after 10.0 s, exit 65 | "unable to attach DB ... database is locked. Possibly there are two concurrent builds running in the same filesystem location." |
| `xcodebuild -list -json` | 1.3 s | |
| `xcodebuild -showBuildSettings -json` | 12.1 s the first time, 1.8 s after | the first call resolves packages |

A cold DerivedData folder holds 516 MB of build output, 275 MB of SDK modules, 147 MB of module cache, 116 MB of package checkouts and 55 MB of index. With Xcode's default location the SDK modules and the module cache sit beside the folder and every project shares them, which leaves about 690 MB per worktree.


### Simulator

iPhone 18 Pro Max on iOS 27.0, in a device set of its own.

| Case | Time | Notes |
|------|------|-------|
| `simctl create` | 0.3 s | |
| First boot of a new device | 27.5 s | boot plus `bootstatus -b` |
| Later boot | 7.1 s | |
| `simctl clone` of a device that booted once | 2.8 s | its first boot: 5.5 s |
| Shutdown | 3.3 to 3.7 s | |
| Install over the running app | 7.4 s | |
| Launch | 2.8 s | |
| `simctl io screenshot` | 0.7 s | 4.5 MB PNG |

A booted, idle simulator runs 212 processes with a 2.2 GB memory footprint. Its data folder takes 1.1 GB after the first boot. `simctl create` with iPhone 17 Pro and iOS 27.1 fails with "Incompatible device": a model and an iOS version pair up only when the runtime lists the model.

### Shell

On this machine a `PATH` entry set in the parent's environment lands at position 16 after a zsh login shell runs, behind `/usr/bin` at 6. pi runs its shell tool as `/bin/bash -c`, with no login shell, so it keeps pi's environment.

## 3. Decisions

### 3.1 The worktree path is the key

A worktree owns three things outside its folder: a DerivedData folder, a simulator and a build slot. Each derives from the worktree's path, so no table maps worktrees to caches and none can go stale. Nothing exists before the worktree's first build or run.

### 3.2 DerivedData: Xcode's own folder for the path

Chord runs `xcodebuild -project|-workspace … -scheme … -destination id=… build` and nothing else: no `-derivedDataPath`, no build setting.

- Xcode already gives each project path its own folder (section 2), so two worktrees never share one.
- Chord's ⌘R, an agent's `xcodebuild` and Xcode in one worktree land in the same folder with the same settings. A build after an agent's build takes 5 to 7 s where a second folder would cost a cold 48 s and 690 MB more.
- One rule keeps this true: every build in a worktree uses the same build settings. A changed setting cost 41.1 s. A feature that needs a setting (J2 needs each file's compile command) must make it reach every `xcodebuild` in the worktree, for example through `XCODE_XCCONFIG_FILE` in the agent's and the terminal's environment, or do without it.
- Chord starts `xcodebuild` through the user's login shell, as it starts the agent and the terminal (`login_shell` in `harness/process.rs`). Swift Build puts a task's environment into its signature, so a different `PATH` links again (source 4).

Chord finds the folder with the path hash, a pure function. Its test hardcodes the folder name Xcode gave a project at a neutral path. The built `.app` comes from one `xcodebuild -showBuildSettings -json` call after the build, with the build's project, scheme and destination: the first target of the scheme with product type `application`, at `TARGET_BUILD_DIR/FULL_PRODUCT_NAME`. A custom DerivedData location needs no special case.


### 3.3 One build per worktree

- Chord's own builds go through a slot per worktree: ⌘R and ⌘B replace the worktree's running build (kill its process group, wait for it to exit, then start). The rules are a pure module. The half where a preview build waits for ⌘R arrives with J1.
- A build Chord did not start (an agent's `xcodebuild`, Xcode) may hold `build.db`. Chord's build then waits, the log says "Waiting for another build in this worktree", and it starts when the database is free. It never kills a build it did not start.
- Builds in different worktrees start at once. Two at once took 87.5 s each against 48.3 s alone, so parallel builds add no throughput. A machine-wide queue waits until three or more agents building at once show what hurts.

### 3.4 One simulator per worktree

- The simulator lives in the default device set, so `xcodebuild`, Simulator and DeviceHub see it. Its name is `Chord <worktree folder> <6 letters of the path hash>`; Chord finds it by name, model and runtime.
- Chord creates it on the first ⌘R with `simctl create` and boots it while the build runs. The first boot (27.5 s) hides inside a cold build (48.3 s).
- A run destination is a model and iOS version that exist together among the user's simulators, so `create` cannot hit "Incompatible device". Chord's own simulators stay out of the run menu.
- After `bootstatus` Chord checks that the device reports Booted; one that does not fails the run with that reason before the install (source 5).
- An install or launch that fails with "SBMainWorkspace Busy" means a wedged simulator. The run fails and its message names the fix: erase the worktree's simulator. Chord erases nothing by itself, because the app's data goes with it.
- Chord never starts or stops a simulator it did not create.
- At most `max(1, RAM in GB / 6)` of Chord's run simulators stay booted: 3 on 18 GB, about 6.6 GB. Booting one more shuts down the one whose worktree was opened longest ago and has no run in progress. This is row H8.
- Quit shuts down every booted Chord simulator without waiting for it.

### 3.5 No cache sharing between worktrees

A second checkout with a shared compilation cache and prefix mapping took 49.2 s and its log holds 21 hit remarks against 600 miss remarks; with no sharing at all a cold build took 48.3 s. Chord builds nothing here. Try it again when an Xcode release notes cache hits across checkout paths for targets that use macros (source 2), and add it only if a second checkout builds in under half the cold time.

A new worktree pays one cold build. Cloning another worktree's DerivedData is no way around it: its dependency files and `build.db` hold the other worktree's absolute paths, and the build goes stale without an error (source 3).

### 3.6 Agents build and run through Chord

The harness adapter gives the agent tools that go through ⌘R's code: the same slot, the same DerivedData, the worktree's simulator.

- `build` (H9): returns the error lines as `file:line: message`, 30 at most, and no log. A cold build prints 13,599 lines.
- `run` (H10): build, install, launch on the worktree's simulator; the UI shows the log and the phase like a ⌘R.
- A call that runs long tells the agent it still runs and when the build last printed, every 30 s, so the agent does not stop a healthy cold build (source 5).


Chord runs one MCP server that holds `build` and `run`, and every harness reaches the tools through it:

- An ACP agent gets the server in the `mcpServers` list of `session/new`. Claude Code's adapter takes stdio, HTTP and SSE servers (`src/mcp-servers.ts`); Codex's takes stdio and HTTP, no SSE (`toCodexMcpServerConfig`, sources 6 and 7). So the server speaks stdio or HTTP.
- Codex's adapter skips a requested server whose name the user's Codex config already holds. Chord names its server `chord`.
- pi gets the tools from the extension Chord ships, which forwards each call to the same server. Chord does not rely on an MCP extension the user may lack.
- An agent the user starts outside Chord's chat, in the terminal panel or in their own terminal, gets no session from Chord and so no tools.

## 4. How ⌘R runs

1. The slot answers: start, and which build to stop first.
2. Resolve the scheme: the project's saved pick, else the scheme named like the project, else one named like a target, else the first. `-list` runs once per project and session (1.3 s), and that first time it runs before the simulator step, so a project without a scheme creates and boots nothing (source 5). Later runs know the scheme and go straight on.
3. Find or create the simulator and boot it, while step 4 builds.
4. `xcodebuild` builds. Its output goes to the build log in batches, every 100 ms or 64 KB.
5. `simctl bootstatus -b`, the Booted check, then `simctl install`.
6. The OSLog stream starts, then `simctl launch --console-pty --terminate-running-process`. Both feed the console.
7. Rust decides the run state (preparing, building, installing, launching, running, failed, stopped, exited with code) and the frontend stores it. A launch that exits non-zero before the run ends fails the run.


Install is the largest part of the warm loop. First thing to try once ⌘R works: skip the install when the build changed nothing in the `.app`.

Known gaps in step 2, each owned by a row:

- H4: ⌘B while the app runs takes the slot and ends the app; Xcode leaves it running.
- H4: a stale run's `simctl terminate` can end an app a newer ⌘R launched.
- H3: `launch` in `simulator.rs` drops the app's console output after the app starts.

A failed build lists its errors above the log. An error that names a file opens it at that line in the editor. The parser is the one the `build` tool uses.

Where each control sits and how it looks: `DESIGN.md`, Components › Build and Run. The run control and its menu sit in the window's top right corner, the run status and the booted simulators in the status bar, the build log and the console in the bottom panel as views beside the terminal, and a run marker on the worktree's card. The UI moves only for the person who asked: a failed build the user started opens the log, an agent's build opens nothing.

## 5. What grows with worktrees

| Resource | Cost per worktree | What bounds it |
|----------|-------------------|----------------|
| Disk, build | about 690 MB after a cold build | Nothing exists before the first build. Deleting a worktree deletes its folder. Prune deletes the folders of worktrees that are gone |
| Disk, simulator | 1.1 GB after the first boot | Created on the first ⌘R. Deleted with the worktree |
| Memory | 2.2 GB per booted simulator | The cap of section 3.4; the status bar lists the running ones (H5) |
| CPU | one build takes every core | One build per worktree; see section 3.3 for the queue |

Deleting a worktree through Chord removes each DerivedData folder whose `info.plist` records a `WorkspacePath` inside it; the worktree is already gone, so its project's name is unknown and the folder cannot be hashed. Prune covers worktrees deleted elsewhere: it removes each folder whose `WorkspacePath` lies in a known project's worktrees folder and is gone from disk, and each Chord simulator whose hash matches no worktree. It never touches the main checkout's folder or a simulator Chord did not create.

## 6. Code

```
crates/chord-ios/src/
  xcode.rs          the worktree's project or workspace, its schemes, the missing-tool check    step 1
  derived_data.rs   the folder Xcode gives a project path, its app, its app                      step 1
  slot.rs           which build holds a worktree's slot (pure)                                  step 1
  build.rs          one xcodebuild: arguments, process group, log batches, the lock wait        step 1
  issues.rs         the error lines of a build log (pure)                                       step 1
  simulator.rs      the worktree's simulator: find, create, boot, shut down, delete, the cap    step 2
  run.rs            install, launch, console                                                    step 2
  caches.rs         the running simulators, forget, prune, the size of the build data           step 4
  destinations.rs   run destinations from simctl and devicectl                                  step 3
src-tauri/src/commands/build.rs, simulator.rs
src/features/ios/build/         the run control and its menu, the run status, the build log, the run state rules
```

- `chord-ios` depends on `chord-core` and has no `tauri` dependency. `src-tauri` wires both: its delete command calls core, then the iOS cleanup.
- Core gains one fact: `xcode: bool` on the worktree record (`ARCHITECTURE.md`, section 9).
- The build log and the console each go over a Tauri `Channel` per worktree, like chat and the terminal.
- The run destination, the scheme and the configuration are per-project fields of `State`.
- The sidebar card, the status bar and the bottom panel's header take their iOS parts as slots from `App.tsx`, as the side panel takes its buttons. None of them imports `features/ios`.
- A file appears with its step. Step numbers are section 7's.

## 7. Order

Every control shows only in a worktree with `xcode: true`.

| Step | Rows | What works at the end | Checks |
|------|------|-----------------------|--------|
| 1 | H7, ⌘B and ⌘. of H1, the build log of H3 | ⌘B builds, the log streams, ⌘. stops | A build after an agent's `xcodebuild` in the same worktree compiles nothing. Two worktrees build at once. After ⌘. no process holds `build.db`. A machine without `xcodebuild` shows the install step |
| 2 | H1, H3, H4 | ⌘R runs the app on the worktree's simulator, the console streams | Two worktrees run their apps at once on two simulators. A second ⌘R replaces the first. A failed launch fails the run |
| 3 | H2 | Destinations in the run menu, devices above simulators | The pick survives a restart. A destination that is gone falls back and says so |
| 4 | H5, H8 | Delete and prune clean up; the status bar shows the running simulators, Settings the size of the build data; the cap holds | Deleting a worktree removes its folder and its simulator. Prune removes nothing that belongs to a live worktree |
| 5 | H9, H10 | The agent builds and runs through tools | Two agents in two worktrees run their apps at once. A build tool call returns error lines and no log. A cold build's call reports that it still runs |
| 7 | I5 | The agent looks and taps | |
| 8 | L1 to L5 | Simulator tools | |
| 9 | J1, J3, J2, J4 | Previews with hot reload | Section 3.2's settings rule holds |


This plan added H8, H9, H10 and I5 to `SPEC.md`, the `tools` capability to its section 13 and Tests to its section 14, and moved the preview half of H4 to J1.

## 8. Not verified

- Section 3.2 with a different `-destination` per tool: an agent that builds for a named simulator and a ⌘R for the worktree's may rerun some tasks.
- Section 3.2 with Xcode open on the worktree, and the relink a different `PATH` costs. Both come from source 4, not from a run here.
- A custom DerivedData or build location in Xcode's settings (`IDECustomDerivedDataLocation`). `-showBuildSettings` gives the app's path; the cleanup needs the same care.
- The path hash on an Xcode other than 27.1, and on a path with a symlink: `/tmp/…` hashed as typed, not as `/private/tmp/…`.
- The wait on a locked database: the error text is Xcode 27.1's. `lsof -t build.db` may name the holder before a build starts; nobody ran it.
- Shared module caches under parallel builds: one run passed. Whether `xcodebuild` honours `CLANG_MODULE_CACHE_PATH` from the environment comes from source 1, not from a run here.
- Every time in section 2 is one run of one project on one machine. Simulator times come from a separate device set and iOS 27.0.
- Section 3.6's MCP server: both adapters pass `mcpServers` on in their code, read on 2026-11-15; nobody ran a session with it. Stdio or HTTP is open; H9 decides it.

## Sources

2. https://zenn.dev/sryu/articles/worktree-compilation-cache: prefix mapping, and macros that block cache hits (swift-build PR 1836).
4. https://sergdort.github.io/xcode-agent-incremental-builds/: one DerivedData for Xcode and an agent, and the relink a different `PATH` causes.
6. https://github.com/agentclientprotocol/claude-agent-acp: `src/mcp-servers.ts` and `src/v1/acp-agent.ts` hand the `session/new` MCP servers to the Claude Agent SDK.
7. https://github.com/agentclientprotocol/codex-acp: `src/McpServerConfig.ts` and `src/CodexAcpClient.ts` write the `session/new` MCP servers into Codex's `mcp_servers` config.

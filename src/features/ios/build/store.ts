// Connects the build rules to IPC: each worktree's build, run and log, started and stopped, and the bottom panel's Build and Console views.
import { Channel } from "@tauri-apps/api/core";
import { commands, type BuildEvent_Deserialize } from "../../../ipc/bindings";
import { externalStore } from "../../../ui/externalStore";
import { bottomPanelOf, bottomPanels, changeBottomPanel } from "../../bottompanel/store";
import { fail, projects } from "../../worktrees/store";
import { emptyConsole, type ConsoleLine } from "./consoleLog";
import { emptyLog, type Log } from "./log";
import { appliedEvent, noBuild, seenEnd, viewAfterEvent, viewToShow, type Build, type BuildAndLog } from "./run";

/** How long a success or a stop shows on the worktree's card. */
const END_MARKER_MS = 5000;

export const builds = externalStore<Record<string, Build>>({});
/** Apart from the builds, since the log changes every 100 ms while the rest changes a few times per build. */
export const buildLogs = externalStore<Record<string, Log>>({});
/** The running app's output, apart from the builds for the same reason. */
export const consoles = externalStore<Record<string, ConsoleLine[]>>({});

/** The newest build's number per worktree: a replaced build's late events are ignored. */
const latest = new Map<string, number>();

function apply(worktree: string, update: (state: BuildAndLog) => BuildAndLog) {
  const before = { build: builds.get()[worktree], log: buildLogs.get()[worktree] ?? emptyLog, console: consoles.get()[worktree] ?? emptyConsole };
  const after = update(before);
  if (after.build !== before.build) {
    const all = { ...builds.get() };
    if (after.build) all[worktree] = after.build;
    else delete all[worktree];
    builds.set(all);
  }
  if (after.log !== before.log) buildLogs.set({ ...buildLogs.get(), [worktree]: after.log });
  if (after.console !== before.console) consoles.set({ ...consoles.get(), [worktree]: after.console });
}

const seeEnd = (worktree: string) => apply(worktree, (state) => (state.build ? { ...state, build: seenEnd(state.build) } : state));

// Going to a worktree shows the user its success or stop.
projects.subscribe(() => {
  const { selected } = projects.get();
  if (selected) seeEnd(selected);
});

export function startBuild(worktree: string, run: boolean) {
  const number = (latest.get(worktree) ?? 0) + 1;
  latest.set(worktree, number);
  const current = () => latest.get(worktree) === number;
  apply(worktree, () => noBuild);
  const channel = new Channel<BuildEvent_Deserialize>();
  channel.onmessage = (event) => {
    if (!current()) return;
    apply(worktree, (state) => appliedEvent(state, event, Date.now()));
    const view = bottomPanelOf(bottomPanels.get(), worktree).view;
    const next = viewAfterEvent(view, event);
    if (next !== view) changeBottomPanel(worktree, { view: next });
    if (event.kind === "succeeded" || event.kind === "stopped") setTimeout(() => current() && seeEnd(worktree), END_MARKER_MS);
  };
  // The log shows beside the terminal, and alone while the terminal is hidden.
  changeBottomPanel(worktree, { view: "build" });
  commands.startBuild(worktree, run, channel).catch(fail);
}

export const stopBuild = (worktree: string) => commands.stopBuild(worktree).catch(fail);

export function toggleLogView(worktree: string) {
  const panel = bottomPanelOf(bottomPanels.get(), worktree);
  changeBottomPanel(worktree, { view: panel.view === "terminal" ? viewToShow(builds.get()[worktree]) : "terminal" });
}

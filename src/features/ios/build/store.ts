import { Channel } from "@tauri-apps/api/core";
import { commands, events, type BuildEvent_Deserialize } from "../../../ipc/bindings";
import { externalStore } from "../../../ui/externalStore";
import { bottomPanelOf, bottomPanels, changeBottomPanel } from "../../bottompanel/store";
import { openSideView } from "../../sidepanel/store";
import { projects } from "../../worktrees/store";
import { fail } from "../../../ui/alert";
import { runDestinationOf } from "./destinationStore";
import { schemePickOf } from "./schemeStore";
import { emptyConsole, type ConsoleLine } from "./consoleLog";
import { appendToLog, emptyLog, type Log } from "./log";
import { appliedEvent, appliedSideEvent, isRunning, noBuild, seenEnd, viewAfterEvent, viewToShow, type Build, type BuildAndLog } from "./run";

const END_MARKER_MS = 5000;

export const builds = externalStore<Record<string, Build>>({});
export const buildLogs = externalStore<Record<string, Log>>({});
export const consoles = externalStore<Record<string, ConsoleLine[]>>({});

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

projects.subscribe(() => {
  const { selected } = projects.get();
  if (selected) seeEnd(selected);
});

const latestSide = new Map<string, number>();

function startBuildBesideApp(worktree: string, agentRequest: number | null) {
  const number = (latestSide.get(worktree) ?? 0) + 1;
  latestSide.set(worktree, number);
  const channel = new Channel<BuildEvent_Deserialize>();
  channel.onmessage = (event) => {
    if (latestSide.get(worktree) !== number) return;
    apply(worktree, (state) => appliedSideEvent(state, event, Date.now()));
  };
  apply(worktree, (state) => appliedSideEvent({ ...state, log: emptyLog, build: state.build && { ...state.build, side: undefined } }, { kind: "phase", phase: "preparing" }, Date.now()));
  changeBottomPanel(worktree, { view: "build" });
  commands.startBuild(worktree, false, runDestinationOf(worktree), schemePickOf(worktree), agentRequest, channel).catch(fail);
}

export function startBuild(worktree: string, run: boolean, agentRequest: number | null = null) {
  if (!run && isRunning(builds.get()[worktree])) return startBuildBesideApp(worktree, agentRequest);
  latestSide.delete(worktree);
  const number = (latest.get(worktree) ?? 0) + 1;
  latest.set(worktree, number);
  const current = () => latest.get(worktree) === number;
  apply(worktree, () => noBuild);
  const channel = new Channel<BuildEvent_Deserialize>();
  channel.onmessage = (event) => {
    if (!current()) return;
    if (event.kind === "runs_on_simulator") openSideView(worktree, "simulator");
    apply(worktree, (state) => appliedEvent(state, event, Date.now()));
    const view = bottomPanelOf(bottomPanels.get(), worktree).view;
    const next = viewAfterEvent(view, event);
    if (next !== view) changeBottomPanel(worktree, { view: next });
    if (event.kind === "succeeded" || event.kind === "stopped") setTimeout(() => current() && projects.get().selected === worktree && seeEnd(worktree), END_MARKER_MS);
  };
  changeBottomPanel(worktree, { view: "build" });
  const destination = runDestinationOf(worktree);
  const target = agentRequest !== null && run && destination?.kind === "device" ? null : destination;
  commands.startBuild(worktree, run, target, schemePickOf(worktree), agentRequest, channel).catch(fail);
}

export const runApp = (worktree: string) => startBuild(worktree, true);

export function watchAgentBuilds(): () => void {
  const unlisten = events.agentBuildRequested.listen(({ payload }) => {
    startBuild(payload.worktree, payload.run, payload.request);
  });
  return () => void unlisten.then((off) => off());
}

export const stopBuild = (worktree: string) => commands.stopBuild(worktree).catch(fail);

export function cleanBuild(worktree: string) {
  latest.set(worktree, (latest.get(worktree) ?? 0) + 1);
  latestSide.delete(worktree);
  apply(worktree, () => noBuild);
  changeBottomPanel(worktree, { view: "build" });
  commands
    .cleanBuild(worktree)
    .then(() => apply(worktree, (state) => ({ ...state, log: appendToLog(state.log, "Clean Finished\n") })))
    .catch(fail);
}

export const showBuildLog = (worktree: string) => changeBottomPanel(worktree, { view: "build" });

export function toggleLogView(worktree: string) {
  const panel = bottomPanelOf(bottomPanels.get(), worktree);
  changeBottomPanel(worktree, { view: panel.view === "terminal" ? viewToShow(builds.get()[worktree]) : "terminal" });
}

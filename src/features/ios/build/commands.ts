// The build's commands for the commands table: run, build, clean, stop, show the errors, show or hide the Build or Console view, and find in it.
import type { AppCommand } from "../../../app/commands";
import { onTarget } from "../../../app/commandTarget";
import type { BottomViewSlot } from "../../bottompanel/BottomPanel";
import { xcodeWorktree } from "../../worktrees/store";
import { findsOf } from "./find";
import { toggleErrorList } from "./errorListStore";
import { failedBuild, isStarting, isRunning } from "./run";
import { builds, cleanBuild, runApp, startBuild, stopBuild, toggleLogView } from "./store";

// The worktree whose build or app runs.
const busyWorktree = (): string | null => {
  const worktree = xcodeWorktree();
  const build = worktree ? builds.get()[worktree] : undefined;
  return isStarting(build) || isRunning(build) ? worktree : null;
};

// The worktree whose last build failed with errors to list.
const failedWorktree = (): string | null => {
  const worktree = xcodeWorktree();
  const failed = worktree ? failedBuild(builds.get()[worktree]) : null;
  return failed && failed.issues.length > 0 ? worktree : null;
};

type LogFocus = { worktree: string; view: BottomViewSlot["id"] };

// The worktree whose Build or Console view has the focus, and which of the two.
const focusedLogView = (): LogFocus | null => {
  const worktree = xcodeWorktree();
  const view = document.activeElement?.closest<HTMLElement>(".log-part")?.dataset.view;
  return worktree && (view === "build" || view === "console") ? { worktree, view } : null;
};

const findingLogView = (): LogFocus | null => {
  const focused = focusedLogView();
  return focused && findsOf[focused.view].finds.get()[focused.worktree] ? focused : null;
};

export const buildCommands: AppCommand[] = [
  { id: "build.run", title: "Run", shortcut: { key: "r", cmd: true }, ...onTarget(xcodeWorktree, runApp) },
  { id: "build.start", title: "Build", shortcut: { key: "b", cmd: true }, ...onTarget(xcodeWorktree, (worktree) => startBuild(worktree, false)) },
  { id: "build.clean", title: "Clean Build Folder", shortcut: { key: "k", cmd: true, shift: true }, ...onTarget(xcodeWorktree, cleanBuild) },
  { id: "build.stop", title: "Stop", shortcut: { key: ".", cmd: true }, ...onTarget(busyWorktree, stopBuild) },
  { id: "build.showErrors", title: "Show Build Errors", shortcut: { key: "'", cmd: true }, ...onTarget(failedWorktree, toggleErrorList) },
  { id: "build.toggleLogView", title: "Show or Hide Build Log or Console", shortcut: { key: "y", cmd: true, shift: true }, ...onTarget(xcodeWorktree, toggleLogView) },
  { id: "build.find", title: "Find in Build Log or Console", shortcut: { key: "f", cmd: true }, ...onTarget(focusedLogView, ({ worktree, view }) => findsOf[view].open(worktree)) },
  { id: "build.findClose", title: "Close Find in Build Log or Console", shortcut: { key: "escape" }, ...onTarget(findingLogView, ({ worktree, view }) => findsOf[view].close(worktree)) },
];

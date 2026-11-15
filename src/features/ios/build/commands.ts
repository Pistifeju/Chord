// The build's commands for the commands table: run, build, stop, show or hide the Build view, and find in it.
import type { AppCommand } from "../../../app/commands";
import { settingsOpen } from "../../settings/store";
import { projects } from "../../worktrees/store";
import { worktreeAt } from "../../worktrees/projects";
import { closeBuildFind, openBuildFind, buildFinds } from "./find";
import { isStarting, isRunning } from "./run";
import { builds, startBuild, stopBuild, toggleBuildLog } from "./store";

// The selected worktree while it has an Xcode project and no sheet-like page covers it.
const xcodeWorktree = (): string | null => {
  const { selected } = projects.get();
  return !settingsOpen.get() && worktreeAt(projects.get(), selected)?.xcode ? selected : null;
};

// The worktree whose Build view has the focus.
const focusedBuildView = (): string | null => {
  const worktree = xcodeWorktree();
  return worktree && document.activeElement?.closest(".build-part") ? worktree : null;
};

export const buildCommands: AppCommand[] = [
  { id: "build.run", title: "Run", shortcut: { key: "r", cmd: true }, when: () => xcodeWorktree() !== null, run: () => startBuild(xcodeWorktree()!, true) },
  { id: "build.start", title: "Build", shortcut: { key: "b", cmd: true }, when: () => xcodeWorktree() !== null, run: () => startBuild(xcodeWorktree()!, false) },
  {
    id: "build.stop",
    title: "Stop",
    shortcut: { key: ".", cmd: true },
    when: () => {
      const worktree = xcodeWorktree();
      const build = worktree ? builds.get()[worktree] : undefined;
      return isStarting(build) || isRunning(build);
    },
    run: () => stopBuild(xcodeWorktree()!),
  },
  { id: "build.toggleLog", title: "Show or Hide Build Log", shortcut: { key: "y", cmd: true, shift: true }, when: () => xcodeWorktree() !== null, run: () => toggleBuildLog(xcodeWorktree()!) },
  {
    id: "build.find",
    title: "Find in Build Log",
    shortcut: { key: "f", cmd: true },
    when: () => focusedBuildView() !== null,
    run: () => {
      const worktree = focusedBuildView();
      if (worktree) openBuildFind(worktree);
    },
  },
  {
    id: "build.findClose",
    title: "Close Find in Build Log",
    shortcut: { key: "escape" },
    when: () => {
      const worktree = focusedBuildView();
      return worktree !== null && buildFinds.get()[worktree] !== undefined;
    },
    run: () => {
      const worktree = focusedBuildView();
      if (worktree) closeBuildFind(worktree);
    },
  },
];

// The build's commands for the commands table: run, build, stop, show or hide the Build or Console view, and find in it.
import type { AppCommand } from "../../../app/commands";
import type { BottomViewSlot } from "../../bottompanel/BottomPanel";
import { settingsOpen } from "../../settings/store";
import { projects } from "../../worktrees/store";
import { worktreeAt } from "../../worktrees/projects";
import { findsOf } from "./find";
import { isStarting, isRunning } from "./run";
import { builds, startBuild, stopBuild, toggleLogView } from "./store";

// The selected worktree while it has an Xcode project and no sheet-like page covers it.
const xcodeWorktree = (): string | null => {
  const { selected } = projects.get();
  return !settingsOpen.get() && worktreeAt(projects.get(), selected)?.xcode ? selected : null;
};

// The worktree whose Build or Console view has the focus, and which of the two.
const focusedLogView = (): { worktree: string; view: BottomViewSlot["id"] } | null => {
  const worktree = xcodeWorktree();
  const view = document.activeElement?.closest<HTMLElement>(".log-part")?.dataset.view;
  return worktree && (view === "build" || view === "console") ? { worktree, view } : null;
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
  { id: "build.toggleLogView", title: "Show or Hide Build Log or Console", shortcut: { key: "y", cmd: true, shift: true }, when: () => xcodeWorktree() !== null, run: () => toggleLogView(xcodeWorktree()!) },
  {
    id: "build.find",
    title: "Find in Build Log or Console",
    shortcut: { key: "f", cmd: true },
    when: () => focusedLogView() !== null,
    run: () => {
      const focused = focusedLogView();
      if (focused) findsOf[focused.view].open(focused.worktree);
    },
  },
  {
    id: "build.findClose",
    title: "Close Find in Build Log or Console",
    shortcut: { key: "escape" },
    when: () => {
      const focused = focusedLogView();
      return focused !== null && findsOf[focused.view].finds.get()[focused.worktree] !== undefined;
    },
    run: () => {
      const focused = focusedLogView();
      if (focused) findsOf[focused.view].close(focused.worktree);
    },
  },
];

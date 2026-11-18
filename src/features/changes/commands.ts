// The Changes view's commands for the commands table: show or hide the view, and the right-click actions of a row or the picked rows: stage and discard under Changes, unstage and discard under Staged.
import type { AppCommand } from "../../app/commands";
import { onTarget } from "../../app/commandTarget";
import type { Change } from "../../ipc/bindings";
import { toggleSideView } from "../sidepanel/store";
import { openWorktree, projects } from "../worktrees/store";
import { changes, requestDiscard, stageChanges, unstageChanges } from "./store";

/** A menu's target for the rows it acts on: their paths, NUL apart, since a path holds any other character. */
export const changesTarget = (list: Change[]) => list.map((c) => c.path).join("\0");

// Runs `act` on the open worktree's changes at the menu's paths, in the group the command acts on.
const onChanges = (staged: boolean, act: (worktree: string, list: Change[]) => void) => (target?: string) => {
  const worktree = projects.get().selected;
  const paths = new Set(target?.split("\0"));
  const list = worktree ? (changes.get()[worktree] ?? []).filter((c) => c.staged === staged && paths.has(c.path)) : [];
  if (worktree && list.length > 0) act(worktree, list);
};

export const changesCommands: AppCommand[] = [
  { id: "changes.toggle", title: "Show or Hide Changes", shortcut: { key: "g", cmd: true, shift: true }, ...onTarget(openWorktree, (worktree) => toggleSideView(worktree, "changes")) },
  { id: "changes.stage", title: "Stage", run: onChanges(false, (worktree, list) => void stageChanges(worktree, list)) },
  { id: "changes.discard", title: "Discard", run: onChanges(false, requestDiscard) },
  { id: "changes.unstage", title: "Unstage", run: onChanges(true, (worktree, list) => void unstageChanges(worktree, list)) },
  { id: "changes.discardStaged", title: "Discard", run: onChanges(true, requestDiscard) },
];

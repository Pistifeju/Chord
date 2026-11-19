// The Changes view's commands for the commands table: show or hide the view, the right-click actions of a row or the picked rows: stage and discard under Changes, unstage and discard under Staged, and a stash's apply, pop and drop.
import type { AppCommand } from "../../app/commands";
import { onTarget } from "../../app/commandTarget";
import type { Change, Stash } from "../../ipc/bindings";
import { toggleSideView } from "../sidepanel/store";
import { openWorktree, projects } from "../worktrees/store";
import { actOnStash, changes, requestDiscard, requestDropStash, stageChanges, stashes, unstageChanges } from "./store";

/** A menu's target for the rows it acts on: their paths, NUL apart, since a path holds any other character. */
export const changesTarget = (list: Change[]) => list.map((c) => c.path).join("\0");

// Runs `act` on the open worktree's changes at the menu's paths, in the group the command acts on.
const onChanges = (staged: boolean, act: (worktree: string, list: Change[]) => void) => (target?: string) => {
  const worktree = projects.get().selected;
  const paths = new Set(target?.split("\0"));
  const list = worktree ? (changes.get()[worktree] ?? []).filter((c) => c.staged === staged && paths.has(c.path)) : [];
  if (worktree && list.length > 0) act(worktree, list);
};

// Runs `act` on the open worktree's stash whose hash is the menu's target.
const onStash = (act: (worktree: string, stash: Stash) => void) => (target?: string) => {
  const worktree = projects.get().selected;
  const stash = worktree && stashes.get()[worktree]?.find((s) => s.hash === target);
  if (worktree && stash) act(worktree, stash);
};

export const changesCommands: AppCommand[] = [
  { id: "changes.toggle", title: "Show or Hide Changes", shortcut: { key: "g", cmd: true, shift: true }, ...onTarget(openWorktree, (worktree) => toggleSideView(worktree, "changes")) },
  { id: "changes.stage", title: "Stage", run: onChanges(false, (worktree, list) => void stageChanges(worktree, list)) },
  { id: "changes.discard", title: "Discard", run: onChanges(false, requestDiscard) },
  { id: "changes.unstage", title: "Unstage", run: onChanges(true, (worktree, list) => void unstageChanges(worktree, list)) },
  { id: "changes.discardStaged", title: "Discard", run: onChanges(true, requestDiscard) },
  { id: "changes.applyStash", title: "Apply", run: onStash((worktree, stash) => void actOnStash(worktree, stash.hash, "apply")) },
  { id: "changes.popStash", title: "Pop", run: onStash((worktree, stash) => void actOnStash(worktree, stash.hash, "pop")) },
  { id: "changes.dropStash", title: "Drop", run: onStash(requestDropStash) },
];

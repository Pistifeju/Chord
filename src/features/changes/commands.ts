import type { AppCommand } from "../../app/commands";
import { onTarget } from "../../app/commandTarget";
import type { Change, Stash } from "../../ipc/bindings";
import { toggleNavigator } from "../sidebar/store";
import { requestDeleteBranch } from "../worktrees/localBranches";
import { startBranchRename } from "../worktrees/rename";
import { gitWorktree, projects } from "../worktrees/store";
import { branchMenuOpen, closeBranchMenu } from "./branch";
import { actOnStash, changes, requestDiscard, requestDropStash, stageChanges, stashes, unstageChanges } from "./store";

export const changesTarget = (list: Change[]) => list.map((c) => c.path).join("\0");

const onChanges = (staged: boolean, act: (worktree: string, list: Change[]) => void) => (target?: string) => {
  const worktree = projects.get().selected;
  const paths = new Set(target?.split("\0"));
  const list = worktree ? (changes.get()[worktree] ?? []).filter((c) => c.staged === staged && paths.has(c.path)) : [];
  if (worktree && list.length > 0) act(worktree, list);
};

const onStash = (act: (worktree: string, stash: Stash) => void) => (target?: string) => {
  const worktree = projects.get().selected;
  const stash = worktree && stashes.get()[worktree]?.find((s) => s.hash === target);
  if (worktree && stash) act(worktree, stash);
};

const onBranch = (act: (worktree: string, branch: string) => void) => (target?: string) => {
  const worktree = projects.get().selected;
  closeBranchMenu();
  if (worktree && target) act(worktree, target);
};

export const changesCommands: AppCommand[] = [
  { id: "changes.toggle", title: "Show or Hide Changes", shortcut: { key: "g", cmd: true, shift: true }, ...onTarget(gitWorktree, () => toggleNavigator("changes")) },
  { id: "changes.stage", title: "Stage", run: onChanges(false, (worktree, list) => void stageChanges(worktree, list)) },
  { id: "changes.discard", title: "Discard", run: onChanges(false, requestDiscard) },
  { id: "changes.unstage", title: "Unstage", run: onChanges(true, (worktree, list) => void unstageChanges(worktree, list)) },
  { id: "changes.discardStaged", title: "Discard", run: onChanges(true, requestDiscard) },
  { id: "changes.applyStash", title: "Apply", run: onStash((worktree, stash) => void actOnStash(worktree, stash.hash, "apply")) },
  { id: "changes.popStash", title: "Pop", run: onStash((worktree, stash) => void actOnStash(worktree, stash.hash, "pop")) },
  { id: "changes.dropStash", title: "Drop", run: onStash(requestDropStash) },
  { id: "branch.rename", title: "Rename Branch", run: onBranch(startBranchRename) },
  { id: "branch.delete", title: "Delete Branch", run: onBranch(requestDeleteBranch) },
  { id: "changes.branchMenuClose", title: "Close Branch Menu", shortcut: { key: "escape" }, when: () => branchMenuOpen.get(), run: closeBranchMenu },
];

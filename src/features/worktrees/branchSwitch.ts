// The switch of a worktree to another local branch, and the reads a worktree's new branch changes: the cards' branches, their git counts and the open Files tree.
import { commands } from "../../ipc/bindings";
import { fail } from "../../ui/alert";
import { reloadFiles, trees } from "../files/store";
import { refreshGitCounts } from "./gitCountsRefresh";
import { ownerOf } from "./projects";
import { projects, refreshWorktrees } from "./store";

export async function switchBranch(worktree: string, branch: string) {
  try {
    await commands.switchBranch(worktree, branch);
  } catch (error) {
    fail(error);
    return;
  }
  await afterSwitch(worktree);
}

/** Reads again what shows a worktree's branch, once the worktree moved to another one. */
export async function afterSwitch(worktree: string) {
  const project = ownerOf(projects.get(), worktree);
  if (project) await refreshWorktrees(project).catch(fail);
  refreshGitCounts();
  if (trees.get()[worktree]) await reloadFiles(worktree);
}

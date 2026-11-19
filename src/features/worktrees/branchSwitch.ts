// The switch of a worktree to another local branch, then the reads it changes: the cards' branches, their git counts and the open Files tree.
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
  const project = ownerOf(projects.get(), worktree);
  if (project) await refreshWorktrees(project).catch(fail);
  refreshGitCounts();
  if (trees.get()[worktree]) await reloadFiles(worktree);
}

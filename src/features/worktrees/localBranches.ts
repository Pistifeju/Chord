// The delete of a local branch from the Changes view's branch menu: the sheets that ask first and the git call.
import { closeSheet, openSheet, sheet, sheetOf } from "../../app/sheet";
import { commands } from "../../ipc/bindings";
import { fail } from "../../ui/alert";

export const requestDeleteBranch = (worktree: string, branch: string) => openSheet({ kind: "deleteBranch", worktree, branch, unmerged: false });

// A branch git calls unmerged asks a second time, then goes with `-D`.
export async function confirmDeleteBranch() {
  const pending = sheetOf(sheet.get(), "deleteBranch");
  if (!pending) return;
  closeSheet("deleteBranch");
  try {
    if ((await commands.deleteBranch(pending.worktree, pending.branch, pending.unmerged)) === "Unmerged") openSheet({ ...pending, unmerged: true });
  } catch (error) {
    fail(error);
  }
}

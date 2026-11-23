// The New Branch sheet's form: the worktree that moves to the new branch, the typed name's live git check, and the create on Return.
import { closeSheet, openSheet, sheet } from "../../app/sheet";
import { commands } from "../../ipc/bindings";
import { externalStore } from "../../ui/externalStore";
import { fail } from "../../ui/alert";
import { afterSwitch } from "./branchSwitch";
import { nameCheck } from "./nameCheck";
import { branchName, ownerOf, worktreeAt } from "./projects";
import { projects } from "./store";

type NewBranch = {
  worktree: string;
  project: string;
  // the branch the new one starts from; null on a detached HEAD
  from: string | null;
  name: string;
  // null while the name is empty, being checked, or valid; also holds git's refusal of the create
  problem: string | null;
  creating: boolean;
};

export const newBranch = externalStore<NewBranch | null>(null);

const update = (change: Partial<NewBranch>) => {
  const current = newBranch.get();
  if (current) newBranch.set({ ...current, ...change });
};
const check = nameCheck();

export function openNewBranch(worktree: string) {
  const project = ownerOf(projects.get(), worktree);
  if (!project) return;
  check.drop();
  newBranch.set({ worktree, project, from: worktreeAt(projects.get(), worktree)?.branch ?? null, name: "", problem: null, creating: false });
  openSheet({ kind: "newBranch" });
}

export function closeNewBranch() {
  check.drop();
  newBranch.set(null);
  closeSheet("newBranch");
}

export async function setNewBranchName(name: string) {
  const form = newBranch.get();
  if (!form) return;
  update({ name, problem: null });
  const typed = branchName(name);
  if (!typed) return check.drop();
  await check.run(() => commands.checkNewBranch(form.project, typed), (problem) => update({ problem }));
}

// Return during the live check creates anyway: git checks the name again.
export async function createBranch() {
  const form = newBranch.get();
  if (!form || form.creating || form.problem) return;
  const name = branchName(form.name);
  if (!name) return;
  check.drop();
  update({ creating: true });
  try {
    await commands.createBranch(form.worktree, name);
  } catch (error) {
    if (sheet.get()?.kind === "newBranch") update({ creating: false, problem: String(error) });
    else fail(error);
    return;
  }
  closeNewBranch();
  await afterSwitch(form.worktree);
}

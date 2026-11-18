// The worktree whose branch name shows as a field on its card, the typed name's live git check, and the rename on Return.
import { commands } from "../../ipc/bindings";
import { externalStore } from "../../ui/externalStore";
import { fail } from "../../ui/alert";
import { nameCheck } from "./nameCheck";
import { branchName, ownerOf, worktreeAt } from "./projects";
import { projects, refreshWorktrees } from "./store";

type Rename = {
  project: string;
  path: string;
  name: string;
  // null while the name is empty, being checked, or valid; also holds git's refusal of the rename
  problem: string | null;
  renaming: boolean;
};

export const rename = externalStore<Rename | null>(null);

const update = (change: Partial<Rename>) => {
  const current = rename.get();
  if (current) rename.set({ ...current, ...change });
};
const check = nameCheck();

export function startRename(path?: string) {
  const project = path ? ownerOf(projects.get(), path) : undefined;
  const branch = path ? worktreeAt(projects.get(), path)?.branch : undefined;
  if (!path || !project || path === project) return;
  check.drop();
  rename.set({ project, path, name: branch ?? "", problem: branch ? null : "A detached HEAD has no branch to rename", renaming: false });
}

// A rename already sent finishes; leaving the field only drops the field.
export const cancelRename = () => {
  check.drop();
  rename.set(null);
};

export async function setRenameName(name: string) {
  update({ name, problem: null });
  const form = rename.get();
  const typed = branchName(name);
  if (!form || !typed) return check.drop();
  await check.run(() => commands.checkBranchRename(form.project, form.path, typed), (problem) => update({ problem }));
}

// Return during the live check renames anyway: Rust checks the name again before `git branch -m`.
export async function finishRename() {
  const form = rename.get();
  if (!form || form.renaming || form.problem) return;
  const name = branchName(form.name);
  if (!name) return;
  if (name === worktreeAt(projects.get(), form.path)?.branch) return rename.set(null);
  check.drop();
  update({ renaming: true });
  const open = () => rename.get()?.path === form.path;
  try {
    await commands.renameBranch(form.project, form.path, name);
  } catch (error) {
    if (open()) update({ renaming: false, problem: String(error) });
    else fail(error);
    return;
  }
  if (open()) rename.set(null);
  await refreshWorktrees(form.project).catch(fail);
}

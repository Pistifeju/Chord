import { closeSheet, openSheet } from "../../app/sheet";
import { commands } from "../../ipc/bindings";
import { externalStore } from "../../ui/externalStore";
import { fail } from "../../ui/alert";
import { nameCheck } from "./nameCheck";
import { branchName, ownerOf, worktreeAt } from "./projects";
import { projects, refreshWorktrees } from "./store";

type Rename = {
  project: string;
  path: string | null;
  branch: string | null;
  name: string;
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
  rename.set({ project, path, branch: branch ?? null, name: branch ?? "", problem: branch ? null : "A detached HEAD has no branch to rename", renaming: false });
}

export function startBranchRename(worktree: string, branch: string) {
  const project = ownerOf(projects.get(), worktree);
  if (!project) return;
  check.drop();
  rename.set({ project, path: null, branch, name: branch, problem: null, renaming: false });
  openSheet({ kind: "renameBranch" });
}

const close = () => {
  rename.set(null);
  closeSheet("renameBranch");
};

export const cancelRename = () => {
  check.drop();
  close();
};

export async function setRenameName(name: string) {
  const form = rename.get();
  if (!form?.branch) return update({ name });
  update({ name, problem: null });
  const typed = branchName(name);
  if (!typed) return check.drop();
  const { project, branch } = form;
  await check.run(() => commands.checkBranchRename(project, branch, typed), (problem) => update({ problem }));
}

export async function finishRename() {
  const form = rename.get();
  if (!form?.branch || form.renaming || form.problem) return;
  const name = branchName(form.name);
  if (!name) return;
  if (name === form.branch) return close();
  check.drop();
  update({ renaming: true });
  const open = () => rename.get()?.path === form.path;
  try {
    await commands.renameBranch(form.project, form.branch, name);
  } catch (error) {
    if (open()) update({ renaming: false, problem: String(error) });
    else fail(error);
    return;
  }
  if (open()) close();
  await refreshWorktrees(form.project).catch(fail);
}

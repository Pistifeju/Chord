// The New Worktree popup's state: open flag, project, branch name and its live git check.
import { commands } from "../../ipc/bindings";
import { externalStore } from "../../ui/externalStore";
import { branchName, canCreate, defaultProject } from "./projects";
import { fail, projects, refreshWorktrees, selectWorktree } from "./store";

export type NewWorktree = {
  open: boolean;
  project: string | null;
  branch: string;
  // null while the name is empty, being checked, or valid
  problem: string | null;
  checked: boolean;
  creating: boolean;
  // set only when git refused the create itself
  createError: string | null;
};

const closed: NewWorktree = { open: false, project: null, branch: "", problem: null, checked: false, creating: false, createError: null };

export const newWorktree = externalStore(closed);

const update = (change: Partial<NewWorktree>) => newWorktree.set({ ...newWorktree.get(), ...change });
let latestCheck = 0;
// Bumped on every open and close, so a create that outlives its popup cannot touch the next one.
let session = 0;

export function openNewWorktree() {
  const project = defaultProject(projects.get());
  if (!project) return;
  session++;
  newWorktree.set({ ...closed, open: true, project });
}

export function closeNewWorktree() {
  session++;
  newWorktree.set(closed);
}

async function check() {
  const { project, branch: typed } = newWorktree.get();
  const branch = branchName(typed);
  const id = ++latestCheck;
  if (!project || !branch) return update({ problem: null, checked: false });
  try {
    const problem = await commands.checkBranchName(project, branch);
    if (id === latestCheck) update({ problem, checked: true });
  } catch (error) {
    if (id === latestCheck) update({ problem: String(error), checked: true });
  }
}

export function setBranch(branch: string) {
  update({ branch, createError: null });
  void check();
}

export function setProject(project: string) {
  update({ project, createError: null });
  void check();
}

export async function createWorktree() {
  const form = newWorktree.get();
  if (!canCreate(form)) return;
  const { project } = form;
  const branch = branchName(form.branch);
  const mine = session;
  update({ creating: true, createError: null });
  let path: string;
  try {
    path = (await commands.createWorktree(project!, branch)).path;
  } catch (error) {
    if (mine === session) update({ creating: false, createError: String(error) });
    return;
  }
  await refreshWorktrees(project!).catch(fail);
  selectWorktree(path);
  if (mine === session) closeNewWorktree();
}

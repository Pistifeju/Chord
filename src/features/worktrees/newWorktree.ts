import { commands } from "../../ipc/bindings";
import { closeSheet, openSheet } from "../../app/sheet";
import { externalStore } from "../../ui/externalStore";
import { startFocused } from "../panes/store";
import { nameCheck } from "./nameCheck";
import { branchName, canCreate, defaultProject, isGitProject } from "./projects";
import { projects, refreshWorktrees, selectWorktree } from "./store";
import { fail, notify } from "../../ui/alert";

type NewWorktree = {
  project: string | null;
  branch: string;
  problem: string | null;
  checked: boolean;
  creating: boolean;
  createError: string | null;
};

const closed: NewWorktree = { project: null, branch: "", problem: null, checked: false, creating: false, createError: null };

export const newWorktree = externalStore(closed);

const update = (change: Partial<NewWorktree>) => newWorktree.set({ ...newWorktree.get(), ...change });
const branchCheck = nameCheck();
let session = 0;

export function openNewWorktree(target?: string) {
  const state = projects.get();
  const preferred = target ?? defaultProject(state);
  const project = preferred && isGitProject(state, preferred) ? preferred : state.list.find((p) => isGitProject(state, p.path))?.path;
  if (!project) {
    if (state.list.length > 0) notify("New worktrees need a git project. Add one, or run git init in this folder.");
    return;
  }
  session++;
  newWorktree.set({ ...closed, project });
  openSheet({ kind: "newWorktree" });
}

export function closeNewWorktree() {
  session++;
  newWorktree.set(closed);
  closeSheet("newWorktree");
}

async function check() {
  const { project, branch: typed } = newWorktree.get();
  const branch = branchName(typed);
  if (!project || !branch) {
    branchCheck.drop();
    return update({ problem: null, checked: false });
  }
  await branchCheck.run(() => commands.checkBranchName(project, branch), (problem) => update({ problem, checked: true }));
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
  void startFocused(path);
  if (mine === session) closeNewWorktree();
}

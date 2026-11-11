// Connects the project rules to IPC: load, add by dialog or drop, remove, select, delete a worktree.
import { commands, type Project } from "../../ipc/bindings";
import { externalStore } from "../../ui/externalStore";
import { sidebar, toggleFold } from "../sidebar/store";
import { noProjects, withProject, withoutProject, withoutWorktree, withSelectionRestored, withWorktrees, type Projects } from "./projects";

export const projects = externalStore(noProjects);

const update = (change: (p: Projects) => Projects) => projects.set(change(projects.get()));
export const fail = (error: unknown) => update((p) => ({ ...p, error: String(error), errorIsNotice: false }));
const notify = (text: string) => update((p) => ({ ...p, error: text, errorIsNotice: true }));

export async function refreshWorktrees(project: string) {
  const list = await commands.listWorktrees(project);
  update((p) => withWorktrees(p, project, list));
}

function show(project: Project) {
  update((p) => withProject(p, project));
  refreshWorktrees(project.path).catch(fail);
}

export function loadProjects(list: Project[]) {
  list.forEach(show);
}

export async function pickProject() {
  try {
    const project = await commands.pickProject();
    if (project) show(project);
  } catch (error) {
    fail(error);
  }
}

export async function addFolders(folders: string[]) {
  for (const folder of folders) {
    try {
      show(await commands.addProject(folder));
    } catch (error) {
      fail(error);
    }
  }
}

export async function removeProject(path: string) {
  try {
    await commands.removeProject(path);
    update((p) => withoutProject(p, path));
  } catch (error) {
    fail(error);
  }
}

export function selectWorktree(path: string) {
  const { list, worktrees } = projects.get();
  const owner = list.find((p) => worktrees[p.path]?.some((w) => w.path === path));
  if (owner && sidebar.get().folded.includes(owner.path)) toggleFold(owner.path);
  update((p) => ({ ...p, selected: path }));
}
export const dismissError = () => update((p) => ({ ...p, error: null }));

// The worktree git refused to remove, waiting for the user to confirm a forced removal.
export const forceRemoval = externalStore<{ project: string; path: string; branch: string | null; reason: string } | null>(null);

// The worktree the user asked to delete, waiting for the confirm sheet.
export const deletion = externalStore<{ project: string; path: string; name: string } | null>(null);

export function requestDelete(project: string, path: string) {
  const worktree = projects.get().worktrees[project]?.find((w) => w.path === path);
  deletion.set({ project, path, name: worktree?.branch ?? path.split("/").pop() ?? path });
}

export const cancelDeletion = () => deletion.set(null);

export function confirmDeletion() {
  const pending = deletion.get();
  deletion.set(null);
  if (pending) void deleteWorktree(pending.project, pending.path);
}

// The card goes at once; git runs in the background and the list comes back if it refuses.
export async function deleteWorktree(project: string, path: string, force = false) {
  const branch = projects.get().worktrees[project]?.find((w) => w.path === path)?.branch ?? null;
  const before = projects.get().selected;
  update((p) => withoutWorktree(p, project, path));
  const afterDelete = projects.get().selected;
  const restore = () => update((p) => withSelectionRestored(p, path, before, afterDelete));
  try {
    const removal = await commands.removeWorktree(project, path, force);
    if (removal.NeedsForce) {
      forceRemoval.set({ project, path, branch, reason: removal.NeedsForce.reason });
      await refreshWorktrees(project);
      restore();
    } else if (removal.Removed?.kept_branch) {
      const { name, reason } = removal.Removed.kept_branch;
      notify(`Kept branch ${name}: ${reason}`);
    }
  } catch (error) {
    fail(error);
    await refreshWorktrees(project).catch(fail);
    restore();
  }
}

export function confirmForceRemoval() {
  const pending = forceRemoval.get();
  forceRemoval.set(null);
  if (pending) void deleteWorktree(pending.project, pending.path, true);
}

export const cancelForceRemoval = () => forceRemoval.set(null);

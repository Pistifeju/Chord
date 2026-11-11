// Connects the project rules to IPC: load, add by dialog or drop, remove, select.
import { commands, type Project } from "../../ipc/bindings";
import { externalStore } from "../../ui/externalStore";
import { sidebar, toggleFold } from "../sidebar/store";
import { noProjects, withProject, withoutProject, withWorktrees, type Projects } from "./projects";

export const projects = externalStore(noProjects);

const update = (change: (p: Projects) => Projects) => projects.set(change(projects.get()));
export const fail = (error: unknown) => update((p) => ({ ...p, error: String(error) }));

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

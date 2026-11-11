// Connects the project rules to IPC: load, add by dialog or drop, remove, select.
import { commands, type Project } from "../../ipc/bindings";
import { externalStore } from "../../ui/externalStore";
import { noProjects, withProject, withoutProject, withWorktrees, type Projects } from "./projects";

export const projects = externalStore(noProjects);

const update = (change: (p: Projects) => Projects) => projects.set(change(projects.get()));
const fail = (error: unknown) => update((p) => ({ ...p, error: String(error) }));

async function refresh(project: Project) {
  const list = await commands.listWorktrees(project.path);
  update((p) => withWorktrees(p, project.path, list));
}

function show(project: Project) {
  update((p) => withProject(p, project));
  refresh(project).catch(fail);
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

export const selectWorktree = (path: string) => update((p) => ({ ...p, selected: path }));
export const dismissError = () => update((p) => ({ ...p, error: null }));

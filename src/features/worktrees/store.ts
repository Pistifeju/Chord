import { commands, type Project } from "../../ipc/bindings";
import { closeSheet, openSheet, sheet, sheetOf } from "../../app/sheet";
import { fail, notify } from "../../ui/alert";
import { externalStore } from "../../ui/externalStore";
import { shownSession } from "../panes/sessions";
import { vanishedWorktrees } from "../panes/agents";
import { agentsWorkIn, forgetPanes, panes } from "../panes/store";
import { settingsOpen } from "../settings/store";
import { goneWorktrees, movesTo, noProjects, withProject, withProjectMoved, withWorktreeMoved, withoutProject, withoutWorktree, withFirstWorktreeSelected, withSavedSelection, withSelectionRestored, withWorktrees, worktreeAt, worktreeName, worktreeOrder, type Projects } from "./projects";
import { fileName } from "../files/paths";

export const projects = externalStore(noProjects);

export const welcomed = externalStore(false);

export const openWorktree = (): string | null => (settingsOpen.get() ? null : projects.get().selected);

export const xcodeWorktree = (): string | null => {
  const worktree = openWorktree();
  return worktree && worktreeAt(projects.get(), worktree)?.xcode ? worktree : null;
};

export const gitWorktree = (): string | null => {
  const worktree = openWorktree();
  return worktree && worktreeAt(projects.get(), worktree)?.git ? worktree : null;
};

export const shownPanes = () => shownSession(panes.get(), projects.get().selected, settingsOpen.get());

export const shownPanesStores = () => [panes, projects, settingsOpen];

const update = (change: (p: Projects) => Projects) => {
  const before = projects.get().selected;
  projects.set(change(projects.get()));
  const { selected } = projects.get();
  if (selected !== before) {
    commands.changeState({ selectedWorktree: selected }).catch(console.error);
    if (selected) commands.worktreeOpened(selected).catch(console.error);
  }
};

const worktreesGone = (paths: string[]) => paths.forEach(forgetPanes);

export async function refreshWorktrees(project: string) {
  const list = await commands.listWorktrees(project);
  const gone = goneWorktrees(projects.get(), project, list);
  worktreesGone(gone);
  update((p) => withWorktrees(gone.reduce((q, path) => withoutWorktree(q, project, path), p), project, list));
}

function show(project: Project) {
  welcomed.set(true);
  update((p) => withProject(p, project));
  return refreshWorktrees(project.path).catch(fail);
}

const add = (project: Project) => show(project).then(() => update((p) => withFirstWorktreeSelected(p, project.path)));

export function loadProjects(list: Project[], saved: string | null) {
  const loaded = list.map((project) => show(project).then(() => update((p) => withSavedSelection(p, saved))));
  return Promise.all(loaded).then(pruneVanishedPanes);
}

function pruneVanishedPanes() {
  const { list, worktrees } = projects.get();
  if (list.some((p) => !worktrees[p.path]?.length)) return;
  worktreesGone(vanishedWorktrees(panes.get(), worktreeOrder(projects.get())));
}

export async function pickProject() {
  try {
    const project = await commands.pickProject();
    if (project) void add(project);
  } catch (error) {
    fail(error);
  }
}

export async function addFolders(folders: string[]) {
  for (const folder of folders) {
    try {
      void add(await commands.addProject(folder));
    } catch (error) {
      fail(error);
    }
  }
}

export function requestRemoveProject(path: string) {
  const { worktrees, list } = projects.get();
  if (!agentsWorkIn((worktrees[path] ?? []).map((w) => w.path))) return void removeProject(path);
  openSheet({ kind: "removeProject", path, name: list.find((p) => p.path === path)?.name ?? path });
}

export function confirmRemoveProject() {
  const pending = sheetOf(sheet.get(), "removeProject");
  closeSheet("removeProject");
  if (pending) void removeProject(pending.path);
}

async function removeProject(path: string) {
  try {
    await commands.removeProject(path);
    worktreesGone(goneWorktrees(projects.get(), path, []));
    update((p) => withoutProject(p, path));
  } catch (error) {
    fail(error);
  }
}

export function selectWorktree(path: string) {
  settingsOpen.set(false);
  update((p) => ({ ...p, selected: path }));
}

export function moveProject(from: number, slot: number) {
  if (!movesTo(from, slot)) return;
  update((p) => withProjectMoved(p, from, slot));
  commands.changeState({ projectOrder: projects.get().list.map((p) => p.path) }).catch(console.error);
}

export function moveWorktree(project: string, from: number, slot: number) {
  if (!movesTo(from, slot)) return;
  update((p) => withWorktreeMoved(p, project, from, slot));
  const worktrees = (projects.get().worktrees[project] ?? []).map((w) => w.path);
  commands.changeState({ worktreeOrder: { project, worktrees } }).catch(console.error);
}

export function requestDelete(project: string, path: string) {
  const worktree = projects.get().worktrees[project]?.find((w) => w.path === path);
  openSheet({ kind: "deleteWorktree", project, path, name: worktree ? worktreeName(worktree) : fileName(path) });
}

export function confirmDeletion() {
  const pending = sheetOf(sheet.get(), "deleteWorktree");
  closeSheet("deleteWorktree");
  if (pending) void deleteWorktree(pending.project, pending.path);
}

async function deleteWorktree(project: string, path: string, force = false) {
  const branch = projects.get().worktrees[project]?.find((w) => w.path === path)?.branch ?? null;
  const before = projects.get().selected;
  update((p) => withoutWorktree(p, project, path));
  const afterDelete = projects.get().selected;
  const restore = () => update((p) => withSelectionRestored(p, path, before, afterDelete));
  try {
    const removal = await commands.removeWorktree(project, path, force);
    if (removal.NeedsForce) {
      openSheet({ kind: "forceRemoval", project, path, branch, reason: removal.NeedsForce.reason });
      await refreshWorktrees(project);
      restore();
    } else {
      worktreesGone([path]);
      if (removal.Removed?.kept_branch) {
        const { name, reason } = removal.Removed.kept_branch;
        notify(`Kept branch ${name}: ${reason}`);
      }
    }
  } catch (error) {
    fail(error);
    await refreshWorktrees(project).catch(fail);
    restore();
  }
}

export function confirmForceRemoval() {
  const pending = sheetOf(sheet.get(), "forceRemoval");
  closeSheet("forceRemoval");
  if (pending) void deleteWorktree(pending.project, pending.path, true);
}

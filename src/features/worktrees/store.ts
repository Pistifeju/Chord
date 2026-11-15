// Connects the project rules to IPC: load, add by dialog or drop, remove, select, delete a worktree.
import { commands, type Project } from "../../ipc/bindings";
import { closeSheet, openSheet, sheet, sheetOf } from "../../app/sheet";
import { externalStore } from "../../ui/externalStore";
import { shownSession } from "../panes/sessions";
import { forgetPanes, forgetPanesExcept, panes } from "../panes/store";
import { settingsOpen } from "../settings/store";
import { noProjects, withProject, withoutProject, withoutWorktree, withSavedSelection, withSelectionRestored, withWorktrees, worktreeOrder, type Projects } from "./projects";

export const projects = externalStore(noProjects);

// The worktree the window shows: the selected one, none while Settings covers the window.
export const openWorktree = (): string | null => (settingsOpen.get() ? null : projects.get().selected);

export const shownPanes = () => shownSession(panes.get(), projects.get().selected, settingsOpen.get());

/** The stores `shownPanes` reads, for a watcher that follows it. */
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
export const fail = (error: unknown) => update((p) => ({ ...p, error: String(error), errorIsNotice: false }));
export const notify = (text: string) => update((p) => ({ ...p, error: text, errorIsNotice: true }));

export async function refreshWorktrees(project: string) {
  const list = await commands.listWorktrees(project);
  // A worktree that vanished outside Chord takes its agent with it.
  for (const gone of projects.get().worktrees[project] ?? []) if (!list.some((w) => w.path === gone.path)) forgetPanes(gone.path);
  update((p) => withWorktrees(p, project, list));
}

function show(project: Project) {
  update((p) => withProject(p, project));
  return refreshWorktrees(project.path).catch(fail);
}

// The saved worktree comes back once its project's list has loaded; one that no longer exists stays unselected.
export function loadProjects(list: Project[], saved: string | null) {
  const loaded = list.map((project) => show(project).then(() => update((p) => withSavedSelection(p, saved))));
  return Promise.all(loaded).then(pruneVanishedPanes);
}

// Only when every project's list loaded and none is empty: a failed load must not wipe saved layouts.
function pruneVanishedPanes() {
  const { list, worktrees } = projects.get();
  if (list.some((p) => !worktrees[p.path]?.length)) return;
  forgetPanesExcept(worktreeOrder(projects.get()));
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
    for (const gone of projects.get().worktrees[path] ?? []) forgetPanes(gone.path);
    update((p) => withoutProject(p, path));
  } catch (error) {
    fail(error);
  }
}

export function selectWorktree(path: string) {
  settingsOpen.set(false);
  update((p) => ({ ...p, selected: path }));
}
export const dismissError = () => update((p) => ({ ...p, error: null }));

export function requestDelete(project: string, path: string) {
  const worktree = projects.get().worktrees[project]?.find((w) => w.path === path);
  openSheet({ kind: "deleteWorktree", project, path, name: worktree?.branch ?? path.split("/").pop() ?? path });
}

export function confirmDeletion() {
  const pending = sheetOf(sheet.get(), "deleteWorktree");
  closeSheet("deleteWorktree");
  if (pending) void deleteWorktree(pending.project, pending.path);
}

// The card goes at once; git runs in the background and the list comes back if it refuses.
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
      forgetPanes(path);
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

// Rules for the project list, its worktrees and the selected worktree.
import type { Project, Worktree_Serialize as Worktree } from "../../ipc/bindings";

export type Projects = {
  list: Project[];
  worktrees: Record<string, Worktree[]>;
  selected: string | null;
  error: string | null;
  /** The toast text is information, not a failure: shown without the warning icon. */
  errorIsNotice: boolean;
};

export const noProjects: Projects = { list: [], worktrees: {}, selected: null, error: null, errorIsNotice: false };

export function withProject(projects: Projects, project: Project): Projects {
  if (projects.list.some((p) => p.path === project.path)) return projects;
  return { ...projects, list: [...projects.list, project] };
}

export function withoutProject(projects: Projects, path: string): Projects {
  const { [path]: removed = [], ...worktrees } = projects.worktrees;
  const selected = removed.some((w) => w.path === projects.selected) ? null : projects.selected;
  return { ...projects, list: projects.list.filter((p) => p.path !== path), worktrees, selected };
}

export function withWorktrees(projects: Projects, path: string, list: Worktree[]): Projects {
  if (!projects.list.some((p) => p.path === path)) return projects;
  return { ...projects, worktrees: { ...projects.worktrees, [path]: list } };
}

// The project that owns the selected worktree, else the first one.
export function defaultProject(projects: Projects): string | null {
  const owner = projects.list.find((p) => projects.worktrees[p.path]?.some((w) => w.path === projects.selected));
  return (owner ?? projects.list[0])?.path ?? null;
}

export const worktreeName = (worktree: Worktree): string => worktree.branch ?? worktree.path.split("/").pop() ?? worktree.path;

// Whitespace in a typed branch name becomes a dash; this is the name git sees.
export function branchName(typed: string): string {
  return typed.trim().replace(/\s+/g, "-");
}

// Create stays disabled until the live git check of a non-empty name passed.
export function canCreate(form: { project: string | null; branch: string; checked: boolean; problem: string | null; creating: boolean }): boolean {
  return form.project !== null && branchName(form.branch) !== "" && form.checked && !form.problem && !form.creating;
}

// The worktree's card goes; a selected one hands the selection to the card below it, else above, else nothing.
export function withoutWorktree(projects: Projects, project: string, path: string): Projects {
  const list = projects.worktrees[project] ?? [];
  const index = list.findIndex((w) => w.path === path);
  if (index < 0) return projects;
  const rest = list.filter((w) => w.path !== path);
  const next = rest[index] ?? rest[index - 1];
  const selected = projects.selected === path ? (next?.path ?? null) : projects.selected;
  return { ...projects, worktrees: { ...projects.worktrees, [project]: rest }, selected };
}

// A refused delete puts the card back; the selection returns to it only if the delete had moved it and the user has not picked another since.
export function withSelectionRestored(projects: Projects, path: string, before: string | null, afterDelete: string | null): Projects {
  return before === path && projects.selected === afterDelete ? { ...projects, selected: path } : projects;
}

// The worktree saved from the last run is selected if it still exists and the user has not picked one since.
export function withSavedSelection(projects: Projects, saved: string | null): Projects {
  if (projects.selected !== null || saved === null) return projects;
  return worktreeOrder(projects).includes(saved) ? { ...projects, selected: saved } : projects;
}

// Worktree paths in sidebar order; the first ones take the select shortcuts.
export function worktreeOrder(projects: Projects): string[] {
  return projects.list.flatMap((p) => (projects.worktrees[p.path] ?? []).map((w) => w.path));
}



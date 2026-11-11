// Rules for the project list, its worktrees and the selected worktree.
import type { Project, Worktree_Serialize as Worktree } from "../../ipc/bindings";

export type Projects = {
  list: Project[];
  worktrees: Record<string, Worktree[]>;
  selected: string | null;
  error: string | null;
};

export const noProjects: Projects = { list: [], worktrees: {}, selected: null, error: null };

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

// Whitespace in a typed branch name becomes a dash; this is the name git sees.
export function branchName(typed: string): string {
  return typed.trim().replace(/\s+/g, "-");
}

// Create stays disabled until the live git check of a non-empty name passed.
export function canCreate(form: { project: string | null; branch: string; checked: boolean; problem: string | null; creating: boolean }): boolean {
  return form.project !== null && branchName(form.branch) !== "" && form.checked && !form.problem && !form.creating;
}

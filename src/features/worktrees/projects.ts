import type { Project, Worktree_Serialize as Worktree } from "../../ipc/bindings";
import { fileName } from "../files/paths.ts";

export type Projects = {
  list: Project[];
  worktrees: Record<string, Worktree[]>;
  selected: string | null;
};

export const noProjects: Projects = { list: [], worktrees: {}, selected: null };

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

export function defaultProject(projects: Projects): string | null {
  const owner = projects.selected === null ? undefined : ownerOf(projects, projects.selected);
  return owner ?? projects.list[0]?.path ?? null;
}

export const ownerOf = (projects: Projects, worktree: string): string | undefined =>
  projects.list.find((p) => projects.worktrees[p.path]?.some((w) => w.path === worktree))?.path;

const allWorktrees = (projects: Projects): Worktree[] => Object.values(projects.worktrees).flat();

export const worktreeAt = (projects: Projects, path: string | null): Worktree | undefined =>
  allWorktrees(projects).find((worktree) => worktree.path === path);

export const hasXcodeProject = (projects: Projects): boolean => allWorktrees(projects).some((worktree) => worktree.xcode);

export const isGitProject = (projects: Projects, path: string): boolean => projects.worktrees[path]?.some((w) => w.git) ?? false;

export const projectsTurnedGit = (projects: Projects, readByGit: string[]): string[] =>
  projects.list.filter((p) => projects.worktrees[p.path]?.some((w) => !w.git && readByGit.includes(w.path))).map((p) => p.path);

export const projectsLosingWorktrees = (projects: Projects, queried: string[], readByGit: string[]): string[] =>
  projects.list.filter((p) => projects.worktrees[p.path]?.some((w) => w.git && queried.includes(w.path) && !readByGit.includes(w.path))).map((p) => p.path);

export const worktreeName = (worktree: Worktree): string => worktree.branch ?? fileName(worktree.path);

export function branchName(typed: string): string {
  return typed.trim().replace(/\s+/g, "-");
}

export function canCreate(form: { project: string | null; branch: string; checked: boolean; problem: string | null; creating: boolean }): boolean {
  return form.project !== null && branchName(form.branch) !== "" && form.checked && !form.problem && !form.creating;
}

export function withoutWorktree(projects: Projects, project: string, path: string): Projects {
  const list = projects.worktrees[project] ?? [];
  const index = list.findIndex((w) => w.path === path);
  if (index < 0) return projects;
  const rest = list.filter((w) => w.path !== path);
  const next = rest[index] ?? rest[index - 1];
  const selected = projects.selected === path ? (next?.path ?? null) : projects.selected;
  return { ...projects, worktrees: { ...projects.worktrees, [project]: rest }, selected };
}

export function withSelectionRestored(projects: Projects, path: string, before: string | null, afterDelete: string | null): Projects {
  return before === path && projects.selected === afterDelete ? { ...projects, selected: path } : projects;
}

export function withFirstWorktreeSelected(projects: Projects, project: string): Projects {
  const first = projects.worktrees[project]?.[0];
  return projects.selected === null && first ? { ...projects, selected: first.path } : projects;
}

export function withSavedSelection(projects: Projects, saved: string | null): Projects {
  if (projects.selected !== null || saved === null) return projects;
  return worktreeOrder(projects).includes(saved) ? { ...projects, selected: saved } : projects;
}

export function goneWorktrees(projects: Projects, project: string, list: Worktree[]): string[] {
  return (projects.worktrees[project] ?? []).filter((w) => !list.some((now) => now.path === w.path)).map((w) => w.path);
}

export function worktreeOrder(projects: Projects): string[] {
  return projects.list.flatMap((p) => (projects.worktrees[p.path] ?? []).map((w) => w.path));
}

export function moved<T>(list: T[], from: number, slot: number): T[] {
  const rest = list.filter((_, i) => i !== from);
  const to = slot > from ? slot - 1 : slot;
  return [...rest.slice(0, to), list[from], ...rest.slice(to)];
}

export const movesTo = (from: number, slot: number): boolean => slot !== from && slot !== from + 1;

export const withProjectMoved = (projects: Projects, from: number, slot: number): Projects => ({ ...projects, list: moved(projects.list, from, slot) });

export function withWorktreeMoved(projects: Projects, project: string, from: number, slot: number): Projects {
  const list = projects.worktrees[project] ?? [];
  return { ...projects, worktrees: { ...projects.worktrees, [project]: moved(list, from, slot) } };
}

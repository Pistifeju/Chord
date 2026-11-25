import type { Change, ChangeStatus, DiffSide, WorktreePanes } from "../../ipc/bindings";
import { groupOf } from "../panes/layout.ts";

export type ChangeGroup = { title: "Staged" | "Changes"; side: "staged" | "unstaged"; changes: Change[] };

export function changeGroups(list: Change[]): ChangeGroup[] {
  const sorted = [...list].sort((a, b) => (a.path < b.path ? -1 : a.path > b.path ? 1 : 0));
  return [
    { title: "Staged", side: "staged", changes: sorted.filter((c) => c.staged) },
    { title: "Changes", side: "unstaged", changes: sorted.filter((c) => !c.staged) },
  ];
}

export const statusLetters: Record<ChangeStatus, string> = { modified: "M", added: "A", deleted: "D", renamed: "R", untracked: "U", conflicted: "!" };

export const statusWord = (status: ChangeStatus) => status[0].toUpperCase() + status.slice(1);

function describeSide(side: DiffSide): { key: string; title: string; changes: boolean } {
  if (side === "staged") return { key: side, title: " (Staged)", changes: true };
  if (side === "unstaged") return { key: side, title: " (Changes)", changes: true };
  if (side.branch !== undefined) return { key: `branch ${side.branch}`, title: ` (Compared to ${side.branch})`, changes: true };
  if (side.commit !== undefined) return { key: `commit ${side.commit}`, title: ` (Commit ${side.commit.slice(0, 7)})`, changes: false };
  return { key: `stash ${side.stash}`, title: ` (Stash ${side.stash.slice(0, 7)})`, changes: false };
}

export const diffKey = (side: DiffSide, path: string) => `${describeSide(side).key}:${path}`;

export const diffTitle = (side: DiffSide) => describeSide(side).title;

export const diffChanges = (side: DiffSide) => describeSide(side).changes;

export function pickRange(group: ChangeGroup, anchor: string | null, key: string): string[] {
  const keys = group.changes.map((c) => diffKey(group.side, c.path));
  const from = anchor ? keys.indexOf(anchor) : -1;
  const to = keys.indexOf(key);
  if (from < 0 || to < 0) return [key];
  return keys.slice(Math.min(from, to), Math.max(from, to) + 1);
}

export function landingAfter(group: ChangeGroup, acted: string[]): string | null {
  const keys = group.changes.map((c) => diffKey(group.side, c.path));
  const last = Math.max(...acted.map((key) => keys.indexOf(key)));
  const after = keys.slice(last + 1).find((key) => !acted.includes(key));
  return after ?? keys.slice(0, last).reverse().find((key) => !acted.includes(key)) ?? null;
}

export function nextSelection(before: Change[], after: Change[], selected: string): string | null {
  const keys = new Set(after.map((c) => diffKey(c.staged ? "staged" : "unstaged", c.path)));
  if (keys.has(selected)) return null;
  const group = changeGroups(before).find((g) => g.changes.some((c) => diffKey(g.side, c.path) === selected));
  if (!group) return null;
  const gone = group.changes.map((c) => diffKey(group.side, c.path)).filter((key) => !keys.has(key));
  const moved = diffKey(group.side === "staged" ? "unstaged" : "staged", selected.slice(selected.indexOf(":") + 1));
  return landingAfter(group, gone) ?? (keys.has(moved) ? moved : null);
}

export function arrivals(before: Change[] | undefined, after: Change[] | undefined, arrived: Set<string>): Set<string> {
  if (!before || !after) return new Set();
  const keyOf = (c: Change) => diffKey(c.staged ? "staged" : "unstaged", c.path);
  const had = new Set(before.map(keyOf));
  return new Set(after.map(keyOf).filter((key) => !had.has(key) || arrived.has(key)));
}

export function focusedDiff(panes: WorktreePanes | undefined): string | null {
  const tab = panes && groupOf(panes.layout, panes.focused)?.tabs.find((t) => t.id === panes.focused);
  return tab?.kind === "file" && tab.diff ? diffKey(tab.diff, tab.path) : null;
}

export const canCommit = (message: string, amend: boolean, staged: boolean) => message.trim() !== "" && (staged || amend);

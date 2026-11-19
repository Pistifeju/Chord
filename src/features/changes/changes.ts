// Rules for the Changes view: its Staged and Changes groups, when Commit runs, the rows a shift-click picks, how a change's status reads, and which row the focused tab's diff selects.
import type { Change, ChangeStatus, DiffSide, WorktreePanes } from "../../ipc/bindings";
import { groupOf } from "../panes/layout.ts";

export type ChangeGroup = { title: "Staged" | "Changes"; side: DiffSide; changes: Change[] };

/** Both groups, Staged first, each in path order; an empty one still shows, so a count of 0 reads. */
export function changeGroups(list: Change[]): ChangeGroup[] {
  const sorted = [...list].sort((a, b) => (a.path < b.path ? -1 : a.path > b.path ? 1 : 0));
  return [
    { title: "Staged", side: "staged", changes: sorted.filter((c) => c.staged) },
    { title: "Changes", side: "unstaged", changes: sorted.filter((c) => !c.staged) },
  ];
}

export const statusLetters: Record<ChangeStatus, string> = { modified: "M", added: "A", deleted: "D", renamed: "R", untracked: "U", conflicted: "!" };

export const statusWord = (status: ChangeStatus) => status[0].toUpperCase() + status.slice(1);

/** A row's key: its side and path, as the diff tab it opens holds them. */
export const diffKey = (side: DiffSide, path: string) => `${side}:${path}`;

/** The keys of the group's rows from `anchor` to `key`, both in; `key` alone while the anchor is not in the group. */
export function pickRange(group: ChangeGroup, anchor: string | null, key: string): string[] {
  const keys = group.changes.map((c) => diffKey(group.side, c.path));
  const from = anchor ? keys.indexOf(anchor) : -1;
  const to = keys.indexOf(key);
  if (from < 0 || to < 0) return [key];
  return keys.slice(Math.min(from, to), Math.max(from, to) + 1);
}

/** The row the focus moves to once `acted` leave the group: the first one after them, else the last one before, else none. */
export function landingAfter(group: ChangeGroup, acted: string[]): string | null {
  const keys = group.changes.map((c) => diffKey(group.side, c.path));
  const last = Math.max(...acted.map((key) => keys.indexOf(key)));
  const after = keys.slice(last + 1).find((key) => !acted.includes(key));
  return after ?? keys.slice(0, last).reverse().find((key) => !acted.includes(key)) ?? null;
}

/** The row to select once a read drops the selected one from its group: the next row the group kept, else the last one before, else the change itself in its new group, else none. */
export function nextSelection(before: Change[], after: Change[], selected: string): string | null {
  const keys = new Set(after.map((c) => diffKey(c.staged ? "staged" : "unstaged", c.path)));
  if (keys.has(selected)) return null;
  const group = changeGroups(before).find((g) => g.changes.some((c) => diffKey(g.side, c.path) === selected));
  if (!group) return null;
  const gone = group.changes.map((c) => diffKey(group.side, c.path)).filter((key) => !keys.has(key));
  const moved = diffKey(group.side === "staged" ? "unstaged" : "staged", selected.slice(selected.indexOf(":") + 1));
  return landingAfter(group, gone) ?? (keys.has(moved) ? moved : null);
}

/** The rows that arrived since the first read, so only they animate in: new in `after`, or arrived before and still there. */
export function arrivals(before: Change[] | undefined, after: Change[] | undefined, arrived: Set<string>): Set<string> {
  if (!before || !after) return new Set();
  const keyOf = (c: Change) => diffKey(c.staged ? "staged" : "unstaged", c.path);
  const had = new Set(before.map(keyOf));
  return new Set(after.map(keyOf).filter((key) => !had.has(key) || arrived.has(key)));
}

/** The key of the diff the focused tab shows, if it shows one. */
export function focusedDiff(panes: WorktreePanes | undefined): string | null {
  const tab = panes && groupOf(panes.layout, panes.focused)?.tabs.find((t) => t.id === panes.focused);
  return tab?.kind === "file" && tab.diff ? diffKey(tab.diff, tab.path) : null;
}

/** Whether Commit runs: a message, and a staged file or the last commit to amend. */
export const canCommit = (message: string, amend: boolean, staged: boolean) => message.trim() !== "" && (staged || amend);

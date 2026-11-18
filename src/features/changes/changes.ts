// Rules for the Changes view: its Staged and Changes groups, how a change's status reads, and which row the focused tab's diff selects.
import type { Change, ChangeStatus, DiffSide, WorktreePanes } from "../../ipc/bindings";
import { groupOf } from "../panes/layout.ts";

export type ChangeGroup = { title: "Staged" | "Changes"; side: DiffSide; changes: Change[] };

/** The groups that hold a change, Staged first, each in path order. */
export function changeGroups(list: Change[]): ChangeGroup[] {
  const sorted = [...list].sort((a, b) => (a.path < b.path ? -1 : a.path > b.path ? 1 : 0));
  const groups: ChangeGroup[] = [
    { title: "Staged", side: "staged", changes: sorted.filter((c) => c.staged) },
    { title: "Changes", side: "unstaged", changes: sorted.filter((c) => !c.staged) },
  ];
  return groups.filter((group) => group.changes.length > 0);
}

export const statusLetters: Record<ChangeStatus, string> = { modified: "M", added: "A", deleted: "D", renamed: "R", untracked: "U", conflicted: "!" };

export const statusWord = (status: ChangeStatus) => status[0].toUpperCase() + status.slice(1);

/** A row's key: its side and path, as the diff tab it opens holds them. */
export const diffKey = (side: DiffSide, path: string) => `${side}:${path}`;

/** The key of the diff the focused tab shows, if it shows one. */
export function focusedDiff(panes: WorktreePanes | undefined): string | null {
  const tab = panes && groupOf(panes.layout, panes.focused)?.tabs.find((t) => t.id === panes.focused);
  return tab?.kind === "file" && tab.diff ? diffKey(tab.diff, tab.path) : null;
}

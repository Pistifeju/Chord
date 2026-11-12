// Rules for which worktree cards get git counts, how fresh results merge in, and how a card shows them.
import type { GitCounts_Serialize as GitCounts } from "../../ipc/bindings";

export function gainsPath(before: string[], after: string[]): boolean {
  return after.some((path) => !before.includes(path));
}

const sameCounts = (a: GitCounts, b: GitCounts) => a.ahead === b.ahead && a.behind === b.behind && a.changed === b.changed;

// Keeps entries of known worktrees, overwrites queried ones with fresh results and drops queried ones git no longer reports. Returns `current` itself when nothing changed.
export function nextCounts(current: Record<string, GitCounts>, known: string[], queried: string[], fresh: GitCounts[]): Record<string, GitCounts> {
  const next: Record<string, GitCounts> = {};
  for (const path of known) if (path in current && !queried.includes(path)) next[path] = current[path];
  for (const counts of fresh) if (queried.includes(counts.path) && known.includes(counts.path)) next[counts.path] = counts;
  const before = Object.keys(current);
  const unchanged = before.length === Object.keys(next).length && before.every((path) => path in next && sameCounts(current[path], next[path]));
  return unchanged ? current : next;
}

// Zero counts and a missing base branch show nothing: "↑2 ↓1 · 3 changed".
export function countsLabel(counts: GitCounts | undefined): string {
  if (!counts) return "";
  const sides = [counts.ahead ? `↑${counts.ahead}` : "", counts.behind ? `↓${counts.behind}` : ""].filter(Boolean).join(" ");
  const changed = counts.changed ? `${counts.changed} changed` : "";
  return [sides, changed].filter(Boolean).join(" · ");
}

export function countsTitle(counts: GitCounts | undefined): string {
  if (!counts) return "";
  return [
    counts.ahead ? `${counts.ahead} ahead` : "",
    counts.behind ? `${counts.behind} behind` : "",
    counts.changed ? `${counts.changed} changed ${counts.changed === 1 ? "file" : "files"}` : "",
  ]
    .filter(Boolean)
    .join(", ");
}

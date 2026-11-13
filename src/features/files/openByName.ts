// Rules for Open File by Name: which of the worktree's files match the typed name, best first.

// shortcut: only the first rows render, so a huge repo stays quick; upgrade to a virtual list if someone misses a file past them.
export const SHOWN_FILES = 100;

// 0: the name starts with the query, 1: the name contains it, 2: only the path does; -1: no match.
function rank(path: string, needle: string): number {
  const lower = path.toLowerCase();
  const name = lower.slice(lower.lastIndexOf("/") + 1);
  if (name.startsWith(needle)) return 0;
  if (name.includes(needle)) return 1;
  return lower.includes(needle) ? 2 : -1;
}

/** The paths that match the query ignoring case, best first and in their given order within a rank, at most `SHOWN_FILES`. */
export function matchingFiles(paths: string[], query: string): string[] {
  const needle = query.trim().toLowerCase();
  if (!needle) return paths.slice(0, SHOWN_FILES);
  // One pass into rank buckets; a full first bucket already holds the answer.
  const ranked: string[][] = [[], [], []];
  for (const path of paths) {
    const at = rank(path, needle);
    if (at !== -1) ranked[at].push(path);
    if (ranked[0].length === SHOWN_FILES) break;
  }
  return ranked.flat().slice(0, SHOWN_FILES);
}

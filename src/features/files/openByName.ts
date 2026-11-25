const SHOWN_FILES = 100;

const IN_NAME = 2;
const WORD_START = 3;
const AFTER_LAST_MATCH = 4;

const isLower = (char: string) => char >= "a" && char <= "z";
const isUpper = (char: string) => char >= "A" && char <= "Z";

function startsWord(path: string, at: number): boolean {
  if (at === 0) return true;
  const before = path[at - 1];
  return "/._- ".includes(before) || (isLower(before) && isUpper(path[at]));
}

function isSubsequence(lower: string, needle: string): boolean {
  let at = 0;
  for (const char of needle) {
    at = lower.indexOf(char, at) + 1;
    if (at === 0) return false;
  }
  return true;
}

let prev = new Float64Array(256);
let row = new Float64Array(256);

function score(path: string, needle: string): number {
  const lower = path.toLowerCase();
  if (!isSubsequence(lower, needle)) return -Infinity;
  const n = path.length;
  if (prev.length < n) [prev, row] = [new Float64Array(n), new Float64Array(n)];
  const nameAt = path.lastIndexOf("/") + 1;
  prev.fill(-Infinity, 0, n);
  for (let i = 0; i < needle.length; i++) {
    row.fill(-Infinity, 0, n);
    let bestBefore = i === 0 ? 0 : -Infinity;
    for (let j = 0; j < n; j++) {
      if (lower[j] === needle[i]) {
        const points = 1 + (j >= nameAt ? IN_NAME : 0) + (startsWord(path, j) ? WORD_START : 0);
        const run = j > 0 ? prev[j - 1] + AFTER_LAST_MATCH : -Infinity;
        row[j] = points + Math.max(bestBefore, run);
      }
      if (i > 0) bestBefore = Math.max(bestBefore, prev[j]);
    }
    [prev, row] = [row, prev];
  }
  let best = -Infinity;
  for (let j = 0; j < n; j++) best = Math.max(best, prev[j]);
  return best;
}

export function matchingFiles(paths: string[], query: string): string[] {
  const needle = query.replace(/\s+/g, "").toLowerCase();
  if (!needle) return paths.slice(0, SHOWN_FILES);
  return paths
    .map((path) => ({ path, points: score(path, needle) }))
    .filter((match) => match.points !== -Infinity)
    .sort((a, b) => b.points - a.points)
    .slice(0, SHOWN_FILES)
    .map((match) => match.path);
}

export function recentFirst(paths: string[], recent: string[]): string[] {
  const listed = new Set(paths);
  const front = recent.filter((path) => listed.has(path));
  const inFront = new Set(front);
  return [...front, ...paths.filter((path) => !inFront.has(path))];
}

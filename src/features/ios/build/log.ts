export const LOG_LINES = 10_000;
const TRIM_EVERY_LINES = 1_000;

export type Log = { lines: string[]; partial: string };

export const emptyLog: Log = { lines: [], partial: "" };

export const cappedLines = <T>(lines: T[]): T[] => (lines.length > LOG_LINES + TRIM_EVERY_LINES ? lines.slice(lines.length - LOG_LINES) : lines);

export function appendToLog(log: Log, text: string): Log {
  const parts = (log.partial + text).split("\n");
  const partial = parts.pop() ?? "";
  return { lines: cappedLines(log.lines.concat(parts)), partial };
}

export const logLines = (log: Log): string[] => (log.partial ? [...log.lines, log.partial] : log.lines);

export type Match = { line: number; start: number };

export function matchesIn(lines: string[], query: string): Match[] {
  if (!query) return [];
  const needle = query.toLowerCase();
  const found: Match[] = [];
  lines.forEach((text, line) => {
    const lower = text.toLowerCase();
    for (let start = lower.indexOf(needle); start >= 0; start = lower.indexOf(needle, start + needle.length)) found.push({ line, start });
  });
  return found;
}

type Segment = { text: string; match: boolean; start: number };

export function segmentsOf(text: string, query: string): Segment[] {
  const parts: Segment[] = [];
  let at = 0;
  for (const { start } of matchesIn([text], query)) {
    if (start > at) parts.push({ text: text.slice(at, start), match: false, start: at });
    parts.push({ text: text.slice(start, start + query.length), match: true, start });
    at = start + query.length;
  }
  if (at < text.length || parts.length === 0) parts.push({ text: text.slice(at), match: false, start: at });
  return parts;
}

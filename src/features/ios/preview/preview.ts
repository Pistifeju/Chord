// Rules for the Preview view: whether the preview simulator's screen shows, the centered line in its place, whether its header says a rebuild runs, the compile error its bar shows, how a `#Preview` reads, which ones the list's filter matches, the list's rows and the preview ↑ or ↓ goes to.
import type { BuildIssue_Deserialize, PreviewPick, PreviewState } from "../../../ipc/bindings";
import { issuePlace } from "../build/run.ts";
import type { Row } from "../../files/tree.ts";
import { fileName } from "../../files/paths.ts";

/** A worktree's preview state, and whether its preview app ran since the preview simulator booted. */
export type Preview = { state: PreviewState; shown: boolean };

/** A rebuild leaves the last preview on screen; a new boot or the limit takes it away. */
export const appliedState = (before: Preview | undefined, state: PreviewState): Preview => ({
  state,
  shown: state.state === "live" || (before?.shown === true && state.state !== "starting" && state.state !== "full" && state.state !== "unpicked"),
});

/** A build replaces the preview on screen; a hot reload never reads as one. */
export const rebuilding = (preview: Preview | undefined): boolean =>
  preview?.shown === true && (preview.state.state === "building" || preview.state.state === "waiting");

/** `LoginView.swift:42: message`, the message alone without a file. */
export function issueLine(issue: BuildIssue_Deserialize): string {
  const place = issuePlace(issue);
  return place ? `${place}: ${issue.message}` : issue.message;
}

/** The compile error the bar over the kept frame shows; null without one or before the preview first showed. */
export const shownError = (preview: Preview | undefined): BuildIssue_Deserialize | null =>
  preview?.shown && preview.state.state === "error" ? preview.state.issue : null;

/** The line that stands in for the screen; null once the preview shows. */
export function previewLine(preview: Preview | undefined): string | null {
  if (preview?.shown) return null;
  const state = preview?.state ?? { state: "starting" };
  switch (state.state) {
    case "starting":
      return "Starting the preview…";
    case "waiting":
      return "Waiting for the Run build…";
    case "building":
      return "Building the preview…";
    case "full":
      return `Two previews already run: ${state.worktrees.join(", ")}.`;
    case "unpicked":
      return "Pick a preview.";
    case "error":
      return issueLine(state.issue);
    case "failed":
      return state.message;
    case "live":
      return null;
  }
}

/** The macro's name, an unnamed one by its place in its file. */
export const previewName = (pick: PreviewPick): string => pick.name ?? `Preview ${pick.index + 1}`;

/** `Views/ContentView.swift#1`: the list row's path. */
export const previewKey = (pick: PreviewPick): string => `${pick.file}#${pick.index}`;

/** A row of the list: a file, as a Files tree folder, or one of its previews under it. */
export type PreviewRow = Row & { pick: PreviewPick | null };

/** The previews whose name or file holds the query, ignoring case, in their file order. */
function matchingPreviews(all: PreviewPick[], query: string): PreviewPick[] {
  const needle = query.trim().toLowerCase();
  return all.filter((pick) => `${previewName(pick)} ${pick.file}`.toLowerCase().includes(needle));
}

/** A row's name: the file's previews drop the "Symptoms — " prefix all of them share, under the file's row that already says it. */
function rowNames(all: PreviewPick[]): Map<PreviewPick, string> {
  const names = new Map<PreviewPick, string>();
  const byFile = new Map<string, PreviewPick[]>();
  for (const pick of all) byFile.set(pick.file, [...(byFile.get(pick.file) ?? []), pick]);
  for (const picks of byFile.values()) {
    const full = picks.map(previewName);
    const prefix = /^.+? — /.exec(full[0])?.[0];
    const shared = prefix !== undefined && full.every((name) => name.startsWith(prefix) && name.length > prefix.length);
    picks.forEach((pick, i) => names.set(pick, shared ? full[i].charAt(prefix.length).toUpperCase() + full[i].slice(prefix.length + 1) : full[i]));
  }
  return names;
}

/** Each file with a matching preview, then its previews unless the file is closed; a filter opens every file. */
export function previewRows(all: PreviewPick[], query: string, closed: string[]): PreviewRow[] {
  const names = rowNames(all);
  const rows: PreviewRow[] = [];
  let file: string | null = null;
  let open = true;
  for (const pick of matchingPreviews(all, query)) {
    if (pick.file !== file) {
      file = pick.file;
      open = query.trim() !== "" || !closed.includes(file);
      rows.push({ entry: { name: fileName(file), path: file, isDir: true }, depth: 0, open, pick: null });
    }
    if (open) rows.push({ entry: { name: names.get(pick) ?? previewName(pick), path: previewKey(pick), isDir: false }, depth: 1, open: false, pick });
  }
  return rows;
}

/** The preview ↑ or ↓ goes to from the row at `from`, past file rows; with ⌥, the first preview of the previous or next file. From no row, the first or the last preview. */
export function steppedPreview(rows: PreviewRow[], from: string | null, down: boolean, byFile: boolean): PreviewPick | null {
  const at = rows.findIndex((row) => row.entry.path === from);
  if (!byFile) {
    const ahead = down ? rows.slice(at + 1) : rows.slice(0, at < 0 ? rows.length : at).reverse();
    return ahead.find((row) => row.pick)?.pick ?? null;
  }
  const files = rows.flatMap((row, i) => (row.pick ? [] : [i]));
  // The file the row sits in: the last file row at or above it.
  const current = files.filter((i) => i <= at).length - 1;
  const targets = down ? files.slice(current + 1) : files.slice(0, Math.max(current, 0)).reverse();
  for (const i of targets) {
    const pick = rows[i + 1]?.pick;
    if (pick) return pick;
  }
  return null;
}

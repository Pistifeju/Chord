import type { BuildIssue_Deserialize, PreviewPick, PreviewState } from "../../../ipc/bindings";
import { issuePlace } from "../build/run.ts";
import type { Row } from "../../files/tree.ts";
import { fileName } from "../../files/paths.ts";

export type Preview = { state: PreviewState; shown: boolean };

export const appliedState = (before: Preview | undefined, state: PreviewState): Preview => ({
  state,
  shown: state.state === "live" || (before?.shown === true && state.state !== "starting" && state.state !== "full" && state.state !== "unpicked"),
});

export const rebuilding = (preview: Preview | undefined): boolean =>
  preview?.shown === true && (preview.state.state === "building" || preview.state.state === "waiting");

export function issueLine(issue: BuildIssue_Deserialize): string {
  const place = issuePlace(issue);
  return place ? `${place}: ${issue.message}` : issue.message;
}

export const shownError = (preview: Preview | undefined): BuildIssue_Deserialize | null =>
  preview?.shown && preview.state.state === "error" ? preview.state.issue : null;

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

export const previewName = (pick: PreviewPick): string => pick.name ?? `Preview ${pick.index + 1}`;

export const previewKey = (pick: PreviewPick): string => `${pick.file}#${pick.index}`;

export type PreviewRow = Row & { pick: PreviewPick | null };

function matchingPreviews(all: PreviewPick[], query: string): PreviewPick[] {
  const needle = query.trim().toLowerCase();
  return all.filter((pick) => `${previewName(pick)} ${pick.file}`.toLowerCase().includes(needle));
}

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

export function previewRows(all: PreviewPick[], query: string, closed: string[]): PreviewRow[] {
  const names = rowNames(all);
  const counts = new Map<string, number>();
  for (const pick of all) counts.set(pick.file, (counts.get(pick.file) ?? 0) + 1);
  const rows: PreviewRow[] = [];
  let file: string | null = null;
  let open = true;
  for (const pick of matchingPreviews(all, query)) {
    if (counts.get(pick.file) === 1) {
      file = pick.file;
      rows.push({ entry: { name: fileName(file), path: previewKey(pick), isDir: false }, depth: 0, open: false, pick });
      continue;
    }
    if (pick.file !== file) {
      file = pick.file;
      open = query.trim() !== "" || !closed.includes(file);
      rows.push({ entry: { name: fileName(file), path: file, isDir: true }, depth: 0, open, pick: null });
    }
    if (open) rows.push({ entry: { name: names.get(pick) ?? previewName(pick), path: previewKey(pick), isDir: false }, depth: 1, open: false, pick });
  }
  return rows;
}

export function steppedPreview(rows: PreviewRow[], from: string | null, down: boolean, byFile: boolean): PreviewPick | null {
  const at = rows.findIndex((row) => row.entry.path === from);
  if (!byFile) {
    const ahead = down ? rows.slice(at + 1) : rows.slice(0, at < 0 ? rows.length : at).reverse();
    return ahead.find((row) => row.pick)?.pick ?? null;
  }
  const files = rows.flatMap((row, i) => (row.depth === 0 ? [i] : []));
  const current = files.filter((i) => i <= at).length - 1;
  const targets = down ? files.slice(current + 1) : files.slice(0, Math.max(current, 0)).reverse();
  for (const i of targets) {
    const pick = rows[i].pick ?? (rows[i + 1]?.depth === 1 ? rows[i + 1].pick : null);
    if (pick) return pick;
  }
  return null;
}

// Rules for the Preview view: whether the preview simulator's screen shows, the centered line in its place, whether its header says a rebuild runs, the compile error its bar shows, how a `#Preview` reads and which ones the picker's search matches.
import type { BuildIssue_Deserialize, PreviewPick, PreviewState } from "../../../ipc/bindings";
import { issuePlace } from "../build/run.ts";
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

/** "ContentView" for `Views/ContentView.swift`. */
export const previewFileName = (pick: PreviewPick): string => fileName(pick.file).replace(/\.swift$/, "");

/** The previews whose name or file holds the query, ignoring case, in their file order. */
export function matchingPreviews(all: PreviewPick[], query: string): PreviewPick[] {
  const needle = query.trim().toLowerCase();
  return all.filter((pick) => `${previewName(pick)} ${pick.file}`.toLowerCase().includes(needle));
}

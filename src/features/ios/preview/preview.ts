// Rules for the Preview view: whether the preview simulator's screen shows, the centered line in its place, and whether its header says a rebuild runs.
import type { PreviewState } from "../../../ipc/bindings";

/** A worktree's preview state, and whether its preview app ran since the preview simulator booted. */
export type Preview = { state: PreviewState; shown: boolean };

/** A rebuild leaves the last preview on screen; a new boot or the limit takes it away. */
export const appliedState = (before: Preview | undefined, state: PreviewState): Preview => ({
  state,
  shown: state.state === "live" || (before?.shown === true && state.state !== "starting" && state.state !== "full"),
});

/** A build replaces the preview on screen; a hot reload never reads as one. */
export const rebuilding = (preview: Preview | undefined): boolean =>
  preview?.shown === true && (preview.state.state === "building" || preview.state.state === "waiting");

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
    case "failed":
      return state.message;
    case "live":
      return null;
  }
}

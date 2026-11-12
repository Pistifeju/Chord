// Rules for the finished marker: a pane that turned done stays marked until you open it.
import type { Status } from "../../ipc/bindings";

/** The status kind each pane last reported; a pane that never ran is absent. */
export type PaneKinds = Record<string, Status["kind"]>;

export function turnedDone(before: PaneKinds, now: PaneKinds): string[] {
  return Object.keys(now).filter((pane) => now[pane] === "done" && before[pane] !== "done");
}

/** The marks that stay: a pane that is gone loses its mark, and so does one on screen. */
export function keptMarks(marks: string[], live: PaneKinds, open: string[]): string[] {
  return [...new Set(marks)].filter((pane) => pane in live && !open.includes(pane));
}

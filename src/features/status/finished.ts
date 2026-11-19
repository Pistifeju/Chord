// Rules for the finished marker: a pane that turned done, failed or exited stays marked until you open it.
import type { Status } from "../../ipc/bindings";

/** The status kind each pane last reported; a pane that never ran is absent. */
export type PaneKinds = Record<string, Status["kind"]>;

const FINISHED: Status["kind"][] = ["done", "error", "exited"];

export function turnedFinished(before: PaneKinds, now: PaneKinds): string[] {
  return Object.keys(now).filter((pane) => FINISHED.includes(now[pane]) && before[pane] !== now[pane]);
}

/** The marks that stay: a pane that is gone loses its mark, and so does one on screen. */
export function keptMarks(marks: string[], live: PaneKinds, open: string[]): string[] {
  return [...new Set(marks)].filter((pane) => pane in live && !open.includes(pane));
}

import type { Status } from "../../ipc/bindings";

export type PaneKinds = Record<string, Status["kind"]>;

const FINISHED: Status["kind"][] = ["done", "waiting", "error", "exited"];

export function turnedFinished(before: PaneKinds, now: PaneKinds): string[] {
  return Object.keys(now).filter((pane) => FINISHED.includes(now[pane]) && before[pane] !== now[pane]);
}

export function keptMarks(marks: string[], live: PaneKinds, open: string[]): string[] {
  return [...new Set(marks)].filter((pane) => pane in live && !open.includes(pane));
}

// Rules for an agent's reported status: its words, which of several is most urgent, and how old it is.
import type { Status } from "../../ipc/bindings";

/** What Rust reported for a pane; the transcript rules stamp `since` when Rust says the status changed. */
export type StatusReport = { status: Status; lastMessage: string; since: number };

// Most urgent first: what needs the user before what runs or rests.
export const URGENCY: Status["kind"][] = ["waiting", "error", "exited", "running", "done"];

export function urgentReport(reports: (StatusReport | null | undefined)[]): StatusReport | null {
  let urgent: StatusReport | null = null;
  for (const report of reports) {
    if (report && (!urgent || URGENCY.indexOf(report.status.kind) < URGENCY.indexOf(urgent.status.kind))) urgent = report;
  }
  return urgent;
}

/** A set of panes' status you already saw: done, or a question with no dialog open, and none of its panes carries the finished mark. */
export const seenStatus = (kind: Status["kind"] | undefined, panes: string[], finished: string[], dialog: boolean): boolean =>
  (kind === "done" || (kind === "waiting" && !dialog)) && !panes.some((pane) => finished.includes(pane));

export function statusLabel(status: Status): string {
  switch (status.kind) {
    case "running":
      return "Working";
    case "waiting":
      return "Awaiting input";
    case "done":
      return "Done";
    case "error":
      return "Failed";
    case "exited":
      return status.code == null ? "Exited" : `Exited with code ${status.code}`;
  }
}

/** "now", "5m", "3h" or "2d". */
export function ageLabel(since: number, now: number): string {
  const minutes = Math.max(0, Math.floor((now - since) / 60_000));
  if (minutes < 1) return "now";
  if (minutes < 60) return `${minutes}m`;
  if (minutes < 60 * 24) return `${Math.floor(minutes / 60)}h`;
  return `${Math.floor(minutes / (60 * 24))}d`;
}

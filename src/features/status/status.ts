// Rules for an agent's reported status: its words, which of several is most urgent, and how old it is.
import type { Status } from "../../ipc/bindings";

/** What Rust reported for a pane; the transcript rules stamp `since` when the status changes. */
export type StatusReport = { status: Status; lastMessage: string; since: number };

// Most urgent first: what needs the user before what runs or rests.
const URGENCY: Status["kind"][] = ["waiting", "error", "exited", "running", "done"];

export function urgentReport(reports: (StatusReport | null | undefined)[]): StatusReport | null {
  let urgent: StatusReport | null = null;
  for (const report of reports) {
    if (report && (!urgent || URGENCY.indexOf(report.status.kind) < URGENCY.indexOf(urgent.status.kind))) urgent = report;
  }
  return urgent;
}

function sameStatus(a: Status, b: Status): boolean {
  return a.kind === b.kind && (a.kind !== "exited" || b.kind !== "exited" || (a.code ?? null) === (b.code ?? null));
}

/** The report to keep after a new one arrives: it keeps its start time while the status stays the same. */
export function stampedReport(previous: StatusReport | null, next: StatusReport, now: number): StatusReport {
  return { ...next, since: previous && sameStatus(previous.status, next.status) ? previous.since : now };
}

export function statusLabel(status: Status): string {
  switch (status.kind) {
    case "running":
      return "Running";
    case "waiting":
      return "Waiting for input";
    case "done":
      return "Done";
    case "error":
      return "Error";
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

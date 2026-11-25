import type { Status } from "../../ipc/bindings";

export type StatusReport = { status: Status; lastMessage: string; since: number };

export const URGENCY: Status["kind"][] = ["waiting", "error", "exited", "running", "done"];

export function urgentReport(reports: (StatusReport | null | undefined)[]): StatusReport | null {
  let urgent: StatusReport | null = null;
  for (const report of reports) {
    if (report && (!urgent || URGENCY.indexOf(report.status.kind) < URGENCY.indexOf(urgent.status.kind))) urgent = report;
  }
  return urgent;
}

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

export function ageLabel(since: number, now: number): string {
  const minutes = Math.max(0, Math.floor((now - since) / 60_000));
  if (minutes < 1) return "now";
  if (minutes < 60) return `${minutes}m`;
  if (minutes < 60 * 24) return `${Math.floor(minutes / 60)}h`;
  return `${Math.floor(minutes / (60 * 24))}d`;
}

import type { WorktreePanes } from "../../ipc/bindings";
import { chatPaneIds } from "../panes/layout.ts";
import { URGENCY, type StatusReport } from "./status.ts";

export type AgentPane = { worktree: string; pane: string };

type Reports = Record<string, { report: StatusReport | null; dialog?: unknown } | undefined>;

export const agentPanes = (order: string[], panes: Record<string, WorktreePanes>): AgentPane[] =>
  order.flatMap((worktree) => (panes[worktree] ? chatPaneIds(panes[worktree].layout).map((pane) => ({ worktree, pane })) : []));

export function attentionQueue(all: AgentPane[], chats: Reports, finished: string[]): AgentPane[] {
  const report = (entry: AgentPane) => chats[entry.pane]?.report;
  const needs = (entry: AgentPane) => {
    const kind = report(entry)?.status.kind;
    return (kind === "waiting" && chats[entry.pane]?.dialog != null) || (kind !== undefined && kind !== "running" && finished.includes(entry.pane));
  };
  const urgency = (entry: AgentPane) => URGENCY.indexOf(report(entry)!.status.kind);
  return all.filter(needs).sort((a, b) => urgency(a) - urgency(b) || report(a)!.since - report(b)!.since);
}

export const runningAgents = (all: AgentPane[], chats: Reports): AgentPane[] => all.filter((entry) => chats[entry.pane]?.report?.status.kind === "running");

export function nextAttention(queue: AgentPane[], current: string | null): AgentPane | null {
  const at = queue.findIndex((entry) => entry.pane === current);
  if (at === -1) return queue[0] ?? null;
  return queue.length > 1 ? queue[(at + 1) % queue.length] : null;
}

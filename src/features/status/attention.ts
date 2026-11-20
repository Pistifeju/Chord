// The agents that need you, in the order Next Agent Needing You opens them: an open dialog or a question you have not opened first, then a Failed, an Exited or a Done you have not opened, the longest wait first within each; the agents that run; and the one after the pane you are on.
import type { WorktreePanes } from "../../ipc/bindings";
import { chatPaneIds } from "../panes/layout.ts";
import { URGENCY, type StatusReport } from "./status.ts";

export type AgentPane = { worktree: string; pane: string };

type Reports = Record<string, { report: StatusReport | null; dialog?: unknown } | undefined>;

/** Every chat pane of the worktrees, in sidebar order. */
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

/** The entry after the current pane, wrapping; the first when the current pane is not in the queue; null when nothing else needs you. */
export function nextAttention(queue: AgentPane[], current: string | null): AgentPane | null {
  const at = queue.findIndex((entry) => entry.pane === current);
  if (at === -1) return queue[0] ?? null;
  return queue.length > 1 ? queue[(at + 1) % queue.length] : null;
}

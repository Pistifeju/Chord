// The subagent lines' text: a run's name, tool and time joined, the summary of several runs, and when the time ticks.
import type { SubagentNode } from "../../ipc/bindings";
import { elapsedText } from "./elapsed";

function subagentElapsed(node: SubagentNode, now: number): string | null {
  if (node.started_at == null) return null;
  if (node.ended_at != null) return elapsedText(node.ended_at - node.started_at);
  return node.state === "running" ? elapsedText(now - node.started_at) : null;
}

export function subagentLine(node: SubagentNode, now: number): string {
  return [node.name, node.tool, subagentElapsed(node, now)].filter(Boolean).join(" · ");
}

const runsOf = (nodes: SubagentNode[]) => nodes.filter((n) => n.depth === 0);

// A chain's steps and a fan-out's workers are the agents the user counts, not the run that holds them.
export const leafCount = (nodes: SubagentNode[]) => nodes.filter((n, i) => (nodes[i + 1]?.depth ?? 0) <= n.depth).length;

const plural = (n: number, word: string) => `${n} ${word}${n === 1 ? "" : "s"}`;

export function subagentSummary(nodes: SubagentNode[], now: number): { text: string; mark: SubagentNode["state"] } | null {
  if (leafCount(nodes) < 2) return null;
  const runs = runsOf(nodes);
  const working = runs.filter((n) => n.state === "running").length;
  const failed = runs.filter((n) => n.state === "error").length;
  const starts = runs.flatMap((n) => (n.started_at == null ? [] : [n.started_at]));
  const ends = runs.flatMap((n) => (n.ended_at == null ? [] : [n.ended_at]));
  const end = working > 0 ? now : ends.length > 0 ? Math.max(...ends) : null;
  const time = starts.length > 0 && end != null ? elapsedText(end - Math.min(...starts)) : null;
  return {
    text: [plural(leafCount(nodes), "subagent"), time].filter(Boolean).join(" · "),
    mark: working > 0 ? "running" : failed > 0 ? "error" : "done",
  };
}

export function isTicking(nodes: SubagentNode[]): boolean {
  return nodes.some((n) => n.state === "running" && n.ended_at == null);
}

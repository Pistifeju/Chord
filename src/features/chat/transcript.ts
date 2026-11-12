// Rules for a chat's transcript: how each harness event changes the messages, the run flag and the notice.
import type { Capability, HarnessEvent } from "../../ipc/bindings";

export type Message = { role: "user" | "assistant"; text: string; streaming: boolean };

export type Step =
  | { kind: "thought"; /** Its place in the group; steps only append, so it stays stable. */ id: number; text: string }
  | { kind: "call"; id: string; title: string; detail: string; state: "running" | "done" | "failed" | "stopped" };

/** The thoughts and tool calls between two messages, folded into one row. */
export type StepGroup = { role: "steps"; steps: Step[] };

export type Item = Message | StepGroup;

export type Chat = {
  /** idle: no agent runs (the Start button shows); starting: the process is coming up; ready: it takes messages. */
  agent: "idle" | "starting" | "ready";
  items: Item[];
  running: boolean;
  /** When the current run started, in ms since the epoch; the store sets it. */
  since: number | null;
  /** What the harness can do; null until the agent has started. */
  capabilities: Capability[] | null;
  notice: string | null;
};

export const emptyChat: Chat = { agent: "idle", items: [], running: false, since: null, capabilities: null, notice: null };

const closed = (items: Item[]): Item[] => items.map((item) => (item.role !== "steps" && item.streaming ? { ...item, streaming: false } : item));

const withStep = (items: Item[], extend: (steps: Step[]) => Step[]): Item[] => {
  const last = items[items.length - 1];
  if (last?.role === "steps") return [...items.slice(0, -1), { role: "steps", steps: extend(last.steps) }];
  return [...closed(items), { role: "steps", steps: extend([]) }];
};

/** What an open group shows: all steps when expanded; while it runs, its last thought and last three calls; otherwise none. */
export function shownSteps(steps: Step[], running: boolean, expanded: boolean): Step[] {
  if (expanded) return steps;
  if (!running) return [];
  const indexes = (kind: Step["kind"]) => steps.flatMap((step, i) => (step.kind === kind ? [i] : []));
  const shown = new Set([...indexes("thought").slice(-1), ...indexes("call").slice(-3)]);
  return steps.filter((_, i) => shown.has(i));
}

export function stepsSummary(steps: Step[]): string {
  const calls = steps.filter((step) => step.kind === "call").length;
  const thoughts = steps.length - calls;
  const parts = [];
  if (thoughts > 0) parts.push(thoughts === 1 ? "1 thought" : `${thoughts} thoughts`);
  if (calls > 0) parts.push(calls === 1 ? "1 call" : `${calls} calls`);
  return parts.join(", ");
}

export function chatAfter(chat: Chat, event: HarnessEvent): Chat {
  switch (event.kind) {
    case "user_message_chunk":
      return { ...chat, items: [...closed(chat.items), { role: "user", text: event.text, streaming: false }] };
    case "message_chunk": {
      const last = chat.items[chat.items.length - 1];
      if (last?.role === "assistant" && last.streaming) return { ...chat, items: [...chat.items.slice(0, -1), { ...last, text: last.text + event.text }] };
      return { ...chat, items: [...chat.items, { role: "assistant", text: event.text, streaming: true }] };
    }
    case "thought_start":
      return { ...chat, items: withStep(chat.items, (steps) => [...steps, { kind: "thought", id: steps.length, text: "" }]) };
    case "thought_chunk":
      return {
        ...chat,
        items: withStep(chat.items, (steps) => {
          const last = steps[steps.length - 1];
          if (last?.kind === "thought") return [...steps.slice(0, -1), { ...last, text: last.text + event.text }];
          return [...steps, { kind: "thought", id: steps.length, text: event.text }];
        }),
      };
    case "tool_start":
      return { ...chat, items: withStep(chat.items, (steps) => [...steps, { kind: "call", id: event.id, title: event.title, detail: event.detail, state: "running" }]) };
    case "tool_end": {
      const ended = (step: Step): Step => (step.kind === "call" && step.id === event.id ? { ...step, state: event.outcome } : step);
      // The call is almost always in the last group.
      for (let i = chat.items.length - 1; i >= 0; i--) {
        const item = chat.items[i];
        if (item.role === "steps" && item.steps.some((step) => step.kind === "call" && step.id === event.id)) {
          return { ...chat, items: chat.items.map((other, j) => (j === i ? { ...item, steps: item.steps.map(ended) } : other)) };
        }
      }
      return chat;
    }
    case "message_end":
      return { ...chat, items: closed(chat.items) };
    case "state":
      return { ...chat, running: event.running, items: event.running ? chat.items : closed(chat.items) };
    case "notice":
      return { ...chat, notice: event.text };
    case "exited":
      return { ...chat, agent: "idle", running: false, items: closed(chat.items) };
  }
}

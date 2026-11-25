import { elapsedText } from "./elapsed.ts";
import type { Capability, DialogAsk, HarnessEvent, Image, LineChanges, SlashCommand, SubagentNode } from "../../ipc/bindings";
import type { StatusReport } from "../status/status.ts";

export type Message = { role: "user" | "assistant"; text: string; streaming: boolean; images?: Image[]; at?: number };

export type Step =
  | { kind: "thought"; id: number; text: string }
  | { kind: "call"; id: string; title: string; detail: string; command?: true; state: "running" | "done" | "failed" | "stopped"; images?: Image[]; ms?: number; changes?: LineChanges };

export type StepGroup = { role: "steps"; steps: Step[] };

type StopMark = { role: "stopped" };

export type CompactionMark = { role: "compaction"; summary: string; tokensBefore: number | null };

export type Item = Message | StepGroup | StopMark | CompactionMark;

export type OpenDialog = { id: string; title: string; ask: DialogAsk };

export type ModelChoice = { id: string | null; name: string; thinking: string | null };

type ContextFill = { tokens: number | null; window: number | null; estimated: boolean };

export type Chat = {
  agent: "idle" | "starting" | "ready";
  items: Item[];
  running: boolean;
  since: number | null;
  compacting: boolean;
  capabilities: Capability[] | null;
  notice: string | null;
  dialog: OpenDialog | null;
  model: ModelChoice | null;
  permissions: { mode: string; asks: boolean } | null;
  context: ContextFill | null;
  commands: SlashCommand[];
  subagents: SubagentNode[];
  subagentsAt: number | null;
  queued: string[];
  steerImages: SteerImages[];
  report: StatusReport | null;
  historyLength: number;
  loading: boolean;
  held: string | null;
};

export const emptyChat: Chat = { agent: "idle", items: [], running: false, since: null, compacting: false, capabilities: null, notice: null, dialog: null, model: null, permissions: null, context: null, commands: [], subagents: [], subagentsAt: null, queued: [], steerImages: [], report: null, historyLength: 0, loading: false, held: null };

export type SteerImages = { text: string; images: Image[] };

export const withSteerImages = (chat: Chat, text: string, images: Image[]): Chat =>
  chat.running && images.length > 0 ? { ...chat, steerImages: [...chat.steerImages, { text, images }] } : chat;

export const queuedImages = (steers: SteerImages[], text: string): Image[] => steers.find((steer) => steer.text === text)?.images ?? [];

const withoutSteer = (steers: SteerImages[], text: string) => {
  const i = steers.findIndex((steer) => steer.text === text);
  return i < 0 ? steers : steers.filter((_, j) => j !== i);
};

export const anyRunning = (chats: Record<string, Chat>, panes: string[]) => panes.some((pane) => chats[pane]?.running);

export const isStreaming = (item: Item) => "streaming" in item && item.streaming;

function closed(items: Item[]): Item[] {
  let from = items.length;
  while (from > 0 && (isStreaming(items[from - 1]) || items[from - 1].role === "stopped")) from--;
  let next = items;
  for (let i = from; i < items.length; i++) {
    const item = items[i];
    if (!("streaming" in item && item.streaming)) continue;
    if (next === items) next = items.slice();
    next[i] = { ...item, streaming: false };
  }
  return next;
}

const withLast = <T,>(list: T[], last: T): T[] => {
  const next = list.slice();
  next[next.length - 1] = last;
  return next;
};

const withStep = (items: Item[], extend: (steps: Step[]) => Step[]): Item[] => {
  const last = items[items.length - 1];
  if (last?.role === "steps") return withLast<Item>(items, { role: "steps", steps: extend(last.steps) });
  return [...closed(items), { role: "steps", steps: extend([]) }];
};

export function stepImages(steps: Step[]): Image[] {
  return steps.flatMap((step) => (step.kind === "call" ? (step.images ?? []) : []));
}

const plural = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`;

export type CallKind = "read" | "search" | "list" | "run" | "edit" | "write" | "other";

const callKinds = new Map<string, CallKind>([
  ["Read", "read"],
  ["Search", "search"],
  ["List", "list"],
  ["Run", "run"],
  ["Edit", "edit"],
  ["Write", "write"],
]);

export const callKind = (title: string): CallKind => callKinds.get(title) ?? "other";

type StepKind = "thoughts" | "read" | "searches" | "listed" | "edited" | "ran" | "other";

function stepKinds(steps: Step[]): { kind: StepKind; count: number; text: string }[] {
  const calls = steps.flatMap((step) => (step.kind === "call" ? [step] : []));
  const count = (...kinds: CallKind[]) => calls.filter((call) => kinds.includes(callKind(call.title))).length;
  const edited = new Set(calls.filter((call) => ["edit", "write"].includes(callKind(call.title))).map((call) => call.detail)).size;
  const thoughts = steps.length - calls.length;
  const [read, searches, listed, ran, other] = [count("read"), count("search"), count("list"), count("run"), count("other")];
  return [
    { kind: "thoughts" as const, count: thoughts, text: plural(thoughts, "thought", "thoughts") },
    { kind: "read" as const, count: read, text: `read ${plural(read, "file", "files")}` },
    { kind: "searches" as const, count: searches, text: plural(searches, "search", "searches") },
    { kind: "listed" as const, count: listed, text: `listed ${plural(listed, "folder", "folders")}` },
    { kind: "edited" as const, count: edited, text: `edited ${plural(edited, "file", "files")}` },
    { kind: "ran" as const, count: ran, text: `ran ${plural(ran, "command", "commands")}` },
    { kind: "other" as const, count: other, text: plural(other, "tool call", "tool calls") },
  ].filter((part) => part.count > 0);
}

const sentence = (text: string) => text.charAt(0).toUpperCase() + text.slice(1);

export function stepsSummary(steps: Step[]): string {
  return sentence(stepKinds(steps).map((part) => part.text).join(", "));
}

const headlineOrder: StepKind[] = ["edited", "ran", "other", "searches", "read", "listed", "thoughts"];

export function stepsHeadline(steps: Step[]): string {
  const parts = stepKinds(steps).sort((a, b) => headlineOrder.indexOf(a.kind) - headlineOrder.indexOf(b.kind));
  const rest = parts.slice(2).reduce((sum, part) => sum + part.count, 0);
  return sentence([...parts.slice(0, 2).map((part) => part.text), ...(rest > 0 ? [`${rest} more`] : [])].join(", "));
}

export const isLive = (items: Item[], index: number, running: boolean) => running && index === items.length - 1;

export const callText = (step: Extract<Step, { kind: "call" }>, worktree: string) => (step.command || !step.detail.startsWith(`${worktree}/`) ? step.detail : step.detail.slice(worktree.length + 1));

const fileName = (path: string) => path.slice(path.lastIndexOf("/") + 1);

const doing = new Map<CallKind, string>([
  ["read", "Reading"],
  ["edit", "Editing"],
  ["write", "Writing"],
]);

export function stepDoing(step: Step): string {
  if (step.kind === "thought") return "Thinking";
  if (step.command) return `Running ${step.detail}`;
  const verb = doing.get(callKind(step.title));
  return verb ? `${verb} ${fileName(step.detail)}` : step.title;
}

export function stepsChanges(steps: Step[]): LineChanges | null {
  const done = steps.flatMap((step) => (step.kind === "call" && step.state === "done" && step.changes ? [step.changes] : []));
  if (done.length === 0) return null;
  return done.reduce((sum, c) => ({ added: sum.added + c.added, removed: sum.removed + c.removed }));
}

const SLOW_MS = 5000;

const seconds = (ms: number) => (ms < 1000 ? `${ms} ms` : ms < 60_000 ? `${(ms / 1000).toFixed(1)} s` : elapsedText(ms));

export function callTime(step: Step): string | null {
  if (step.kind !== "call" || step.ms === undefined) return null;
  return step.ms >= SLOW_MS || step.state === "failed" ? seconds(step.ms) : null;
}

export function stepsTime(steps: Step[]): string | null {
  const ms = steps.reduce((sum, step) => sum + (step.kind === "call" ? (step.ms ?? 0) : 0), 0);
  return ms >= SLOW_MS ? seconds(ms) : null;
}

export type Replay = HarnessEvent[] | null;

export function chatFed(chat: Chat, replay: Replay, event: HarnessEvent, now: number): { chat: Chat; replay: Replay } {
  const held = event.kind === "history_start" ? (replay ?? []) : replay;
  if (!held) return { chat: chatAfter(chat, event, now), replay: null };
  held.push(event);
  if (event.kind !== "history_end" && event.kind !== "exited") return { chat: event.kind === "history_start" ? { ...chat, loading: true } : chat, replay: held };
  return { chat: held.reduce((next, replayed) => chatAfter(next, replayed, now), chat), replay: null };
}

export function chatAfter(chat: Chat, event: HarnessEvent, now: number): Chat {
  switch (event.kind) {
    case "user_message_chunk":
      return { ...chat, steerImages: withoutSteer(chat.steerImages, event.text), items: [...closed(chat.items), { role: "user", text: event.text, streaming: false, images: event.images, at: event.at ?? now }] };
    case "message_chunk": {
      const last = chat.items[chat.items.length - 1];
      if (last?.role === "assistant" && last.streaming) return { ...chat, items: withLast<Item>(chat.items, { ...last, text: last.text + event.text }) };
      return { ...chat, items: [...chat.items, { role: "assistant", text: event.text, streaming: true }] };
    }
    case "thought_start":
      return { ...chat, items: withStep(chat.items, (steps) => [...steps, { kind: "thought", id: steps.length, text: "" }]) };
    case "thought_chunk":
      return {
        ...chat,
        items: withStep(chat.items, (steps) => {
          const last = steps[steps.length - 1];
          if (last?.kind === "thought") return withLast<Step>(steps, { ...last, text: last.text + event.text });
          return [...steps, { kind: "thought", id: steps.length, text: event.text }];
        }),
      };
    case "tool_start":
      return { ...chat, items: withStep(chat.items, (steps) => [...steps, { kind: "call", id: event.id, title: event.title, detail: event.detail, state: "running", ...(event.command && { command: true as const }), ...(event.changes && { changes: event.changes }) }]) };
    case "tool_end": {
      const ended = (step: Step): Step => (step.kind === "call" && step.id === event.id ? { ...step, state: event.outcome, images: event.images, ...(event.ms != null && { ms: event.ms }), ...(event.changes && { changes: event.changes }) } : step);
      for (let i = chat.items.length - 1; i >= 0; i--) {
        const item = chat.items[i];
        if (item.role === "steps" && item.steps.some((step) => step.kind === "call" && step.id === event.id)) {
          const items = chat.items.slice();
          items[i] = { ...item, steps: item.steps.map(ended) };
          return { ...chat, items };
        }
      }
      return chat;
    }
    case "dialog":
      return { ...chat, dialog: { id: event.id, title: event.title, ask: event.ask } };
    case "dialog_closed":
      return chat.dialog && chat.dialog.ask.kind !== "menu" ? { ...chat, dialog: null } : chat;
    case "model":
      return { ...chat, model: { id: event.id ?? null, name: event.name, thinking: event.thinking ?? null } };
    case "context":
      return { ...chat, context: { tokens: event.tokens ?? null, window: event.window ?? null, estimated: event.estimated } };
    case "permissions":
      return { ...chat, permissions: { mode: event.mode, asks: event.asks } };
    case "commands":
      return { ...chat, commands: event.commands };
    case "subagents":
      return { ...chat, subagents: event.nodes, subagentsAt: event.nodes.length === 0 ? null : (chat.subagentsAt ?? chat.items.length) };
    case "queue":
      return { ...chat, queued: event.steering };
    case "message_end": {
      const last = chat.items[chat.items.length - 1];
      const items = last?.role === "assistant" && last.streaming ? withLast<Item>(chat.items, { ...last, at: event.at ?? now }) : chat.items;
      return { ...chat, items: closed(items) };
    }
    case "state":
      return {
        ...chat,
        running: event.running,
        since: event.running && !chat.running ? now : chat.since,
        compacting: event.running && chat.compacting,
        items: event.running ? chat.items : closed(chat.items),
      };
    case "compacting":
      return { ...chat, compacting: event.active };
    case "aborted":
      return { ...chat, items: [...chat.items, { role: "stopped" }] };
    case "compaction":
      return { ...chat, items: [...closed(chat.items), { role: "compaction", summary: event.summary, tokensBefore: event.tokens_before ?? null }] };
    case "status":
      return { ...chat, report: { status: event.status, lastMessage: event.last_message, since: event.changed ? now : (chat.report?.since ?? now) } };
    case "notice":
      return { ...chat, notice: event.text };
    case "history_start":
      return { ...chat, items: [], subagentsAt: null, queued: [], steerImages: [], report: null, historyLength: 0, context: null };
    case "history_end":
      return { ...chat, loading: false, historyLength: chat.items.length, subagentsAt: chat.subagents.length > 0 ? chat.items.length : null };
    case "session_info":
    case "draft":
    case "exported":
    case "usage":
    case "models":
      return chat;
    case "exited":
      return { ...chat, agent: "idle", running: false, compacting: false, queued: [], steerImages: [], context: null, permissions: null, subagents: [], subagentsAt: null, dialog: null, items: closed(chat.items) };
  }
}

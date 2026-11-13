// Rules for a chat's transcript: how each harness event changes the messages, the run flag, the notice and the status report.
import { elapsedText } from "./elapsed.ts";
import type { Capability, DialogAsk, HarnessEvent, Image, LineChanges, SlashCommand, SubagentNode } from "../../ipc/bindings";
import { stampedReport, type StatusReport } from "../status/status.ts";

export type Message = { role: "user" | "assistant"; text: string; streaming: boolean; /** The images a user message carried. */ images?: Image[]; /** When it was sent or written, in ms since the epoch; none for a reply that never ended. */ at?: number };

export type Step =
  | { kind: "thought"; /** Its place in the group; steps only append, so it stays stable. */ id: number; text: string }
  | { kind: "call"; id: string; title: string; detail: string; state: "running" | "done" | "failed" | "stopped"; /** The images the tool returned. */ images?: Image[]; /** How long it ran, when the agent measured it. */ ms?: number; /** The lines it added to and removed from a file. */ changes?: LineChanges };

/** The thoughts and tool calls between two messages, folded into one row. */
export type StepGroup = { role: "steps"; steps: Step[] };

/** Marks where the user stopped a run. */
type StopMark = { role: "stopped" };

/** Where the agent replaced the earlier messages with a summary to free the context. */
export type CompactionMark = { role: "compaction"; summary: string; /** How many tokens the replaced messages filled, when the agent says. */ tokensBefore: number | null };

export type Item = Message | StepGroup | StopMark | CompactionMark;

/** A dialog the agent waits on; `id` goes back with the answer. */
export type OpenDialog = { id: string; title: string; ask: DialogAsk };

/** A model's name and, when it reasons, its thinking level. */
export type ModelChoice = { name: string; thinking: string | null };

/** The model's context window and the tokens the session fills of it; null tokens right after a compaction. */
type ContextFill = { tokens: number | null; window: number };

export type Chat = {
  /** idle: no agent runs (the Start button shows); starting: the process is coming up; ready: it takes messages. */
  agent: "idle" | "starting" | "ready";
  items: Item[];
  running: boolean;
  /** When the current run started, in ms since the epoch. */
  since: number | null;
  /** What the harness can do; null until the agent has started. */
  capabilities: Capability[] | null;
  notice: string | null;
  /** The dialog the agent waits on, shown over the composer. */
  dialog: OpenDialog | null;
  /** The model the agent runs; null until the agent tells. */
  model: ModelChoice | null;
  /** How full the context window is; null until the agent tells. */
  context: ContextFill | null;
  /** The commands the agent offers after a slash; none until it lists them. */
  commands: SlashCommand[];
  /** The background subagent runs and what each holds, each node before its children; none without a subagent extension. */
  subagents: SubagentNode[];
  /** Steering messages the agent has not taken in yet. */
  queued: string[];
  /** The status Rust reported; null until the first run. */
  report: StatusReport | null;
  /** How many of the items came back as the session's history; they show without the arrival motion. */
  historyLength: number;
};

export const emptyChat: Chat = { agent: "idle", items: [], running: false, since: null, capabilities: null, notice: null, dialog: null, model: null, context: null, commands: [], subagents: [], queued: [], report: null, historyLength: 0 };

export const isStreaming = (item: Item) => "streaming" in item && item.streaming;

const closed = (items: Item[]): Item[] => items.map((item) => ("streaming" in item && item.streaming ? { ...item, streaming: false } : item));

// One copy of the list instead of the two a spread of a slice makes; it runs on every chunk.
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

/** What an open group shows: all steps when expanded; while it runs, its last thought and last three calls; otherwise none. */
export function shownSteps(steps: Step[], running: boolean, expanded: boolean): Step[] {
  if (expanded) return steps;
  if (!running) return [];
  const indexes = (kind: Step["kind"]) => steps.flatMap((step, i) => (step.kind === kind ? [i] : []));
  const shown = new Set([...indexes("thought").slice(-1), ...indexes("call").slice(-3)]);
  return steps.filter((_, i) => shown.has(i));
}

/** Every image the group's tools returned, in call order. */
export function stepImages(steps: Step[]): Image[] {
  return steps.flatMap((step) => (step.kind === "call" ? (step.images ?? []) : []));
}

const plural = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`;

/** What the group did, by kind: "2 searches, edited 2 files, ran 4 commands". */
export function stepsSummary(steps: Step[]): string {
  const calls = steps.flatMap((step) => (step.kind === "call" ? [step] : []));
  const count = (...titles: string[]) => calls.filter((call) => titles.includes(call.title)).length;
  const edited = new Set(calls.filter((call) => call.title === "Edit" || call.title === "Write").map((call) => call.detail)).size;
  const other = calls.filter((call) => !["Read", "Search", "List", "Run", "Edit", "Write"].includes(call.title)).length;
  const thoughts = steps.length - calls.length;
  const parts = [
    thoughts > 0 && plural(thoughts, "thought", "thoughts"),
    count("Read") > 0 && `read ${plural(count("Read"), "file", "files")}`,
    count("Search") > 0 && plural(count("Search"), "search", "searches"),
    count("List") > 0 && `listed ${plural(count("List"), "folder", "folders")}`,
    edited > 0 && `edited ${plural(edited, "file", "files")}`,
    count("Run") > 0 && `ran ${plural(count("Run"), "command", "commands")}`,
    other > 0 && plural(other, "tool call", "tool calls"),
  ].filter((part) => part !== false);
  const text = parts.join(", ");
  return text.charAt(0).toUpperCase() + text.slice(1);
}

/** The lines the group's finished calls added and removed, or null when none changed a file. */
export function stepsChanges(steps: Step[]): LineChanges | null {
  const done = steps.flatMap((step) => (step.kind === "call" && step.state === "done" && step.changes ? [step.changes] : []));
  if (done.length === 0) return null;
  return done.reduce((sum, c) => ({ added: sum.added + c.added, removed: sum.removed + c.removed }));
}

// Below this a call's run time is noise.
const SLOW_MS = 5000;

const seconds = (ms: number) => (ms < 1000 ? `${ms} ms` : ms < 60_000 ? `${(ms / 1000).toFixed(1)} s` : elapsedText(ms));

/** A call's run time, shown when it ran slow or failed. */
export function callTime(step: Step): string | null {
  if (step.kind !== "call" || step.ms === undefined) return null;
  return step.ms >= SLOW_MS || step.state === "failed" ? seconds(step.ms) : null;
}

/** The group's total call time, shown when it ran slow. */
export function stepsTime(steps: Step[]): string | null {
  const ms = steps.reduce((sum, step) => sum + (step.kind === "call" ? (step.ms ?? 0) : 0), 0);
  return ms >= SLOW_MS ? seconds(ms) : null;
}

// Mirrors the events that clear a dialog in Rust's `status_for`; `exited` also clears it, as nobody is left to answer.
const movesOn = (event: HarnessEvent): boolean =>
  event.kind === "message_chunk" ||
  event.kind === "thought_start" ||
  event.kind === "thought_chunk" ||
  event.kind === "tool_start" ||
  event.kind === "tool_end" ||
  event.kind === "exited" ||
  (event.kind === "state" && event.running);

/** `now` is the event's arrival time in ms since the epoch; it stamps when a run starts and when the status changes. */
export function chatAfter(chat: Chat, event: HarnessEvent, now: number): Chat {
  const next = applied(chat, event, now);
  // A picker menu the user opened waits for the user, not for the run.
  const keeps = next.dialog?.ask.kind === "menu" && event.kind !== "exited";
  return next.dialog && !keeps && movesOn(event) ? { ...next, dialog: null } : next;
}

/** The history events held until the replay ends, or null while events apply one by one. */
export type Replay = HarnessEvent[] | null;

/** The chat and the held replay after `event`: a session's history changes the chat once, at its end, stamped with that `now`. The held array is the caller's to pass back. */
export function chatFed(chat: Chat, replay: Replay, event: HarnessEvent, now: number): { chat: Chat; replay: Replay } {
  const held = event.kind === "history_start" ? [] : replay;
  if (!held) return { chat: chatAfter(chat, event, now), replay: null };
  held.push(event);
  if (event.kind !== "history_end") return { chat, replay: held };
  return { chat: held.reduce((next, replayed) => chatAfter(next, replayed, now), chat), replay: null };
}

function applied(chat: Chat, event: HarnessEvent, now: number): Chat {
  switch (event.kind) {
    case "user_message_chunk":
      return { ...chat, items: [...closed(chat.items), { role: "user", text: event.text, streaming: false, images: event.images, at: event.at ?? now }] };
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
      return { ...chat, items: withStep(chat.items, (steps) => [...steps, { kind: "call", id: event.id, title: event.title, detail: event.detail, state: "running", ...(event.changes && { changes: event.changes }) }]) };
    case "tool_end": {
      const ended = (step: Step): Step => (step.kind === "call" && step.id === event.id ? { ...step, state: event.outcome, images: event.images, ...(event.ms != null && { ms: event.ms }), ...(event.changes && { changes: event.changes }) } : step);
      // The call is almost always in the last group.
      for (let i = chat.items.length - 1; i >= 0; i--) {
        const item = chat.items[i];
        if (item.role === "steps" && item.steps.some((step) => step.kind === "call" && step.id === event.id)) {
          return { ...chat, items: chat.items.map((other, j) => (j === i ? { ...item, steps: item.steps.map(ended) } : other)) };
        }
      }
      return chat;
    }
    case "dialog":
      return { ...chat, dialog: { id: event.id, title: event.title, ask: event.ask } };
    case "model":
      return { ...chat, model: { name: event.name, thinking: event.thinking ?? null } };
    case "context":
      return { ...chat, context: event.window == null ? null : { tokens: event.tokens ?? null, window: event.window } };
    case "commands":
      return { ...chat, commands: event.commands };
    case "subagents":
      return { ...chat, subagents: event.nodes };
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
        items: event.running ? chat.items : closed(chat.items),
      };
    case "aborted":
      return { ...chat, items: [...chat.items, { role: "stopped" }] };
    case "compaction":
      return { ...chat, items: [...closed(chat.items), { role: "compaction", summary: event.summary, tokensBefore: event.tokens_before ?? null }] };
    case "status":
      return { ...chat, report: stampedReport(chat.report, { status: event.status, lastMessage: event.last_message, since: now }, now) };
    case "notice":
      return { ...chat, notice: event.text };
    // The lineage is not cleared here: the extension empties it itself on a new session, and a snapshot it restores arrives before this event. Rust's `status_for` starts the status over here.
    case "history_start":
      return { ...chat, items: [], queued: [], report: null, historyLength: 0 };
    case "history_end":
      return { ...chat, historyLength: chat.items.length };
    // Rust saves the session; the store puts a draft in the composer and tells of an export; the transcript holds none of them.
    case "session_info":
    case "draft":
    case "exported":
    case "usage":
      return chat;
    case "exited":
      return { ...chat, agent: "idle", running: false, queued: [], context: null, subagents: [], items: closed(chat.items) };
  }
}

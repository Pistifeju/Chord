// Rules for a chat's transcript: how each harness event changes the messages, the run flag, the notice and the status report.
import { elapsedText } from "./elapsed.ts";
import type { Capability, DialogAsk, HarnessEvent, Image, LineChanges, SlashCommand, SubagentNode } from "../../ipc/bindings";
import type { StatusReport } from "../status/status.ts";

export type Message = { role: "user" | "assistant"; text: string; streaming: boolean; /** The images a user message carried. */ images?: Image[]; /** When it was sent or written, in ms since the epoch; none for a reply that never ended. */ at?: number };

export type Step =
  | { kind: "thought"; /** Its place in the group; steps only append, so it stays stable. */ id: number; text: string }
  | { kind: "call"; id: string; title: string; detail: string; /** The detail is a shell command. */ command?: true; state: "running" | "done" | "failed" | "stopped"; /** The images the tool returned. */ images?: Image[]; /** How long it ran, when the agent measured it. */ ms?: number; /** The lines it added to and removed from a file. */ changes?: LineChanges };

/** The thoughts and tool calls between two messages, folded into one row. */
export type StepGroup = { role: "steps"; steps: Step[] };

/** Marks where the user stopped a run. */
type StopMark = { role: "stopped" };

/** Where the agent replaced the earlier messages with a summary to free the context. */
export type CompactionMark = { role: "compaction"; summary: string; /** How many tokens the replaced messages filled, when the agent says. */ tokensBefore: number | null };

export type Item = Message | StepGroup | StopMark | CompactionMark;

/** A dialog the agent waits on; `id` goes back with the answer. */
export type OpenDialog = { id: string; title: string; ask: DialogAsk };

/** A model's name and, when it reasons, its thinking level; `id` names it in the harness's model list, when the harness says. */
export type ModelChoice = { id: string | null; name: string; thinking: string | null };

/** The model's context window and the tokens the session fills of it; null tokens right after a compaction. */
type ContextFill = { tokens: number | null; window: number; /** Chord counted the tokens itself, until a reply measures them. */ estimated: boolean };

export type Chat = {
  /** idle: no agent runs (the Start button shows); starting: the process is coming up; ready: it takes messages. */
  agent: "idle" | "starting" | "ready";
  items: Item[];
  running: boolean;
  /** When the current run started, in ms since the epoch. */
  since: number | null;
  /** The agent compacts the context now. */
  compacting: boolean;
  /** What the harness can do; null until the agent has started. */
  capabilities: Capability[] | null;
  notice: string | null;
  /** The dialog the agent waits on, shown over the composer. */
  dialog: OpenDialog | null;
  /** The model the agent runs; null until the agent tells. */
  model: ModelChoice | null;
  /** The session's permission mode and whether it still asks; null for a harness that sends none. */
  permissions: { mode: string; asks: boolean } | null;
  /** How full the context window is; null until the agent tells. */
  context: ContextFill | null;
  /** The commands the agent offers after a slash; none until it lists them. */
  commands: SlashCommand[];
  /** The background subagent runs and what each holds, each node before its children; none without a subagent extension. */
  subagents: SubagentNode[];
  /** The item the subagent lines sit before, fixed when the first run shows; null puts them at the end. */
  subagentsAt: number | null;
  /** Steering messages the agent has not taken in yet. */
  queued: string[];
  /** The images of steers sent while the agent ran, until it takes them in; its queue holds only their text. */
  steerImages: SteerImages[];
  /** The status Rust reported; null until the first run. */
  report: StatusReport | null;
  /** How many of the items came back as the session's history; they show without the arrival motion. */
  historyLength: number;
  /** From the start or a `history_start` until its `history_end`: the harness may not take a message yet. */
  loading: boolean;
  /** The message sent while the chat loaded; it goes out when the history ends. */
  held: string | null;
};

export const emptyChat: Chat = { agent: "idle", items: [], running: false, since: null, compacting: false, capabilities: null, notice: null, dialog: null, model: null, permissions: null, context: null, commands: [], subagents: [], subagentsAt: null, queued: [], steerImages: [], report: null, historyLength: 0, loading: false, held: null };

export type SteerImages = { text: string; images: Image[] };

/** Remembers the images of a message sent while the agent runs, so its queued row shows them. */
export const withSteerImages = (chat: Chat, text: string, images: Image[]): Chat =>
  chat.running && images.length > 0 ? { ...chat, steerImages: [...chat.steerImages, { text, images }] } : chat;

export const queuedImages = (steers: SteerImages[], text: string): Image[] => steers.find((steer) => steer.text === text)?.images ?? [];

const withoutSteer = (steers: SteerImages[], text: string) => {
  const i = steers.findIndex((steer) => steer.text === text);
  return i < 0 ? steers : steers.filter((_, j) => j !== i);
};

export const anyRunning = (chats: Record<string, Chat>, panes: string[]) => panes.some((pane) => chats[pane]?.running);

export const isStreaming = (item: Item) => "streaming" in item && item.streaming;

// Every item but a reply or a stop mark arrives after a close, so a streaming reply sits only in the run of those at the end.
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

/** Every image the group's tools returned, in call order. */
export function stepImages(steps: Step[]): Image[] {
  return steps.flatMap((step) => (step.kind === "call" ? (step.images ?? []) : []));
}

const plural = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`;

/** What a call does, by its title; a title the harnesses do not share is "other". */
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

/** What the group did, by kind: "2 searches, edited 2 files, ran 4 commands". */
export function stepsSummary(steps: Step[]): string {
  return sentence(stepKinds(steps).map((part) => part.text).join(", "));
}

// Changes to the worktree outrank what only looked at it.
const headlineOrder: StepKind[] = ["edited", "ran", "other", "searches", "read", "listed", "thoughts"];

/** The group's two weightiest kinds and a count of the rest: "Edited 4 files, ran 1 command, 3 more". */
export function stepsHeadline(steps: Step[]): string {
  const parts = stepKinds(steps).sort((a, b) => headlineOrder.indexOf(a.kind) - headlineOrder.indexOf(b.kind));
  const rest = parts.slice(2).reduce((sum, part) => sum + part.count, 0);
  return sentence([...parts.slice(0, 2).map((part) => part.text), ...(rest > 0 ? [`${rest} more`] : [])].join(", "));
}

/** Whether the item at `index` is the run's newest, so a step group there reads its steps live. */
export const isLive = (items: Item[], index: number, running: boolean) => running && index === items.length - 1;

/** A call's detail as its row shows it: a command as written, a path inside the worktree from the worktree's root. */
export const callText = (step: Extract<Step, { kind: "call" }>, worktree: string) => (step.command || !step.detail.startsWith(`${worktree}/`) ? step.detail : step.detail.slice(worktree.length + 1));

const fileName = (path: string) => path.slice(path.lastIndexOf("/") + 1);

const doing = new Map<CallKind, string>([
  ["read", "Reading"],
  ["edit", "Editing"],
  ["write", "Writing"],
]);

/** What a running group's newest step does, as one line: "Running cargo check", "Editing mod.rs", "Thinking". */
export function stepDoing(step: Step): string {
  if (step.kind === "thought") return "Thinking";
  if (step.command) return `Running ${step.detail}`;
  const verb = doing.get(callKind(step.title));
  return verb ? `${verb} ${fileName(step.detail)}` : step.title;
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

/** The history events held until the replay ends, or null while events apply one by one. */
export type Replay = HarnessEvent[] | null;

/** The chat and the held replay after `event`: a session's history changes the chat once, at its end, stamped with that `now`. The held array is the caller's to pass back. */
export function chatFed(chat: Chat, replay: Replay, event: HarnessEvent, now: number): { chat: Chat; replay: Replay } {
  // A start inside a held replay keeps what came before it; replaying the start clears the transcript again.
  const held = event.kind === "history_start" ? (replay ?? []) : replay;
  if (!held) return { chat: chatAfter(chat, event, now), replay: null };
  held.push(event);
  // An exit ends a replay it cut short.
  if (event.kind !== "history_end" && event.kind !== "exited") return { chat: event.kind === "history_start" ? { ...chat, loading: true } : chat, replay: held };
  return { chat: held.reduce((next, replayed) => chatAfter(next, replayed, now), chat), replay: null };
}

/** `now` is the event's arrival time in ms since the epoch; it stamps when a run starts and when the status changes. */
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
      // The call is almost always in the last group.
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
    // A picker menu the user opened waits for the user, not for the run.
    case "dialog_closed":
      return chat.dialog && chat.dialog.ask.kind !== "menu" ? { ...chat, dialog: null } : chat;
    case "model":
      return { ...chat, model: { id: event.id ?? null, name: event.name, thinking: event.thinking ?? null } };
    case "context":
      return { ...chat, context: event.window == null ? null : { tokens: event.tokens ?? null, window: event.window, estimated: event.estimated } };
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
    // The lineage is not cleared here: the extension empties it itself on a new session, and a snapshot it restores arrives before this event. Rust's `status_for` starts the status over here.
    case "history_start":
      return { ...chat, items: [], subagentsAt: null, queued: [], steerImages: [], report: null, historyLength: 0 };
    case "history_end":
      return { ...chat, loading: false, historyLength: chat.items.length, subagentsAt: chat.subagents.length > 0 ? chat.items.length : null };
    // Rust saves the session; the intake hands the draft, the export, the usage and the model list to the store; the transcript holds none of them.
    case "session_info":
    case "draft":
    case "exported":
    case "usage":
    case "models":
      return chat;
    case "exited":
      // Nobody is left to answer a dialog, a picker menu included.
      return { ...chat, agent: "idle", running: false, compacting: false, queued: [], steerImages: [], context: null, permissions: null, subagents: [], subagentsAt: null, dialog: null, items: closed(chat.items) };
  }
}

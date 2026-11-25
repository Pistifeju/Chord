import type { HarnessEvent, ModelOption, UsageWindow } from "../../ipc/bindings";
import { chatFed, type Chat, type Replay } from "./transcript.ts";

type Effect = { kind: "draft"; text: string } | { kind: "usage"; harness: string; windows: UsageWindow[] } | { kind: "models"; harness: string; models: ModelOption[] } | { kind: "notify"; text: string } | { kind: "gitCounts" };

function effectOf(event: HarnessEvent): Effect | null {
  switch (event.kind) {
    case "draft":
      return { kind: "draft", text: event.text };
    case "usage":
      return { kind: "usage", harness: event.harness, windows: event.windows };
    case "models":
      return { kind: "models", harness: event.harness, models: event.models };
    case "exported":
      return { kind: "notify", text: `Exported to ${event.path}` };
    case "tool_end":
      return { kind: "gitCounts" };
    case "status":
      return event.changed && event.status.kind !== "running" ? { kind: "gitCounts" } : null;
    default:
      return null;
  }
}

export function intakes() {
  const current = new Map<string, { start: number; replay: Replay; loaded: () => void }>();
  let count = 0;
  return {
    begin(pane: string, loaded: () => void): number {
      current.set(pane, { start: ++count, replay: null, loaded });
      return count;
    },
    latest: (pane: string, start: number) => current.get(pane)?.start === start,
    has: (pane: string) => current.has(pane),
    end(pane: string) {
      current.get(pane)?.loaded();
      current.delete(pane);
    },
    fed(pane: string, start: number, chat: Chat, event: HarnessEvent, now: number): { chat: Chat; effect: Effect | null } | null {
      const intake = current.get(pane);
      if (intake?.start !== start) return null;
      if (event.kind === "history_end" || event.kind === "exited") intake.loaded();
      const fed = chatFed(chat, intake.replay, event, now);
      intake.replay = fed.replay;
      return { chat: fed.chat, effect: effectOf(event) };
    },
  };
}

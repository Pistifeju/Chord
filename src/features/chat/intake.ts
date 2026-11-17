// Each pane's agent start: drops the events of an older start, holds a replayed history, ends the wait for it, and names what an event does beyond the transcript.
import type { HarnessEvent, UsageWindow } from "../../ipc/bindings";
import { chatFed, type Chat, type Replay } from "./transcript.ts";

/** What an event does outside the transcript, for the store to carry out. */
type Effect = { kind: "draft"; text: string } | { kind: "usage"; windows: UsageWindow[] } | { kind: "notify"; text: string };

function effectOf(event: HarnessEvent): Effect | null {
  switch (event.kind) {
    case "draft":
      return { kind: "draft", text: event.text };
    case "usage":
      return { kind: "usage", windows: event.windows };
    case "exported":
      return { kind: "notify", text: `Exported to ${event.path}` };
    default:
      return null;
  }
}

export function intakes() {
  const current = new Map<string, { start: number; replay: Replay; loaded: () => void }>();
  let count = 0;
  return {
    /** Starts over for the pane; `loaded` runs once its history is in, the agent exits, or the pane ends. */
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
    /** The chat after the event and what the event does beyond it; null for an event of an older start. */
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

// Rules for the context meter: how full the model's context window is, and how alarming that fill is.
import type { Chat } from "./transcript.ts";

/** What the composer bar shows of the context window. */
type ContextMeter = { percent: number | null; level: FillLevel; title: string };

/** How alarming a meter's fill is: it picks the fill's color. */
export type FillLevel = "calm" | "warning" | "critical";

/** The ring warns from half the window and turns critical from three quarters. */
const contextLevel = (percent: number): FillLevel => (percent >= 75 ? "critical" : percent >= 50 ? "warning" : "calm");

const tokenLabel = (tokens: number): string => (tokens >= 1000 ? `${Math.round(tokens / 1000)}K` : String(tokens));

/** Null when the harness lacks the context capability or has not told the window yet, unless the session is empty; a missing token count (right after a compaction) shows as unknown. */
export function contextMeter({ capabilities, context }: Chat): ContextMeter | null {
  if (!capabilities?.includes("context") || context === null) return null;
  // An empty session fills none of any window, so its meter needs none.
  if (context.window === null) return context.tokens === 0 ? { percent: 0, level: "calm", title: "Context: empty" } : null;
  if (context.tokens === null) return { percent: null, level: "calm", title: "Context: unknown until the next reply" };
  const percent = Math.min(100, Math.round((context.tokens / context.window) * 100));
  const level = contextLevel(percent);
  return { percent, level, title: `Context: ${tokenLabel(context.tokens)} of ${tokenLabel(context.window)} tokens${level === "critical" ? ", nearly full" : ""}` };
}

/** The folded line of a compaction in the transcript. */
export const compactionLabel = (tokensBefore: number | null): string => (tokensBefore === null ? "Compacted the context" : `Compacted ${tokenLabel(tokensBefore)} tokens`);

// Rules for the context meter: how full the model's context window is, and when that fill is a warning.
import type { Chat } from "./transcript.ts";

/** What the composer bar shows of the context window. */
export type ContextMeter = { percent: number | null; warning: boolean; title: string };

/** The fill, in percent, from which the meter warns. */
const WARNING_PERCENT = 80;

const tokenLabel = (tokens: number): string => (tokens >= 1000 ? `${Math.round(tokens / 1000)}K` : String(tokens));

/** Null when the harness lacks the context capability or has not told the window yet; a missing token count (right after a compaction) shows as unknown. */
export function contextMeter({ capabilities, context }: Chat): ContextMeter | null {
  if (!capabilities?.includes("context") || context === null) return null;
  if (context.tokens === null) return { percent: null, warning: false, title: "Context: unknown until the next reply" };
  const percent = Math.min(100, Math.round((context.tokens / context.window) * 100));
  const warning = percent >= WARNING_PERCENT;
  return { percent, warning, title: `Context: ${tokenLabel(context.tokens)} of ${tokenLabel(context.window)} tokens${warning ? ", nearly full" : ""}` };
}

import type { Chat } from "./transcript.ts";

type ContextMeter = { percent: number | null; estimated: boolean; level: FillLevel; title: string };

export type FillLevel = "calm" | "warning" | "critical";

const contextLevel = (percent: number): FillLevel => (percent >= 75 ? "critical" : percent >= 50 ? "warning" : "calm");

const tokenLabel = (tokens: number): string => (tokens >= 1000 ? `${Math.round(tokens / 1000)}K` : String(tokens));

export function contextMeter({ capabilities, context }: Chat): ContextMeter | null {
  if (!capabilities?.includes("context") || context === null) return null;
  if (context.window === null) return context.tokens === 0 ? { percent: 0, estimated: false, level: "calm", title: "Context: empty" } : null;
  if (context.tokens === null) return { percent: null, estimated: false, level: "calm", title: "Context: unknown until the next reply" };
  const percent = Math.min(100, Math.round((context.tokens / context.window) * 100));
  const level = contextLevel(percent);
  const about = context.estimated ? "about " : "";
  const until = context.estimated ? ", estimated until the next reply" : "";
  return { percent, estimated: context.estimated, level, title: `Context: ${about}${tokenLabel(context.tokens)} of ${tokenLabel(context.window)} tokens${level === "critical" ? ", nearly full" : ""}${until}` };
}

export const compactionLabel = (tokensBefore: number | null): string => (tokensBefore === null ? "Compacted the context" : `Compacted ${tokenLabel(tokensBefore)} tokens`);

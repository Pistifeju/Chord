// Rules for the composer's model, thinking and provider labels: what each shows and whether it shows.
import type { Chat } from "./transcript.ts";

/** The labels of the composer bar; the thinking one is null for a model that does not reason. Null when the harness lacks the models capability or has not told the model yet. */
export function modelLabels({ capabilities, model }: Chat): { name: string; thinking: string | null } | null {
  if (!capabilities?.includes("models") || model === null) return null;
  return { name: model.name.replace(/^(claude|codex)\s+/i, ""), thinking: model.thinking ? levelLabel(model.thinking) : null };
}

/** A thinking level as the harness names it ("xhigh"), capitalized for the composer and the agent picker. */
export const levelLabel = (level: string): string => level[0].toUpperCase() + level.slice(1);

const brandSpellings: Record<string, string> = { openai: "OpenAI", github: "GitHub" };

/** A provider id ("openai-codex") as its brand writes it ("OpenAI Codex"), for the agent picker. */
export const providerLabel = (provider: string): string =>
  provider
    .split("-")
    .map((word) => brandSpellings[word] ?? levelLabel(word))
    .join(" ");

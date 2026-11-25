import type { Chat } from "./transcript.ts";

export function modelLabels({ capabilities, model }: Chat): { name: string; thinking: string | null } | null {
  if (!capabilities?.includes("models") || model === null) return null;
  return { name: model.name.replace(/^(claude|codex)\s+/i, ""), thinking: model.thinking ? levelLabel(model.thinking) : null };
}

export const levelLabel = (level: string): string => level[0].toUpperCase() + level.slice(1);

const brandSpellings: Record<string, string> = { openai: "OpenAI", github: "GitHub" };

export const providerLabel = (provider: string): string =>
  provider
    .split("-")
    .map((word) => brandSpellings[word] ?? levelLabel(word))
    .join(" ");

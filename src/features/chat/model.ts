// Rules for the composer's model and thinking labels: what each shows and whether it shows.
import type { Chat, ModelChoice } from "./transcript.ts";

/** The labels of the composer bar; the thinking one is null for a model that does not reason. Null when the harness lacks the models capability or has not told the model yet. */
export function modelLabels({ capabilities, model }: Chat): ModelChoice | null {
  if (!capabilities?.includes("models") || model === null) return null;
  const level = model.thinking;
  return { name: model.name.replace(/^(claude|codex)\s+/i, ""), thinking: level ? level[0].toUpperCase() + level.slice(1) : null };
}

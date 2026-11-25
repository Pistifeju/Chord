import type { ModelOption } from "../../ipc/bindings";
import type { Chat } from "./transcript.ts";

export type Opening = Pick<Chat, "capabilities" | "model" | "permissions" | "context" | "commands">;

const noOpening: Opening = { capabilities: null, model: null, permissions: null, context: null, commands: [] };

export const openingOf = ({ agent, loading, items, capabilities, model, permissions, context, commands }: Chat): Opening | null =>
  agent === "ready" && !loading && items.length === 0 ? { capabilities, model, permissions, context, commands } : null;

export function openingWith(last: Opening | undefined, picked: ModelOption | undefined, level: string | null): Opening {
  if (!last) return noOpening;
  return picked ? { ...last, model: { id: picked.id, name: picked.name, thinking: level } } : last;
}

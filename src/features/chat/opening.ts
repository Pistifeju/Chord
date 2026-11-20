// A new chat's opening: what its harness's last new session showed in the composer bar, shown again before the agent answers.
import type { ModelOption } from "../../ipc/bindings";
import type { Chat } from "./transcript.ts";

export type Opening = Pick<Chat, "capabilities" | "model" | "permissions" | "context" | "commands">;

const noOpening: Opening = { capabilities: null, model: null, permissions: null, context: null, commands: [] };

/** What the chat's session opened with; null once it holds a message, and while the agent takes none yet. */
export const openingOf = ({ agent, loading, items, capabilities, model, permissions, context, commands }: Chat): Opening | null =>
  agent === "ready" && !loading && items.length === 0 ? { capabilities, model, permissions, context, commands } : null;

/** What a new chat shows: the harness's `last` opening, at the model and thinking level the user `picked` for this chat; nothing without a last one. */
export function openingWith(last: Opening | undefined, picked: ModelOption | undefined, level: string | null): Opening {
  if (!last) return noOpening;
  return picked ? { ...last, model: { id: picked.id, name: picked.name, thinking: level } } : last;
}

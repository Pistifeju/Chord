import type { SlashCommand } from "../../ipc/bindings";
import { steppedIndex } from "../../ui/listStep.ts";
import { imageMime } from "./attachments.ts";
import type { Chat } from "./transcript.ts";

export function composerState(chat: Chat) {
  const ready = chat.agent === "ready";
  const steers = chat.running && chat.capabilities?.includes("steer") === true;
  const menu = chat.dialog?.ask.kind === "menu" ? { dialog: chat.dialog, ask: chat.dialog.ask } : null;
  return {
    ready,
    steers,
    canSend: ready && (chat.loading ? chat.held === null : !chat.running || steers),
    menu,
    shown: chat.agent !== "idle" && (!chat.dialog || menu !== null),
  };
}

export function heldMessage({ held, agent, loading }: Chat): { kind: "send" | "restore"; text: string } | null {
  if (held === null) return null;
  if (agent === "idle") return { kind: "restore", text: held };
  return agent === "ready" && !loading ? { kind: "send", text: held } : null;
}

export const sendable = (canSend: boolean, draft: string, attachments: number): boolean => canSend && (draft.trim() !== "" || attachments > 0);

type ComposerKey = { key: string; shift: boolean; composing: boolean };
type KeyAction = { kind: "step"; to: number } | { kind: "complete"; command: SlashCommand } | { kind: "send" } | null;

export function composerKey({ key, shift, composing }: ComposerKey, draft: string, rows: SlashCommand[], at: number): KeyAction {
  if (rows.length > 0 && !composing) {
    if (key === "ArrowDown" || key === "ArrowUp") return { kind: "step", to: steppedIndex(at, key === "ArrowDown" ? 1 : -1, rows.length) };
    const picked = rows[at];
    if (key === "Tab" || (key === "Enter" && !shift && draft !== `/${picked.name}`)) return { kind: "complete", command: picked };
  }
  return key === "Enter" && !shift && !composing ? { kind: "send" } : null;
}

export function droppedFiles(paths: string[], canAttach: boolean): { attach: string[]; draft: string[] } {
  const isAttached = (path: string) => canAttach && imageMime(path) !== null;
  return {
    attach: paths.filter(isAttached),
    draft: paths.filter((path) => !isAttached(path)).map((path) => (/\s/.test(path) ? `"${path}"` : path)),
  };
}

export const pickerClick = (openTitle: string | null, title: string) => ({ open: openTitle !== title });

export const composerMenu = (picker: boolean, agent: boolean, slash: boolean): "picker" | "agent" | "slash" | null => (picker ? "picker" : agent ? "agent" : slash ? "slash" : null);

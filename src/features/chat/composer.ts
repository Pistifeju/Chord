// Rules for the composer: whether it shows and steers, whether a send goes through, what a key does in it, where dropped files go, what a picker button does and which menu floats above it.
import type { SlashCommand } from "../../ipc/bindings";
import { steppedIndex } from "../../ui/listStep.ts";
import { imageMime } from "./attachments.ts";
import type { Chat } from "./transcript.ts";

/** What the composer is for the chat: shown under a dialog only for its menu, and a steer while the agent runs and takes steers. */
export function composerState(chat: Chat) {
  const ready = chat.agent === "ready";
  const steers = chat.running && chat.capabilities?.includes("steer") === true;
  const menu = chat.dialog?.ask.kind === "menu" ? { dialog: chat.dialog, ask: chat.dialog.ask } : null;
  return {
    ready,
    steers,
    canSend: ready && (!chat.running || steers),
    menu,
    shown: chat.agent !== "idle" && (!chat.dialog || menu !== null),
  };
}

/** Whether a send goes through: the composer takes sends and holds text or an attachment. */
export const sendable = (canSend: boolean, draft: string, attachments: number): boolean => canSend && (draft.trim() !== "" || attachments > 0);

type ComposerKey = { key: string; shift: boolean; composing: boolean };
type KeyAction = { kind: "step"; to: number } | { kind: "complete"; command: SlashCommand } | { kind: "send" } | null;

/** What a key does with the slash menu's rows open at `at`: arrows step the rows, Tab completes, Return completes unless the draft holds the command in full, then Return sends. */
export function composerKey({ key, shift, composing }: ComposerKey, draft: string, rows: SlashCommand[], at: number): KeyAction {
  if (rows.length > 0 && !composing) {
    if (key === "ArrowDown" || key === "ArrowUp") return { kind: "step", to: steppedIndex(at, key === "ArrowDown" ? 1 : -1, rows.length) };
    const picked = rows[at];
    if (key === "Tab" || (key === "Enter" && !shift && draft !== `/${picked.name}`)) return { kind: "complete", command: picked };
  }
  return key === "Enter" && !shift && !composing ? { kind: "send" } : null;
}

/** Where dropped files go: the images into the attachments while the chat takes them, any other path into the draft, quoted when it holds whitespace. */
export function droppedFiles(paths: string[], canAttach: boolean): { attach: string[]; draft: string[] } {
  const isAttached = (path: string) => canAttach && imageMime(path) !== null;
  return {
    attach: paths.filter(isAttached),
    draft: paths.filter((path) => !isAttached(path)).map((path) => (/\s/.test(path) ? `"${path}"` : path)),
  };
}

/** What a picker button does once the open menus close: the open picker's button leaves it closed, the other picker's button opens its own. A picker's dialog title is its button's title. */
export const pickerClick = (openTitle: string | null, title: string) => ({ open: openTitle !== title });

/** The one menu floating above the composer: an open picker first, then the branch menu, then the slash menu. */
export const composerMenu = (picker: boolean, branch: boolean, slash: boolean): "picker" | "branch" | "slash" | null =>
  picker ? "picker" : branch ? "branch" : slash ? "slash" : null;

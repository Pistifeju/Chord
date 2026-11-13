// The one sheet the window shows, if any: opening a sheet replaces the one that was open.
import { externalStore } from "../ui/externalStore";

export type Sheet =
  | { kind: "newWorktree" }
  | { kind: "deleteWorktree"; project: string; path: string; name: string }
  | { kind: "forceRemoval"; project: string; path: string; branch: string | null; reason: string }
  | { kind: "trashFile"; worktree: string; path: string }
  | { kind: "quit" }
  | { kind: "closeAgents"; worktree: string; target: "pane" | "session"; id: string };

export const sheet = externalStore<Sheet | null>(null);

export const openSheet = (next: Sheet) => sheet.set(next);

// With a kind, closes the sheet only while that kind is open, so a late close cannot take down the sheet that replaced it.
export function closeSheet(kind?: Sheet["kind"]) {
  if (!kind || sheet.get()?.kind === kind) sheet.set(null);
}

// The open sheet when it has this kind, typed to that kind.
export const sheetOf = <K extends Sheet["kind"]>(current: Sheet | null, kind: K) => (current?.kind === kind ? (current as Extract<Sheet, { kind: K }>) : null);

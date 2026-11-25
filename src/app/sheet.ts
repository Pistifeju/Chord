import type { Change, Hunk, Stash } from "../ipc/bindings";
import { externalStore } from "../ui/externalStore";

export type Sheet =
  | { kind: "newWorktree" }
  | { kind: "deleteWorktree"; project: string; path: string; name: string }
  | { kind: "forceRemoval"; project: string; path: string; branch: string | null; reason: string }
  | { kind: "renameBranch" }
  | { kind: "newBranch" }
  | { kind: "deleteBranch"; worktree: string; branch: string; unmerged: boolean }
  | { kind: "trashFile"; worktree: string; path: string }
  | { kind: "discardChanges"; worktree: string; changes: Change[]; hunk?: Hunk }
  | { kind: "dropStash"; worktree: string; stash: Stash }
  | { kind: "openFileByName"; worktree: string }
  | { kind: "quit" }
  | { kind: "update" }
  | { kind: "help" }
  | { kind: "closeAgents"; worktree: string; id: string }
  | { kind: "removeProject"; path: string; name: string };

export const sheet = externalStore<Sheet | null>(null);

export const openSheet = (next: Sheet) => sheet.set(next);

export function closeSheet(kind?: Sheet["kind"]) {
  if (!kind || sheet.get()?.kind === kind) sheet.set(null);
}

export const sheetOf = <K extends Sheet["kind"]>(current: Sheet | null, kind: K) => (current?.kind === kind ? (current as Extract<Sheet, { kind: K }>) : null);

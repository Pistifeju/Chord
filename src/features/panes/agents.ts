import type { WorktreePanes } from "../../ipc/bindings";
import { chatPaneIds } from "./layout.ts";

export const allPaneIds = (saved: WorktreePanes | undefined): string[] => (saved ? chatPaneIds(saved.layout) : []);

export function paneChanges(before: WorktreePanes | undefined, after: WorktreePanes | undefined): { started: string[]; gone: string[] } {
  const was = allPaneIds(before);
  const now = allPaneIds(after);
  return { started: now.filter((pane) => !was.includes(pane)), gone: was.filter((pane) => !now.includes(pane)) };
}

export function resumedPanes(saved: Record<string, WorktreePanes>, running: string[]): [worktree: string, pane: string][] {
  return Object.entries(saved).flatMap(([worktree, panes]) => allPaneIds(panes).filter((pane) => running.includes(pane)).map((pane): [string, string] => [worktree, pane]));
}

export const vanishedWorktrees = (saved: Record<string, WorktreePanes>, existing: string[]): string[] => Object.keys(saved).filter((worktree) => !existing.includes(worktree));

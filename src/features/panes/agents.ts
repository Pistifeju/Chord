// Rules for which panes' agents start and stop: a pane that appears or goes, and the agents that come back at launch.
import type { WorktreePanes } from "../../ipc/bindings";
import { chatPaneIds } from "./layout.ts";

// Only chat panes run an agent; a file pane has none.
export const allPaneIds = (saved: WorktreePanes | undefined): string[] => (saved?.sessions ?? []).flatMap((session) => chatPaneIds(session.layout));

export function paneChanges(before: WorktreePanes | undefined, after: WorktreePanes | undefined): { started: string[]; gone: string[] } {
  const was = allPaneIds(before);
  const now = allPaneIds(after);
  return { started: now.filter((pane) => !was.includes(pane)), gone: was.filter((pane) => !now.includes(pane)) };
}

// Agents that ran at quit start again, each on its pane's saved session; a worktree found gone later forgets its panes, which stops them.
export function resumedPanes(saved: Record<string, WorktreePanes>, running: string[]): [worktree: string, pane: string][] {
  return Object.entries(saved).flatMap(([worktree, panes]) => allPaneIds(panes).filter((pane) => running.includes(pane)).map((pane): [string, string] => [worktree, pane]));
}

export const vanishedWorktrees = (saved: Record<string, WorktreePanes>, existing: string[]): string[] => Object.keys(saved).filter((worktree) => !existing.includes(worktree));

// Where a file shows: its tab, the pane that already shows it, or a pane or new session it leaves its tab for. The Files tree selects the file that shows.
import { closeSheet } from "../../app/sheet";
import type { Direction } from "../../ipc/bindings";
import { selectFile } from "../files/store";
import type { Edge } from "../panes/layout";
import { dropFilePane, filePaneOf, focusPane, splitFileSession } from "../panes/store";
import { closeTab, openTab, showSessions } from "./store";

export function openFile(worktree: string, path: string) {
  openTab(worktree, path);
  selectFile(worktree, path);
}

export function closeFile(worktree: string, path: string) {
  const active = closeTab(worktree, path);
  if (active) selectFile(worktree, active);
}

// A file a pane already shows focuses that pane; any other opens in a tab.
export function showFile(worktree: string, path: string) {
  const pane = filePaneOf(worktree, path);
  if (!pane) return openFile(worktree, path);
  selectFile(worktree, path);
  focusPane(worktree, pane.id);
  showSessions(worktree);
}

export function pickFileByName(worktree: string, path: string) {
  closeSheet("openFileByName");
  showFile(worktree, path);
}

// The file leaves its tab for a pane beside `pane`.
export function dropFile(worktree: string, path: string, pane: string, edge: Edge) {
  dropFilePane(worktree, path, pane, edge);
  closeFile(worktree, path);
  showSessions(worktree);
}

// The file leaves its tab for a new session tab, with a new agent beside it.
export function splitFile(worktree: string, path: string, direction: Direction) {
  splitFileSession(worktree, path, direction);
  // Hidden first, so closing the tab does not select a neighbour that is not shown.
  showSessions(worktree);
  closeFile(worktree, path);
}

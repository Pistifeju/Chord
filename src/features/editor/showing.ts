// Where a file shows: the tab that already shows it, else a new tab in the focused group, scrolled to a line when asked. The Files tree selects the file that shows.
import { closeSheet } from "../../app/sheet";
import { selectFile } from "../files/store";
import { openFileTab } from "../panes/store";
import { requestLine } from "./reveal";

export function showFile(worktree: string, path: string) {
  openFileTab(worktree, path);
  selectFile(worktree, path);
}

/** A compile error's file, at its line when it has one. */
export function showFileAt(worktree: string, path: string, line: number | null | undefined) {
  if (line) requestLine(worktree, path, line);
  showFile(worktree, path);
}

export function pickFileByName(worktree: string, path: string) {
  closeSheet("openFileByName");
  showFile(worktree, path);
}

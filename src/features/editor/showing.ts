// Where a file shows: the tab that already shows it, else a new tab in the focused group. The Files tree selects the file that shows.
import { closeSheet } from "../../app/sheet";
import { selectFile } from "../files/store";
import { openFileTab } from "../panes/store";

export function showFile(worktree: string, path: string) {
  openFileTab(worktree, path);
  selectFile(worktree, path);
}

export function pickFileByName(worktree: string, path: string) {
  closeSheet("openFileByName");
  showFile(worktree, path);
}

import { closeSheet } from "../../app/sheet";
import { noteOpened, selectFile } from "../files/store";
import { openFileTab } from "../panes/store";
import { requestLine } from "./reveal";

export function showFile(worktree: string, path: string, preview = true) {
  openFileTab(worktree, path, preview);
  selectFile(worktree, path);
  noteOpened(worktree, path);
}

export function showFileAt(worktree: string, path: string, line: number | null | undefined) {
  if (line) requestLine(worktree, path, line);
  showFile(worktree, path);
}

export function pickFileByName(worktree: string, path: string) {
  closeSheet("openFileByName");
  showFile(worktree, path);
}

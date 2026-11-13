// Connects the file tab rules to IPC: each worktree's open files, a file read when its tab shows, saved 1 s after the last edit and when the tab goes, unless the file changed on disk.
import type { Text } from "@codemirror/state";
import { commands } from "../../ipc/bindings";
import { externalStore } from "../../ui/externalStore";
import { selectFile } from "../files/store";
import { fail } from "../worktrees/store";
import { noFiles, withFileClosed, withFileOpened, withSessionsShown, type OpenFiles } from "./openFiles";

export const openFiles = externalStore<Record<string, OpenFiles>>({});

/** Milliseconds after the last edit before the file saves. */
const SAVE_DELAY = 1000;

const filesOf = (worktree: string): OpenFiles => openFiles.get()[worktree] ?? noFiles;
const update = (worktree: string, change: (open: OpenFiles) => OpenFiles) => openFiles.set({ ...openFiles.get(), [worktree]: change(filesOf(worktree)) });

export function openFile(worktree: string, path: string) {
  update(worktree, (open) => withFileOpened(open, path));
  selectFile(worktree, path);
}

export function closeFile(worktree: string, path: string) {
  update(worktree, (open) => withFileClosed(open, path));
  const { active } = filesOf(worktree);
  if (active) selectFile(worktree, active);
}

export const showSessions = (worktree: string) => {
  if (filesOf(worktree).active !== null) update(worktree, withSessionsShown);
};

// The document is kept as is and turned into a string only when it saves, not on every keystroke.
type Edit = { worktree: string; path: string; text: Text; timer: number };

const edits = new Map<string, Edit>();
// The last write of each file, so a read or the next write waits for it.
const writes = new Map<string, Promise<unknown>>();
// The text each file held on disk when the editor last read or wrote it; a save only lands while the disk still holds it.
const onDisk = new Map<string, string>();
const keyOf = (worktree: string, path: string) => `${worktree}\0${path}`;

export function scheduleSave(worktree: string, path: string, text: Text) {
  const key = keyOf(worktree, path);
  clearTimeout(edits.get(key)?.timer);
  edits.set(key, { worktree, path, text, timer: window.setTimeout(() => flushSave(worktree, path), SAVE_DELAY) });
}

export function flushSave(worktree: string, path: string) {
  const key = keyOf(worktree, path);
  const edit = edits.get(key);
  if (!edit) return;
  clearTimeout(edit.timer);
  edits.delete(key);
  const text = edit.text.toString();
  const write = (writes.get(key) ?? Promise.resolve())
    .then(() => commands.writeFile(worktree, path, onDisk.get(key) ?? "", text))
    .then(() => onDisk.set(key, text))
    .catch(fail);
  writes.set(key, write);
}

/** Saves every pending edit and resolves once all writes landed, for the quit. */
export async function flushAllSaves() {
  for (const { worktree, path } of [...edits.values()]) flushSave(worktree, path);
  await Promise.all(writes.values());
}

export async function readFile(worktree: string, path: string): Promise<string> {
  const key = keyOf(worktree, path);
  await writes.get(key);
  const text = await commands.readFile(worktree, path);
  onDisk.set(key, text);
  return text;
}

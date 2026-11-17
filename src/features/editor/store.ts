// Connects the editor to IPC: a file read when its tab shows and again when it changes on disk, saved 1 s after the last edit and when the tab goes, unless the file changed on disk; its text let go once its editor closed.
import type { Text } from "@codemirror/state";
import { commands } from "../../ipc/bindings";
import { fail } from "../../ui/alert";

/** Milliseconds after the last edit before the file saves. */
const SAVE_DELAY = 1000;

// The document is kept as is and turned into a string only when it saves, not on every keystroke.
type Edit = { worktree: string; path: string; text: Text; lineBreak: string; timer: number };

const edits = new Map<string, Edit>();
// The last write of each file, so a read or the next write waits for it.
const writes = new Map<string, Promise<unknown>>();
// The text each file held on disk when the editor last read or wrote it; a save only lands while the disk still holds it.
const onDisk = new Map<string, string>();
const keyOf = (worktree: string, path: string) => `${worktree}\0${path}`;

export function scheduleSave(worktree: string, path: string, text: Text, lineBreak: string) {
  const key = keyOf(worktree, path);
  clearTimeout(edits.get(key)?.timer);
  edits.set(key, { worktree, path, text, lineBreak, timer: window.setTimeout(() => flushSave(worktree, path), SAVE_DELAY) });
}

function flushSave(worktree: string, path: string) {
  const key = keyOf(worktree, path);
  const edit = edits.get(key);
  if (!edit) return;
  clearTimeout(edit.timer);
  edits.delete(key);
  const text = edit.text.sliceString(0, edit.text.length, edit.lineBreak);
  const write = (writes.get(key) ?? Promise.resolve())
    .then(() => commands.writeFile(worktree, path, onDisk.get(key) ?? "", text))
    .then(() => onDisk.set(key, text))
    .catch(fail);
  writes.set(key, write);
}

/** The editor of the file went: its pending edit saves, then the store lets go of the file's text unless a newer write started. */
export function closeFile(worktree: string, path: string) {
  flushSave(worktree, path);
  const key = keyOf(worktree, path);
  const last = writes.get(key);
  if (!last) return void onDisk.delete(key);
  void last.then(() => {
    if (writes.get(key) !== last) return;
    writes.delete(key);
    onDisk.delete(key);
  });
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

/** The file's text when it changed on disk since the editor last read or wrote it and holds no unsaved edit; null otherwise. */
export async function changedOnDisk(worktree: string, path: string): Promise<string | null> {
  const key = keyOf(worktree, path);
  const write = writes.get(key);
  if (edits.has(key)) return null;
  await write;
  const text = await commands.readFile(worktree, path);
  if (edits.has(key) || writes.get(key) !== write || text === onDisk.get(key)) return null;
  onDisk.set(key, text);
  return text;
}

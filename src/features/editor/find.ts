// Find in the editor: which editors have the find bar open, what they search for, and each open bar's own close.
import { findStore } from "../../ui/find";

export const { finds: editorFinds, openFind: openEditorFind, setFindQuery: setEditorFindQuery, closeFind: closeEditorFind } = findStore();

// Esc runs the open bar's close, which puts the caret on the current match; the command table cannot reach the editor.
const closers = new Map<string, () => void>();

export function onEditorFindEnd(key: string, close: () => void) {
  closers.set(key, close);
  return () => void closers.delete(key);
}

export const endEditorFind = (key: string) => (closers.get(key) ?? (() => closeEditorFind(key)))();

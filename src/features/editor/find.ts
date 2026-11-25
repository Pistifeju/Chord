import { findStore } from "../../ui/find";

export const { finds: editorFinds, openFind: openEditorFind, setFindQuery: setEditorFindQuery, closeFind: closeEditorFind } = findStore();

const closers = new Map<string, () => void>();

export function onEditorFindEnd(key: string, close: () => void) {
  closers.set(key, close);
  return () => void closers.delete(key);
}

export const endEditorFind = (key: string) => (closers.get(key) ?? (() => closeEditorFind(key)))();

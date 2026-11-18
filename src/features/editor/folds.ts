// The folded blocks an editor keeps when an agent rewrites its file: each one moved past the agent's change, gone when the change touches it.
import { foldedRanges, foldEffect } from "@codemirror/language";
import { ChangeSet, type EditorState, type StateEffect } from "@codemirror/state";

/** The change from `before` to `after` as one replaced span between their common start and common end. */
export function rewrite(before: string, after: string): ChangeSet {
  const shorter = Math.min(before.length, after.length);
  let start = 0;
  while (start < shorter && before[start] === after[start]) start++;
  let end = 0;
  while (end < shorter - start && before[before.length - 1 - end] === after[after.length - 1 - end]) end++;
  return ChangeSet.of({ from: start, to: before.length - end, insert: after.slice(start, after.length - end) }, before.length);
}

export function keptFolds(before: EditorState, after: EditorState): StateEffect<unknown>[] {
  const change = rewrite(before.doc.toString(), after.doc.toString());
  const effects: StateEffect<unknown>[] = [];
  foldedRanges(before).between(0, before.doc.length, (from, to) => {
    if (!change.touchesRange(from, to)) effects.push(foldEffect.of({ from: change.mapPos(from), to: change.mapPos(to) }));
  });
  return effects;
}

import { SearchQuery } from "@codemirror/search";
import { StateEffect, StateField, type EditorState } from "@codemirror/state";
import { Decoration, EditorView, type DecorationSet } from "@codemirror/view";

export type Match = { from: number; to: number };

export function matchesOf(state: EditorState, search: string): Match[] {
  if (!search) return [];
  const found: Match[] = [];
  const cursor = new SearchQuery({ search, literal: true }).getCursor(state);
  for (let next = cursor.next(); !next.done; next = cursor.next()) found.push({ from: next.value.from, to: next.value.to });
  return found;
}

const paint = StateEffect.define<DecorationSet>();
const mark = Decoration.mark({ class: "cm-find-match" });
const currentMark = Decoration.mark({ class: "cm-find-match cm-find-current" });

export const findMarks = StateField.define<DecorationSet>({
  create: () => Decoration.none,
  update: (marks, tr) => tr.effects.reduce((painted, effect) => (effect.is(paint) ? effect.value : painted), marks.map(tr.changes)),
  provide: (field) => EditorView.decorations.from(field),
});

export const paintMatches = (view: EditorView, matches: Match[], current: number) =>
  view.dispatch({ effects: paint.of(Decoration.set(matches.map((m, i) => (i === current ? currentMark : mark).range(m.from, m.to)))) });

export const revealMatch = (view: EditorView, match: Match | undefined) =>
  match && view.dispatch({ effects: EditorView.scrollIntoView(match.from, { y: "center" }) });

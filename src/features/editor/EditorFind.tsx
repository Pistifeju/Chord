import type { EditorView } from "@codemirror/view";
import { useEffect, useRef, useState } from "react";
import { tooltip } from "../../app/commands";
import { FindBar } from "../../ui/FindBar";
import type { Find } from "../../ui/find";
import { steppedIndex } from "../../ui/listStep";
import { closeEditorFind, onEditorFindEnd, setEditorFindQuery } from "./find";
import { matchesOf, paintMatches, revealMatch, type Match } from "./findMarks";

export function EditorFind({ id, view, find, version }: { id: string; view: EditorView; find: Find; version: number }) {
  const [matches, setMatches] = useState<Match[]>([]);
  const [current, setCurrent] = useState(0);
  const searched = useRef<string | null>(null);

  useEffect(() => {
    const found = matchesOf(view.state, find.query);
    setMatches(found);
    if (searched.current === find.query) {
      setCurrent((at) => Math.min(at, Math.max(0, found.length - 1)));
      return;
    }
    const head = view.state.selection.main.head;
    const first = Math.max(0, found.findIndex((m) => m.from >= head));
    searched.current = find.query;
    setCurrent(first);
    revealMatch(view, found[first]);
  }, [view, find.query, version]);

  useEffect(() => {
    paintMatches(view, matches, current);
  }, [view, matches, current]);

  useEffect(() => () => void (view.dom.isConnected && paintMatches(view, [], 0)), [view]);

  const close = (toEditor: boolean) => {
    closeEditorFind(id);
    if (!toEditor) return;
    const match = matches[current];
    if (match) view.dispatch({ selection: { anchor: match.from, head: match.to } });
    view.focus();
  };

  useEffect(() => onEditorFindEnd(id, () => close(true)));

  const step = (by: 1 | -1) => {
    const next = steppedIndex(current, by, matches.length);
    setCurrent(next);
    revealMatch(view, matches[next]);
  };

  return (
    <FindBar
      query={find.query}
      focus={find.focus}
      current={current}
      count={matches.length}
      label="Find in File"
      closeTitle={tooltip("editor.findClose")}
      onQuery={(query) => setEditorFindQuery(id, query)}
      onStep={step}
      onClose={(byButton) => close(byButton === true)}
    />
  );
}

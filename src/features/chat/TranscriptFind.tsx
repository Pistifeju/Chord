// The find bar over a pane's transcript. Matches are painted with the CSS highlight API.
import { useEffect, useRef, useState, type RefObject } from "react";
import { tooltip } from "../../app/commands";
import { FindBar } from "../../ui/FindBar";
import { steppedIndex } from "../../ui/listStep";
import type { Find } from "../../ui/find";
import { closeFind, setFindQuery } from "./find";
import { clearFind, matchesIn, paintFind } from "./findMatches";
import type { Item } from "./transcript";

const reveal = (range: Range | undefined) => range?.startContainer.parentElement?.scrollIntoView({ block: "center" });

export function TranscriptFind({ pane, find, transcript, items }: { pane: string; find: Find; transcript: RefObject<HTMLDivElement | null>; items: Item[] }) {
  const [matches, setMatches] = useState<Range[]>([]);
  const count = matches.length;
  const revealed = useRef<string | null>(null);
  const [current, setCurrent] = useState(0);

  // The shown text changes with every item and every query, so the ranges are found again.
  useEffect(() => {
    const root = transcript.current?.querySelector(".chat-messages");
    const found = root ? matchesIn(root, find.query) : [];
    setMatches(found);
    if (revealed.current !== find.query) reveal(found[0]);
    revealed.current = find.query;
    setCurrent((at) => Math.min(at, Math.max(0, found.length - 1)));
  }, [find.query, items, transcript]);

  useEffect(() => {
    setCurrent(0);
  }, [find.query]);

  useEffect(() => {
    paintFind(pane, matches, matches[current]);
  }, [pane, matches, current]);

  useEffect(() => () => clearFind(pane), [pane]);

  const step = (by: 1 | -1) => {
    const next = steppedIndex(current, by, count);
    setCurrent(next);
    reveal(matches[next]);
  };

  return (
    <FindBar
      query={find.query}
      focus={find.focus}
      current={current}
      count={count}
      label="Find in Chat"
      closeTitle={tooltip("chat.findClose")}
      onQuery={(query) => setFindQuery(pane, query)}
      onStep={step}
      onClose={() => closeFind(pane)}
    />
  );
}

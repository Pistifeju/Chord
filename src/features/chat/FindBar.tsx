// The find bar over a pane's transcript: a query, the match count, previous and next, close. Matches are painted with the CSS highlight API.
import { useEffect, useRef, useState, type KeyboardEvent, type RefObject } from "react";
import { tooltip } from "../../app/commands";
import { ArrowDownIcon, ArrowUpIcon, CloseIcon } from "../../ui/icons";
import { useOutsidePointer } from "../../ui/outsidePointer";
import { closeFind, setFindQuery, type Find } from "./find";
import { clearFind, matchesIn, paintFind } from "./findMatches";
import type { Item } from "./transcript";

const reveal = (range: Range | undefined) => range?.startContainer.parentElement?.scrollIntoView({ block: "center" });

export function FindBar({ pane, find, transcript, items }: { pane: string; find: Find; transcript: RefObject<HTMLDivElement | null>; items: Item[] }) {
  const input = useRef<HTMLInputElement>(null);
  const bar = useRef<HTMLDivElement>(null);
  useOutsidePointer(bar, () => closeFind(pane));
  const [matches, setMatches] = useState<Range[]>([]);
  const count = matches.length;
  const revealed = useRef<string | null>(null);
  const [current, setCurrent] = useState(0);

  useEffect(() => {
    input.current?.focus();
    input.current?.select();
  }, [find.focus]);

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
    if (count === 0) return;
    const next = (current + by + count) % count;
    setCurrent(next);
    reveal(matches[next]);
  };

  const onKey = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter" && !e.nativeEvent.isComposing) {
      e.preventDefault();
      step(e.shiftKey ? -1 : 1);
    }
  };

  return (
    <div className="chat-find" ref={bar}>
      <input
        ref={input}
        className="chat-find-input"
        value={find.query}
        placeholder="Find"
        aria-label="Find in Chat"
        onChange={(e) => setFindQuery(pane, e.target.value)}
        onKeyDown={onKey}
      />
      <span className="chat-find-count">{find.query && (count > 0 ? `${current + 1} of ${count}` : "No results")}</span>
      <button type="button" className="icon-button" title="Previous Match (⇧↩)" aria-label="Previous Match" disabled={count === 0} onClick={() => step(-1)}>
        <ArrowUpIcon />
      </button>
      <button type="button" className="icon-button" title="Next Match (↩)" aria-label="Next Match" disabled={count === 0} onClick={() => step(1)}>
        <ArrowDownIcon />
      </button>
      <button type="button" className="icon-button" title={tooltip("chat.findClose")} aria-label="Close Find" onClick={() => closeFind(pane)}>
        <CloseIcon />
      </button>
    </div>
  );
}

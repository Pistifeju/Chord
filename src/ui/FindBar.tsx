import { useEffect, useRef, type KeyboardEvent } from "react";
import { ArrowDownIcon, ArrowUpIcon, CloseIcon } from "./icons";
import { useOutsidePointer } from "./outsidePointer";

type Props = {
  query: string;
  focus: number;
  current: number;
  count: number;
  label: string;
  closeTitle: string;
  onQuery: (query: string) => void;
  onStep: (by: 1 | -1) => void;
  onClose: (byButton?: boolean) => void;
};

export function FindBar({ query, focus, current, count, label, closeTitle, onQuery, onStep, onClose }: Props) {
  const input = useRef<HTMLInputElement>(null);
  const bar = useRef<HTMLDivElement>(null);
  useOutsidePointer(bar, onClose);

  useEffect(() => {
    input.current?.focus();
    input.current?.select();
  }, [focus]);

  const onKey = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter" && !e.nativeEvent.isComposing) {
      e.preventDefault();
      if (count > 0) onStep(e.shiftKey ? -1 : 1);
    }
  };

  return (
    <div className="find-bar" ref={bar}>
      <input ref={input} className="find-bar-input" value={query} placeholder="Find" aria-label={label} onChange={(e) => onQuery(e.target.value)} onKeyDown={onKey} />
      <span className="find-bar-count">{query && (count > 0 ? `${current + 1} of ${count}` : "No results")}</span>
      <button type="button" className="icon-button" title="Previous Match (⇧↩)" aria-label="Previous Match" disabled={count === 0} onClick={() => onStep(-1)}>
        <ArrowUpIcon />
      </button>
      <button type="button" className="icon-button" title="Next Match (↩)" aria-label="Next Match" disabled={count === 0} onClick={() => onStep(1)}>
        <ArrowDownIcon />
      </button>
      <button type="button" className="icon-button" title={closeTitle} aria-label="Close Find" onClick={() => onClose(true)}>
        <CloseIcon />
      </button>
    </div>
  );
}

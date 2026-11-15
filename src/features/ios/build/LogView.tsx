// A log in the bottom panel: lines in a scroll area that stays at the end while output arrives, only the lines in view mounted, and the find bar over it.
import { useEffect, useLayoutEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { tooltip } from "../../../app/commands";
import type { BottomViewSlot } from "../../bottompanel/BottomPanel";
import type { ConsoleTone } from "../../../ipc/bindings";
import { FindBar } from "../../../ui/FindBar";
import type { Find } from "../../../ui/find";
import { matchesIn, segmentsOf } from "./log";
import "./build.css";

/** A log line's height at the default text size, in pixels; it scales with the text size. */
const LINE_HEIGHT = 18;
/** Rows mounted beyond the ones in view, so a fast scroll shows no gap. */
const OVERSCAN = 20;
const FOLLOW_SLACK = 8;

const textScale = () => parseFloat(getComputedStyle(document.documentElement).getPropertyValue("--text-scale")) || 1;

type Viewport = { top: number; height: number; offset: number };

type Props = {
  /** Names the view for the command that finds in the one with the focus. */
  view: BottomViewSlot["id"];
  label: string;
  lines: string[];
  toneAt?: (line: number) => ConsoleTone;
  /** A new value follows the end of an empty log again. */
  restartKey: unknown;
  /** Shown above the lines, in the scroll area. */
  head?: ReactNode;
  /** True scrolls to the start, where `head` is. */
  atStart: boolean;
  find: Find | undefined;
  findLabel: string;
  onFindQuery: (query: string) => void;
  onFindClose: () => void;
};

export function LogView({ view, label, lines, toneAt, restartKey, head, atStart, find, findLabel, onFindQuery, onFindClose }: Props) {
  const scroller = useRef<HTMLDivElement>(null);
  const linesBox = useRef<HTMLDivElement>(null);
  const following = useRef(true);
  const [viewport, setViewport] = useState<Viewport>({ top: 0, height: 0, offset: 0 });
  const [current, setCurrent] = useState(0);
  const rowHeight = LINE_HEIGHT * textScale();

  const sync = () => {
    const el = scroller.current;
    if (!el) return;
    following.current = el.scrollTop + el.clientHeight >= el.scrollHeight - FOLLOW_SLACK;
    const top = Math.floor(el.scrollTop / rowHeight) * rowHeight;
    const next = { top, height: el.clientHeight, offset: linesBox.current?.offsetTop ?? 0 };
    setViewport((was) => (was.top === next.top && was.height === next.height && was.offset === next.offset ? was : next));
  };

  useEffect(() => {
    const el = scroller.current;
    if (!el) return;
    const observer = new ResizeObserver(sync);
    observer.observe(el);
    return () => observer.disconnect();
  }, [rowHeight]);

  useLayoutEffect(() => {
    following.current = true;
  }, [restartKey]);

  useLayoutEffect(() => {
    const el = scroller.current;
    if (el && following.current) el.scrollTop = el.scrollHeight;
    sync();
  }, [lines.length, head !== undefined]);

  useLayoutEffect(() => {
    const el = scroller.current;
    if (!atStart || !el) return;
    following.current = false;
    el.scrollTop = 0;
    sync();
  }, [atStart, restartKey]);

  const matches = useMemo(() => matchesIn(lines, find?.query ?? ""), [lines, find?.query]);
  const at = matches.length > 0 ? Math.min(current, matches.length - 1) : 0;
  useEffect(() => setCurrent(0), [find?.query]);
  const showMatch = (index: number) => {
    const el = scroller.current;
    const match = matches[index];
    if (!el || !match) return;
    following.current = false;
    el.scrollTop = Math.max(0, (linesBox.current?.offsetTop ?? 0) + match.line * rowHeight - el.clientHeight / 2);
    sync();
  };
  useEffect(() => showMatch(at), [find?.query]);

  const first = Math.max(0, Math.floor((viewport.top - viewport.offset) / rowHeight) - OVERSCAN);
  const last = Math.min(lines.length, Math.ceil((viewport.top + viewport.height - viewport.offset) / rowHeight) + OVERSCAN);
  const currentMatch = matches[at];
  return (
    <div className="log-part" data-view={view}>
      <div ref={scroller} className="log-scroll" tabIndex={0} aria-label={label} data-selectable onScroll={sync}>
        {head}
        <div ref={linesBox} className="log-lines" style={{ height: lines.length * rowHeight, paddingTop: first * rowHeight }}>
          {lines.slice(first, last).map((text, n) => {
            const line = first + n;
            return (
              <div key={line} className="log-line" data-tone={toneAt?.(line)}>
                {find?.query
                  ? segmentsOf(text, find.query).map((part, k) =>
                      part.match ? (
                        <mark key={k} className="log-match" data-current={(currentMatch?.line === line && currentMatch.start === part.start) || undefined}>
                          {part.text}
                        </mark>
                      ) : (
                        part.text
                      ),
                    )
                  : text}
              </div>
            );
          })}
        </div>
      </div>
      {find && (
        <FindBar
          query={find.query}
          focus={find.focus}
          current={at}
          count={matches.length}
          label={findLabel}
          closeTitle={tooltip("build.findClose")}
          onQuery={onFindQuery}
          onStep={(by) => {
            const next = (at + by + matches.length) % matches.length;
            setCurrent(next);
            showMatch(next);
          }}
          onClose={(byButton) => {
            onFindClose();
            if (byButton) scroller.current?.focus();
          }}
        />
      )}
    </div>
  );
}

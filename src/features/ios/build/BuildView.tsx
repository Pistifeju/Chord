// The Build view in the bottom panel: the build's errors above its log, which stays at the end while output arrives. Only the lines in view are mounted.
import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { tooltip } from "../../../app/commands";
import type { BuildIssue_Deserialize } from "../../../ipc/bindings";
import { FindBar } from "../../../ui/FindBar";
import { WarningIcon } from "../../../ui/icons";
import { requestLine } from "../../editor/reveal";
import { showFile } from "../../editor/showing";
import { buildFinds, closeBuildFind, setBuildFindQuery } from "./find";
import { logLines, matchesIn, segmentsOf } from "./log";
import { pathInWorktree } from "./run";
import { buildLogs, builds } from "./store";
import "./build.css";

/** A log line's height at the default text size, in pixels; it scales with the text size. */
const LINE_HEIGHT = 18;
/** Rows mounted beyond the ones in view, so a fast scroll shows no gap. */
const OVERSCAN = 20;
const FOLLOW_SLACK = 8;

const textScale = () => parseFloat(getComputedStyle(document.documentElement).getPropertyValue("--text-scale")) || 1;

type Viewport = { top: number; height: number; offset: number };

function IssueRow({ worktree, issue }: { worktree: string; issue: BuildIssue_Deserialize }) {
  const name = issue.file?.split("/").pop();
  const place = name && (issue.line ? `${name}:${issue.line}` : name);
  const path = issue.file ? pathInWorktree(worktree, issue.file) : null;
  const content = (
    <>
      <span className="build-issue-icon">
        <WarningIcon />
      </span>
      {place && <span className="build-issue-place">{place}</span>}
      <span className="build-issue-message" title={issue.message}>
        {issue.message}
      </span>
    </>
  );
  if (path === null) return <li className="build-issue">{content}</li>;
  return (
    <li>
      <button
        type="button"
        className="build-issue"
        onClick={() => {
          if (issue.line) requestLine(worktree, path, issue.line);
          showFile(worktree, path);
        }}
      >
        {content}
      </button>
    </li>
  );
}

export function BuildView({ worktree }: { worktree: string }) {
  const build = builds.usePart((all) => all[worktree]);
  const log = buildLogs.usePart((all) => all[worktree]);
  const find = buildFinds.usePart((all) => all[worktree]);
  const lines = useMemo(() => (log ? logLines(log) : []), [log]);
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
  });

  // A new build starts at the end of an empty log again.
  useLayoutEffect(() => {
    following.current = true;
  }, [build?.startedAt]);

  useLayoutEffect(() => {
    const el = scroller.current;
    if (el && following.current) el.scrollTop = el.scrollHeight;
    sync();
  }, [lines.length, build?.phase]);

  // A failed build shows its errors first.
  const failed = build?.phase === "failed";
  useLayoutEffect(() => {
    const el = scroller.current;
    if (!failed || !el) return;
    following.current = false;
    el.scrollTop = 0;
    sync();
  }, [failed, build?.startedAt]);

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

  if (!build) return <div className="build-part build-empty">No build yet</div>;
  const first = Math.max(0, Math.floor((viewport.top - viewport.offset) / rowHeight) - OVERSCAN);
  const last = Math.min(lines.length, Math.ceil((viewport.top + viewport.height - viewport.offset) / rowHeight) + OVERSCAN);
  const currentMatch = matches[at];
  return (
    <div className="build-part">
      <div ref={scroller} className="build-scroll" tabIndex={0} aria-label="Build log" data-selectable onScroll={sync}>
        {failed && build.issues.length > 0 && (
          <ul className="build-issues">
            {build.issues.map((issue, n) => (
              <IssueRow key={n} worktree={worktree} issue={issue} />
            ))}
            {build.more > 0 && <li className="build-more">and {build.more} more</li>}
          </ul>
        )}
        <div ref={linesBox} className="build-lines" style={{ height: lines.length * rowHeight, paddingTop: first * rowHeight }}>
          {lines.slice(first, last).map((text, n) => {
            const line = first + n;
            return (
              <div key={line} className="build-line">
                {find?.query
                  ? segmentsOf(text, find.query).map((part, k) =>
                      part.match ? (
                        <mark key={k} className="build-match" data-current={(currentMatch?.line === line && currentMatch.start === part.start) || undefined}>
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
          label="Find in Build Log"
          closeTitle={tooltip("build.findClose")}
          onQuery={(query) => setBuildFindQuery(worktree, query)}
          onStep={(by) => {
            const next = (at + by + matches.length) % matches.length;
            setCurrent(next);
            showMatch(next);
          }}
          onClose={(byButton) => {
            closeBuildFind(worktree);
            if (byButton) scroller.current?.focus();
          }}
        />
      )}
    </div>
  );
}

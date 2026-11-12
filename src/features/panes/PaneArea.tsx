// The panes of one worktree: the session tab bar and the split layout of the open session.
import { Fragment, useEffect, useRef, useState, type PointerEvent, type RefObject, type SyntheticEvent } from "react";
import type { Direction, Layout } from "../../ipc/bindings";
import { tooltip } from "../../app/commands";
import { ChatPane } from "../chat/ChatPane";
import { chats, usePaneNames } from "../chat/store";
import { shortcutChanges } from "../settings/store";
import type { Status } from "../../ipc/bindings";
import { statusLabel, urgentReport } from "../status/status";
import { StatusMark } from "../status/StatusMark";
import { finished } from "../status/store";
import { sidebar } from "../sidebar/store";
import { ChatIcon, CloseIcon, PlusIcon } from "../../ui/icons";
import { dropEdge, dropPreview, paneIds, resizedSizes, type Box, type Edge } from "./layout";
import { activeSession, tabName } from "./sessions";
import { closePane, closeSession, dropSession, endResize, ensurePanes, evenOutSplit, focusPane, newSession, panes, resizeSplit, selectSession } from "./store";
import "./panes.css";

type Place = { worktree: string; session: string; focused: string; split: boolean };

type TabDrag = { session: string; startX: number; startY: number; started: boolean };
type Drop = { pane: string; edge: Edge; box: Box };

/** Pixels a tab moves before a press becomes a drag, so a click still selects. */
const DRAG_THRESHOLD = 5;

// The edge strip of the pane under the pointer where a drop would split it.
function dropAt(x: number, y: number): Drop | null {
  const element = document.elementFromPoint(x, y)?.closest<HTMLElement>("[data-pane]");
  if (!element?.dataset.pane) return null;
  const rect = element.getBoundingClientRect();
  const edge = dropEdge(x, y, rect);
  if (!edge) return null;
  const box = dropPreview(edge, rect);
  return { pane: element.dataset.pane, edge, box };
}

function SessionTabs({ worktree }: { worktree: string }) {
  shortcutChanges.use();
  const { hidden } = sidebar.use();
  const current = panes.use()[worktree];
  const statusesJson = chats.usePart((all) =>
    JSON.stringify((current?.sessions ?? []).map((s) => urgentReport(paneIds(s.layout).map((pane) => all[pane]?.report))?.status ?? null)),
  );
  const statuses: (Status | null)[] = JSON.parse(statusesJson);
  const names = usePaneNames((current?.sessions ?? []).map((s) => s.focused));
  const finishedPanes = finished.use();
  const drag = useRef<TabDrag | null>(null);
  const suppressClick = useRef(false);
  const [dragging, setDragging] = useState<string | null>(null);
  const [drop, setDrop] = useState<Drop | null>(null);
  const dropRef = useRef<Drop | null>(null);
  const bar = useRef<HTMLDivElement>(null);
  useEffect(() => {
    bar.current?.querySelector('[aria-selected="true"]')?.scrollIntoView({ block: "nearest", inline: "nearest" });
  }, [current?.active]);
  if (!current) return null;

  const onTabDown = (e: PointerEvent<HTMLElement>, session: string) => {
    if (e.button !== 0) return;
    e.currentTarget.setPointerCapture(e.pointerId);
    drag.current = { session, startX: e.clientX, startY: e.clientY, started: false };
  };

  const onTabMove = (e: PointerEvent) => {
    const d = drag.current;
    if (!d) return;
    if (!d.started) {
      if (Math.hypot(e.clientX - d.startX, e.clientY - d.startY) < DRAG_THRESHOLD) return;
      d.started = true;
      setDragging(d.session);
    }
    // The open session's own tab has nowhere to go.
    dropRef.current = d.session === current.active ? null : dropAt(e.clientX, e.clientY);
    setDrop(dropRef.current);
  };

  const onTabEnd = (e: PointerEvent) => {
    const d = drag.current;
    drag.current = null;
    if (!d?.started) return;
    suppressClick.current = true;
    setDragging(null);
    setDrop(null);
    const target = dropRef.current;
    dropRef.current = null;
    if (e.type === "pointerup" && target) dropSession(worktree, d.session, target.pane, target.edge);
  };

  return (
    <div ref={bar} className="tabbar" role="tablist" data-tauri-drag-region data-sidebar-hidden={hidden}>
      {current.sessions.map((session, i) => {
        // The open session's panes show their own Working row, so its tab shows no status; a done you already saw shows none either.
        const report = session.id === current.active ? null : statuses[i];
        const unseenDone = paneIds(session.layout).some((pane) => finishedPanes.includes(pane));
        const status = report?.kind === "done" && !unseenDone ? null : report;
        return (
          <div key={session.id} className="tab" data-selected={session.id === current.active} data-dragging={session.id === dragging}>
            <button
              type="button"
              role="tab"
              className="tab-name"
              aria-selected={session.id === current.active}
              onPointerDown={(e) => onTabDown(e, session.id)}
              onPointerMove={onTabMove}
              onPointerUp={onTabEnd}
              onPointerCancel={onTabEnd}
              onClick={() => {
                if (suppressClick.current) suppressClick.current = false;
                else selectSession(worktree, session.id);
              }}
            >
              <span className="tab-status" data-status={status?.kind} title={status ? statusLabel(status) : undefined}>
                {status ? <StatusMark kind={status.kind} /> : <ChatIcon />}
              </span>
              <span className="tab-label">{tabName(session, names)}</span>
            </button>
            {current.sessions.length > 1 && (
              <button type="button" className="tab-close" title="Close Session" aria-label="Close Session" onClick={() => closeSession(worktree, session.id)}>
                <CloseIcon />
              </button>
            )}
          </div>
        );
      })}
      <button type="button" className="icon-button" title={tooltip("session.new")} aria-label="New Session" onClick={() => newSession(worktree)}>
        <PlusIcon />
      </button>
      {drop && <div className="drop-edge" style={drop.box} />}
    </div>
  );
}

type DividerProps = { place: Place; path: number[]; index: number; direction: Direction; sizes: number[]; container: RefObject<HTMLDivElement | null> };
type Drag = { start: number; sizes: number[]; total: number; frame: number; moved: number | null };

function Divider({ place, path, index, direction, sizes, container }: DividerProps) {
  const drag = useRef<Drag | null>(null);
  const along = (e: PointerEvent) => (direction === "row" ? e.clientX : e.clientY);

  const apply = (d: Drag) => {
    if (d.moved !== null) resizeSplit(place.worktree, place.session, path, resizedSizes(d.sizes, index, d.moved / d.total));
  };

  const onPointerDown = (e: PointerEvent<HTMLDivElement>) => {
    const box = container.current?.getBoundingClientRect();
    if (!box) return;
    e.currentTarget.setPointerCapture(e.pointerId);
    drag.current = { start: along(e), sizes, total: direction === "row" ? box.width : box.height, frame: 0, moved: null };
  };

  // At most one layout update per frame.
  const onPointerMove = (e: PointerEvent) => {
    const d = drag.current;
    if (!d) return;
    d.moved = along(e) - d.start;
    if (!d.frame)
      d.frame = requestAnimationFrame(() => {
        d.frame = 0;
        apply(d);
      });
  };

  const onPointerEnd = () => {
    const d = drag.current;
    drag.current = null;
    if (!d || d.moved === null) return;
    cancelAnimationFrame(d.frame);
    apply(d);
    endResize(place.worktree);
  };

  return (
    <div
      className="divider"
      data-direction={direction}
      role="separator"
      aria-orientation={direction === "row" ? "vertical" : "horizontal"}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerEnd}
      onPointerCancel={onPointerEnd}
      onDoubleClick={() => evenOutSplit(place.worktree, place.session, path)}
    />
  );
}

function LayoutView({ layout, path, place }: { layout: Layout; path: number[]; place: Place }) {
  const container = useRef<HTMLDivElement>(null);
  if (layout.kind === "pane") {
    const { id } = layout;
    // A press on the pane's close button keeps the focus where it was.
    const focusUnlessClose = (e: SyntheticEvent) => {
      if (!(e.target as Element).closest(".pane-close")) focusPane(place.worktree, id);
    };
    return (
      <div className="pane" data-pane={id} data-split={place.split || undefined} data-focused={id === place.focused || undefined} onPointerDownCapture={focusUnlessClose} onFocusCapture={focusUnlessClose}>
        <ChatPane worktree={place.worktree} pane={id} focused={id === place.focused} />
        {place.split && (
          <button type="button" className="pane-close" title={tooltip("pane.close")} aria-label="Close Pane" onClick={() => closePane(place.worktree, id)}>
            <CloseIcon size={15} />
          </button>
        )}
      </div>
    );
  }
  const sizes = layout.sizes.map(Number);
  return (
    <div className="split" data-direction={layout.direction} ref={container}>
      {layout.children.map((child, i) => (
        <Fragment key={child.kind === "pane" ? child.id : `split-${i}`}>
          {i > 0 && <Divider place={place} path={path} index={i - 1} direction={layout.direction} sizes={sizes} container={container} />}
          <div className="split-child" style={{ flex: `${sizes[i]} 1 0` }}>
            <LayoutView layout={child} path={[...path, i]} place={place} />
          </div>
        </Fragment>
      ))}
    </div>
  );
}

export function PaneArea({ worktree }: { worktree: string }) {
  const current = panes.use()[worktree];
  useEffect(() => ensurePanes(worktree), [worktree]);
  if (!current) return null;
  const session = activeSession(current);
  return (
    <div className="panes">
      <SessionTabs worktree={worktree} />
      <div className="pane-layout">
        <LayoutView key={session.id} layout={session.layout} path={[]} place={{ worktree, session: session.id, focused: session.focused, split: session.layout.kind === "split" }} />
      </div>
    </div>
  );
}

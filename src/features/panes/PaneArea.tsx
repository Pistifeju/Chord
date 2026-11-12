// The panes of one worktree: the session tab bar and the split layout of the open session.
import { Fragment, useEffect, useRef, type PointerEvent, type RefObject } from "react";
import type { Direction, Layout } from "../../ipc/bindings";
import { tooltip } from "../../app/commands";
import { ChatPane } from "../chat/ChatPane";
import { chats } from "../chat/store";
import { shortcutChanges } from "../settings/store";
import { sidebar } from "../sidebar/store";
import { ChatIcon, CloseIcon, PlusIcon } from "../../ui/icons";
import { paneIds, resizedSizes } from "./layout";
import { activeSession } from "./sessions";
import { closeSession, endResize, ensurePanes, evenOutSplit, focusPane, newSession, panes, resizeSplit, selectSession } from "./store";
import "./panes.css";

type Place = { worktree: string; session: string; focused: string; split: boolean };

function SessionTabs({ worktree }: { worktree: string }) {
  shortcutChanges.use();
  const { hidden } = sidebar.use();
  const all = chats.use();
  const current = panes.use()[worktree];
  if (!current) return null;
  return (
    <div className="tabbar" role="tablist" data-tauri-drag-region data-sidebar-hidden={hidden}>
      {current.sessions.map((session) => {
        const working = paneIds(session.layout).some((pane) => all[pane]?.running);
        return (
          <div key={session.id} className="tab" data-selected={session.id === current.active}>
            <button type="button" role="tab" className="tab-name" aria-selected={session.id === current.active} onClick={() => selectSession(worktree, session.id)}>
              <span className="tab-status">{working ? <span className="tab-spinner" /> : <ChatIcon />}</span>
              <span className="tab-label">{session.name}</span>
            </button>
            {current.sessions.length > 1 && (
              <button type="button" className="tab-close" aria-label="Close Session" onClick={() => closeSession(worktree, session.id)}>
                <CloseIcon />
              </button>
            )}
          </div>
        );
      })}
      <button type="button" className="icon-button" title={tooltip("session.new")} aria-label="New Session" onClick={() => newSession(worktree)}>
        <PlusIcon />
      </button>
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
    return (
      <div className="pane" data-focused={place.split && id === place.focused} onPointerDownCapture={() => focusPane(place.worktree, id)} onFocusCapture={() => focusPane(place.worktree, id)}>
        <ChatPane worktree={place.worktree} pane={id} focused={id === place.focused} />
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

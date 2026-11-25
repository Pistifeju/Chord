import { Fragment, useEffect, useRef, useState, type PointerEvent, type ReactNode, type RefObject } from "react";
import type { DiffSide, Direction, Layout, Status, Tab } from "../../ipc/bindings";
import { tooltip } from "../../app/commands";
import { diffView, editorView } from "../../app/deferredViews";
import { imageMime } from "../chat/attachments";
import { ChatPane } from "../chat/ChatPane";
import { chats, paneHarnesses, usePaneNames } from "../chat/store";
import { chatLogo, harnessMarks } from "../harnesses/logos";
import { shortcutChanges } from "../settings/store";
import { seenStatus, statusLabel } from "../status/status";
import { StatusMark } from "../status/StatusMark";
import { finished } from "../status/store";
import { BottomPanel, type BottomViewSlot } from "../bottompanel/BottomPanel";
import { SidePanel, type SideViewSlot } from "../sidepanel/SidePanel";
import { sidebar } from "../sidebar/store";
import { ImageView } from "../editor/ImageView";
import { selectFile } from "../files/store";
import { CloseIcon, DiffIcon, FileIcon, GlobeIcon, PlusIcon } from "../../ui/icons";
import { BrowserView } from "../browser/BrowserView";
import { usePointerDrag } from "../../ui/pointerDrag";
import { chatPaneIds, dropEdge, dropPreview, edgeDirection, fitsSplit, groupOf, minExtent, paneIds, resizedSizes, type Box, type Group } from "./layout";
import { diffTitle } from "../changes/changes";
import { movesTab, tabName, type PaneNames, type TabDrop } from "./sessions";
import { closePane, endResize, ensurePanes, evenOutSplit, focusPane, keepFilePane, moveTab, newSession, panes, resizeSplit } from "./store";
import "./panes.css";

type Drop = TabDrop & { box: Box };

type TabDrag = { handlers: (tab: string) => object; clicked: () => boolean; dragging: string | null };

type Edges = { top: boolean; left: boolean };

type Place = { worktree: string; focused: string; split: boolean; names: PaneNames; drag: TabDrag; seen: Set<string> };

const rectOf = (element: Element): Box => {
  const { left, top, width, height } = element.getBoundingClientRect();
  return { left, top, width, height };
};

function dropAt(x: number, y: number, last: Drop | null): Drop | null {
  const under = document.elementFromPoint(x, y);
  if (under?.closest(".divider")) return last;
  const group = under?.closest<HTMLElement>("[data-group]");
  const body = group?.querySelector(".pane");
  if (!group?.dataset.group || !body) return null;
  if (under?.closest("[data-group-bar]")) return { group: group.dataset.group, edge: null, box: rectOf(group) };
  const box = rectOf(body);
  const edge = dropEdge(x, y, box);
  if (!fitsSplit(rectOf(group), edgeDirection(edge))) return null;
  return { group: group.dataset.group, edge, box: dropPreview(edge, box) };
}

function useTabDrag(worktree: string) {
  const [dragging, setDragging] = useState<string | null>(null);
  const [drop, setDrop] = useState<Drop | null>(null);
  const dropRef = useRef<Drop | null>(null);
  const drag = usePointerDrag<string>({
    onStart: setDragging,
    onMove: (e, tab) => {
      const at = dropAt(e.clientX, e.clientY, dropRef.current);
      const current = panes.get()[worktree];
      dropRef.current = at && current && movesTab(current, tab, at) ? at : null;
      const next = dropRef.current;
      setDrop((prev) => (prev && next && prev.group === next.group && prev.edge === next.edge ? prev : next));
    },
    onEnd: (tab, dropped) => {
      const target = dropRef.current;
      dropRef.current = null;
      setDragging(null);
      setDrop(null);
      if (dropped && target) moveTab(worktree, tab, { group: target.group, edge: target.edge });
    },
  });
  return { tabDrag: { handlers: drag.handlers, clicked: drag.clicked, dragging }, drop, ghost: drag.ghost };
}

const tabIcon = (tab: Tab, harness: string | null) => {
  if (tab.kind === "file") return tab.diff ? <DiffIcon /> : <FileIcon />;
  if (tab.kind === "browser") return <GlobeIcon />;
  const Logo = chatLogo(harness);
  return <Logo />;
};

const closeLabel = (tab: Tab) => (tab.kind === "file" ? "Close File" : tab.kind === "browser" ? "Close Page" : "Close Session");

function TabView({ tab, group, place, status }: { tab: Tab; group: Group; place: Place; status: Status | null }) {
  const { worktree, drag } = place;
  const open = tab.id === group.active;
  const file = tab.kind === "file";
  const kept = tab.kind !== "chat";
  const harness = paneHarnesses.usePart((all) => (tab.kind === "chat" ? (all[tab.id] ?? null) : null));
  const [fresh] = useState(() => !place.seen.has(tab.id));
  useEffect(() => void place.seen.add(tab.id), [place.seen, tab.id]);
  const show = () => {
    if (!drag.clicked()) return;
    focusPane(worktree, tab.id);
    if (tab.kind === "file") selectFile(worktree, tab.path);
  };
  return (
    <div className="tab" data-selected={open} data-dragging={tab.id === drag.dragging} data-fresh={fresh || undefined} data-preview={(file && tab.preview) || undefined}>
      <button
        type="button"
        role="tab"
        className="tab-name"
        aria-selected={open}
        title={file ? `${tab.path}${tab.diff ? diffTitle(tab.diff) : ""}` : tab.kind === "browser" ? tab.url : harness ? (harnessMarks[harness]?.title ?? harness) : undefined}
        {...drag.handlers(tab.id)}
        onClick={show}
        onDoubleClick={() => keepFilePane(worktree, tab.id)}
      >
        <span className="tab-status" data-status={status?.kind} title={status ? statusLabel(status) : undefined}>
          {status ? <StatusMark kind={status.kind} /> : tabIcon(tab, harness)}
        </span>
        <span className="tab-label">{tabName(tab, place.names)}</span>
      </button>
      <button
        type="button"
        className="tab-close"
        data-always={kept || undefined}
        title={closeLabel(tab)}
        aria-label={closeLabel(tab)}
        onClick={() => closePane(worktree, tab.id)}
      >
        <CloseIcon />
      </button>
    </div>
  );
}

function GroupTabs({ group, place, edges }: { group: Group; place: Place; edges: Edges }) {
  shortcutChanges.use();
  const { hidden } = sidebar.use();
  const { worktree } = place;
  const chatIds = chatPaneIds(group);
  const statusesJson = chats.usePart((all) => JSON.stringify(chatIds.map((tab) => all[tab]?.report?.status ?? null)));
  const statuses: (Status | null)[] = JSON.parse(statusesJson);
  const dialogs = chats.usePart((all) => chatIds.map((tab) => all[tab]?.dialog != null).join());
  const finishedPanes = finished.use();
  const statusOf = (tab: Tab): Status | null => {
    const report = tab.id === group.active ? null : statuses[chatIds.indexOf(tab.id)] ?? null;
    return seenStatus(report?.kind, [tab.id], finishedPanes, dialogs.split(",")[chatIds.indexOf(tab.id)] === "true") ? null : report;
  };
  return (
    <div className="tabbar" role="tablist" data-group-bar data-tauri-drag-region={edges.top || undefined} data-sidebar-hidden={(edges.top && edges.left && hidden) || undefined}>
      {group.tabs.map((tab) => (
        <TabView key={tab.id} tab={tab} group={group} place={place} status={statusOf(tab)} />
      ))}
      <button type="button" className="icon-button" title={tooltip("session.new")} aria-label="New Session" onClick={() => newSession(worktree, group.id)}>
        <PlusIcon />
      </button>
    </div>
  );
}

type DividerProps = { worktree: string; path: number[]; index: number; direction: Direction; sizes: number[]; mins: [number, number]; container: RefObject<HTMLDivElement | null> };
type Drag = { start: number; sizes: number[]; total: number; frame: number; moved: number | null };

function Divider({ worktree, path, index, direction, sizes, mins, container }: DividerProps) {
  const drag = useRef<Drag | null>(null);
  const along = (e: PointerEvent) => (direction === "row" ? e.clientX : e.clientY);

  const apply = (d: Drag) => {
    if (d.moved !== null) resizeSplit(worktree, path, resizedSizes(d.sizes, index, d.moved / d.total, [mins[0] / d.total, mins[1] / d.total]));
  };

  const onPointerDown = (e: PointerEvent<HTMLDivElement>) => {
    const box = container.current?.getBoundingClientRect();
    if (!box) return;
    e.currentTarget.setPointerCapture(e.pointerId);
    drag.current = { start: along(e), sizes, total: direction === "row" ? box.width : box.height, frame: 0, moved: null };
  };

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
    endResize(worktree);
  };

  return (
    <div
      className="divider"
      data-direction={direction}
      role="separator"
      aria-orientation={direction === "row" ? "vertical" : "horizontal"}
      onPointerDown={onPointerDown}
      onMouseDown={(e) => e.preventDefault()}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerEnd}
      onPointerCancel={onPointerEnd}
      onDoubleClick={() => evenOutSplit(worktree, path)}
    />
  );
}

function FileView({ worktree, path, diff, autoFocus, onGone, onEdit }: { worktree: string; path: string; diff: DiffSide | undefined; autoFocus: boolean; onGone: () => void; onEdit: () => void }) {
  const mime = imageMime(path);
  const editor = editorView.use();
  const differ = diffView.use();
  if (diff) return differ && <differ.DiffView worktree={worktree} path={path} side={diff} />;
  if (mime) return <ImageView worktree={worktree} path={path} mime={mime} onGone={onGone} />;
  return editor && <editor.Editor worktree={worktree} path={path} autoFocus={autoFocus} onGone={onGone} onEdit={onEdit} />;
}

function GroupView({ group, place, edges }: { group: Group; place: Place; edges: Edges }) {
  const { worktree } = place;
  const tab = group.tabs.find((t) => t.id === group.active) ?? group.tabs[0];
  if (!tab)
    return (
      <div className="group" data-group={group.id}>
        {edges.top && <div className="tabbar" data-empty data-tauri-drag-region />}
        <div className="pane">
          <div className="chat-empty">
            <h1 className="chat-empty-title">No tabs open</h1>
            <p className="chat-empty-text">Open a session in this worktree.</p>
            <button type="button" className="button-primary" onClick={() => newSession(worktree, group.id)}>
              New Session
            </button>
          </div>
        </div>
      </div>
    );
  const focused = tab.id === place.focused;
  const focus = () => focusPane(worktree, tab.id);
  return (
    <div className="group" data-group={group.id}>
      <GroupTabs group={group} place={place} edges={edges} />
      <div className="pane" data-pane={tab.id} data-split={place.split || undefined} data-focused={focused || undefined} onPointerDownCapture={focus} onFocusCapture={focus}>
        {tab.kind === "file" ? (
          <FileView key={tab.id} worktree={worktree} path={tab.path} diff={tab.diff ?? undefined} autoFocus={focused} onGone={() => closePane(worktree, tab.id)} onEdit={() => keepFilePane(worktree, tab.id)} />
        ) : tab.kind === "browser" ? (
          <BrowserView key={tab.id} tab={tab.id} url={tab.url} dragging={place.drag.dragging !== null} />
        ) : (
          <ChatPane key={tab.id} worktree={worktree} pane={tab.id} focused={focused} />
        )}
      </div>
    </div>
  );
}

function LayoutView({ layout, path, place, edges }: { layout: Layout; path: number[]; place: Place; edges: Edges }) {
  const container = useRef<HTMLDivElement>(null);
  if (layout.kind === "group") return <GroupView group={layout} place={place} edges={edges} />;
  const sizes = layout.sizes.map(Number);
  const row = layout.direction === "row";
  return (
    <div className="split" data-direction={layout.direction} ref={container}>
      {layout.children.map((child, i) => (
        <Fragment key={child.kind === "split" ? `split-${i}` : child.id}>
          {i > 0 && <Divider worktree={place.worktree} path={path} index={i - 1} direction={layout.direction} sizes={sizes} mins={[minExtent(layout.children[i - 1], layout.direction), minExtent(child, layout.direction)]} container={container} />}
          <div className="split-child" style={{ flex: `${sizes[i]} 1 0` }}>
            <LayoutView
              layout={child}
              path={[...path, i]}
              place={place}
              edges={{
                top: edges.top && (row || i === 0),
                left: edges.left && (!row || i === 0),
              }}
            />
          </div>
        </Fragment>
      ))}
    </div>
  );
}

function TabGhost({ tab, ghost, names }: { tab: Tab; ghost: (el: HTMLElement | null) => void; names: PaneNames }) {
  const harness = paneHarnesses.usePart((all) => all[tab.id] ?? null);
  return (
    <div className="drag-ghost" ref={ghost}>
      <span className="tab-status">{tabIcon(tab, harness)}</span>
      <span className="tab-label">{tabName(tab, names)}</span>
    </div>
  );
}

export function PaneArea({ worktree, toolbar, bottomViews, sideViews }: { worktree: string; toolbar: ReactNode; bottomViews: BottomViewSlot[]; sideViews: SideViewSlot[] }) {
  const current = panes.use()[worktree];
  useEffect(() => ensurePanes(worktree), [worktree]);
  const names = usePaneNames(current ? chatPaneIds(current.layout) : []);
  const { tabDrag, drop, ghost } = useTabDrag(worktree);
  const seen = useRef<Set<string> | null>(null);
  if (!current) return null;
  seen.current ??= new Set(paneIds(current.layout));
  const place: Place = { worktree, focused: current.focused, split: current.layout.kind === "split", names, drag: tabDrag, seen: seen.current };
  const dragged = tabDrag.dragging ? groupOf(current.layout, tabDrag.dragging)?.tabs.find((t) => t.id === tabDrag.dragging) : undefined;
  return (
    <div className="panes-frame" data-toolbar={toolbar ? true : undefined}>
      {toolbar}
      <div className="panes">
        <div className="panes-work">
          <div className="pane-layout">
            <LayoutView layout={current.layout} path={[]} place={place} edges={{ top: !toolbar, left: true }} />
          </div>
          <BottomPanel worktree={worktree} views={bottomViews} />
        </div>
        <SidePanel worktree={worktree} views={sideViews} />
        {drop && <div className="drop-edge" style={drop.box} />}
        {dragged && <TabGhost tab={dragged} ghost={ghost} names={names} />}
      </div>
    </div>
  );
}

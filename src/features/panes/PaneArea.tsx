// The panes of one worktree: the split layout of its tab groups, each group's tab bar over the tab it shows, and a tab's drag between them.
import { Fragment, useEffect, useRef, useState, type PointerEvent, type ReactNode, type RefObject } from "react";
import type { Direction, Layout, Status, Tab } from "../../ipc/bindings";
import { tooltip } from "../../app/commands";
import { editorView } from "../../app/deferredViews";
import { imageMime } from "../chat/attachments";
import { ChatPane } from "../chat/ChatPane";
import { chats, usePaneNames } from "../chat/store";
import { shortcutChanges } from "../settings/store";
import { seenDone, statusLabel } from "../status/status";
import { StatusMark } from "../status/StatusMark";
import { finished } from "../status/store";
import { BottomPanel, type BottomViewSlot } from "../bottompanel/BottomPanel";
import { SidePanel, type SideViewSlot } from "../sidepanel/SidePanel";
import { sidePanelOf, sidePanels, showsSideView, toggleSideView } from "../sidepanel/store";
import { sidebar } from "../sidebar/store";
import { ImageView } from "../editor/ImageView";
import { selectFile } from "../files/store";
import { CloseIcon, FileIcon, PiIcon, PlusIcon, SidePanelIcon } from "../../ui/icons";
import { usePointerDrag } from "../../ui/pointerDrag";
import { chatPaneIds, dropEdge, dropPreview, edgeDirection, fitsSplit, groupOf, minExtent, paneIds, resizedSizes, type Box, type Group } from "./layout";
import { canClosePane, movesTab, tabName, type PaneNames, type TabDrop } from "./sessions";
import { closeFilePane, closePane, endResize, ensurePanes, evenOutSplit, focusPane, moveTab, newSession, panes, resizeSplit } from "./store";
import "./panes.css";

type Drop = TabDrop & { box: Box };

/** The tab drag shared by every group's bar. */
type TabDrag = { handlers: (tab: string) => object; clicked: () => boolean; dragging: string | null };

/** Which window edges a group touches: its bar holds the room for the traffic lights or the Files button there. */
type Edges = { top: boolean; left: boolean; right: boolean };

// seen: the tabs already shown, so a tab that remounts after a split or a move does not animate in again.
// corner: the buttons at the window's top right end: the run control and the side panel's view buttons.
type Place = { worktree: string; focused: string; split: boolean; names: PaneNames; drag: TabDrag; seen: Set<string>; corner: ReactNode };

const rectOf = (element: Element): Box => {
  const { left, top, width, height } = element.getBoundingClientRect();
  return { left, top, width, height };
};

// Where a drop under the pointer lands: a group's tab bar adds the tab there, a group's body splits it at the nearest edge. A divider keeps the last drop, so the mark does not blink between groups.
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
  const [pointer, setPointer] = useState<{ left: number; top: number } | null>(null);
  const dropRef = useRef<Drop | null>(null);
  const drag = usePointerDrag<string>({
    onStart: setDragging,
    onMove: (e, tab) => {
      setPointer({ left: e.clientX, top: e.clientY });
      const at = dropAt(e.clientX, e.clientY, dropRef.current);
      const current = panes.get()[worktree];
      dropRef.current = at && current && movesTab(current, tab, at) ? at : null;
      setDrop(dropRef.current);
    },
    onEnd: (tab, dropped) => {
      const target = dropRef.current;
      dropRef.current = null;
      setDragging(null);
      setPointer(null);
      setDrop(null);
      if (dropped && target) moveTab(worktree, tab, { group: target.group, edge: target.edge });
    },
  });
  return { tabDrag: { handlers: drag.handlers, clicked: drag.clicked, dragging }, drop, pointer };
}

function FilesToggle({ worktree }: { worktree: string }) {
  shortcutChanges.use();
  const shown = sidePanels.usePart((all) => showsSideView(all, worktree, "files"));
  return (
    <button
      type="button"
      className="icon-button"
      title={tooltip("files.toggle")}
      aria-label="Show or Hide Files"
      aria-pressed={shown}
      onClick={() => toggleSideView(worktree, "files")}
    >
      <SidePanelIcon />
    </button>
  );
}

function TabView({ tab, group, place, status, closable }: { tab: Tab; group: Group; place: Place; status: Status | null; closable: boolean }) {
  const { worktree, drag } = place;
  const open = tab.id === group.active;
  const file = tab.kind === "file";
  const [fresh] = useState(() => !place.seen.has(tab.id));
  useEffect(() => void place.seen.add(tab.id), [place.seen, tab.id]);
  const show = () => {
    if (!drag.clicked()) return;
    focusPane(worktree, tab.id);
    if (tab.kind === "file") selectFile(worktree, tab.path);
  };
  return (
    <div className="tab" data-selected={open} data-dragging={tab.id === drag.dragging} data-fresh={fresh || undefined}>
      <button type="button" role="tab" className="tab-name" aria-selected={open} title={file ? tab.path : undefined} {...drag.handlers(tab.id)} onClick={show}>
        <span className="tab-status" data-status={status?.kind} title={status ? statusLabel(status) : undefined}>
          {status ? <StatusMark kind={status.kind} /> : file ? <FileIcon /> : <PiIcon />}
        </span>
        <span className="tab-label">{tabName(tab, place.names)}</span>
      </button>
      {(file || closable) && (
        <button
          type="button"
          className="tab-close"
          data-always={file || undefined}
          title={file ? "Close File" : "Close Session"}
          aria-label={file ? "Close File" : "Close Session"}
          onClick={() => (file ? closeFilePane(worktree, tab.id) : closePane(worktree, tab.id))}
        >
          <CloseIcon />
        </button>
      )}
    </div>
  );
}

function GroupTabs({ group, place, edges }: { group: Group; place: Place; edges: Edges }) {
  shortcutChanges.use();
  const { hidden } = sidebar.use();
  const { worktree } = place;
  const sidePanelOpen = sidePanels.usePart((all) => sidePanelOf(all, worktree).open);
  const closable = canClosePane(panes.use()[worktree]);
  const chatIds = chatPaneIds(group);
  const statusesJson = chats.usePart((all) => JSON.stringify(chatIds.map((tab) => all[tab]?.report?.status ?? null)));
  const statuses: (Status | null)[] = JSON.parse(statusesJson);
  const finishedPanes = finished.use();
  // The open tab's pane shows its own Working row, so its tab shows no status; a done you already saw shows none either.
  const statusOf = (tab: Tab): Status | null => {
    const report = tab.id === group.active ? null : statuses[chatIds.indexOf(tab.id)] ?? null;
    return seenDone(report?.kind, [tab.id], finishedPanes) ? null : report;
  };
  return (
    <div className="tabbar" role="tablist" data-group-bar data-tauri-drag-region={edges.top || undefined} data-sidebar-hidden={(edges.top && edges.left && hidden) || undefined}>
      {group.tabs.map((tab) => (
        <TabView key={tab.id} tab={tab} group={group} place={place} status={statusOf(tab)} closable={closable} />
      ))}
      <button type="button" className="icon-button" title={tooltip("session.new")} aria-label="New Session" onClick={() => newSession(worktree, group.id)}>
        <PlusIcon />
      </button>
      {edges.top && edges.right && !sidePanelOpen && (
        place.corner
      )}
    </div>
  );
}

/** mins: the pixels the children before and after the divider need. */
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
    endResize(worktree);
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
      onDoubleClick={() => evenOutSplit(worktree, path)}
    />
  );
}

// An open file: the image view for an image, otherwise the editor.
function FileView({ worktree, path, autoFocus, onGone }: { worktree: string; path: string; autoFocus: boolean; onGone: () => void }) {
  const mime = imageMime(path);
  const editor = editorView.use();
  if (mime) return <ImageView worktree={worktree} path={path} mime={mime} onGone={onGone} />;
  return editor && <editor.Editor worktree={worktree} path={path} autoFocus={autoFocus} onGone={onGone} />;
}

function GroupView({ group, place, edges }: { group: Group; place: Place; edges: Edges }) {
  const { worktree } = place;
  const tab = group.tabs.find((t) => t.id === group.active) ?? group.tabs[0];
  const focused = tab.id === place.focused;
  const focus = () => focusPane(worktree, tab.id);
  return (
    <div className="group" data-group={group.id}>
      <GroupTabs group={group} place={place} edges={edges} />
      <div className="pane" data-pane={tab.id} data-split={place.split || undefined} data-focused={focused || undefined} onPointerDownCapture={focus} onFocusCapture={focus}>
        {tab.kind === "file" ? (
          <FileView key={tab.id} worktree={worktree} path={tab.path} autoFocus={focused} onGone={() => closeFilePane(worktree, tab.id)} />
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
  const last = layout.children.length - 1;
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
                right: edges.right && (!row || i === last),
              }}
            />
          </div>
        </Fragment>
      ))}
    </div>
  );
}

// The chip that follows the pointer while a tab drags: the tab's icon and name.
function TabGhost({ tab, pointer, names }: { tab: Tab; pointer: { left: number; top: number }; names: PaneNames }) {
  return (
    <div className="tab-ghost" style={pointer}>
      <span className="tab-status">{tab.kind === "file" ? <FileIcon /> : <PiIcon />}</span>
      <span className="tab-label">{tabName(tab, names)}</span>
    </div>
  );
}

// `runControl`, `bottomViews` and `sideViews` are the iOS parts of the window, passed in by App.
export function PaneArea({ worktree, runControl, bottomViews, sideViews }: { worktree: string; runControl: ReactNode; bottomViews: BottomViewSlot[]; sideViews: SideViewSlot[] }) {
  const current = panes.use()[worktree];
  useEffect(() => ensurePanes(worktree), [worktree]);
  const names = usePaneNames(current ? chatPaneIds(current.layout) : []);
  const { tabDrag, drop, pointer } = useTabDrag(worktree);
  const seen = useRef<Set<string> | null>(null);
  if (!current) return null;
  // The tabs a worktree opens with show still.
  seen.current ??= new Set(paneIds(current.layout));
  const corner = (
    <>
      {runControl}
      <div className="side-view-toggles">
        {sideViews.map((view) => (
          <Fragment key={view.id}>{view.button}</Fragment>
        ))}
        <FilesToggle worktree={worktree} />
      </div>
    </>
  );
  const place: Place = { worktree, focused: current.focused, split: current.layout.kind === "split", names, drag: tabDrag, seen: seen.current, corner };
  const dragged = tabDrag.dragging ? groupOf(current.layout, tabDrag.dragging)?.tabs.find((t) => t.id === tabDrag.dragging) : undefined;
  return (
    <div className="panes">
      <div className="panes-work">
        <div className="pane-layout">
          <LayoutView layout={current.layout} path={[]} place={place} edges={{ top: true, left: true, right: true }} />
        </div>
        <BottomPanel worktree={worktree} views={bottomViews} />
      </div>
      <SidePanel worktree={worktree} views={sideViews} bar={corner} />
      {drop && <div className="drop-edge" style={drop.box} />}
      {dragged && pointer && <TabGhost tab={dragged} pointer={pointer} names={names} />}
    </div>
  );
}

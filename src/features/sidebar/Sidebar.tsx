// The sidebar: each project's worktrees as cards under its name, in project order, each with a row per chat session and its status; the drags that reorder the projects and the cards; its show/hide button and the Settings and Help buttons at its foot.
import { useEffect, useRef, useState, type RefObject } from "react";
import type { GitCounts_Serialize as GitCounts, Project, Worktree_Serialize as Worktree } from "../../ipc/bindings";
import { shortcutLabel, showMenu, tooltip } from "../../app/commands";
import { openSheet } from "../../app/sheet";
import { chats, usePaneNames } from "../chat/store";
import { chatTabs } from "../panes/layout";
import { tabName } from "../panes/sessions";
import { focusPane, panes } from "../panes/store";
import { openSettings, settingsOpen, shortcutChanges } from "../settings/store";
import { finished } from "../status/store";
import { ageLabel, seenDone, statusLabel, urgentReport } from "../status/status";
import { StatusMark } from "../status/StatusMark";
import { GearIcon, PlusIcon, QuestionIcon, SidebarIcon } from "../../ui/icons";
import { countsLabel, countsTitle } from "../worktrees/gitCounts";
import { gitCounts } from "../worktrees/gitCountsRefresh";
import { movesTo, worktreeName, worktreeOrder } from "../worktrees/projects";
import { cancelRename, finishRename, rename, setRenameName, startRename } from "../worktrees/rename";
import { moveProject, moveWorktree, pickProject, projects, selectWorktree } from "../worktrees/store";
import { usePointerDrag } from "../../ui/pointerDrag";
import { sidebar, toggleSidebar } from "./store";
import "./sidebar.css";

function SettingsButton() {
  shortcutChanges.use();
  return (
    <button type="button" className="icon-button" title={tooltip("settings.open")} aria-label="Settings" onClick={openSettings}>
      <GearIcon />
    </button>
  );
}

export function SidebarToggle() {
  shortcutChanges.use();
  return (
    <button type="button" className="icon-button" title={tooltip("sidebar.toggle")} aria-label="Show or Hide Sidebar" onClick={toggleSidebar}>
      <SidebarIcon />
    </button>
  );
}

/** A short mark on a worktree's meta line; its tone picks the color. */
type CardMarker = { text: string; tone: "busy" | "done" | "stopped" | "failed" };

type Dragged = { kind: "card" | "project"; project: string; path: string; name: string };
type SidebarDrag = { handlers: (item: Dragged) => object; clicked: () => boolean; dragging: string | null };
type Box = { top: number; left: number; width: number };

// The slot the pointer is over, by the items' middles, and the line between the items at that slot; `gap` is half the space after the last item.
function slotAt(items: HTMLElement[], y: number, gap: number): { slot: number; line: Box } {
  const boxes = items.map((item) => item.getBoundingClientRect());
  const slot = boxes.filter((box) => box.top + box.height / 2 < y).length;
  const before = boxes[slot - 1];
  const after = boxes[slot];
  const top = !before ? after.top : !after ? before.bottom + gap : (before.bottom + after.top) / 2;
  const { left, width } = after ?? before;
  return { slot, line: { top, left, width } };
}

// shortcut: scrolls only while the pointer moves near an edge; add a timer if holding still there should scroll on.
function scrollNearEdge(scroller: HTMLElement, y: number) {
  const { top, bottom } = scroller.getBoundingClientRect();
  if (y < top + 24) scroller.scrollBy(0, -8);
  else if (y > bottom - 24) scroller.scrollBy(0, 8);
}

// Its place in its list now, which a refresh during the drag can change.
function placeOf(item: Dragged): number {
  const { list, worktrees } = projects.get();
  return (item.kind === "card" ? (worktrees[item.project] ?? []) : list).findIndex((entry) => entry.path === item.path);
}

// A card moves only within its project: the line stays on the project's cards wherever the pointer goes.
function useSidebarDrag(scroller: RefObject<HTMLDivElement | null>) {
  const [dragged, setDragged] = useState<Dragged | null>(null);
  const [line, setLine] = useState<Box | null>(null);
  const [pointer, setPointer] = useState<{ left: number; top: number } | null>(null);
  const slot = useRef<number | null>(null);
  const drag = usePointerDrag<Dragged>({
    onStart: setDragged,
    onMove: (e, item) => {
      setPointer({ left: e.clientX, top: e.clientY });
      const box = scroller.current;
      if (!box) return;
      scrollNearEdge(box, e.clientY);
      const card = item.kind === "card";
      const items = card ? box.querySelectorAll<HTMLElement>(`[data-project="${CSS.escape(item.project)}"] .worktree-card`) : box.querySelectorAll<HTMLElement>(".project-group");
      const at = slotAt([...items], e.clientY, card ? 1 : 6);
      slot.current = movesTo(placeOf(item), at.slot) ? at.slot : null;
      setLine(slot.current === null ? null : at.line);
    },
    onEnd: (item, dropped) => {
      const to = slot.current;
      slot.current = null;
      setDragged(null);
      setLine(null);
      setPointer(null);
      if (!dropped || to === null) return;
      const from = placeOf(item);
      if (from < 0) return;
      if (item.kind === "card") moveWorktree(item.project, from, to);
      else moveProject(from, to);
    },
  });
  return { drag: { handlers: drag.handlers, clicked: drag.clicked, dragging: dragged?.path ?? null }, dragged, line, pointer };
}

// `markers` are the iOS part of the cards, passed in by App by worktree path.
export function Sidebar({ markers }: { markers: Record<string, CardMarker> }) {
  const { hidden } = sidebar.use();
  shortcutChanges.use();
  const state = projects.use();
  const counts = gitCounts.use();
  const finishedPanes = finished.use();
  const settings = settingsOpen.use();
  const scroller = useRef<HTMLDivElement>(null);
  const { drag, dragged, line, pointer } = useSidebarDrag(scroller);
  if (hidden) return null;
  const order = worktreeOrder(state);
  return (
    <nav className="sidebar" aria-label="Workspaces">
      <div className="sidebar-header">
        <span className="sidebar-title">Workspaces</span>
        <button type="button" className="icon-button" title={tooltip("project.add")} aria-label="Add Project" onClick={pickProject}>
          <PlusIcon />
        </button>
      </div>
      <div className="sidebar-scroll" ref={scroller}>
        {state.list.map((project) => (
          <section key={project.path} className="project-group" aria-label={project.name} data-project={project.path} data-dragging={drag.dragging === project.path}>
            <div
              className="project-name"
              title={project.path}
              {...drag.handlers({ kind: "project", project: project.path, path: project.path, name: project.name })}
              onContextMenu={(e) => showMenu(e, [["worktree.new", project.path], ["project.remove", project.path]])}
            >
              {project.name}
            </div>
            <ul className="worktree-list">
              {(state.worktrees[project.path] ?? []).map((worktree) => (
                <WorktreeCard
                  key={worktree.path}
                  project={project}
                  worktree={worktree}
                  drag={drag}
                  selected={worktree.path === state.selected}
                  shown={worktree.path === state.selected && !settings}
                  hint={shortcutLabel(`worktree.select.${order.indexOf(worktree.path) + 1}`)}
                  counts={counts[worktree.path]}
                  marker={markers[worktree.path]}
                  finishedPanes={finishedPanes}
                />
              ))}
            </ul>
          </section>
        ))}
      </div>
      {line && <div className="drop-line" style={line} />}
      {dragged && pointer && (
        <div className="drag-ghost" style={pointer}>
          <span className="drag-ghost-label">{dragged.name}</span>
        </div>
      )}
      <div className="sidebar-footer">
        <SettingsButton />
        <button type="button" className="icon-button sidebar-help" title="Help" aria-label="Help" onClick={() => openSheet({ kind: "help" })}>
          <QuestionIcon />
        </button>
      </div>
    </nav>
  );
}

function useNow(): number {
  const [now, setNow] = useState(Date.now);
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), 30_000);
    return () => clearInterval(timer);
  }, []);
  return now;
}

// A session row's right slot: "Done" for a finish nobody opened, the mark and age while an agent runs, the status word while it needs attention, otherwise the age of the last status.
function SessionStatus({ paneIdList, finishedPanes }: { paneIdList: string[]; finishedPanes: string[] }) {
  const report = chats.usePart((all) => urgentReport(paneIdList.map((pane) => all[pane]?.report)));
  const now = useNow();
  if (!report) return null;
  const kind = report.status.kind;
  const age = ageLabel(report.since, now);
  if (seenDone(kind, paneIdList, finishedPanes)) return <span key="age" className="session-age">{age}</span>;
  if (kind === "done") {
    return (
      <span key="unseen-done" className="session-status" data-status="done">
        <StatusMark kind="done" />
        {statusLabel({ kind: "done" })}
      </span>
    );
  }
  const label = statusLabel(report.status);
  return (
    <span key={kind} className="session-status" data-status={kind} title={report.lastMessage.trim() || label}>
      <StatusMark kind={report.status.kind} />
      {kind === "running" ? age !== "now" && age : label}
    </span>
  );
}

// Reads only its own worktree's panes, so a resize or a focus in another worktree leaves it alone.
function WorktreeCard({
  project,
  worktree,
  drag,
  selected,
  shown,
  hint,
  counts,
  marker,
  finishedPanes,
}: {
  project: Project;
  worktree: Worktree;
  drag: SidebarDrag;
  selected: boolean;
  /** Its panes are on screen, so its focused session row reads as open. */
  shown: boolean;
  hint: string | undefined;
  counts: GitCounts | undefined;
  marker: CardMarker | undefined;
  finishedPanes: string[];
}) {
  const saved = panes.usePart((all) => all[worktree.path]);
  const countsText = countsLabel(counts);
  const sessions = saved ? chatTabs(saved.layout) : [];
  const names = usePaneNames(sessions.map((tab) => tab.id));
  const branch = worktreeName(worktree);
  const renaming = rename.usePart((form) => form?.path === worktree.path);
  const main = worktree.path === project.path;
  const menu: [string, string][] = [["worktree.new", project.path]];
  if (!main) menu.push(["worktree.rename", worktree.path], ["worktree.delete", worktree.path]);
  return (
    <li className="worktree-card" aria-current={selected} data-dragging={drag.dragging === worktree.path}>
      {renaming ? (
        <BranchNameField />
      ) : (
        <button
          type="button"
          className="worktree-head"
          title={worktree.path}
          {...drag.handlers({ kind: "card", project: project.path, path: worktree.path, name: branch })}
          onClick={() => drag.clicked() && selectWorktree(worktree.path)}
          onContextMenu={(e) => showMenu(e, [...menu, ["project.remove", project.path]])}
        >
          <span className="worktree-line">
            <span className="worktree-title" onDoubleClick={main ? undefined : () => startRename(worktree.path)}>
              {branch}
            </span>
            {hint && <span className="worktree-hint">{hint}</span>}
          </span>
          {(countsText.sides || countsText.changed || marker) && (
            <span className="worktree-line worktree-meta">
              <span className="worktree-counts" title={countsTitle(counts)}>
                {countsText.sides && <span>{countsText.sides}</span>}
                {countsText.changed && <span className="worktree-changed">{countsText.changed}</span>}
              </span>
              {marker && (
                <span className="worktree-marker" data-tone={marker.tone}>
                  {marker.text}
                </span>
              )}
            </span>
          )}
        </button>
      )}
      {sessions.length > 0 && (
        <ul className="session-list">
          {sessions.map((tab) => (
            <li key={tab.id}>
              <button
                type="button"
                className="session-row"
                aria-current={shown && tab.id === saved?.focused}
                onClick={() => {
                  selectWorktree(worktree.path);
                  focusPane(worktree.path, tab.id);
                }}
              >
                <span className="session-name">{tabName(tab, names)}</span>
                <SessionStatus paneIdList={[tab.id]} finishedPanes={finishedPanes} />
              </button>
            </li>
          ))}
        </ul>
      )}
    </li>
  );
}

// Leaving the field drops it, so a name git refuses never lingers on the card.
function BranchNameField() {
  const form = rename.use();
  const field = useRef<HTMLInputElement>(null);
  useEffect(() => {
    field.current?.focus();
    field.current?.select();
  }, []);
  if (!form) return null;
  return (
    <div className="worktree-head worktree-rename">
      <input
        ref={field}
        className="worktree-name-field"
        aria-label="Branch Name"
        value={form.name}
        spellCheck={false}
        readOnly={form.renaming}
        onChange={(e) => void setRenameName(e.currentTarget.value)}
        onContextMenu={(e) => e.stopPropagation()}
        onBlur={cancelRename}
        onKeyDown={(e) => {
          if (e.key !== "Enter" && e.key !== "Escape") return;
          e.preventDefault();
          e.stopPropagation();
          if (e.key === "Escape") cancelRename();
          else void finishRename();
        }}
      />
      {form.problem && <span className="field-hint" data-error>{form.problem}</span>}
    </div>
  );
}

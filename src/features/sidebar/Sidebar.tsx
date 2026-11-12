// The sidebar: every worktree as a card under its project's name, in project order, and its show/hide button.
import { Menu } from "@tauri-apps/api/menu";
import { useEffect, useState, type MouseEvent } from "react";
import type { GitCounts_Serialize as GitCounts, Project, Worktree_Serialize as Worktree } from "../../ipc/bindings";
import { menuItem, shortcutLabel, tooltip } from "../../app/commands";
import { chats, usePaneNames } from "../chat/store";
import { allPaneIds } from "../panes/agents";
import { panes } from "../panes/store";
import { settingsOpen, shortcutChanges } from "../settings/store";
import { finished } from "../status/store";
import { ageLabel, statusLabel, urgentReport } from "../status/status";
import { StatusMark } from "../status/StatusMark";
import { PlusIcon, SidebarIcon } from "../../ui/icons";
import { countsLabel, countsTitle } from "../worktrees/gitCounts";
import { gitCounts } from "../worktrees/gitCountsRefresh";
import { worktreeName, worktreeOrder } from "../worktrees/projects";
import { pickProject, projects, selectWorktree } from "../worktrees/store";
import { sidebar, toggleSidebar } from "./store";
import "./sidebar.css";

export function SidebarToggle() {
  shortcutChanges.use();
  return (
    <button type="button" className="icon-button" title={tooltip("sidebar.toggle")} aria-label="Show or Hide Sidebar" onClick={toggleSidebar}>
      <SidebarIcon />
    </button>
  );
}

export function Sidebar() {
  const { hidden } = sidebar.use();
  shortcutChanges.use();
  const state = projects.use();
  const counts = gitCounts.use();
  const finishedPanes = finished.use();
  const settings = settingsOpen.use();
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
      <ul className="worktree-list">
        {state.list.flatMap((project) =>
          (state.worktrees[project.path] ?? []).map((worktree) => (
            <WorktreeCard
              key={worktree.path}
              project={project}
              worktree={worktree}
              selected={worktree.path === state.selected}
              shown={worktree.path === state.selected && !settings}
              hint={shortcutLabel(`worktree.select.${order.indexOf(worktree.path) + 1}`)}
              counts={counts[worktree.path]}
              finishedPanes={finishedPanes}
            />
          )),
        )}
      </ul>
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

// The card's right slot: "Done" for a finish nobody opened, the mark and age while an agent runs, the status word while it needs attention, otherwise the age of the last status.
function WorktreeStatus({ paneIdList, unseenDone }: { paneIdList: string[]; unseenDone: boolean }) {
  const report = chats.usePart((all) => urgentReport(paneIdList.map((pane) => all[pane]?.report)));
  const now = useNow();
  const kind = report?.status.kind;
  if (unseenDone && (kind === undefined || kind === "done")) {
    return (
      <span key="unseen-done" className="worktree-status" data-status="done">
        <StatusMark kind="done" />
        {statusLabel({ kind: "done" })}
      </span>
    );
  }
  if (!report) return null;
  const age = ageLabel(report.since, now);
  if (kind === "done") return <span key="age" className="worktree-age">{age}</span>;
  const label = statusLabel(report.status);
  return (
    <span key={kind} className="worktree-status" data-status={kind} title={report.lastMessage.trim() || label}>
      <StatusMark kind={report.status.kind} />
      {kind === "running" ? age !== "now" && age : label}
    </span>
  );
}

async function showMenu(event: MouseEvent, items: [commandId: string, target: string][]) {
  event.preventDefault();
  const menu = await Menu.new({ items: items.map(([id, target]) => menuItem(id, target)) });
  await menu.popup();
}

// Reads only its own worktree's panes, so a resize or a focus in another worktree leaves it alone.
function WorktreeCard({
  project,
  worktree,
  selected,
  shown,
  hint,
  counts,
  finishedPanes,
}: {
  project: Project;
  worktree: Worktree;
  selected: boolean;
  /** Its panes are on screen, so the tabs show their status and the card does not. */
  shown: boolean;
  hint: string | undefined;
  counts: GitCounts | undefined;
  finishedPanes: string[];
}) {
  const saved = panes.usePart((all) => all[worktree.path]);
  const countsText = countsLabel(counts);
  const paneIdList = allPaneIds(saved);
  const names = usePaneNames(paneIdList);
  const branch = worktreeName(worktree);
  const title = paneIdList.map((pane) => names[pane]).find(Boolean);
  const unseenDone = paneIdList.some((pane) => finishedPanes.includes(pane));
  const menu: [string, string][] = [["worktree.new", project.path]];
  if (worktree.path !== project.path) menu.push(["worktree.delete", worktree.path]);
  return (
    <li>
      <button
        type="button"
        className="worktree-card"
        aria-current={selected}
        title={worktree.path}
        onClick={() => selectWorktree(worktree.path)}
        onContextMenu={(e) => showMenu(e, [...menu, ["project.remove", project.path]])}
      >
        <span className="worktree-line worktree-meta">
          <span className="worktree-project">{project.name}</span>
          {!shown && <WorktreeStatus paneIdList={paneIdList} unseenDone={unseenDone} />}
        </span>
        <span className="worktree-line worktree-title">{title ?? branch}</span>
        <span className="worktree-line worktree-meta worktree-branch-line">
          <span className="worktree-branch">{branch}</span>
          {(countsText.sides || countsText.changed) && (
            <span className="worktree-counts" title={countsTitle(counts)}>
              {countsText.sides && <span>{countsText.sides}</span>}
              {countsText.changed && <span className="worktree-changed">{countsText.changed}</span>}
            </span>
          )}
          {hint && <span className="worktree-hint">{hint}</span>}
        </span>
      </button>
    </li>
  );
}

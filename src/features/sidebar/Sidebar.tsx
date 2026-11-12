// The sidebar: one card per worktree across all projects, and its show/hide button.
import { Menu } from "@tauri-apps/api/menu";
import { useEffect, useState, type MouseEvent } from "react";
import type { GitCounts_Serialize as GitCounts, Project, Worktree_Serialize as Worktree } from "../../ipc/bindings";
import { menuItem, shortcutLabel, tooltip } from "../../app/commands";
import { chats, usePaneNames } from "../chat/store";
import { paneRows } from "../panes/sessions";
import { allPaneIds } from "../panes/agents";
import { focusPane, panes } from "../panes/store";
import { shortcutChanges } from "../settings/store";
import { finished } from "../status/store";
import { ageLabel, statusLabel, urgentReport } from "../status/status";
import { CheckIcon, PlusIcon, SidebarIcon } from "../../ui/icons";
import { countsLabel, countsTitle } from "../worktrees/gitCounts";
import { gitCounts } from "../worktrees/gitCountsRefresh";
import { projectInitials, worktreeName, worktreeOrder } from "../worktrees/projects";
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
  if (hidden) return null;
  const order = worktreeOrder(state);
  return (
    <nav className="sidebar" aria-label="Projects">
      <div className="sidebar-header">
        <span className="sidebar-title">Projects</span>
        <button type="button" className="icon-button" title={tooltip("project.add")} onClick={pickProject}>
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

// The top-right slot: "Done" for a finish nobody opened, the status word while an agent needs attention or runs, otherwise the age of the last status.
function WorktreeStatus({ paneIdList, unseenDone }: { paneIdList: string[]; unseenDone: boolean }) {
  const report = chats.usePart((all) => urgentReport(paneIdList.map((pane) => all[pane]?.report)));
  const [now, setNow] = useState(Date.now);
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), 30_000);
    return () => clearInterval(timer);
  }, []);
  const kind = report?.status.kind;
  if (unseenDone && (kind === undefined || kind === "done")) {
    return (
      <span className="worktree-status" data-status="done">
        <CheckIcon />
        Done
      </span>
    );
  }
  if (!report) return null;
  if (kind === "done") return <span className="worktree-age">{ageLabel(report.since, now)}</span>;
  const label = statusLabel(report.status);
  return (
    <span className="worktree-status" data-status={kind} title={report.lastMessage.trim() || label}>
      <span className="status-dot" />
      {label}
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
  hint,
  counts,
  finishedPanes,
}: {
  project: Project;
  worktree: Worktree;
  selected: boolean;
  hint: string | undefined;
  counts: GitCounts | undefined;
  finishedPanes: string[];
}) {
  const saved = panes.usePart((all) => all[worktree.path]);
  const countsText = countsLabel(counts);
  const countsHint = countsTitle(counts);
  const paneIdList = allPaneIds(saved);
  const names = usePaneNames(paneIdList);
  const rows = paneRows(saved, selected, names);
  const branch = worktreeName(worktree);
  const title = paneIdList.map((pane) => names[pane]).find(Boolean);
  const unseenDone = paneIdList.some((pane) => finishedPanes.includes(pane));
  const menu: [string, string][] = worktree.path === project.path ? [] : [["worktree.delete", worktree.path]];
  return (
    <li>
      <button
        type="button"
        className="worktree-card"
        aria-current={selected}
        data-pane-selected={rows.some((row) => row.focused) || undefined}
        title={worktree.path}
        onClick={() => selectWorktree(worktree.path)}
        onContextMenu={(e) => showMenu(e, [...menu, ["project.remove", project.path]])}
      >
        <span className="worktree-line">
          <span className="project-badge" aria-hidden="true">{projectInitials(project.name)}</span>
          <span className="worktree-project">{project.name}</span>
          <WorktreeStatus paneIdList={paneIdList} unseenDone={unseenDone} />
        </span>
        <span className="worktree-title">{title ?? branch}</span>
        {(title || countsText || hint) && (
          <span className="worktree-line">
            <span className="worktree-branch">{title && branch}</span>
            {countsText && <span className="worktree-counts" title={countsHint}>{countsText}</span>}
            {hint && <span className="worktree-hint">{hint}</span>}
          </span>
        )}
      </button>
      {rows.length > 0 && (
        <ul className="pane-list">
          {rows.map(({ pane, label, focused }) => (
            <li key={pane}>
              <button
                type="button"
                className="pane-row"
                data-finished={finishedPanes.includes(pane) || undefined}
                aria-current={focused}
                onClick={() => {
                  selectWorktree(worktree.path);
                  focusPane(worktree.path, pane);
                }}
              >
                {label}
              </button>
            </li>
          ))}
        </ul>
      )}
    </li>
  );
}

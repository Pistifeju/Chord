// The sidebar: one foldable group per project, one card per worktree, and its show/hide button.
import { Menu } from "@tauri-apps/api/menu";
import { useEffect, useState, type MouseEvent } from "react";
import type { GitCounts_Serialize as GitCounts, Project, Worktree_Serialize as Worktree } from "../../ipc/bindings";
import { menuItem, shortcutLabel, tooltip } from "../../app/commands";
import { chats } from "../chat/store";
import { paneRows } from "../panes/sessions";
import { allPaneIds } from "../panes/agents";
import { focusPane, panes } from "../panes/store";
import { shortcutChanges } from "../settings/store";
import { finished } from "../status/store";
import { ageLabel, statusLabel, urgentReport } from "../status/status";
import { ChevronIcon, PlusIcon, SidebarIcon } from "../../ui/icons";
import { countsLabel, countsTitle } from "../worktrees/gitCounts";
import { gitCounts } from "../worktrees/gitCountsRefresh";
import { worktreeName, worktreeOrder } from "../worktrees/projects";
import { pickProject, projects, selectWorktree } from "../worktrees/store";
import { sidebar, toggleFold, toggleSidebar } from "./store";
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
  const { list } = projects.use();
  if (hidden) return null;
  return (
    <nav className="sidebar" aria-label="Projects">
      <div className="sidebar-header">
        <span className="sidebar-title">Projects</span>
        <button type="button" className="icon-button" title={tooltip("project.add")} onClick={pickProject}>
          <PlusIcon />
        </button>
      </div>
      <div className="sidebar-list">
        {list.map((project) => (
          <ProjectGroup key={project.path} project={project} />
        ))}
      </div>
    </nav>
  );
}

// The status of the worktree's most urgent pane: its last message and how long ago the status began.
function WorktreeStatus({ paneIdList, unseenDone }: { paneIdList: string[]; unseenDone: boolean }) {
  const report = chats.usePart((all) => urgentReport(paneIdList.map((pane) => all[pane]?.report)));
  const [now, setNow] = useState(Date.now);
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), 30_000);
    return () => clearInterval(timer);
  }, []);
  if (!report) {
    if (!unseenDone) return null;
    return (
      <span className="worktree-status">
        <span className="status-dot" data-status="done" data-finished role="img" aria-label="Done" title="Done" />
      </span>
    );
  }
  const label = statusLabel(report.status);
  return (
    <span className="worktree-status">
      <span className="status-dot" data-status={report.status.kind} data-finished={unseenDone || undefined} role="img" aria-label={label} title={label} />
      <span className="worktree-last-message">{report.lastMessage.trim() || label}</span>
      <span className="worktree-age">{ageLabel(report.since, now)}</span>
    </span>
  );
}

async function showMenu(event: MouseEvent, commandId: string, target: string) {
  event.preventDefault();
  const menu = await Menu.new({ items: [menuItem(commandId, target)] });
  await menu.popup();
}

function ProjectGroup({ project }: { project: Project }) {
  const { folded } = sidebar.use();
  const state = projects.use();
  const { worktrees, selected } = state;
  const order = worktreeOrder(state);
  const counts = gitCounts.use();
  const finishedPanes = finished.use();
  shortcutChanges.use();
  const open = !folded.includes(project.path);
  return (
    <section className="project-group">
      <div className="project-header" onContextMenu={(e) => showMenu(e, "project.remove", project.path)}>
        <button type="button" className="project-name" aria-expanded={open} onClick={() => toggleFold(project.path)}>
          <span className="project-title">{project.name}</span>
          <span className="project-chevron turn-chevron" data-open={open}>
            <ChevronIcon />
          </span>
        </button>
      </div>
      {open && (
        <ul className="worktree-list">
          {(worktrees[project.path] ?? []).map((worktree) => (
            <WorktreeCard
              key={worktree.path}
              project={project}
              worktree={worktree}
              selected={worktree.path === selected}
              hint={shortcutLabel(`worktree.select.${order.indexOf(worktree.path) + 1}`)}
              counts={counts[worktree.path]}
              finishedPanes={finishedPanes}
            />
          ))}
        </ul>
      )}
    </section>
  );
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
  const rows = paneRows(saved, selected);
  return (
    <li>
      <button
        type="button"
        className="worktree-card"
        aria-current={selected}
        data-pane-selected={rows.some((row) => row.focused) || undefined}
        data-finished={paneIdList.some((pane) => finishedPanes.includes(pane)) || undefined}
        title={worktree.path}
        onClick={() => selectWorktree(worktree.path)}
        onContextMenu={worktree.path === project.path ? undefined : (e) => showMenu(e, "worktree.delete", worktree.path)}
      >
        <span className="worktree-line">
          <span className="worktree-name">{worktreeName(worktree)}</span>
          {countsText && <span className="worktree-counts" title={countsHint}>{countsText}</span>}
          {hint && <span className="worktree-hint">{hint}</span>}
        </span>
        <WorktreeStatus paneIdList={paneIdList} unseenDone={paneIdList.some((pane) => finishedPanes.includes(pane))} />
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

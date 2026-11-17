// The sidebar: each project's worktrees as cards under its name, in project order, each with a row per chat session and its status; its show/hide button and the Settings and Help buttons at its foot.
import { useEffect, useState } from "react";
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
import { worktreeName, worktreeOrder } from "../worktrees/projects";
import { pickProject, projects, selectWorktree } from "../worktrees/store";
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

// `markers` are the iOS part of the cards, passed in by App by worktree path.
export function Sidebar({ markers }: { markers: Record<string, CardMarker> }) {
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
      <div className="sidebar-scroll">
        {state.list.map((project) => (
          <section key={project.path} className="project-group" aria-label={project.name}>
            <div
              className="project-name"
              title={project.path}
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
  selected,
  shown,
  hint,
  counts,
  marker,
  finishedPanes,
}: {
  project: Project;
  worktree: Worktree;
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
  const menu: [string, string][] = [["worktree.new", project.path]];
  if (worktree.path !== project.path) menu.push(["worktree.delete", worktree.path]);
  return (
    <li className="worktree-card" aria-current={selected}>
      <button
        type="button"
        className="worktree-head"
        title={worktree.path}
        onClick={() => selectWorktree(worktree.path)}
        onContextMenu={(e) => showMenu(e, [...menu, ["project.remove", project.path]])}
      >
        <span className="worktree-line">
          <span className="worktree-title">{branch}</span>
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

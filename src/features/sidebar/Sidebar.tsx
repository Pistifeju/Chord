// The sidebar: one foldable group per project, one card per worktree, and its show/hide button.
import { Menu } from "@tauri-apps/api/menu";
import type { MouseEvent } from "react";
import type { Project } from "../../ipc/bindings";
import { menuItem, shortcutLabel, tooltip } from "../../app/commands";
import { shortcutChanges } from "../settings/store";
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
    <button type="button" className="icon-button" title={tooltip("sidebar.toggle")} onClick={toggleSidebar}>
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
  shortcutChanges.use();
  const open = !folded.includes(project.path);
  return (
    <section className="project-group">
      <div className="project-header" onContextMenu={(e) => showMenu(e, "project.remove", project.path)}>
        <button type="button" className="project-name" aria-expanded={open} onClick={() => toggleFold(project.path)}>
          <span className="project-title">{project.name}</span>
          <span className="project-chevron" data-open={open}>
            <ChevronIcon />
          </span>
        </button>
      </div>
      {open && (
        <ul className="worktree-list">
          {(worktrees[project.path] ?? []).map((worktree) => {
            const hint = shortcutLabel(`worktree.select.${order.indexOf(worktree.path) + 1}`);
            const countsText = countsLabel(counts[worktree.path]);
            const countsHint = countsTitle(counts[worktree.path]);
            return (
              <li key={worktree.path}>
                <button
                  type="button"
                  className="worktree-card"
                  aria-current={worktree.path === selected}
                  title={worktree.path}
                  onClick={() => selectWorktree(worktree.path)}
                  onContextMenu={worktree.path === project.path ? undefined : (e) => showMenu(e, "worktree.delete", worktree.path)}
                >
                  <span className="worktree-name">{worktreeName(worktree)}</span>
                  {countsText && <span className="worktree-counts" title={countsHint}>{countsText}</span>}
                  {hint && <span className="worktree-hint">{hint}</span>}
                </button>
              </li>
            );
          })}
        </ul>
      )}
    </section>
  );
}

// The window's layout. Each feature adds its own region.
import { useEffect } from "react";
import { ChatPane } from "../features/chat/ChatPane";
import { SettingsPage } from "../features/settings/SettingsPage";
import { settingsOpen } from "../features/settings/store";
import { Sidebar, SidebarToggle } from "../features/sidebar/Sidebar";
import { sidebar } from "../features/sidebar/store";
import { NoProjects } from "../features/worktrees/NoProjects";
import { DeleteWorktreePopup } from "../features/worktrees/DeleteWorktreePopup";
import { ForceRemovalPopup } from "../features/worktrees/ForceRemovalPopup";
import { NewWorktreePopup } from "../features/worktrees/NewWorktreePopup";
import { ProjectDrop } from "../features/worktrees/ProjectDrop";
import { watchGitCounts } from "../features/worktrees/gitCountsRefresh";
import { selectedNames } from "../features/worktrees/projects";
import { dismissError, projects } from "../features/worktrees/store";
import { WarningToast } from "../ui/Toast";
import { runShortcut } from "./commands";
import { launch } from "./launch";
import "./app.css";

export default function App() {
  const state = projects.use();
  const { list, selected, error, errorIsNotice } = state;
  const { hidden } = sidebar.use();
  const settings = settingsOpen.use();
  const names = settings ? null : selectedNames(state);
  useEffect(() => {
    launch().catch(console.error);
    window.addEventListener("keydown", runShortcut);
    const unwatch = watchGitCounts();
    return () => {
      window.removeEventListener("keydown", runShortcut);
      unwatch();
    };
  }, []);
  return (
    <div className="app">
      <div className="titlebar" data-tauri-drag-region>
        <span className="titlebar-sidebar" data-hidden={hidden}>
          <SidebarToggle />
        </span>
        {names && (
          <span className="titlebar-title" data-sidebar-hidden={hidden}>
            {names.project}
            <span aria-hidden="true">/</span>
            <span className="titlebar-worktree">{names.worktree}</span>
          </span>
        )}
      </div>
      <Sidebar />
      <main className="main">{settings ? <SettingsPage /> : list.length === 0 ? <NoProjects /> : selected && <ChatPane key={selected} worktree={selected} />}</main>
      <ProjectDrop />
      <NewWorktreePopup />
      <DeleteWorktreePopup />
      <ForceRemovalPopup />
      {error && <WarningToast text={error} notice={errorIsNotice} onDismiss={dismissError} />}
    </div>
  );
}

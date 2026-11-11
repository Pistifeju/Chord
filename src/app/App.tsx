// The window's layout. Each feature adds its own region.
import { useEffect } from "react";
import { Sidebar, SidebarToggle } from "../features/sidebar/Sidebar";
import { sidebar } from "../features/sidebar/store";
import { NoProjects } from "../features/worktrees/NoProjects";
import { DeleteWorktreePopup } from "../features/worktrees/DeleteWorktreePopup";
import { ForceRemovalPopup } from "../features/worktrees/ForceRemovalPopup";
import { NewWorktreePopup } from "../features/worktrees/NewWorktreePopup";
import { ProjectDrop } from "../features/worktrees/ProjectDrop";
import { watchGitCounts } from "../features/worktrees/gitCountsRefresh";
import { dismissError, projects } from "../features/worktrees/store";
import { WarningToast } from "../ui/Toast";
import { runShortcut } from "./commands";
import { launch } from "./launch";
import "./app.css";

export default function App() {
  const { list, error, errorIsNotice } = projects.use();
  const { hidden } = sidebar.use();
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
      </div>
      <Sidebar />
      <main className="main">{list.length === 0 && <NoProjects />}</main>
      <ProjectDrop />
      <NewWorktreePopup />
      <DeleteWorktreePopup />
      <ForceRemovalPopup />
      {error && <WarningToast text={error} notice={errorIsNotice} onDismiss={dismissError} />}
    </div>
  );
}

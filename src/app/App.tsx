// The window's layout. Each feature adds its own region.
import { useEffect } from "react";
import { OpenFileByNamePopup } from "../features/files/OpenFileByNamePopup";
import { TrashFilePopup } from "../features/files/TrashFilePopup";
import { CloseAgentsPopup } from "../features/panes/CloseAgentsPopup";
import { PaneArea } from "../features/panes/PaneArea";
import { QuitPopup } from "../features/quit/QuitPopup";
import { watchQuit } from "../features/quit/store";
import { SettingsPage } from "../features/settings/SettingsPage";
import { settingsOpen } from "../features/settings/store";
import { Sidebar, SidebarToggle } from "../features/sidebar/Sidebar";
import { sidebar } from "../features/sidebar/store";
import { watchFinished, watchNotificationClicks, watchShownPanes } from "../features/status/store";
import { NoProjects } from "../features/worktrees/NoProjects";
import { DeleteWorktreePopup } from "../features/worktrees/DeleteWorktreePopup";
import { ForceRemovalPopup } from "../features/worktrees/ForceRemovalPopup";
import { NewWorktreePopup } from "../features/worktrees/NewWorktreePopup";
import { ProjectDrop } from "../features/worktrees/ProjectDrop";
import { watchGitCounts } from "../features/worktrees/gitCountsRefresh";
import { dismissError, projects } from "../features/worktrees/store";
import { WarningToast } from "../ui/Toast";
import { StatusBar } from "./StatusBar";
import { runShortcut } from "./commands";
import { sheet } from "./sheet";
import { launch } from "./launch";
import "./app.css";

export default function App() {
  const state = projects.use();
  const { list, selected, error, errorIsNotice } = state;
  const { hidden } = sidebar.use();
  const settings = settingsOpen.use();
  const sheetOpen = sheet.use() !== null;
  useEffect(() => {
    launch().catch(console.error);
    window.addEventListener("keydown", runShortcut);
    const unwatch = watchGitCounts();
    const unwatchQuit = watchQuit();
    const unwatchFinished = watchFinished();
    const unwatchClicks = watchNotificationClicks();
    const unwatchShown = watchShownPanes();
    return () => {
      window.removeEventListener("keydown", runShortcut);
      unwatch();
      unwatchQuit();
      unwatchFinished();
      unwatchClicks();
      unwatchShown();
    };
  }, []);
  return (
    <div className="app">
      {/* An open sheet or Settings keeps keyboard focus and clicks inside itself. */}
      <div className="app-content" inert={sheetOpen || settings}>
        <div className="titlebar" data-tauri-drag-region>
          <span className="titlebar-sidebar" data-hidden={hidden}>
            <SidebarToggle />
          </span>
        </div>
        <div className="app-body">
          <Sidebar />
          <main className="main">{list.length === 0 ? <NoProjects /> : selected && <PaneArea key={selected} worktree={selected} />}</main>
        </div>
        <StatusBar />
      </div>
      {settings && <SettingsPage />}
      <ProjectDrop />
      <NewWorktreePopup />
      <DeleteWorktreePopup />
      <ForceRemovalPopup />
      <TrashFilePopup />
      <OpenFileByNamePopup />
      <QuitPopup />
      <CloseAgentsPopup />
      {error && <WarningToast text={error} notice={errorIsNotice} onDismiss={dismissError} />}
    </div>
  );
}

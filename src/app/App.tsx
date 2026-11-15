// The window's layout. Each feature adds its own region.
import { useEffect, useMemo } from "react";
import { BuildView } from "../features/ios/build/BuildView";
import { ConsoleView } from "../features/ios/build/ConsoleView";
import { RunControl } from "../features/ios/build/RunControl";
import { markerText } from "../features/ios/build/run";
import { builds } from "../features/ios/build/store";
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
import { worktreeAt } from "../features/worktrees/projects";
import { dismissError, projects } from "../features/worktrees/store";
import { WarningToast } from "../ui/Toast";
import { StatusBar } from "./StatusBar";
import { runShortcut } from "./commands";
import { sheet } from "./sheet";
import { launch, launched } from "./launch";
import "./app.css";

// A shortcut during launch would open a sheet the window does not show yet.
const shortcutWhenLaunched = (e: KeyboardEvent) => launched.get() && runShortcut(e);

export default function App() {
  const state = projects.use();
  const { list, selected, error, errorIsNotice } = state;
  const { hidden } = sidebar.use();
  const settings = settingsOpen.use();
  const sheetOpen = sheet.use() !== null;
  const ready = launched.use();
  const buildStates = builds.use();
  const xcode = worktreeAt(state, selected)?.xcode === true;
  // The iOS parts of the sidebar, the tab bar, and the bottom panel are slots, so no core feature imports features/ios.
  const markers = useMemo(
    () => Object.fromEntries(Object.entries(buildStates).flatMap(([path, build]) => { const marker = markerText(build); return marker ? [[path, marker]] : []; })),
    [buildStates],
  );
  useEffect(() => {
    launch().catch(console.error);
    window.addEventListener("keydown", shortcutWhenLaunched);
    const unwatch = watchGitCounts();
    const unwatchQuit = watchQuit();
    const unwatchFinished = watchFinished();
    const unwatchClicks = watchNotificationClicks();
    const unwatchShown = watchShownPanes();
    return () => {
      window.removeEventListener("keydown", shortcutWhenLaunched);
      unwatch();
      unwatchQuit();
      unwatchFinished();
      unwatchClicks();
      unwatchShown();
    };
  }, []);
  if (!ready) {
    return (
      <div className="app" data-tauri-drag-region>
        <span className="spinner launch-spinner" role="status" aria-label="Loading" />
      </div>
    );
  }
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
          <Sidebar markers={markers} />
          <div className="app-column">
            <main className="main">{list.length === 0 ? (
                <NoProjects />
              ) : (
                selected && (
                  <PaneArea
                    key={selected}
                    worktree={selected}
                    runControl={xcode ? <RunControl worktree={selected} /> : null}
                    bottomViews={
                      xcode
                        ? [
                            { id: "build", title: "Build", toggleCommand: "build.toggleLogView", body: <BuildView worktree={selected} /> },
                            { id: "console", title: "Console", toggleCommand: "build.toggleLogView", body: <ConsoleView worktree={selected} /> },
                          ]
                        : []
                    }
                  />
                )
              )}
            </main>
            <StatusBar />
          </div>
        </div>
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

// The window's layout. Each feature adds its own region.
import { useEffect, useMemo, useRef } from "react";
import { BuildView } from "../features/ios/build/BuildView";
import { ConsoleView } from "../features/ios/build/ConsoleView";
import { RunControl } from "../features/ios/build/RunControl";
import { BuildDataSize } from "../features/ios/simulators/BuildDataSize";
import { ServiceRows } from "../features/ios/simulators/ServiceRows";
import { SimulatorsButton } from "../features/ios/simulators/SimulatorsButton";
import { SimulatorToggle, SimulatorView } from "../features/ios/simulator/SimulatorView";
import { markerText } from "../features/ios/build/run";
import { builds, watchAgentBuilds } from "../features/ios/build/store";
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
import { RemoveProjectPopup } from "../features/worktrees/RemoveProjectPopup";
import { ForceRemovalPopup } from "../features/worktrees/ForceRemovalPopup";
import { NewWorktreePopup } from "../features/worktrees/NewWorktreePopup";
import { ProjectDrop } from "../features/worktrees/ProjectDrop";
import { watchGitCounts } from "../features/worktrees/gitCountsRefresh";
import { hasXcodeProject, worktreeAt } from "../features/worktrees/projects";
import { dismissError, projects } from "../features/worktrees/store";
import { seeThrough } from "../ui/seeThrough";
import { WarningToast } from "../ui/Toast";
import { StatusBar } from "./StatusBar";
import { runShortcut } from "./commands";
import { sheet } from "./sheet";
import { launch, launched } from "./launch";
import "./app.css";

// A shortcut during launch would open a sheet the window does not show yet.
const shortcutWhenLaunched = (e: KeyboardEvent) => launched.get() && runShortcut(e);

function SeeThroughHole() {
  const hole = seeThrough.use();
  return hole && <div className="see-through" style={{ left: hole.x, top: hole.y, width: hole.width, height: hole.height, borderRadius: hole.radius }} />;
}

export default function App() {
  const state = projects.use();
  const { list, selected, error, errorIsNotice } = state;
  const { hidden } = sidebar.use();
  const settings = settingsOpen.use();
  const sheetOpen = sheet.use() !== null;
  const ready = launched.use();
  const overlay = sheetOpen || settings;
  // A sheet and Settings blur the window, and the blur cannot reach the native view under the hole: the hole closes under them.
  // Only open or closed here: the hole's rect moves on every resize frame and redraws just `SeeThroughHole`.
  const hole = !overlay && seeThrough.usePart((open) => open !== null);
  const lastFocus = useRef<HTMLElement | null>(null);
  // inert drops the focus when a sheet or Settings opens; it goes back once they close, unless their action placed it.
  useEffect(() => {
    const remember = (e: FocusEvent) => {
      if (e.target instanceof HTMLElement && e.target.closest(".app-content")) lastFocus.current = e.target;
    };
    document.addEventListener("focusin", remember);
    return () => document.removeEventListener("focusin", remember);
  }, []);
  useEffect(() => {
    const target = lastFocus.current;
    if (overlay || !target?.isConnected) return;
    if (document.activeElement === null || document.activeElement === document.body) target.focus();
  }, [overlay]);
  const buildStates = builds.use();
  const projectHasXcode = hasXcodeProject(state);
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
    const unwatchAgentBuilds = watchAgentBuilds();
    return () => {
      window.removeEventListener("keydown", shortcutWhenLaunched);
      unwatch();
      unwatchQuit();
      unwatchFinished();
      unwatchClicks();
      unwatchShown();
      unwatchAgentBuilds();
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
    <div className="app" data-see-through={hole ? true : undefined}>
      {hole && <SeeThroughHole />}
      {/* An open sheet or Settings keeps keyboard focus and clicks inside itself. */}
      <div className="app-content" inert={overlay}>
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
                    sideViews={xcode ? [{ id: "simulator", title: "Simulator", button: <SimulatorToggle worktree={selected} />, body: <SimulatorView worktree={selected} /> }] : []}
                  />
                )
              )}
            </main>
            <StatusBar simulators={xcode && selected ? <SimulatorsButton worktree={selected} /> : null} />
          </div>
        </div>
      </div>
      {settings && <SettingsPage buildDataSize={projectHasXcode ? <BuildDataSize /> : null} simulatorServices={projectHasXcode ? <ServiceRows /> : null} />}
      <ProjectDrop />
      <NewWorktreePopup />
      <DeleteWorktreePopup />
      <ForceRemovalPopup />
      <TrashFilePopup />
      <OpenFileByNamePopup />
      <QuitPopup />
      <CloseAgentsPopup />
      <RemoveProjectPopup />
      {error && <WarningToast text={error} notice={errorIsNotice} onDismiss={dismissError} />}
    </div>
  );
}

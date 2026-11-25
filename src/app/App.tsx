import { useEffect, useMemo, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { BuildView } from "../features/ios/build/BuildView";
import { ConsoleView } from "../features/ios/build/ConsoleView";
import { Toolbar } from "../features/ios/build/Toolbar";
import { BuildDataSize } from "../features/ios/simulators/BuildDataSize";
import { ScreenshotRows } from "../features/ios/simulator/ScreenshotRows";
import { ServiceRows } from "../features/ios/simulators/ServiceRows";
import { SimulatorsButton } from "../features/ios/simulators/SimulatorsButton";
import { SimulatorToggle, SimulatorView } from "../features/ios/simulator/SimulatorView";
import { PreviewToggle, PreviewView } from "../features/ios/preview/PreviewView";
import { markerText } from "../features/ios/build/run";
import { builds, watchAgentBuilds } from "../features/ios/build/store";
import { useMacOnly } from "../features/ios/build/schemeStore";
import { watchBrowsers } from "../features/browser/store";
import { OpenFileByNamePopup } from "../features/files/OpenFileByNamePopup";
import { TrashFilePopup } from "../features/files/TrashFilePopup";
import { DiscardChangesPopup } from "../features/changes/DiscardChangesPopup";
import { DropStashPopup } from "../features/changes/DropStashPopup";
import { CloseAgentsPopup } from "../features/panes/CloseAgentsPopup";
import { PaneArea } from "../features/panes/PaneArea";
import { QuitPopup } from "../features/quit/QuitPopup";
import { watchQuit } from "../features/quit/store";
import { UpdatePopup } from "../features/update/UpdatePopup";
import { watchUpdates } from "../features/update/store";
import { HelpSheet } from "../features/help/HelpSheet";
import { NoProjects } from "../features/worktrees/NoProjects";
import { Onboarding } from "../features/onboarding/Onboarding";
import { SettingsPage } from "../features/settings/SettingsPage";
import { settingsOpen } from "../features/settings/store";
import { UsagePanel } from "../features/usage/UsagePanel";
import { usagePanelOpen } from "../features/usage/store";
import { Sidebar, SidebarToggle } from "../features/sidebar/Sidebar";
import { sidebar } from "../features/sidebar/store";
import { watchFinished, watchNotificationClicks, watchShownPanes } from "../features/status/store";
import { DeleteWorktreePopup } from "../features/worktrees/DeleteWorktreePopup";
import { RemoveProjectPopup } from "../features/worktrees/RemoveProjectPopup";
import { ForceRemovalPopup } from "../features/worktrees/ForceRemovalPopup";
import { DeleteBranchPopup } from "../features/worktrees/DeleteBranchPopup";
import { RenameBranchPopup } from "../features/worktrees/RenameBranchPopup";
import { NewBranchPopup } from "../features/worktrees/NewBranchPopup";
import { NewWorktreePopup } from "../features/worktrees/NewWorktreePopup";
import { ProjectDrop } from "../features/worktrees/ProjectDrop";
import { watchGitCounts } from "../features/worktrees/gitCountsRefresh";
import { hasXcodeProject, ownerOf, worktreeAt } from "../features/worktrees/projects";
import { projects, welcomed } from "../features/worktrees/store";
import { alert, dismissAlert } from "../ui/alert";
import { seeThrough } from "../ui/seeThrough";
import { WarningToast } from "../ui/Toast";
import { StatusBar } from "./StatusBar";
import { runShortcut } from "./commands";
import { sheet } from "./sheet";
import { launch, launched } from "./launch";
import "./app.css";

const shortcutWhenLaunched = (e: KeyboardEvent) => launched.get() && runShortcut(e);

function SeeThroughHole() {
  const hole = seeThrough.use();
  return hole && <div className="see-through" style={{ left: hole.x, top: hole.y, width: hole.width, height: hole.height, borderRadius: hole.radius }} />;
}

export default function App() {
  const state = projects.use();
  const { list, selected } = state;
  const empty = list.length === 0;
  const welcome = welcomed.use();
  const welcoming = empty && !welcome;
  const shownAlert = alert.use();
  const { hidden } = sidebar.use();
  const settings = settingsOpen.use();
  const usagePanel = usagePanelOpen.use();
  const sheetOpen = sheet.use() !== null;
  const ready = launched.use();
  const overlay = sheetOpen || settings || usagePanel;
  const holeOpen = seeThrough.usePart((open) => open !== null);
  const hole = !overlay && holeOpen;
  const [fullscreen, setFullscreen] = useState(false);
  useEffect(() => {
    const win = getCurrentWindow();
    const check = () => void win.isFullscreen().then(setFullscreen);
    check();
    const unlisten = win.onResized(check);
    return () => void unlisten.then((stop) => stop());
  }, []);
  const lastFocus = useRef<HTMLElement | null>(null);
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
  const macOnly = useMacOnly(selected);
  const simulator = xcode && !macOnly;
  const owner = selected === null ? undefined : ownerOf(state, selected);
  const project = list.find((entry) => entry.path === owner);
  const markers = useMemo(
    () => Object.fromEntries(Object.entries(buildStates).flatMap(([path, build]) => { const marker = markerText(build); return marker ? [[path, marker]] : []; })),
    [buildStates],
  );
  useEffect(() => {
    launch().catch(console.error);
    window.addEventListener("keydown", shortcutWhenLaunched);
    const unwatch = watchGitCounts();
    const unwatchQuit = watchQuit();
    const unwatchUpdates = watchUpdates();
    const unwatchFinished = watchFinished();
    const unwatchClicks = watchNotificationClicks();
    const unwatchShown = watchShownPanes();
    const unwatchAgentBuilds = watchAgentBuilds();
    const unwatchBrowsers = watchBrowsers();
    return () => {
      unwatchBrowsers();
      window.removeEventListener("keydown", shortcutWhenLaunched);
      unwatch();
      unwatchQuit();
      unwatchUpdates();
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
    <div className="app" data-see-through={hole ? true : undefined} data-fullscreen={fullscreen || undefined}>
      {hole && <SeeThroughHole />}
      <div className="app-content" inert={overlay}>
        <div className="titlebar" data-tauri-drag-region>
          {!empty && !hidden && project && (
            <span className="titlebar-project" title={project.path} data-tauri-drag-region>
              {project.name}
            </span>
          )}
          {!empty && (
            <span className="titlebar-sidebar" data-hidden={hidden}>
              <SidebarToggle />
            </span>
          )}
        </div>
        <div className="app-body">
          {!empty && <Sidebar markers={markers} />}
          <div className="app-column">
            <main className="main">{welcoming ? (
                <Onboarding />
              ) : empty ? (
                <NoProjects />
              ) : (
                selected && (
                  <PaneArea
                    key={selected}
                    worktree={selected}
                    toolbar={
                      xcode ? (
                        <Toolbar
                          worktree={selected}
                          viewButtons={
                            simulator && (
                              <>
                                <SimulatorToggle worktree={selected} />
                                <PreviewToggle worktree={selected} />
                              </>
                            )
                          }
                        />
                      ) : null
                    }
                    bottomViews={
                      xcode
                        ? [
                            { id: "build", title: "Build", toggleCommand: "build.toggleLogView", body: <BuildView worktree={selected} /> },
                            { id: "console", title: "Console", toggleCommand: "build.toggleLogView", body: <ConsoleView worktree={selected} /> },
                          ]
                        : []
                    }
                    sideViews={
                      simulator
                        ? [
                            { id: "simulator", title: "Simulator", body: <SimulatorView worktree={selected} /> },
                            { id: "preview", title: "Preview", body: <PreviewView worktree={selected} /> },
                          ]
                        : []
                    }
                  />
                )
              )}
            </main>
            {!empty && <StatusBar simulators={xcode && selected ? <SimulatorsButton worktree={selected} /> : null} />}
          </div>
        </div>
      </div>
      {settings && <SettingsPage buildDataSize={projectHasXcode ? <BuildDataSize /> : null} simulatorScreenshots={projectHasXcode ? <ScreenshotRows /> : null} simulatorServices={projectHasXcode ? <ServiceRows /> : null} />}
      {usagePanel && <UsagePanel />}
      <ProjectDrop />
      <NewWorktreePopup />
      <DeleteWorktreePopup />
      <ForceRemovalPopup />
      <RenameBranchPopup />
      <NewBranchPopup />
      <DeleteBranchPopup />
      <TrashFilePopup />
      <DiscardChangesPopup />
      <DropStashPopup />
      <OpenFileByNamePopup />
      <QuitPopup />
      <UpdatePopup />
      <HelpSheet />
      <CloseAgentsPopup />
      <RemoveProjectPopup />
      {shownAlert && <WarningToast text={shownAlert.text} notice={shownAlert.notice} onDismiss={dismissAlert} />}
    </div>
  );
}

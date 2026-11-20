// Launch: loads the saved state from Rust into the feature stores, and holds the window back until all of it, the git counts, the resumed chats' history and the deferred views have arrived.
import { commands } from "../ipc/bindings";
import { loadShortcutChanges } from "./commands";
import { installAppMenu } from "./appMenu";
import { loadDeferredViews } from "./deferredViews";
import { keepAwake, linksInDefaultBrowser, loadAppearance, muteNotifications } from "../features/settings/store";
import { loadPanes, resumeAgents } from "../features/panes/store";
import { loadModelCatalogs, loadPaneHarnesses, refreshModelCatalogs } from "../features/chat/store";
import { checkHarnesses } from "../features/harnesses/store";
import { sidebar } from "../features/sidebar/store";
import { loadSidePanels } from "../features/sidepanel/store";
import { screenshotFolder } from "../features/ios/simulator/store";
import { loadRunDestinations, refreshDestinations } from "../features/ios/build/destinationStore";
import { loadSchemePicks } from "../features/ios/build/schemeStore";
import { loadPreviewChoices } from "../features/ios/preview/store";
import { loadBottomPanels } from "../features/bottompanel/store";
import { pushAfterCommit } from "../features/changes/store";
import { loadGitCounts } from "../features/worktrees/gitCountsRefresh";
import { hasXcodeProject, worktreeAt } from "../features/worktrees/projects";
import { loadProjects, projects, welcomed } from "../features/worktrees/store";
import { externalStore } from "../ui/externalStore";

export const launched = externalStore(false);

const HISTORY_WAIT_MS = 3000;
const historyWait = () => new Promise<void>((resolve) => setTimeout(resolve, HISTORY_WAIT_MS));

export async function launch() {
  try {
    installAppMenu().catch(console.error);
    const views = loadDeferredViews();
    const state = await commands.loadState();
    loadAppearance(state.appearance);
    keepAwake.set(state.keepAwake);
    muteNotifications.set(state.muteNotifications);
    linksInDefaultBrowser.set(state.linksInDefaultBrowser);
    pushAfterCommit.set(state.pushAfterCommit);
    screenshotFolder.set(state.screenshotFolder);
    loadShortcutChanges(state.shortcuts);
    sidebar.set(state.sidebar);
    loadPanes(state.panes);
    loadPaneHarnesses(state.agents);
    loadModelCatalogs(state.models);
    refreshModelCatalogs();
    loadSidePanels(state.sidePanels);
    loadBottomPanels(state.bottomPanels);
    loadRunDestinations(state.runDestinations);
    loadSchemePicks(state.schemePicks);
    loadPreviewChoices(state.previewPicks, state.previewModels);
    welcomed.set(state.welcomed);
    // A pi that never sends its history must not keep the window hidden; its chat fills in once it does.
    const resumed = Promise.race([resumeAgents(state.runningPanes), historyWait()]);
    await loadProjects(state.projects, state.selected);
    // Build data of worktrees deleted elsewhere goes in the background.
    if (hasXcodeProject(projects.get())) commands.pruneBuildData().catch(console.error);
    // Settings › Agents opens with rows instead of a Checking row.
    void checkHarnesses();
    // Listing takes a second or two and only checks the saved pick: the window does not wait for it.
    if (state.selected && worktreeAt(projects.get(), state.selected)?.xcode) refreshDestinations(state.selected).catch(console.error);
    await Promise.all([loadGitCounts(), resumed, views]);
  } finally {
    launched.set(true);
  }
}

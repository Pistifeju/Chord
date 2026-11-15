// Launch: loads the saved state from Rust into the feature stores, and holds the window back until all of it, the git counts, the resumed chats' history and the deferred views have arrived.
import { commands } from "../ipc/bindings";
import { loadShortcutChanges } from "./commands";
import { installAppMenu } from "./appMenu";
import { loadDeferredViews } from "./deferredViews";
import { keepAwake, loadAppearance } from "../features/settings/store";
import { loadPanes, resumeAgents } from "../features/panes/store";
import { sidebar } from "../features/sidebar/store";
import { loadSidePanels } from "../features/sidepanel/store";
import { loadRunDestinations, refreshDestinations } from "../features/ios/build/destinationStore";
import { loadBottomPanels } from "../features/bottompanel/store";
import { loadGitCounts } from "../features/worktrees/gitCountsRefresh";
import { worktreeAt } from "../features/worktrees/projects";
import { loadProjects, projects } from "../features/worktrees/store";
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
    loadShortcutChanges(state.shortcuts);
    sidebar.set(state.sidebar);
    loadPanes(state.panes);
    loadSidePanels(state.sidePanels);
    loadBottomPanels(state.bottomPanels);
    loadRunDestinations(state.runDestinations);
    // A pi that never sends its history must not keep the window hidden; its chat fills in once it does.
    const resumed = Promise.race([resumeAgents(state.runningPanes), historyWait()]);
    await loadProjects(state.projects, state.selected);
    // Listing takes a second or two and only checks the saved pick: the window does not wait for it.
    if (state.selected && worktreeAt(projects.get(), state.selected)?.xcode) refreshDestinations(state.selected).catch(console.error);
    await Promise.all([loadGitCounts(), resumed, views]);
  } finally {
    launched.set(true);
  }
}

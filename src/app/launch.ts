// Launch: loads the saved state from Rust into the feature stores, and holds the window back until all of it, the git counts, the resumed chats' history and the deferred views have arrived.
import { commands } from "../ipc/bindings";
import { loadShortcutChanges } from "./commands";
import { installAppMenu } from "./appMenu";
import { loadDeferredViews } from "./deferredViews";
import { keepAwake, loadAppearance } from "../features/settings/store";
import { loadPanes, resumeAgents } from "../features/panes/store";
import { sidebar } from "../features/sidebar/store";
import { worktreeOrder } from "../features/worktrees/projects";
import { loadSidePanels } from "../features/sidepanel/store";
import { loadBottomPanels } from "../features/bottompanel/store";
import { loadGitCounts } from "../features/worktrees/gitCountsRefresh";
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
    await loadProjects(state.projects, state.selected);
    // A pi that never sends its history must not keep the window hidden; its chat fills in once it does.
    const resumed = Promise.race([resumeAgents(state.runningPanes, worktreeOrder(projects.get())), historyWait()]);
    await Promise.all([loadGitCounts(), resumed, views]);
  } finally {
    launched.set(true);
  }
}

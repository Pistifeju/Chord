// Launch: loads the saved state from Rust into the feature stores.
import { commands } from "../ipc/bindings";
import { loadShortcutChanges } from "./commands";
import { installAppMenu } from "./appMenu";
import { keepAwake, loadAppearance } from "../features/settings/store";
import { loadPanes, resumeAgents } from "../features/panes/store";
import { sidebar } from "../features/sidebar/store";
import { loadProjects, projects } from "../features/worktrees/store";

export async function launch() {
  installAppMenu().catch(console.error);
  const state = await commands.loadState();
  loadAppearance(state.appearance);
  keepAwake.set(state.keepAwake);
  loadShortcutChanges(state.shortcuts);
  sidebar.set(state.sidebar);
  loadPanes(state.panes);
  await loadProjects(state.projects, state.selected);
  const { worktrees } = projects.get();
  resumeAgents(state.runningPanes, new Set(Object.values(worktrees).flatMap((list) => list.map((w) => w.path))));
}

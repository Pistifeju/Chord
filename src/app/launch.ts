// Launch: loads the saved state from Rust into the feature stores.
import { commands } from "../ipc/bindings";
import { sidebar } from "../features/sidebar/store";
import { loadProjects } from "../features/worktrees/store";

export async function launch() {
  const state = await commands.loadState();
  sidebar.set(state.sidebar);
  loadProjects(state.projects, state.selected);
}

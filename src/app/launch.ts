// Launch: loads the saved state from Rust into the feature stores.
import { commands } from "../ipc/bindings";
import { loadShortcutChanges } from "./commands";
import { installAppMenu } from "./appMenu";
import { loadAppearance } from "../features/settings/store";
import { sidebar } from "../features/sidebar/store";
import { loadProjects } from "../features/worktrees/store";

export async function launch() {
  installAppMenu().catch(console.error);
  const state = await commands.loadState();
  loadAppearance(state.appearance);
  loadShortcutChanges(state.shortcuts);
  sidebar.set(state.sidebar);
  loadProjects(state.projects, state.selected);
}

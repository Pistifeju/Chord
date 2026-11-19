// The sidebar's shown state, saved through Rust, and the navigator view it shows: Workspaces, Files, Changes or Tests.
import { commands, type Sidebar_Serialize as Sidebar } from "../../ipc/bindings";
import { externalStore } from "../../ui/externalStore";

export const sidebar = externalStore<Sidebar>({ hidden: false });

export function toggleSidebar() {
  const hidden = !sidebar.get().hidden;
  sidebar.set({ ...sidebar.get(), hidden });
  commands.changeState({ sidebarHidden: hidden }).catch(console.error);
}

export type NavigatorView = "workspaces" | "files" | "changes" | "tests";

// shortcut: lasts until the app quits; save it through Rust if the developer wants it back after a relaunch.
export const navigatorView = externalStore<NavigatorView>("workspaces");

/** Shows `view`, and the sidebar with it. */
export function showNavigator(view: NavigatorView) {
  navigatorView.set(view);
  if (sidebar.get().hidden) toggleSidebar();
}

/** ⌘⇧E and ⌘⇧G: back to Workspaces while the sidebar shows `view`, else shows it. */
export const toggleNavigator = (view: NavigatorView) => (!sidebar.get().hidden && navigatorView.get() === view ? navigatorView.set("workspaces") : showNavigator(view));

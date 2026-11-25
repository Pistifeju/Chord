import { commands, type Sidebar_Serialize as Sidebar } from "../../ipc/bindings";
import { externalStore } from "../../ui/externalStore";

export const sidebar = externalStore<Sidebar>({ hidden: false });

export function toggleSidebar() {
  const hidden = !sidebar.get().hidden;
  sidebar.set({ ...sidebar.get(), hidden });
  commands.changeState({ sidebarHidden: hidden }).catch(console.error);
}

export type NavigatorView = "workspaces" | "files" | "changes";

export const navigatorView = externalStore<NavigatorView>("workspaces");

export function showNavigator(view: NavigatorView) {
  navigatorView.set(view);
  if (sidebar.get().hidden) toggleSidebar();
}

export const toggleNavigator = (view: NavigatorView) => (!sidebar.get().hidden && navigatorView.get() === view ? navigatorView.set("workspaces") : showNavigator(view));

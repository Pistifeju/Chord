// The sidebar's shown state, saved through Rust.
import { commands, type Sidebar_Serialize as Sidebar } from "../../ipc/bindings";
import { externalStore } from "../../ui/externalStore";

export const sidebar = externalStore<Sidebar>({ hidden: false });

export function toggleSidebar() {
  const hidden = !sidebar.get().hidden;
  sidebar.set({ ...sidebar.get(), hidden });
  commands.changeState({ sidebarHidden: hidden }).catch(console.error);
}


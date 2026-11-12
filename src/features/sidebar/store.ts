// The sidebar's shown state and folded projects, saved through Rust.
import { commands, type Sidebar_Serialize as Sidebar } from "../../ipc/bindings";
import { externalStore } from "../../ui/externalStore";

export const sidebar = externalStore<Sidebar>({ hidden: false, folded: [] });

export function toggleSidebar() {
  const hidden = !sidebar.get().hidden;
  sidebar.set({ ...sidebar.get(), hidden });
  commands.changeState({ sidebarHidden: hidden }).catch(console.error);
}

export function toggleFold(project: string) {
  const fold = !sidebar.get().folded.includes(project);
  commands
    .changeState({ projectFolded: { project, folded: fold } })
    .then((state) => sidebar.set({ ...sidebar.get(), folded: state.sidebar.folded }))
    .catch(console.error);
}

// The side panel's open state, width and shown view per worktree, saved through Rust.
import { commands, type SidePanel_Serialize as SidePanel, type SideView } from "../../ipc/bindings";
import { panelStore } from "../../ui/panel";

const store = panelStore<"width", SidePanel>({
  fallback: { open: false, width: 300, view: "files" },
  size: "width",
  min: 200,
  max: 640,
  save: (worktree, panel) => commands.changeState({ sidePanel: { worktree, panel } }),
});

export const { panels: sidePanels, panelOf: sidePanelOf, load: loadSidePanels, resize: resizeSidePanel, endResize: endSidePanelResize } = store;

/** Whether the panel shows `view` now. */
export const showsSideView = (all: Record<string, SidePanel>, worktree: string, view: SideView) => {
  const panel = sidePanelOf(all, worktree);
  return panel.open && panel.view === view;
};

/** Shows `view` the next time the panel opens, without opening it. */
export const keepSideView = (worktree: string, view: SideView) => store.change(worktree, { view });

export const openSideView = (worktree: string, view: SideView) => store.change(worktree, { open: true, view });

/** Hides the panel while it shows `view`, else opens it on `view`. */
export const toggleSideView = (worktree: string, view: SideView) =>
  showsSideView(sidePanels.get(), worktree, view) ? store.show(worktree, false) : openSideView(worktree, view);

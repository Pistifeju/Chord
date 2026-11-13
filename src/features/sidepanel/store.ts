// The side panel's open state and width per worktree, saved through Rust.
import { commands } from "../../ipc/bindings";
import { panelStore } from "../../ui/panel";

const store = panelStore({
  fallback: { open: false, width: 300 },
  size: "width",
  min: 200,
  max: 640,
  save: (worktree, panel) => commands.changeState({ sidePanel: { worktree, panel } }),
});

export const { panels: sidePanels, panelOf: sidePanelOf, load: loadSidePanels, resize: resizeSidePanel, endResize: endSidePanelResize } = store;

export const toggleSidePanel = (worktree: string) => store.show(worktree, !sidePanelOf(sidePanels.get(), worktree).open);

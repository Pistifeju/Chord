// The bottom panel's open state and height per worktree, saved through Rust.
import { commands } from "../../ipc/bindings";
import { panelStore } from "../../ui/panel";

export const {
  panels: bottomPanels,
  panelOf: bottomPanelOf,
  load: loadBottomPanels,
  show: showBottomPanel,
  resize: resizeBottomPanel,
  endResize: endBottomPanelResize,
} = panelStore({
  fallback: { open: false, height: 240 },
  size: "height",
  min: 120,
  max: 800,
  save: (worktree, panel) => commands.changeState({ bottomPanel: { worktree, panel } }),
});

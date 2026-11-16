// The bottom panel's open state, height and shown view per worktree, saved through Rust.
import { commands, type BottomPanel_Serialize as BottomPanel } from "../../ipc/bindings";
import { panelStore } from "../../ui/panel";

export const {
  panels: bottomPanels,
  panelOf: bottomPanelOf,
  load: loadBottomPanels,
  change: changeBottomPanel,
  show: showBottomPanel,
  resize: resizeBottomPanel,
  endResize: endBottomPanelResize,
} = panelStore<"height", BottomPanel>({
  fallback: { open: false, height: 240, view: "terminal", sideWidth: 480 },
  size: "height",
  min: 120,
  max: 800,
  save: (worktree, panel) => commands.changeState({ bottomPanel: { worktree, panel } }),
});

/** A drag of the view column's left edge sets its width; endBottomPanelResize saves it. */
export const resizeBottomSide = (worktree: string, width: number) =>
  bottomPanels.set({ ...bottomPanels.get(), [worktree]: { ...bottomPanelOf(bottomPanels.get(), worktree), sideWidth: Math.round(Math.max(200, width)) } });

/** A drag of the top edge: below 60px it hides the panel with every view in it, else it sets the height. */
export const dragBottomPanel = (worktree: string, size: number) =>
  size < 60 ? changeBottomPanel(worktree, { open: false, view: "terminal" }) : resizeBottomPanel(worktree, size);

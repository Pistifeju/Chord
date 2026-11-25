import { commands, type BottomPanel_Serialize as BottomPanel } from "../../ipc/bindings";
import { panelStore } from "../../ui/panel";

export const bottomPanelMin = 90;

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
  min: bottomPanelMin,
  max: 800,
  save: (worktree, panel) => commands.changeState({ bottomPanel: { worktree, panel } }),
});

export const resizeBottomSide = (worktree: string, width: number) =>
  bottomPanels.set({ ...bottomPanels.get(), [worktree]: { ...bottomPanelOf(bottomPanels.get(), worktree), sideWidth: Math.round(Math.max(200, width)) } });

export const hideBottomPanel = (worktree: string) => changeBottomPanel(worktree, { open: false, view: "terminal" });

import { commands, type SidePanel_Serialize as SidePanel, type SideView } from "../../ipc/bindings";
import { panelStore } from "../../ui/panel";

const store = panelStore<"width", SidePanel>({
  fallback: { open: false, width: 300, view: "simulator" },
  size: "width",
  min: 200,
  max: 640,
  save: (worktree, panel) => commands.changeState({ sidePanel: { worktree, panel } }),
});

export const { panels: sidePanels, panelOf: sidePanelOf, load: loadSidePanels, resize: resizeSidePanel, endResize: endSidePanelResize } = store;

export const showsSideView = (all: Record<string, SidePanel>, worktree: string, view: SideView) => {
  const panel = sidePanelOf(all, worktree);
  return panel.open && panel.view === view;
};

export const keepSideView = (worktree: string, view: SideView) => store.change(worktree, { view });

export const leaveSidebarView = (worktree: string, view: SideView) => store.change(worktree, { open: false, view });

const minWidths: Partial<Record<SideView, number>> = { preview: 560 };

const widthsBefore = new Map<string, number>();

export function openSideView(worktree: string, view: SideView) {
  const min = minWidths[view] ?? 0;
  let { width } = sidePanelOf(sidePanels.get(), worktree);
  const before = widthsBefore.get(worktree);
  if (width < min) {
    widthsBefore.set(worktree, width);
    width = min;
  } else if (!min && before !== undefined) {
    widthsBefore.delete(worktree);
    width = before;
  }
  store.change(worktree, { open: true, view, width });
}

export const toggleSideView = (worktree: string, view: SideView) =>
  showsSideView(sidePanels.get(), worktree, view) ? store.show(worktree, false) : openSideView(worktree, view);

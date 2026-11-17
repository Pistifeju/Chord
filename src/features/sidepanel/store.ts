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

// The Preview view's list sits beside its screen and needs the room.
const minWidths: Partial<Record<SideView, number>> = { preview: 560 };

/** The width a view with a minimum widened the panel from, which the next view without one gets back. */
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

/** Hides the panel while it shows `view`, else opens it on `view`. */
export const toggleSideView = (worktree: string, view: SideView) =>
  showsSideView(sidePanels.get(), worktree, view) ? store.show(worktree, false) : openSideView(worktree, view);

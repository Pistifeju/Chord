// The side panel's open state and width per worktree, saved through Rust.
import { commands, type SidePanel_Serialize as SidePanel } from "../../ipc/bindings";
import { externalStore } from "../../ui/externalStore";

export const MIN_WIDTH = 200;
export const MAX_WIDTH = 640;
const DEFAULT: SidePanel = { open: false, width: 300 };

export const sidePanels = externalStore<Record<string, SidePanel>>({});

export const loadSidePanels = (saved: Record<string, SidePanel>) => sidePanels.set(saved);

export const sidePanelOf = (all: Record<string, SidePanel>, worktree: string): SidePanel => all[worktree] ?? DEFAULT;

const set = (worktree: string, panel: SidePanel) => sidePanels.set({ ...sidePanels.get(), [worktree]: panel });

const save = (worktree: string) =>
  commands.changeState({ sidePanel: { worktree, panel: sidePanelOf(sidePanels.get(), worktree) } }).catch(console.error);

export function toggleSidePanel(worktree: string) {
  const current = sidePanelOf(sidePanels.get(), worktree);
  set(worktree, { ...current, open: !current.open });
  save(worktree);
}

// A drag changes the width many times and saves once, when it ends.
export const resizeSidePanel = (worktree: string, width: number) =>
  set(worktree, { ...sidePanelOf(sidePanels.get(), worktree), width: Math.round(Math.min(MAX_WIDTH, Math.max(MIN_WIDTH, width))) });

export const endSidePanelResize = save;

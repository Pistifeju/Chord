// The bottom panel's open state and height per worktree, saved through Rust.
import { commands, type BottomPanel_Serialize as BottomPanel } from "../../ipc/bindings";
import { externalStore } from "../../ui/externalStore";

const MIN_HEIGHT = 120;
const MAX_HEIGHT = 800;
const DEFAULT: BottomPanel = { open: false, height: 240 };

export const bottomPanels = externalStore<Record<string, BottomPanel>>({});

export const loadBottomPanels = (saved: Record<string, BottomPanel>) => bottomPanels.set(saved);

export const bottomPanelOf = (all: Record<string, BottomPanel>, worktree: string): BottomPanel => all[worktree] ?? DEFAULT;

const set = (worktree: string, panel: BottomPanel) => bottomPanels.set({ ...bottomPanels.get(), [worktree]: panel });

const save = (worktree: string) =>
  commands.changeState({ bottomPanel: { worktree, panel: bottomPanelOf(bottomPanels.get(), worktree) } }).catch(console.error);

export function showBottomPanel(worktree: string, open: boolean) {
  const current = bottomPanelOf(bottomPanels.get(), worktree);
  if (current.open === open) return;
  set(worktree, { ...current, open });
  save(worktree);
}

// A drag changes the height many times and saves once, when it ends.
export const resizeBottomPanel = (worktree: string, height: number) =>
  set(worktree, { ...bottomPanelOf(bottomPanels.get(), worktree), height: Math.round(Math.min(MAX_HEIGHT, Math.max(MIN_HEIGHT, height))) });

export const endBottomPanelResize = save;

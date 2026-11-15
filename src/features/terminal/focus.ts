// The worktree whose terminal takes the focus once it shows, and ⌘T that sets it, apart from the xterm store so the commands skip loading xterm.
import { bottomPanelOf, bottomPanels, changeBottomPanel } from "../bottompanel/store";

let focusOnShow: string | null = null;

export function toggleTerminal(worktree: string) {
  const panel = bottomPanelOf(bottomPanels.get(), worktree);
  focusOnShow = panel.open ? null : worktree;
  changeBottomPanel(worktree, { open: !panel.open });
}

export function takesFocusOnShow(worktree: string): boolean {
  if (focusOnShow !== worktree) return false;
  focusOnShow = null;
  return true;
}

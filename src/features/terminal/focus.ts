// The worktree whose terminal takes the focus once it shows, and ⌘T that sets it, apart from the xterm store so the commands skip loading xterm.
import { bottomPanelOf, bottomPanels, showBottomPanel } from "../bottompanel/store";

let focusOnShow: string | null = null;

export function toggleTerminal(worktree: string) {
  const open = !bottomPanelOf(bottomPanels.get(), worktree).open;
  focusOnShow = open ? worktree : null;
  showBottomPanel(worktree, open);
}

export function takesFocusOnShow(worktree: string): boolean {
  if (focusOnShow !== worktree) return false;
  focusOnShow = null;
  return true;
}

// The tab and split commands for the commands table.
import type { AppCommand } from "../../app/commands";
import type { Direction } from "../../ipc/bindings";
import { chatOf, chats } from "../chat/store";
import { openWorktree, shownPanes } from "../worktrees/store";
import { filePanes } from "./layout";
import { canClosePane } from "./sessions";
import { closeFocused, newSession, panes, splitFocused } from "./store";

// A split goes beside a focused file tab, or a chat whose agent started.
const canSplit = (): boolean => {
  const shown = shownPanes();
  if (!shown) return false;
  if (filePanes(shown.layout).some((file) => file.id === shown.focused)) return true;
  return chatOf(chats.get(), shown.focused).agent !== "idle";
};

const split = (direction: Direction) => onWorktree((worktree) => splitFocused(worktree, direction));

const onWorktree = (act: (worktree: string) => void) => () => {
  const worktree = openWorktree();
  if (worktree) act(worktree);
};

export const paneCommands: AppCommand[] = [
  { id: "pane.splitRight", title: "Split Right", shortcut: { key: "d", cmd: true }, when: canSplit, run: split("row") },
  { id: "pane.splitDown", title: "Split Down", shortcut: { key: "d", cmd: true, shift: true }, when: canSplit, run: split("column") },
  {
    id: "pane.close",
    title: "Close Tab",
    shortcut: { key: "w", cmd: true, shift: true },
    when: () => {
      const worktree = openWorktree();
      return worktree !== null && canClosePane(panes.get()[worktree]);
    },
    run: onWorktree(closeFocused),
  },
  { id: "session.new", title: "New Session", shortcut: { key: "t", cmd: true, shift: true }, when: () => openWorktree() !== null, run: onWorktree(newSession) },
];

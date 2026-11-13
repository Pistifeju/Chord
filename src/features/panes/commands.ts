// The pane and session-tab commands for the commands table.
import type { AppCommand } from "../../app/commands";
import type { Direction } from "../../ipc/bindings";
import { chatOf, chats } from "../chat/store";
import { openWorktree, shownPanes } from "../worktrees/store";
import { splitFile } from "../editor/showing";
import { openFiles } from "../editor/store";
import { filePanes } from "./layout";
import { canClosePane } from "./sessions";
import { closeFocused, newSession, panes, splitFocused } from "./store";

const shownFile = (): string | null => {
  const worktree = openWorktree();
  return (worktree && openFiles.get()[worktree]?.active) ?? null;
};

// A split goes beside a shown file tab, a focused file pane, or a chat whose agent started.
const canSplit = (): boolean => {
  const shown = shownPanes();
  if (!shown) return false;
  if (shownFile() || filePanes(shown.layout).some((file) => file.id === shown.focused)) return true;
  return chatOf(chats.get(), shown.focused).agent !== "idle";
};

const split = (direction: Direction) =>
  onWorktree((worktree) => {
    const file = shownFile();
    if (file) splitFile(worktree, file, direction);
    else splitFocused(worktree, direction);
  });

const onWorktree = (act: (worktree: string) => void) => () => {
  const worktree = openWorktree();
  if (worktree) act(worktree);
};

export const paneCommands: AppCommand[] = [
  { id: "pane.splitRight", title: "Split Right", shortcut: { key: "d", cmd: true }, when: canSplit, run: split("row") },
  { id: "pane.splitDown", title: "Split Down", shortcut: { key: "d", cmd: true, shift: true }, when: canSplit, run: split("column") },
  {
    id: "pane.close",
    title: "Close Pane",
    shortcut: { key: "w", cmd: true, shift: true },
    when: () => {
      const worktree = openWorktree();
      return worktree !== null && canClosePane(panes.get()[worktree]);
    },
    run: onWorktree(closeFocused),
  },
  { id: "session.new", title: "New Session", shortcut: { key: "t", cmd: true, shift: true }, when: () => openWorktree() !== null, run: onWorktree(newSession) },
];

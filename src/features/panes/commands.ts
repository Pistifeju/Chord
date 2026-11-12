// The pane and session-tab commands for the commands table.
import type { AppCommand } from "../../app/commands";
import { settingsOpen } from "../settings/store";
import { chatOf, chats } from "../chat/store";
import { projects } from "../worktrees/store";
import { activeSession, canClosePane } from "./sessions";
import { closeFocused, newSession, panes, splitFocused } from "./store";

const openWorktree = (): string | null => (settingsOpen.get() ? null : projects.get().selected);

const hasChat = (): boolean => {
  const worktree = openWorktree();
  const current = worktree && panes.get()[worktree];
  return !!current && chatOf(chats.get(), activeSession(current).focused).agent !== "idle";
};

const onWorktree = (act: (worktree: string) => void) => () => {
  const worktree = openWorktree();
  if (worktree) act(worktree);
};

export const paneCommands: AppCommand[] = [
  { id: "pane.splitRight", title: "Split Right", shortcut: { key: "d", cmd: true }, when: hasChat, run: onWorktree((w) => splitFocused(w, "row")) },
  { id: "pane.splitDown", title: "Split Down", shortcut: { key: "d", cmd: true, shift: true }, when: hasChat, run: onWorktree((w) => splitFocused(w, "column")) },
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

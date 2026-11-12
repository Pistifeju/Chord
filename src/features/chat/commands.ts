// The chat's commands for the commands table.
import type { AppCommand } from "../../app/commands";
import { activeSession } from "../panes/sessions";
import { panes } from "../panes/store";
import { settingsOpen } from "../settings/store";
import { projects } from "../worktrees/store";
import { closeFind, finds, openFind } from "./find";
import { abortRun, chatOf, chats } from "./store";

const focusedPane = (): string | null => {
  const { selected } = projects.get();
  const open = selected ? panes.get()[selected] : undefined;
  const pane = open && activeSession(open).focused;
  return pane && !settingsOpen.get() ? pane : null;
};

const runningPane = (): string | null => {
  const pane = focusedPane();
  return pane && chatOf(chats.get(), pane).running ? pane : null;
};

const findingPane = (): string | null => {
  const pane = focusedPane();
  return pane && finds.get()[pane] ? pane : null;
};

export const chatCommands: AppCommand[] = [
  {
    id: "chat.find",
    title: "Find in Chat",
    shortcut: { key: "f", cmd: true },
    when: () => focusedPane() !== null,
    run: () => {
      const pane = focusedPane();
      if (pane) openFind(pane);
    },
  },
  {
    id: "chat.findClose",
    title: "Close Find",
    shortcut: { key: "escape" },
    when: () => findingPane() !== null,
    run: () => {
      const pane = findingPane();
      if (pane) closeFind(pane);
    },
  },
  {
    id: "chat.stop",
    title: "Stop Agent",
    shortcut: { key: "escape" },
    when: () => runningPane() !== null,
    run: () => {
      const pane = runningPane();
      if (pane) abortRun(pane);
    },
  },
];

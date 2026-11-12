// The chat's commands for the commands table.
import type { AppCommand } from "../../app/commands";
import { activeSession } from "../panes/sessions";
import { panes } from "../panes/store";
import { settingsOpen } from "../settings/store";
import { projects } from "../worktrees/store";
import { closeFind, finds, openFind } from "./find";
import { closeLightbox, lightboxes, stepLightbox } from "./viewing";
import { abortRun, chatOf, chats, pickAttachments } from "./store";

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

const lightboxPane = (): string | null => {
  const pane = focusedPane();
  return pane && lightboxes.get()[pane] ? pane : null;
};

const attachingPane = (): string | null => {
  const pane = focusedPane();
  const chat = pane && chatOf(chats.get(), pane);
  return chat && chat.capabilities?.includes("attachments") && chat.agent === "ready" ? pane : null;
};

export const chatCommands: AppCommand[] = [
  {
    id: "chat.lightboxClose",
    title: "Close Image",
    shortcut: { key: "escape" },
    when: () => lightboxPane() !== null,
    run: () => {
      const pane = lightboxPane();
      if (pane) closeLightbox(pane);
    },
  },
  {
    id: "chat.lightboxPrevious",
    title: "Previous Image",
    shortcut: { key: "arrowleft" },
    when: () => lightboxPane() !== null,
    run: () => {
      const pane = lightboxPane();
      if (pane) stepLightbox(pane, -1);
    },
  },
  {
    id: "chat.lightboxNext",
    title: "Next Image",
    shortcut: { key: "arrowright" },
    when: () => lightboxPane() !== null,
    run: () => {
      const pane = lightboxPane();
      if (pane) stepLightbox(pane, 1);
    },
  },
  {
    id: "chat.attach",
    title: "Attach Images",
    shortcut: { key: "u", cmd: true },
    when: () => attachingPane() !== null,
    run: () => {
      const pane = attachingPane();
      if (pane) pickAttachments(pane);
    },
  },
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

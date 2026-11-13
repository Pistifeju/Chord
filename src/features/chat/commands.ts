// The chat's commands for the commands table.
import type { AppCommand } from "../../app/commands";
import { chatPaneIds } from "../panes/layout";
import { shownSession } from "../panes/sessions";
import { panes } from "../panes/store";
import { settingsOpen } from "../settings/store";
import { projects } from "../worktrees/store";
import { sheet } from "../../app/sheet";
import { closeFind, finds, openFind } from "./find";
import { closeLightbox, lightboxes, stepLightbox } from "./viewing";
import { slashMenu } from "./slash";
import { abortRun, answerDialog, chatOf, chats, closeSlash, drafts, pickAttachments, slashClosed } from "./store";

// A focused file pane is no chat.
const focusedPane = (): string | null => {
  const shown = shownSession(panes.get(), projects.get().selected, settingsOpen.get());
  return shown?.focused && chatPaneIds(shown.layout).includes(shown.focused) ? shown.focused : null;
};

const runningPane = (): string | null => {
  const pane = focusedPane();
  return pane && chatOf(chats.get(), pane).running ? pane : null;
};

const findingPane = (): string | null => {
  const pane = focusedPane();
  return pane && finds.get()[pane] ? pane : null;
};

const dialogPane = (): string | null => {
  const pane = focusedPane();
  return pane && sheet.get() === null && chatOf(chats.get(), pane).dialog ? pane : null;
};

const slashPane = (): string | null => {
  const pane = focusedPane();
  if (!pane || sheet.get() !== null) return null;
  const draft = drafts.get()[pane] ?? "";
  return slashMenu(chatOf(chats.get(), pane).commands, draft, slashClosed.get()[pane]).length > 0 ? pane : null;
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
    id: "chat.dialogCancel",
    title: "Cancel Dialog",
    shortcut: { key: "escape" },
    when: () => dialogPane() !== null,
    run: () => {
      const pane = dialogPane();
      const dialog = pane && chatOf(chats.get(), pane).dialog;
      if (pane && dialog) answerDialog(pane, dialog.id, { kind: "cancelled" });
    },
  },
  {
    id: "chat.slashClose",
    title: "Close Command Menu",
    shortcut: { key: "escape" },
    when: () => slashPane() !== null,
    run: () => {
      const pane = slashPane();
      if (pane) closeSlash(pane, drafts.get()[pane] ?? "");
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

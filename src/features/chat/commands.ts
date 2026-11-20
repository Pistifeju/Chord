// The chat's commands for the commands table.
import type { AppCommand } from "../../app/commands";
import { onTarget } from "../../app/commandTarget";
import { chatPaneIds } from "../panes/layout";
import { requestDeleteBranch } from "../worktrees/localBranches";
import { startBranchRename } from "../worktrees/rename";
import { projects, shownPanes } from "../worktrees/store";
import { sheet } from "../../app/sheet";
import { closeFind, finds, openFind } from "./find";
import { closeLightbox, lightboxes, stepLightbox } from "./viewing";
import { agentPickerPane, closeAgentPicker } from "./agentPicker";
import { branchMenuPane, closeBranchMenu } from "./branch";
import { slashMenu } from "./slash";
import { abortRun, answerDialog, chatOf, chats, closeSlash, drafts, pickAttachments, slashClosed } from "./store";

// A focused file tab is no chat; an open sheet and the focus in a side or bottom panel own the keys.
const focusedPane = (): string | null => {
  if (sheet.get() !== null || document.activeElement?.closest(".side-panel, .bottom-panel")) return null;
  const shown = shownPanes();
  return shown?.focused && chatPaneIds(shown.layout).includes(shown.focused) ? shown.focused : null;
};

const onBranch = (act: (worktree: string, branch: string) => void) => (target?: string) => {
  const worktree = projects.get().selected;
  closeBranchMenu();
  if (worktree && target) act(worktree, target);
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
  return pane && chatOf(chats.get(), pane).dialog ? pane : null;
};

const slashPane = (): string | null => {
  const pane = focusedPane();
  if (!pane) return null;
  const draft = drafts.get()[pane] ?? "";
  return slashMenu(chatOf(chats.get(), pane).commands, draft, slashClosed.get()[pane]).length > 0 ? pane : null;
};

const branchMenuOpenPane = (): string | null => {
  const pane = focusedPane();
  return pane && branchMenuPane.get() === pane ? pane : null;
};

const agentPickerOpenPane = (): string | null => {
  const pane = focusedPane();
  return pane && agentPickerPane.get() === pane ? pane : null;
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
  { id: "chat.lightboxClose", title: "Close Image", shortcut: { key: "escape" }, ...onTarget(lightboxPane, closeLightbox) },
  { id: "chat.lightboxPrevious", title: "Previous Image", shortcut: { key: "arrowleft" }, ...onTarget(lightboxPane, (pane) => stepLightbox(pane, -1)) },
  { id: "chat.lightboxNext", title: "Next Image", shortcut: { key: "arrowright" }, ...onTarget(lightboxPane, (pane) => stepLightbox(pane, 1)) },
  { id: "chat.attach", title: "Attach Images", shortcut: { key: "u", cmd: true }, ...onTarget(attachingPane, pickAttachments) },
  { id: "chat.find", title: "Find in Chat", shortcut: { key: "f", cmd: true }, ...onTarget(focusedPane, openFind) },
  { id: "chat.findClose", title: "Close Find", shortcut: { key: "escape" }, ...onTarget(findingPane, closeFind) },
  {
    id: "chat.dialogCancel",
    title: "Cancel Dialog",
    shortcut: { key: "escape" },
    ...onTarget(dialogPane, (pane) => {
      const dialog = chatOf(chats.get(), pane).dialog;
      if (dialog) answerDialog(pane, dialog.id, { kind: "cancelled" });
    }),
  },
  { id: "chat.slashClose", title: "Close Command Menu", shortcut: { key: "escape" }, ...onTarget(slashPane, (pane) => closeSlash(pane, drafts.get()[pane] ?? "")) },
  { id: "branch.rename", title: "Rename Branch", run: onBranch(startBranchRename) },
  { id: "branch.delete", title: "Delete Branch", run: onBranch(requestDeleteBranch) },
  { id: "chat.branchMenuClose", title: "Close Branch Menu", shortcut: { key: "escape" }, ...onTarget(branchMenuOpenPane, closeBranchMenu) },
  { id: "chat.agentPickerClose", title: "Close Agent Picker", shortcut: { key: "escape" }, ...onTarget(agentPickerOpenPane, closeAgentPicker) },
  { id: "chat.stop", title: "Stop Agent", shortcut: { key: "escape" }, ...onTarget(runningPane, abortRun) },
];

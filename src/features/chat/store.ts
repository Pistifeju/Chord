// Connects the transcript rules to IPC: one chat per pane, started, fed by the agent's events, stopped.
import { Channel } from "@tauri-apps/api/core";
import { commands, type DialogAnswer, type HarnessEvent, type Image } from "../../ipc/bindings";
import { externalStore } from "../../ui/externalStore";
import { paneNames, type PaneNames } from "../panes/sessions";
import { usage } from "../usage/store";
import { notify } from "../worktrees/store";
import { fileName, imageExtensions, imageMime, type Attachment } from "./attachments";
import { chatFed, emptyChat, type Chat, type Replay } from "./transcript";

export const chats = externalStore<Record<string, Chat>>({});

/** The name of each listed pane's agent session; a component re-renders only when one changes. */
export const usePaneNames = (panes: string[]): PaneNames => JSON.parse(chats.usePart((all) => JSON.stringify(paneNames(all, panes))));

export const chatOf = (all: Record<string, Chat>, pane: string): Chat => all[pane] ?? emptyChat;

// An event that leaves the chat as it was wakes no view.
function update(pane: string, change: (chat: Chat) => Chat) {
  const before = chatOf(chats.get(), pane);
  const after = change(before);
  if (after !== before) chats.set({ ...chats.get(), [pane]: after });
}

// The unsent composer text of each pane, so it survives a split and a worktree switch.
export const drafts = externalStore<Record<string, string>>({});

export const setDraft = (pane: string, text: string) => drafts.set({ ...drafts.get(), [pane]: text });

// The draft each pane's slash menu was closed on with Esc; it stays closed until the draft changes.
export const slashClosed = externalStore<Record<string, string>>({});

export const closeSlash = (pane: string, draft: string) => slashClosed.set({ ...slashClosed.get(), [pane]: draft });

// Each pane's latest agent start and the history it replays; events of an older start are dropped.
const intakes = new Map<string, { start: number; replay: Replay }>();
let startCount = 0;

const latest = (pane: string, start: number) => intakes.get(pane)?.start === start;

function received(pane: string, start: number, event: HarnessEvent) {
  const intake = intakes.get(pane);
  if (intake?.start !== start) return;
  if (event.kind === "draft") setDraft(pane, event.text);
  if (event.kind === "usage") usage.set(event.windows);
  if (event.kind === "exported") notify(`Exported to ${event.path}`);
  const fed = chatFed(chatOf(chats.get(), pane), intake.replay, event, Date.now());
  intake.replay = fed.replay;
  update(pane, () => fed.chat);
}

export async function startChat(worktree: string, pane: string) {
  if (chatOf(chats.get(), pane).agent !== "idle") return;
  const start = ++startCount;
  intakes.set(pane, { start, replay: null });
  const channel = new Channel<HarnessEvent>();
  channel.onmessage = (event) => received(pane, start, event);
  update(pane, (chat) => ({ ...chat, agent: "starting", notice: null, report: null, commands: [] }));
  try {
    const capabilities = await commands.startChat(worktree, pane, channel);
    if (latest(pane, start)) update(pane, (chat) => (chat.agent === "starting" ? { ...chat, agent: "ready", capabilities } : chat));
  } catch (error) {
    if (latest(pane, start)) update(pane, (chat) => ({ ...chat, agent: "idle", notice: String(error) }));
  }
}

// The images waiting in each pane's composer.
export const attachments = externalStore<Record<string, Attachment[]>>({});

let attachmentCount = 0;

const attach = (pane: string, name: string, image: Image) =>
  attachments.set({ ...attachments.get(), [pane]: [...(attachments.get()[pane] ?? []), { id: ++attachmentCount, name, image }] });

export const removeAttachment = (pane: string, id: number) => attachments.set({ ...attachments.get(), [pane]: (attachments.get()[pane] ?? []).filter((a) => a.id !== id) });

const noticeOf = (pane: string, error: unknown) => update(pane, (chat) => ({ ...chat, notice: String(error) }));

// A file that is not an image is skipped.
export async function attachFiles(pane: string, paths: string[]) {
  for (const path of paths) {
    const mime = imageMime(path);
    if (!mime) continue;
    try {
      attach(pane, fileName(path), { data: await commands.readImageFile(path), mime_type: mime });
    } catch (error) {
      noticeOf(pane, error);
    }
  }
}

export async function pickAttachments(pane: string) {
  try {
    await attachFiles(pane, await commands.pickImages(imageExtensions));
  } catch (error) {
    noticeOf(pane, error);
  }
}

// A pasted screenshot arrives as a file with a generic name.
export function attachPasted(pane: string, file: File) {
  const reader = new FileReader();
  reader.onload = () => {
    const url = String(reader.result);
    attach(pane, file.name, { data: url.slice(url.indexOf(",") + 1), mime_type: file.type });
  };
  reader.onerror = () => noticeOf(pane, "The image could not be read");
  reader.readAsDataURL(file);
}

// The waiting images leave the composer once the harness took the message; a failed send keeps them.
export function sendMessage(pane: string, text: string) {
  const sent = attachments.get()[pane] ?? [];
  update(pane, (chat) => ({ ...chat, notice: null }));
  commands
    .sendMessage(pane, text, sent.map((a) => a.image))
    .then(() => attachments.set({ ...attachments.get(), [pane]: (attachments.get()[pane] ?? []).filter((a) => !sent.includes(a)) }))
    .catch((error) => noticeOf(pane, error));
}

// The dialog closes at once; a refused answer shows as a notice.
export function answerDialog(pane: string, id: string, answer: DialogAnswer) {
  update(pane, (chat) => (chat.dialog?.id === id ? { ...chat, dialog: null } : chat));
  commands.answerDialog(pane, id, answer).catch((error) => noticeOf(pane, error));
}

// The picker opens as a dialog; a refusal shows as a notice.
export function openPicker(pane: string, command: "/model" | "/thinking") {
  commands.sendMessage(pane, command, []).catch((error) => noticeOf(pane, error));
}

export function abortRun(pane: string) {
  commands.abortRun(pane).catch(console.error);
}

// The pane is gone: its agent stops and its transcript and draft go.
export function forgetChat(pane: string) {
  intakes.delete(pane);
  const { [pane]: gone, ...rest } = chats.get();
  chats.set(rest);
  const { [pane]: goneDraft, ...restDrafts } = drafts.get();
  drafts.set(restDrafts);
  const { [pane]: goneAttachments, ...restAttachments } = attachments.get();
  attachments.set(restAttachments);
  commands.stopChat(pane).catch(console.error);
}

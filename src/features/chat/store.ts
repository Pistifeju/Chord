// Connects the transcript rules to IPC: one chat per pane, started, fed by the agent's events, stopped.
import { Channel } from "@tauri-apps/api/core";
import { commands, type DialogAnswer, type HarnessEvent, type Image, type PaneAgent } from "../../ipc/bindings";
import { externalStore } from "../../ui/externalStore";
import { paneNames, type PaneNames } from "../panes/sessions";
import { usage } from "../usage/store";
import { notify } from "../../ui/alert";
import { fileName } from "../files/paths";
import { imageExtensions, imageMime, type Attachment } from "./attachments";
import { intakes } from "./intake";
import { emptyChat, withSteerImages, type Chat } from "./transcript";

export const chats = externalStore<Record<string, Chat>>({});

/** The name of each listed pane's agent session; a component re-renders only when one changes. */
export const usePaneNames = (panes: string[]): PaneNames => JSON.parse(chats.usePart((all) => JSON.stringify(paneNames(all, panes))));

// A pane without a harness takes the default at its first start.
export const paneHarnesses = externalStore<Record<string, string>>({});

export const loadPaneHarnesses = (agents: Record<string, PaneAgent>) =>
  paneHarnesses.set(Object.fromEntries(Object.entries(agents).flatMap(([pane, agent]) => (agent.harness ? [[pane, agent.harness]] : []))));

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

/** Adds a file's relative path to the end of a pane's draft, a space apart from the text before it and after it. */
export function addPathToDraft(pane: string, path: string) {
  const draft = drafts.get()[pane] ?? "";
  setDraft(pane, `${draft}${draft && !/\s$/.test(draft) ? " " : ""}${path} `);
}

// The pane whose composer a file dragged from the Files tree is over.
export const pathDropPane = externalStore<string | null>(null);

// The draft each pane's slash menu was closed on with Esc; it stays closed until the draft changes.
export const slashClosed = externalStore<Record<string, string>>({});

export const closeSlash = (pane: string, draft: string) => slashClosed.set({ ...slashClosed.get(), [pane]: draft });

const intake = intakes();

function received(pane: string, start: number, event: HarnessEvent) {
  const fed = intake.fed(pane, start, chatOf(chats.get(), pane), event, Date.now());
  if (!fed) return;
  const { effect } = fed;
  if (effect?.kind === "draft") setDraft(pane, effect.text);
  if (effect?.kind === "usage") usage.set(effect.windows);
  if (effect?.kind === "notify") notify(effect.text);
  update(pane, () => fed.chat);
}

// Resolves once the agent's history has arrived, or the start failed.
export async function startChat(worktree: string, pane: string) {
  if (chatOf(chats.get(), pane).agent !== "idle") return;
  let loaded = () => {};
  const history = new Promise<void>((resolve) => (loaded = resolve));
  const start = intake.begin(pane, loaded);
  const channel = new Channel<HarnessEvent>();
  channel.onmessage = (event) => received(pane, start, event);
  update(pane, (chat) => ({ ...chat, agent: "starting", notice: null, report: null, commands: [] }));
  try {
    const { harness, capabilities } = await commands.startChat(worktree, pane, channel);
    paneHarnesses.set({ ...paneHarnesses.get(), [pane]: harness });
    // The pane closed while its agent started, so the stop came first and missed it.
    if (!intake.has(pane)) commands.stopChat(pane).catch(console.error);
    if (intake.latest(pane, start)) update(pane, (chat) => (chat.agent === "starting" ? { ...chat, agent: "ready", capabilities } : chat));
  } catch (error) {
    if (intake.latest(pane, start)) update(pane, (chat) => ({ ...chat, agent: "idle", notice: String(error) }));
    loaded();
  }
  return history;
}

// The images waiting in each pane's composer.
export const attachments = externalStore<Record<string, Attachment[]>>({});

let attachmentCount = 0;

const attach = (pane: string, name: string, image: Image) =>
  attachments.set({ ...attachments.get(), [pane]: [...(attachments.get()[pane] ?? []), { id: ++attachmentCount, name, image }] });

export const removeAttachment = (pane: string, id: number) => attachments.set({ ...attachments.get(), [pane]: (attachments.get()[pane] ?? []).filter((a) => a.id !== id) });

const noticeOf = (pane: string, error: unknown) => update(pane, (chat) => ({ ...chat, notice: String(error) }));

/** Attachments need the capability and an agent that is not idle; one still starting takes them. */
export const takesAttachments = (chat: Chat) => chat.capabilities?.includes("attachments") === true && chat.agent !== "idle";

export const focusComposer = (pane: string) => document.querySelector<HTMLElement>(`[data-pane="${CSS.escape(pane)}"] .chat-input`)?.focus();

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

// The waiting images leave the composer once the harness took the message; a failed send keeps them and puts the text back in an empty draft.
export function sendMessage(pane: string, text: string) {
  const sent = attachments.get()[pane] ?? [];
  const images = sent.map((a) => a.image);
  update(pane, (chat) => withSteerImages({ ...chat, notice: null }, text, images));
  commands
    .sendMessage(pane, text, images)
    .then(() => attachments.set({ ...attachments.get(), [pane]: (attachments.get()[pane] ?? []).filter((a) => !sent.includes(a)) }))
    .catch((error) => {
      if (!drafts.get()[pane]) setDraft(pane, text);
      update(pane, (chat) => ({ ...chat, steerImages: chat.steerImages.filter((steer) => steer.images !== images) }));
      noticeOf(pane, error);
    });
}

// The dialog closes at once; a refused answer shows as a notice.
export function answerDialog(pane: string, id: string, answer: DialogAnswer) {
  update(pane, (chat) => (chat.dialog?.id === id ? { ...chat, dialog: null } : chat));
  commands.answerDialog(pane, id, answer).catch((error) => noticeOf(pane, error));
}

// The picker opens as a dialog; a refusal shows as a notice.
export function openPicker(pane: string, command: "/model" | "/thinking" | "/permissions") {
  commands.sendMessage(pane, command, []).catch((error) => noticeOf(pane, error));
}

export function abortRun(pane: string) {
  commands.abortRun(pane).catch(console.error);
}

// The pane is gone: its agent stops and its transcript and draft go.
export function forgetChat(pane: string) {
  intake.end(pane);
  const { [pane]: gone, ...rest } = chats.get();
  chats.set(rest);
  const { [pane]: goneDraft, ...restDrafts } = drafts.get();
  drafts.set(restDrafts);
  const { [pane]: goneAttachments, ...restAttachments } = attachments.get();
  attachments.set(restAttachments);
  const { [pane]: goneHarness, ...restHarnesses } = paneHarnesses.get();
  paneHarnesses.set(restHarnesses);
  commands.stopChat(pane).catch(console.error);
}

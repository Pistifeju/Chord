import { Channel } from "@tauri-apps/api/core";
import { commands, type DialogAnswer, type HarnessEvent, type Image, type ModelOption, type ModelPick, type PaneAgent } from "../../ipc/bindings";
import { externalStore } from "../../ui/externalStore";
import { paneNames, type PaneNames } from "../panes/sessions";
import { usage } from "../usage/store";
import { notify } from "../../ui/alert";
import { fileName } from "../files/paths";
import { refreshGitCountsAfterAgent } from "../worktrees/gitCountsRefresh";
import { imageExtensions, imageMime, type Attachment } from "./attachments";
import { heldMessage } from "./composer";
import { intakes } from "./intake";
import { openingOf, openingWith, type Opening } from "./opening";
import { emptyChat, withSteerImages, type Chat } from "./transcript";

export const chats = externalStore<Record<string, Chat>>({});

export const usePaneNames = (panes: string[]): PaneNames => JSON.parse(chats.usePart((all) => JSON.stringify(paneNames(all, panes))));

export const paneHarnesses = externalStore<Record<string, string>>({});

export const loadPaneHarnesses = (agents: Record<string, PaneAgent>) =>
  paneHarnesses.set(Object.fromEntries(Object.entries(agents).flatMap(([pane, agent]) => (agent.harness ? [[pane, agent.harness]] : []))));

export const modelCatalogs = externalStore<Record<string, ModelOption[]>>({});

export const loadModelCatalogs = (models: Record<string, ModelOption[]>) => modelCatalogs.set(models);

export function refreshModelCatalogs() {
  commands
    .refreshModels()
    .then((models) => modelCatalogs.set({ ...modelCatalogs.get(), ...models }))
    .catch(console.error);
}

export const chatOf = (all: Record<string, Chat>, pane: string): Chat => all[pane] ?? emptyChat;

function update(pane: string, change: (chat: Chat) => Chat) {
  const before = chatOf(chats.get(), pane);
  const after = change(before);
  if (after !== before) chats.set({ ...chats.get(), [pane]: after });
}

export const drafts = externalStore<Record<string, string>>({});

export const setDraft = (pane: string, text: string) => drafts.set({ ...drafts.get(), [pane]: text });

export function addPathToDraft(pane: string, path: string) {
  const draft = drafts.get()[pane] ?? "";
  setDraft(pane, `${draft}${draft && !/\s$/.test(draft) ? " " : ""}${path} `);
}

export const pathDropPane = externalStore<string | null>(null);

export const slashClosed = externalStore<Record<string, string>>({});

export const closeSlash = (pane: string, draft: string) => slashClosed.set({ ...slashClosed.get(), [pane]: draft });

const intake = intakes();

const openings = new Map<string, Opening>();
let defaultHarness: string | null = null;

const opened = (harness: string | null, pick: ModelPick | null): Opening =>
  openingWith(harness ? openings.get(harness) : undefined, harness && pick ? modelCatalogs.get()[harness]?.find((model) => model.id === pick.id) : undefined, pick?.level ?? null);

function rememberOpening(pane: string) {
  const harness = paneHarnesses.get()[pane];
  const opening = openingOf(chatOf(chats.get(), pane));
  if (harness && opening) openings.set(harness, opening);
}

function settleHeld(pane: string) {
  const settled = heldMessage(chatOf(chats.get(), pane));
  if (!settled) return;
  update(pane, (chat) => ({ ...chat, held: null }));
  if (settled.kind === "send") return sendMessage(pane, settled.text);
  const draft = drafts.get()[pane];
  setDraft(pane, draft ? `${settled.text}\n${draft}` : settled.text);
}

function received(pane: string, start: number, event: HarnessEvent) {
  const fed = intake.fed(pane, start, chatOf(chats.get(), pane), event, Date.now());
  if (!fed) return;
  const { effect } = fed;
  if (effect?.kind === "draft") setDraft(pane, effect.text);
  if (effect?.kind === "usage") usage.set({ ...usage.get(), [effect.harness]: effect.windows });
  if (effect?.kind === "models") modelCatalogs.set({ ...modelCatalogs.get(), [effect.harness]: effect.models });
  if (effect?.kind === "notify") notify(effect.text);
  if (effect?.kind === "gitCounts") refreshGitCountsAfterAgent();
  update(pane, () => fed.chat);
  settleHeld(pane);
  rememberOpening(pane);
}

export async function startChat(worktree: string, pane: string, switchTo: string | null = null, pick: ModelPick | null = null) {
  if (chatOf(chats.get(), pane).agent !== "idle") return;
  let loaded = () => {};
  const history = new Promise<void>((resolve) => (loaded = resolve));
  const start = intake.begin(pane, loaded);
  const channel = new Channel<HarnessEvent>();
  channel.onmessage = (event) => received(pane, start, event);
  const fresh = switchTo !== null || !(pane in paneHarnesses.get());
  const expected = switchTo ?? defaultHarness;
  const previous = paneHarnesses.get()[pane];
  if (fresh && expected) paneHarnesses.set({ ...paneHarnesses.get(), [pane]: expected });
  update(pane, (chat) => ({ ...chat, agent: "starting", loading: true, notice: null, report: null, commands: [], ...(fresh && opened(expected, pick)) }));
  try {
    const { harness, capabilities } = await commands.startChat(worktree, pane, switchTo, pick, channel);
    if (fresh && switchTo === null) defaultHarness = harness;
    if (fresh && harness !== expected) update(pane, (chat) => ({ ...chat, ...opened(harness, pick) }));
    paneHarnesses.set({ ...paneHarnesses.get(), [pane]: harness });
    if (!intake.has(pane)) commands.stopChat(pane).catch(console.error);
    if (intake.latest(pane, start)) update(pane, (chat) => (chat.agent === "starting" ? { ...chat, agent: "ready", capabilities } : chat));
  } catch (error) {
    if (intake.latest(pane, start)) {
      const { [pane]: _, ...rest } = paneHarnesses.get();
      if (fresh) paneHarnesses.set(previous ? { ...rest, [pane]: previous } : rest);
      update(pane, (chat) => ({ ...chat, agent: "idle", notice: String(error) }));
    }
    loaded();
  }
  return history;
}

export async function switchEmptyChat(worktree: string, pane: string, harness: string, pick: ModelPick | null) {
  update(pane, () => emptyChat);
  await startChat(worktree, pane, harness, pick);
  const { agent, notice } = chatOf(chats.get(), pane);
  if (agent !== "idle") return;
  await startChat(worktree, pane);
  update(pane, (chat) => ({ ...chat, notice: chat.notice ?? notice }));
}

export const attachments = externalStore<Record<string, Attachment[]>>({});

let attachmentCount = 0;

const attach = (pane: string, name: string, image: Image) =>
  attachments.set({ ...attachments.get(), [pane]: [...(attachments.get()[pane] ?? []), { id: ++attachmentCount, name, image }] });

export const removeAttachment = (pane: string, id: number) => attachments.set({ ...attachments.get(), [pane]: (attachments.get()[pane] ?? []).filter((a) => a.id !== id) });

const noticeOf = (pane: string, error: unknown) => update(pane, (chat) => ({ ...chat, notice: String(error) }));

export const takesAttachments = (chat: Chat) => chat.capabilities?.includes("attachments") === true && chat.agent !== "idle";

export const focusComposer = (pane: string) => document.querySelector<HTMLElement>(`[data-pane="${CSS.escape(pane)}"] .chat-input`)?.focus();

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

export function attachPasted(pane: string, file: File) {
  const reader = new FileReader();
  reader.onload = () => {
    const url = String(reader.result);
    attach(pane, file.name, { data: url.slice(url.indexOf(",") + 1), mime_type: file.type });
  };
  reader.onerror = () => noticeOf(pane, "The image could not be read");
  reader.readAsDataURL(file);
}

export function sendMessage(pane: string, text: string) {
  if (chatOf(chats.get(), pane).loading) return update(pane, (chat) => ({ ...chat, held: text, notice: null }));
  const sent = attachments.get()[pane] ?? [];
  const images = sent.map((a) => a.image);
  const forgetSteer = () => update(pane, (chat) => ({ ...chat, steerImages: chat.steerImages.filter((steer) => steer.images !== images) }));
  update(pane, (chat) => withSteerImages({ ...chat, notice: null }, text, images));
  commands
    .sendMessage(pane, text, images)
    .then((took) => {
      if (took) attachments.set({ ...attachments.get(), [pane]: (attachments.get()[pane] ?? []).filter((a) => !sent.includes(a)) });
      else forgetSteer();
    })
    .catch((error) => {
      if (!drafts.get()[pane]) setDraft(pane, text);
      forgetSteer();
      noticeOf(pane, error);
    });
}

export function answerDialog(pane: string, id: string, answer: DialogAnswer) {
  update(pane, (chat) => (chat.dialog?.id === id ? { ...chat, dialog: null } : chat));
  commands.answerDialog(pane, id, answer).catch((error) => noticeOf(pane, error));
}

export function pickModel(pane: string, id: string, level: string | null) {
  commands.pickModel(pane, id, level).catch((error) => noticeOf(pane, error));
}

export function openPicker(pane: string, command: "/model" | "/thinking" | "/permissions") {
  commands.sendMessage(pane, command, []).catch((error) => noticeOf(pane, error));
}

export function abortRun(pane: string) {
  commands.abortRun(pane).catch(console.error);
}

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

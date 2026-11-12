// Connects the transcript rules to IPC: one chat per pane, started, fed by the agent's events, stopped.
import { Channel } from "@tauri-apps/api/core";
import { commands, type HarnessEvent } from "../../ipc/bindings";
import { externalStore } from "../../ui/externalStore";
import { chatAfter, emptyChat, type Chat } from "./transcript";

export const chats = externalStore<Record<string, Chat>>({});

export const chatOf = (all: Record<string, Chat>, pane: string): Chat => all[pane] ?? emptyChat;

const update = (pane: string, change: (chat: Chat) => Chat) => chats.set({ ...chats.get(), [pane]: change(chatOf(chats.get(), pane)) });

// The unsent composer text of each pane, so it survives a split and a worktree switch.
export const drafts = externalStore<Record<string, string>>({});

export const setDraft = (pane: string, text: string) => drafts.set({ ...drafts.get(), [pane]: text });

// The latest start per pane; events of an older agent are dropped.
const starts = new Map<string, number>();
let startCount = 0;

export async function startChat(worktree: string, pane: string) {
  if (chatOf(chats.get(), pane).agent !== "idle") return;
  const start = ++startCount;
  starts.set(pane, start);
  const channel = new Channel<HarnessEvent>();
  channel.onmessage = (event) => {
    if (starts.get(pane) === start) update(pane, (chat) => chatAfter(chat, event));
  };
  update(pane, (chat) => ({ ...chat, agent: "starting", notice: null }));
  try {
    const capabilities = await commands.startChat(worktree, pane, channel);
    if (starts.get(pane) === start) update(pane, (chat) => (chat.agent === "starting" ? { ...chat, agent: "ready", capabilities } : chat));
  } catch (error) {
    if (starts.get(pane) === start) update(pane, (chat) => ({ ...chat, agent: "idle", notice: String(error) }));
  }
}

export function sendMessage(pane: string, text: string) {
  update(pane, (chat) => ({ ...chat, notice: null }));
  commands.sendMessage(pane, text).catch((error) => update(pane, (chat) => ({ ...chat, notice: String(error) })));
}

export function abortRun(pane: string) {
  commands.abortRun(pane).catch(console.error);
}

// The pane is gone: its agent stops and its transcript and draft go.
export function forgetChat(pane: string) {
  starts.delete(pane);
  const { [pane]: gone, ...rest } = chats.get();
  chats.set(rest);
  const { [pane]: goneDraft, ...restDrafts } = drafts.get();
  drafts.set(restDrafts);
  commands.stopChat(pane).catch(console.error);
}

// Connects the transcript rules to IPC: one chat per worktree, started, fed by the agent's events, stopped.
import { Channel } from "@tauri-apps/api/core";
import { commands, type HarnessEvent } from "../../ipc/bindings";
import { externalStore } from "../../ui/externalStore";
import { chatAfter, emptyChat, type Chat } from "./transcript";

export const chats = externalStore<Record<string, Chat>>({});

export const chatOf = (all: Record<string, Chat>, worktree: string): Chat => all[worktree] ?? emptyChat;

const update = (worktree: string, change: (chat: Chat) => Chat) => chats.set({ ...chats.get(), [worktree]: change(chatOf(chats.get(), worktree)) });

// The latest start per worktree; events of an older agent are dropped.
const starts = new Map<string, number>();
let startCount = 0;

export async function startChat(worktree: string) {
  if (chatOf(chats.get(), worktree).agent !== "idle") return;
  const start = ++startCount;
  starts.set(worktree, start);
  const channel = new Channel<HarnessEvent>();
  channel.onmessage = (event) => {
    if (starts.get(worktree) === start) update(worktree, (chat) => chatAfter(chat, event));
  };
  update(worktree, (chat) => ({ ...chat, agent: "starting", notice: null }));
  try {
    const capabilities = await commands.startChat(worktree, channel);
    if (starts.get(worktree) === start) update(worktree, (chat) => (chat.agent === "starting" ? { ...chat, agent: "ready", capabilities } : chat));
  } catch (error) {
    if (starts.get(worktree) === start) update(worktree, (chat) => ({ ...chat, agent: "idle", notice: String(error) }));
  }
}

export function sendMessage(worktree: string, text: string) {
  update(worktree, (chat) => ({ ...chat, notice: null }));
  commands.sendMessage(worktree, text).catch((error) => update(worktree, (chat) => ({ ...chat, notice: String(error) })));
}

export function abortRun(worktree: string) {
  commands.abortRun(worktree).catch(console.error);
}

// The worktree is gone: its agent stops and its transcript goes.
export function forgetChat(worktree: string) {
  starts.delete(worktree);
  const { [worktree]: gone, ...rest } = chats.get();
  chats.set(rest);
  commands.stopChat(worktree).catch(console.error);
}

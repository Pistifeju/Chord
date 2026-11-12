// Rules for a chat's transcript: how each harness event changes the messages, the run flag and the notice.
import type { Capability, HarnessEvent } from "../../ipc/bindings";

export type Message = { role: "user" | "assistant"; text: string; streaming: boolean };

export type Chat = {
  /** idle: no agent runs (the Start button shows); starting: the process is coming up; ready: it takes messages. */
  agent: "idle" | "starting" | "ready";
  messages: Message[];
  running: boolean;
  /** What the harness can do; null until the agent has started. */
  capabilities: Capability[] | null;
  notice: string | null;
};

export const emptyChat: Chat = { agent: "idle", messages: [], running: false, capabilities: null, notice: null };

const closed = (messages: Message[]): Message[] => messages.map((m) => (m.streaming ? { ...m, streaming: false } : m));

export function chatAfter(chat: Chat, event: HarnessEvent): Chat {
  switch (event.kind) {
    case "user_message_chunk":
      return { ...chat, messages: [...closed(chat.messages), { role: "user", text: event.text, streaming: false }] };
    case "message_chunk": {
      const last = chat.messages[chat.messages.length - 1];
      if (last?.streaming) return { ...chat, messages: [...chat.messages.slice(0, -1), { ...last, text: last.text + event.text }] };
      return { ...chat, messages: [...chat.messages, { role: "assistant", text: event.text, streaming: true }] };
    }
    case "message_end":
      return { ...chat, messages: closed(chat.messages) };
    case "state":
      return { ...chat, running: event.running, messages: event.running ? chat.messages : closed(chat.messages) };
    case "notice":
      return { ...chat, notice: event.text };
    case "exited":
      return { ...chat, agent: "idle", running: false, messages: closed(chat.messages) };
  }
}

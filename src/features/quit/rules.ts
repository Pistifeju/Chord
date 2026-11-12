// The rule for when a quit asks first.
import type { Chat } from "../chat/transcript";

export const quitNeedsAsking = (chats: Record<string, Chat>) => Object.values(chats).some((chat) => chat.running);

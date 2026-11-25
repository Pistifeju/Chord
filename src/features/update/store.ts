import { Channel } from "@tauri-apps/api/core";
import { commands } from "../../ipc/bindings";
import { closeSheet, openSheet } from "../../app/sheet";
import { fail } from "../../ui/alert";
import { externalStore } from "../../ui/externalStore";
import { chats } from "../chat/store";
import { anyRunning } from "../chat/transcript";
import { flushAllSaves } from "../editor/store";

const RECHECK_MS = 6 * 60 * 60 * 1000;

export const newerVersion = externalStore<string | null>(null);
export const installing = externalStore<{ percent: number | null } | null>(null);

const look = () => {
  commands.checkUpdate().then(newerVersion.set).catch(console.error);
};

export function watchUpdates() {
  look();
  const timer = setInterval(look, RECHECK_MS);
  return () => clearInterval(timer);
}

export const cancelUpdate = () => closeSheet("update");

export const confirmUpdate = () => {
  closeSheet("update");
  installing.set({ percent: null });
  const onPercent = new Channel<number>();
  onPercent.onmessage = (percent) => installing.set({ percent });
  flushAllSaves()
    .then(() => commands.installUpdate(onPercent))
    .catch(fail)
    .finally(() => installing.set(null));
};

export const requestUpdate = () => {
  const all = chats.get();
  if (anyRunning(all, Object.keys(all))) openSheet({ kind: "update" });
  else confirmUpdate();
};

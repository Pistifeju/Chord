import { listen } from "@tauri-apps/api/event";
import { commands } from "../../ipc/bindings";
import { chats } from "../chat/store";
import { closeSheet, openSheet } from "../../app/sheet";
import { flushAllSaves } from "../editor/store";
import { anyRunning } from "../chat/transcript";

export const cancelQuit = () => closeSheet("quit");

export const confirmQuit = () => {
  flushAllSaves()
    .then(() => commands.quitApp())
    .catch(console.error);
};

export const requestQuit = () => {
  const all = chats.get();
  if (anyRunning(all, Object.keys(all))) openSheet({ kind: "quit" });
  else confirmQuit();
};

export function watchQuit() {
  const unlisten = listen("quit-requested", requestQuit);
  return () => void unlisten.then((off) => off());
}

// Connects the app's quit request to IPC: it quits at once, or waits for the sheet while an agent works.
import { listen } from "@tauri-apps/api/event";
import { commands } from "../../ipc/bindings";
import { chats } from "../chat/store";
import { closeSheet, openSheet } from "../../app/sheet";
import { flushAllSaves } from "../editor/store";
import { anyRunning } from "../chat/transcript";

export const cancelQuit = () => closeSheet("quit");

// The menu item and the window's close button reach the same ask.
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

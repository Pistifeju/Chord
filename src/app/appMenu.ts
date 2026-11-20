// The macOS app menu: the default one with Settings and Usage under Chord, a Quit that asks while agents run, Undo and Redo that reach the editor's own history, and the Product menu after View.
import { Menu, MenuItem, PredefinedMenuItem, type Submenu } from "@tauri-apps/api/menu";
import { requestQuit } from "../features/quit/store";
import { openSettings } from "../features/settings/store";
import { openUsagePanel } from "../features/usage/commands";
import { productMenu } from "../features/ios/build/productMenu";
import { launched } from "./launch";

export async function installAppMenu() {
  const menu = await Menu.default();
  const appMenu = (await menu.items())[0] as Submenu;
  const settings = await MenuItem.new({ id: "settings", text: "Settings", action: () => launched.get() && openSettings() });
  const usage = await MenuItem.new({ id: "usage", text: "Usage", action: () => launched.get() && openUsagePanel() });
  const separator = await PredefinedMenuItem.new({ item: "Separator" });
  await appMenu.insert([separator, settings, usage], 1);
  // The predefined Quit ends the app without a request the window could answer.
  for (const item of await appMenu.items()) if (item.id === "quit" || (await (item as MenuItem).text()).startsWith("Quit")) await appMenu.remove(item);
  await appMenu.append(await MenuItem.new({ id: "quit", text: "Quit Chord", accelerator: "CmdOrCtrl+Q", action: requestQuit }));
  const menus = await menu.items();
  const texts = await Promise.all(menus.map((item) => (item as Submenu).text()));
  const edit = texts.indexOf("Edit");
  if (edit >= 0) await replaceUndo(menus[edit] as Submenu);
  const view = texts.indexOf("View");
  await menu.insert(await productMenu(), view >= 0 ? view + 1 : texts.length);
  await menu.setAsAppMenu();
}

// WebKit's own undo knows only the edits the editor let it type, so the editor takes Undo as the `beforeinput` its history listens for; any other field keeps WebKit's.
async function replaceUndo(edit: Submenu) {
  const ours = { Undo: ["CmdOrCtrl+Z", "historyUndo", "undo"], Redo: ["CmdOrCtrl+Shift+Z", "historyRedo", "redo"] } as const;
  for (const [index, item] of (await edit.items()).entries()) {
    const text = await (item as MenuItem).text();
    if (text !== "Undo" && text !== "Redo") continue;
    const [accelerator, inputType, command] = ours[text];
    const action = () => {
      const event = new InputEvent("beforeinput", { inputType, bubbles: true, cancelable: true });
      if (document.activeElement?.dispatchEvent(event) !== false) document.execCommand(command);
    };
    await edit.remove(item);
    await edit.insert(await MenuItem.new({ text, accelerator, action }), index);
  }
}

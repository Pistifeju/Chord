// The macOS app menu: the default one with Settings under Chord and a Quit that asks while agents run.
import { Menu, MenuItem, PredefinedMenuItem, type Submenu } from "@tauri-apps/api/menu";
import { requestQuit } from "../features/quit/store";
import { openSettings } from "../features/settings/store";

export async function installAppMenu() {
  const menu = await Menu.default();
  const appMenu = (await menu.items())[0] as Submenu;
  const settings = await MenuItem.new({ id: "settings", text: "Settings", action: openSettings });
  const separator = await PredefinedMenuItem.new({ item: "Separator" });
  await appMenu.insert([separator, settings], 1);
  // The predefined Quit ends the app without a request the window could answer.
  for (const item of await appMenu.items()) if (item.id === "quit" || (await (item as MenuItem).text()).startsWith("Quit")) await appMenu.remove(item);
  await appMenu.append(await MenuItem.new({ id: "quit", text: "Quit Chord", accelerator: "CmdOrCtrl+Q", action: requestQuit }));
  await menu.setAsAppMenu();
}

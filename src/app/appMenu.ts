// The macOS app menu: the default one with Settings under Chord, a Quit that asks while agents run, and the Product menu after View.
import { Menu, MenuItem, PredefinedMenuItem, type Submenu } from "@tauri-apps/api/menu";
import { requestQuit } from "../features/quit/store";
import { openSettings } from "../features/settings/store";
import { productMenu } from "../features/ios/build/productMenu";
import { launched } from "./launch";

export async function installAppMenu() {
  const menu = await Menu.default();
  const appMenu = (await menu.items())[0] as Submenu;
  const settings = await MenuItem.new({ id: "settings", text: "Settings", action: () => launched.get() && openSettings() });
  const separator = await PredefinedMenuItem.new({ item: "Separator" });
  await appMenu.insert([separator, settings], 1);
  // The predefined Quit ends the app without a request the window could answer.
  for (const item of await appMenu.items()) if (item.id === "quit" || (await (item as MenuItem).text()).startsWith("Quit")) await appMenu.remove(item);
  await appMenu.append(await MenuItem.new({ id: "quit", text: "Quit Chord", accelerator: "CmdOrCtrl+Q", action: requestQuit }));
  const menus = await menu.items();
  const texts = await Promise.all(menus.map((item) => (item as Submenu).text()));
  const view = texts.indexOf("View");
  await menu.insert(await productMenu(), view >= 0 ? view + 1 : texts.length);
  await menu.setAsAppMenu();
}

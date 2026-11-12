// The macOS app menu: the default one with Settings… under Chord.
import { Menu, MenuItem, PredefinedMenuItem, type Submenu } from "@tauri-apps/api/menu";
import { openSettings } from "../features/settings/store";

export async function installAppMenu() {
  const menu = await Menu.default();
  const appMenu = (await menu.items())[0] as Submenu;
  const settings = await MenuItem.new({ id: "settings", text: "Settings…", action: openSettings });
  const separator = await PredefinedMenuItem.new({ item: "Separator" });
  await appMenu.insert([separator, settings], 1);
  await menu.setAsAppMenu();
}

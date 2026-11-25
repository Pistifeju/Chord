import { MenuItem, PredefinedMenuItem, Submenu } from "@tauri-apps/api/menu";
import { commandEnabled, menuItem } from "../../../app/commands";
import { settingsOpen, shortcutChanges } from "../../settings/store";
import { projects } from "../../worktrees/store";
import { builds } from "./store";

const groups = [["build.run", "build.start"], ["build.clean"], ["build.stop"], ["build.showErrors"]];

export async function productMenu(): Promise<Submenu> {
  const ids = groups.flat();
  const items = await Promise.all(ids.map((id) => MenuItem.new({ ...menuItem(id, ""), enabled: commandEnabled(id) })));
  const enabled = ids.map(commandEnabled);
  const refreshEnabled = () =>
    ids.forEach((id, i) => {
      if (commandEnabled(id) === enabled[i]) return;
      enabled[i] = !enabled[i];
      items[i].setEnabled(enabled[i]).catch(console.error);
    });
  for (const store of [builds, projects, settingsOpen]) store.subscribe(refreshEnabled);
  shortcutChanges.subscribe(() => ids.forEach((id, i) => items[i].setAccelerator(menuItem(id, "").accelerator ?? null).catch(console.error)));

  const entries: (MenuItem | PredefinedMenuItem)[] = [];
  for (const group of groups) {
    if (entries.length) entries.push(await PredefinedMenuItem.new({ item: "Separator" }));
    entries.push(...group.map((id) => items[ids.indexOf(id)]));
  }
  return Submenu.new({ text: "Product", items: entries });
}

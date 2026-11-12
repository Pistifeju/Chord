// The appearance commands for the commands table.
import type { AppCommand } from "../../app/commands";
import { sheet } from "../../app/sheet";
import { changeTextSize, closeSettings, openSettings, settingsOpen } from "./store";

export const settingsCommands: AppCommand[] = [
  { id: "text.larger", title: "Increase Text Size", shortcut: { key: "=", cmd: true }, run: () => changeTextSize("larger") },
  { id: "text.smaller", title: "Decrease Text Size", shortcut: { key: "-", cmd: true }, run: () => changeTextSize("smaller") },
  { id: "text.reset", title: "Reset Text Size", shortcut: { key: "0", cmd: true }, run: () => changeTextSize("reset") },
  { id: "settings.open", title: "Settings", shortcut: { key: ",", cmd: true }, run: openSettings },
  // A sheet opened over Settings closes first.
  { id: "settings.close", title: "Close Settings", shortcut: { key: "escape" }, when: () => settingsOpen.get() && sheet.get() === null, run: closeSettings },
];

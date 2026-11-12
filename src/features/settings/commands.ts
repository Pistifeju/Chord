// The appearance commands for the commands table.
import type { AppCommand } from "../../app/commands";
import { changeTextSize } from "./store";

export const settingsCommands: AppCommand[] = [
    { id: "text.larger", title: "Increase Text Size", shortcut: { key: "=", cmd: true }, run: () => changeTextSize("larger") },
  { id: "text.smaller", title: "Decrease Text Size", shortcut: { key: "-", cmd: true }, run: () => changeTextSize("smaller") },
  { id: "text.reset", title: "Reset Text Size", shortcut: { key: "0", cmd: true }, run: () => changeTextSize("reset") },
];

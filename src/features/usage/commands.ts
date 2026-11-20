// The Usage panel's commands for the commands table, and its open and close.
import type { AppCommand } from "../../app/commands";
import { sheet } from "../../app/sheet";
import { settingsOpen } from "../settings/store";
import { usagePanelOpen } from "./store";

// Settings and the Usage panel take the same place over the window, so one closes the other.
export function openUsagePanel() {
  settingsOpen.set(false);
  usagePanelOpen.set(true);
}

export const closeUsagePanel = () => usagePanelOpen.set(false);

export const usageCommands: AppCommand[] = [
  { id: "usage.open", title: "Usage", shortcut: { key: "u", cmd: true, shift: true }, run: openUsagePanel },
  // A sheet opened over the panel closes first.
  { id: "usage.close", title: "Close Usage", shortcut: { key: "escape" }, when: () => usagePanelOpen.get() && sheet.get() === null, run: closeUsagePanel },
];

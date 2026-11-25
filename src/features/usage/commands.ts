import type { AppCommand } from "../../app/commands";
import { sheet } from "../../app/sheet";
import { settingsOpen } from "../settings/store";
import { usagePanelOpen } from "./store";

export function openUsagePanel() {
  settingsOpen.set(false);
  usagePanelOpen.set(true);
}

export const closeUsagePanel = () => usagePanelOpen.set(false);

export const usageCommands: AppCommand[] = [
  { id: "usage.open", title: "Usage", shortcut: { key: "u", cmd: true, shift: true }, run: openUsagePanel },
  { id: "usage.close", title: "Close Usage", shortcut: { key: "escape" }, when: () => usagePanelOpen.get() && sheet.get() === null, run: closeUsagePanel },
];

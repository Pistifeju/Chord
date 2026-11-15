// The commands table: every feature's commands, the shortcuts that run them, their menu accelerators and tooltips.
import { Menu } from "@tauri-apps/api/menu";
import type { MouseEvent } from "react";
import { commands } from "../ipc/bindings";
import { chatCommands } from "../features/chat/commands";
import { buildCommands } from "../features/ios/build/commands";
import { filesCommands } from "../features/files/commands";
import { paneCommands } from "../features/panes/commands";
import { settingsCommands } from "../features/settings/commands";
import { shortcutChanges } from "../features/settings/store";
import { sidebarCommands } from "../features/sidebar/commands";
import { terminalCommands } from "../features/terminal/commands";
import { worktreeCommands } from "../features/worktrees/commands";
import { changedShortcut, currentShortcuts, loadedChanges, pressMatches, type Shortcut, type ShortcutChanges, type ShortcutMap } from "./shortcuts";

export type AppCommand = { id: string; title: string; shortcut?: Shortcut; run: (target?: string) => void;
  /** Escape and other keys shared by commands: the first one whose condition holds runs. */
  when?: () => boolean;
};

const table: AppCommand[] = [...sidebarCommands, ...filesCommands, ...terminalCommands, ...settingsCommands, ...chatCommands, ...paneCommands, ...worktreeCommands, ...buildCommands];

const defaultShortcuts: ShortcutMap = Object.fromEntries(table.map((c) => [c.id, c.shortcut]));

// The key a command has now: its default, or the one the user chose.
const shortcutOf = (id: string): Shortcut | undefined => currentShortcuts(defaultShortcuts, shortcutChanges.get())[id];

export function loadShortcutChanges(saved: ShortcutChanges) {
  shortcutChanges.set(loadedChanges(defaultShortcuts, saved));
}

// Returns the titles of the commands that lost the key.
export function changeShortcut(id: string, shortcut: Shortcut | null): string[] {
  const { changes, lost } = changedShortcut(defaultShortcuts, shortcutChanges.get(), id, shortcut);
  shortcutChanges.set(changes);
  commands.changeState({ shortcuts: changes }).catch(console.error);
  return table.filter((c) => lost.includes(c.id)).map((c) => c.title);
}

export const resetShortcut = (id: string) => changeShortcut(id, defaultShortcuts[id] ?? null);

export const commandTitles = table.map(({ id, title }) => ({ id, title }));

export function runShortcut(e: KeyboardEvent) {
  const current = currentShortcuts(defaultShortcuts, shortcutChanges.get());
  const command = table.find((c) => current[c.id] && pressMatches(current[c.id]!, e) && (c.when?.() ?? true));
  if (!command || e.repeat) return;
  e.preventDefault();
  command.run();
}

const modifiers = [
  ["ctrl", "⌃", "Ctrl"],
  ["alt", "⌥", "Alt"],
  ["shift", "⇧", "Shift"],
  ["cmd", "⌘", "Cmd"],
] as const;

const keySymbols: Record<string, string> = { backspace: "⌫" };

const keyParts = (s: Shortcut, form: 1 | 2) => [...modifiers.filter(([flag]) => s[flag]).map((m) => m[form]), (form === 1 && keySymbols[s.key]) || s.key.toUpperCase()];

export function shortcutLabel(id: string): string | undefined {
  const s = shortcutOf(id);
  return s && keyParts(s, 1).join("");
}

export function tooltip(id: string): string {
  const command = table.find((c) => c.id === id);
  const keys = shortcutLabel(id);
  return command && keys ? `${command.title} (${keys})` : id;
}

const accelerator = (s: Shortcut) => keyParts(s, 2).join("+");

// A native menu item that runs a command on the item it was opened for.
export function menuItem(id: string, target: string) {
  const command = table.find((c) => c.id === id);
  const shortcut = shortcutOf(id);
  return {
    id,
    text: command?.title ?? id,
    accelerator: shortcut && accelerator(shortcut),
    action: () => command?.run(target),
  };
}

// Opens a context menu of commands, each run on its target.
export async function showMenu(event: MouseEvent, items: [commandId: string, target: string][]) {
  event.preventDefault();
  event.stopPropagation();
  const menu = await Menu.new({ items: items.map(([id, target]) => menuItem(id, target)) });
  await menu.popup();
}

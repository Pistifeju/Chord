// The commands table: every feature's commands, the shortcuts that run them and their tooltips.
import { sidebarCommands } from "../features/sidebar/commands";
import { worktreeCommands } from "../features/worktrees/commands";

export type Shortcut = { key: string; cmd?: boolean; ctrl?: boolean; alt?: boolean; shift?: boolean };

export type AppCommand = { id: string; title: string; shortcut?: Shortcut; run: (target?: string) => void };

const table: AppCommand[] = [...sidebarCommands, ...worktreeCommands];

const matches = (s: Shortcut, e: KeyboardEvent) =>
  e.key.toLowerCase() === s.key &&
  e.metaKey === !!s.cmd &&
  e.ctrlKey === !!s.ctrl &&
  e.altKey === !!s.alt &&
  e.shiftKey === !!s.shift;

export function runShortcut(e: KeyboardEvent) {
  const command = table.find((c) => c.shortcut && matches(c.shortcut, e));
  if (!command || e.repeat) return;
  e.preventDefault();
  command.run();
}

export function shortcutLabel(id: string): string | undefined {
  const s = table.find((c) => c.id === id)?.shortcut;
  return s && `${s.ctrl ? "⌃" : ""}${s.alt ? "⌥" : ""}${s.shift ? "⇧" : ""}${s.cmd ? "⌘" : ""}${s.key.toUpperCase()}`;
}

export function tooltip(id: string): string {
  const command = table.find((c) => c.id === id);
  const keys = shortcutLabel(id);
  return command && keys ? `${command.title} (${keys})` : id;
}

// A context-menu item that runs a command on the item it was opened for.
export function menuItem(id: string, target: string) {
  const command = table.find((c) => c.id === id);
  return { id, text: command?.title ?? id, action: () => command?.run(target) };
}

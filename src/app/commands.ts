// The commands table: every feature's commands, the shortcuts that run them and their tooltips.
import { sidebarCommands } from "../features/sidebar/commands";
import { worktreeCommands } from "../features/worktrees/commands";

export type Shortcut = { key: string; cmd?: boolean; ctrl?: boolean; alt?: boolean; shift?: boolean };

export type AppCommand = { id: string; title: string; shortcut: Shortcut; run: () => void };

const table: AppCommand[] = [...sidebarCommands, ...worktreeCommands];

const matches = (s: Shortcut, e: KeyboardEvent) =>
  e.key.toLowerCase() === s.key &&
  e.metaKey === !!s.cmd &&
  e.ctrlKey === !!s.ctrl &&
  e.altKey === !!s.alt &&
  e.shiftKey === !!s.shift;

export function runShortcut(e: KeyboardEvent) {
  const command = table.find((c) => matches(c.shortcut, e));
  if (!command || e.repeat) return;
  e.preventDefault();
  command.run();
}

export function tooltip(id: string): string {
  const command = table.find((c) => c.id === id);
  if (!command) return id;
  const s = command.shortcut;
  const keys = `${s.ctrl ? "⌃" : ""}${s.alt ? "⌥" : ""}${s.shift ? "⇧" : ""}${s.cmd ? "⌘" : ""}${s.key.toUpperCase()}`;
  return `${command.title} (${keys})`;
}

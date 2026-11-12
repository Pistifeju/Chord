// Rules for shortcuts: matching a key press, recording one, and the saved changes to the default keys.
export type Shortcut = { key: string; cmd?: boolean; ctrl?: boolean; alt?: boolean; shift?: boolean };

// A command id to its key. A command without a key is absent.
export type ShortcutMap = Record<string, Shortcut | undefined>;

// The changes to the defaults, by command id. null means the command lost its key.
export type ShortcutChanges = Record<string, Shortcut | null>;

type KeyPress = Pick<KeyboardEvent, "key" | "metaKey" | "ctrlKey" | "altKey" | "shiftKey">;

export const pressMatches = (s: Shortcut, e: KeyPress) =>
  e.key.toLowerCase() === s.key && e.metaKey === !!s.cmd && e.ctrlKey === !!s.ctrl && e.altKey === !!s.alt && e.shiftKey === !!s.shift;

export const sameShortcut = (a: Shortcut | undefined, b: Shortcut | undefined) =>
  !!a && !!b && a.key === b.key && !!a.cmd === !!b.cmd && !!a.ctrl === !!b.ctrl && !!a.alt === !!b.alt && !!a.shift === !!b.shift;

const modifierKeys = ["meta", "control", "alt", "shift"];

// A press worth saving holds cmd, ctrl or alt, so it never steals typing. Rust checks the same rule on load.
export function recordedShortcut(e: KeyPress): Shortcut | undefined {
  const key = e.key.toLowerCase();
  if (modifierKeys.includes(key) || key.trim() === "" || !(e.metaKey || e.ctrlKey || e.altKey)) return undefined;
  return { key, cmd: e.metaKey, ctrl: e.ctrlKey, alt: e.altKey, shift: e.shiftKey };
}

export function currentShortcuts(defaults: ShortcutMap, changes: ShortcutChanges): ShortcutMap {
  const current: ShortcutMap = { ...defaults };
  for (const [id, change] of Object.entries(changes)) if (id in defaults) current[id] = change ?? undefined;
  return current;
}

// Commands may share a default key (they run in different contexts); a changed key must be unique.
function hasTwoCommandsOnOneKey(defaults: ShortcutMap, map: ShortcutMap) {
  const ids = Object.keys(map).filter((id) => map[id] !== undefined);
  return ids.some((id, i) => ids.some((other, j) => j !== i && sameShortcut(map[id], map[other]) && !(sameShortcut(map[id], defaults[id]) && sameShortcut(map[other], defaults[other]))));
}

// Saved changes for commands that exist. If they put two commands on one key, none of them load.
export function loadedChanges(defaults: ShortcutMap, saved: ShortcutChanges): ShortcutChanges {
  const known = Object.fromEntries(Object.entries(saved).filter(([id]) => id in defaults));
  return hasTwoCommandsOnOneKey(defaults, currentShortcuts(defaults, known)) ? {} : known;
}

// Gives a command a key, or null for no key. The command that held the key loses it and is named in `lost`.
export function changedShortcut(
  defaults: ShortcutMap,
  changes: ShortcutChanges,
  id: string,
  shortcut: Shortcut | null,
): { changes: ShortcutChanges; lost?: string } {
  const current = currentShortcuts(defaults, changes);
  const lost = shortcut ? Object.keys(current).find((other) => other !== id && sameShortcut(current[other], shortcut)) : undefined;
  const next: ShortcutChanges = { ...changes, [id]: shortcut };
  if (lost) next[lost] = null;
  const isDefault = (changed: string) => (next[changed] === null ? !defaults[changed] : sameShortcut(next[changed] ?? undefined, defaults[changed]));
  for (const changed of [id, lost]) if (changed && isDefault(changed)) delete next[changed];
  return { changes: next, lost };
}

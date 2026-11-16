// The theme, the text size, the terminal font size, keep-awake, the changed shortcuts, whether the Settings page is open and the section it opens on. The first four are saved through Rust; the theme and text size are applied to the document.
import { commands, type Appearance_Serialize as Appearance, type Theme } from "../../ipc/bindings";
import type { ShortcutChanges } from "../../app/shortcuts";
import { externalStore } from "../../ui/externalStore";
import { defaultTextPercent, shownTheme, steppedTerminalFontSize, steppedTextPercent } from "./appearance";

export const appearance = externalStore<Appearance>({ theme: "system", textPercent: defaultTextPercent, terminalFontSize: 12 });
export const keepAwake = externalStore(false);
export const shortcutChanges = externalStore<ShortcutChanges>({});
export const settingsOpen = externalStore(false);
export type SettingsSection = "general" | "appearance" | "shortcuts" | "simulators";
// The section Settings opens on.
export const settingsSection = externalStore<SettingsSection>("general");

const systemDark = window.matchMedia("(prefers-color-scheme: dark)");

function apply({ theme, textPercent }: Appearance) {
  const root = document.documentElement;
  root.dataset.theme = shownTheme(theme, systemDark.matches);
  root.style.setProperty("--text-scale", String(textPercent / 100));
}

document.documentElement.dataset.theme = shownTheme("system", systemDark.matches);
systemDark.addEventListener("change", () => apply(appearance.get()));

export function loadAppearance(saved: Appearance) {
  appearance.set(saved);
  apply(saved);
}

export function setTheme(theme: Theme) {
  loadAppearance({ ...appearance.get(), theme });
  commands.changeState({ theme }).catch(console.error);
}

export function changeTextSize(direction: "larger" | "smaller" | "reset") {
  const textPercent = steppedTextPercent(appearance.get().textPercent, direction);
  loadAppearance({ ...appearance.get(), textPercent });
  commands.changeState({ textPercent }).catch(console.error);
}

export function changeTerminalFontSize(step: 1 | -1) {
  const terminalFontSize = steppedTerminalFontSize(appearance.get().terminalFontSize, step);
  appearance.set({ ...appearance.get(), terminalFontSize });
  commands.changeState({ terminalFontSize }).catch(console.error);
}

// Rust refuses the switch when caffeinate is missing, so the row shows why instead of a setting that does nothing.
export async function setKeepAwake(enabled: boolean): Promise<string | null> {
  try {
    await commands.setKeepAwake(enabled);
    keepAwake.set(enabled);
    return null;
  } catch (error) {
    return String(error);
  }
}

export function openSettings() {
  settingsSection.set("general");
  settingsOpen.set(true);
}

export function openShortcuts() {
  settingsSection.set("shortcuts");
  settingsOpen.set(true);
}

export function closeSettings() {
  settingsOpen.set(false);
}

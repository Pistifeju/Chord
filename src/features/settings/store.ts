// The theme, the text size, the terminal and editor font sizes, keep-awake, notifications, where links open, the changed shortcuts, whether the Settings page is open and the section it opens on. The first eight are saved through Rust; the theme, the text size and the editor font size are applied to the document.
import { commands, type Appearance_Serialize as Appearance, type Theme } from "../../ipc/bindings";
import type { ShortcutChanges } from "../../app/shortcuts";
import { externalStore } from "../../ui/externalStore";
import { defaultTextPercent, shownTheme, steppedFontSize, steppedTextPercent } from "./appearance";

export const appearance = externalStore<Appearance>({ theme: "system", textPercent: defaultTextPercent, terminalFontSize: 12, editorFontSize: 12 });
export const keepAwake = externalStore(false);
export const muteNotifications = externalStore(false);
export const linksInDefaultBrowser = externalStore(false);
export const shortcutChanges = externalStore<ShortcutChanges>({});
export const settingsOpen = externalStore(false);
export type SettingsSection = "general" | "agents" | "appearance" | "shortcuts" | "simulators";
// The section Settings opens on.
export const settingsSection = externalStore<SettingsSection>("general");

const systemDark = window.matchMedia("(prefers-color-scheme: dark)");

function apply({ theme, textPercent, editorFontSize }: Appearance) {
  const root = document.documentElement;
  root.dataset.theme = shownTheme(theme, systemDark.matches);
  root.style.setProperty("--text-scale", String(textPercent / 100));
  root.style.setProperty("--editor-font-size", `${editorFontSize}px`);
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

export type FontSizeSetting = "terminalFontSize" | "editorFontSize";

export function changeFontSize(setting: FontSizeSetting, step: 1 | -1) {
  const size = steppedFontSize(appearance.get()[setting], step);
  loadAppearance({ ...appearance.get(), [setting]: size });
  commands.changeState(setting === "terminalFontSize" ? { terminalFontSize: size } : { editorFontSize: size }).catch(console.error);
}

export function setMuteNotifications(mute: boolean) {
  muteNotifications.set(mute);
  commands.changeState({ muteNotifications: mute }).catch(console.error);
}

export function setLinksInDefaultBrowser(outside: boolean) {
  linksInDefaultBrowser.set(outside);
  commands.changeState({ linksInDefaultBrowser: outside }).catch(console.error);
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

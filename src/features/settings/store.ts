// The theme, the text size, the changed shortcuts and whether the Settings page is open. The theme and text size are saved through Rust and applied to the document.
import { commands, type Appearance_Serialize as Appearance, type Theme } from "../../ipc/bindings";
import type { ShortcutChanges } from "../../app/shortcuts";
import { externalStore } from "../../ui/externalStore";
import { defaultTextPercent, shownTheme, steppedTextPercent } from "./appearance";

export const appearance = externalStore<Appearance>({ theme: "system", textPercent: defaultTextPercent });
export const shortcutChanges = externalStore<ShortcutChanges>({});
export const settingsOpen = externalStore(false);

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

export function openSettings() {
  settingsOpen.set(true);
}

// Rules for the text size and the terminal and editor font size steps and which theme the window shows.
import type { Theme } from "../../ipc/bindings";

export const defaultTextPercent = 100;
const minTextPercent = 80;
const maxTextPercent = 150;
const textPercentStep = 10;

export function steppedTextPercent(current: number, direction: "larger" | "smaller" | "reset"): number {
  if (direction === "reset") return defaultTextPercent;
  const next = current + (direction === "larger" ? textPercentStep : -textPercentStep);
  return Math.min(maxTextPercent, Math.max(minTextPercent, next));
}

const minFontSize = 9;
const maxFontSize = 24;

export const steppedFontSize = (current: number, step: 1 | -1) => Math.min(maxFontSize, Math.max(minFontSize, current + step));

export function shownTheme(theme: Theme, systemIsDark: boolean): "light" | "dark" {
  return theme === "system" ? (systemIsDark ? "dark" : "light") : theme;
}

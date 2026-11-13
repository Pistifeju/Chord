// Rules for the subscription usage in the status bar: which windows show and how a window reads.
import type { UsageWindow } from "../../ipc/bindings";
import type { FillLevel } from "../chat/context";

const MINUTE = 60000;
const DAY = 86400000;

/** The windows still open at `now`; one whose reset has passed is stale and hidden. */
export const openWindows = (windows: UsageWindow[], now: number): UsageWindow[] => windows.filter((window) => window.resets_at == null || window.resets_at > now);

/** The share of the window used, read from its label ("5h 42%" is 42); none when the label has no percentage. */
export function usedPercent({ label }: UsageWindow): number | undefined {
  const match = label.match(/(\d+(?:\.\d+)?)%/);
  return match ? Number(match[1]) : undefined;
}

/** The bar warns above 70 percent and turns critical above 85. */
export const usageLevel = (percent: number): FillLevel => (percent > 85 ? "critical" : percent > 70 ? "warning" : "calm");

/** The time until a reset in its two largest units: "4d 6h", "2h 48m", "12m". */
function timeLeft(ms: number): string {
  const minutes = Math.max(0, Math.floor(ms / MINUTE));
  const days = Math.floor(minutes / 1440);
  const hours = Math.floor(minutes / 60) % 24;
  if (days > 0) return `${days}d ${hours}h`;
  if (hours > 0) return `${hours}h ${minutes % 60}m`;
  return `${minutes}m`;
}

/** The window as the bar shows it: the percentage and the time left, "42% 2h 48m". */
export function usageText(window: UsageWindow, now: number): string {
  const percent = usedPercent(window);
  const used = percent == null ? window.label : `${percent}%`;
  return window.resets_at == null ? used : `${used} ${timeLeft(window.resets_at - now)}`;
}

/** The window with its name and reset as a clock time, for the tooltip: "5h 42% (resets 14:30)". */
export function usageTitle({ label, resets_at }: UsageWindow, now: number): string {
  if (resets_at == null) return label;
  const at = new Date(resets_at);
  const time = { hour: "2-digit", minute: "2-digit" } as const;
  const clock = resets_at - now < DAY ? at.toLocaleTimeString([], time) : at.toLocaleString([], { weekday: "short", ...time });
  return `${label} (resets ${clock})`;
}

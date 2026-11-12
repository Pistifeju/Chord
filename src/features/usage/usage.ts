// Rules for the subscription usage in the status bar: which windows show and how a window reads.
import type { UsageWindow } from "../../ipc/bindings";

const DAY = 86400000;

/** The windows still open at `now`; one whose reset has passed is stale and hidden. */
export const openWindows = (windows: UsageWindow[], now: number): UsageWindow[] => windows.filter((window) => window.resets_at == null || window.resets_at > now);

/** The reset as a clock time, with the weekday from a day away; a clock time stays right between updates, a countdown does not. */
export function usageText({ label, resets_at }: UsageWindow, now: number): string {
  if (resets_at == null) return label;
  const at = new Date(resets_at);
  const time = { hour: "2-digit", minute: "2-digit" } as const;
  const clock = resets_at - now < DAY ? at.toLocaleTimeString([], time) : at.toLocaleString([], { weekday: "short", ...time });
  return `${label} (resets ${clock})`;
}

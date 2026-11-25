import type { UsageWindow } from "../../ipc/bindings";
import type { FillLevel } from "../chat/context";

const MINUTE = 60000;
const DAY = 86400000;

export const openWindows = (windows: UsageWindow[], now: number): UsageWindow[] => windows.filter((window) => window.resets_at == null || window.resets_at > now);

export function usedPercent({ label }: UsageWindow): number | undefined {
  const match = label.match(/(\d+(?:\.\d+)?)%/);
  return match ? Number(match[1]) : undefined;
}

export const usageLevel = (percent: number): FillLevel => (percent > 85 ? "critical" : percent > 70 ? "warning" : "calm");

function timeLeft(ms: number): string {
  const minutes = Math.max(0, Math.floor(ms / MINUTE));
  const days = Math.floor(minutes / 1440);
  const hours = Math.floor(minutes / 60) % 24;
  if (days > 0) return `${days}d ${hours}h`;
  if (hours > 0) return `${hours}h ${minutes % 60}m`;
  return `${minutes}m`;
}

export function usageText(window: UsageWindow, now: number): string {
  const percent = usedPercent(window);
  const used = percent == null ? window.label : `${percent}%`;
  return window.resets_at == null ? used : `${used} ${timeLeft(window.resets_at - now)}`;
}

export function usageTitle({ label, resets_at }: UsageWindow, now: number): string {
  if (resets_at == null) return label;
  const at = new Date(resets_at);
  const time = { hour: "2-digit", minute: "2-digit" } as const;
  const clock = resets_at - now < DAY ? at.toLocaleTimeString([], time) : at.toLocaleString([], { weekday: "short", ...time });
  return `${label} (resets ${clock})`;
}

// Rules for the running simulators and the build data: the button's tooltip, a menu line, whether Shut Down Others has any, and how a size reads.
import type { RunningSimulator } from "../../../ipc/bindings";

export const simulatorsTitle = (count: number): string => `${count} ${count === 1 ? "Simulator" : "Simulators"} Running`;

export const simulatorLine = (simulator: RunningSimulator): string => `${simulator.worktree} · ${simulator.model}`;

export const hasOtherSimulators = (running: RunningSimulator[]): boolean => running.some((simulator) => !simulator.open);

/** Decimal units, as macOS shows file sizes: "12.4 GB", "512 MB". */
export function sizeText(bytes: number): string {
  if (bytes >= 1e9) return `${(bytes / 1e9).toFixed(1)} GB`;
  if (bytes >= 1e6) return `${Math.round(bytes / 1e6)} MB`;
  return `${Math.round(bytes / 1e3)} KB`;
}

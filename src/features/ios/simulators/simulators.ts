// Rules for the running simulators, the build data and the service groups: the button's tooltip, a menu line, whether Shut Down Others has any, how a size reads and what a service group row says.
import type { RunningSimulator, ServiceRow } from "../../../ipc/bindings";

export const simulatorsTitle = (count: number): string => `${count} ${count === 1 ? "Simulator" : "Simulators"} Running`;

export const simulatorLine = (simulator: RunningSimulator): string => `${simulator.worktree} · ${simulator.model}`;

export const hasOtherSimulators = (running: RunningSimulator[]): boolean => running.some((simulator) => !simulator.open);

/** Decimal units, as macOS shows file sizes: "12.4 GB", "512 MB". */
export function sizeText(bytes: number): string {
  if (bytes >= 1e9) return `${(bytes / 1e9).toFixed(1)} GB`;
  if (bytes >= 1e6) return `${Math.round(bytes / 1e6)} MB`;
  return `${Math.round(bytes / 1e3)} KB`;
}

/** What a row says about a group or subgroup: what stops working and the memory it frees; a group with some subgroups on says how many. */
export function serviceDescription(row: Pick<ServiceRow, "effect" | "memoryMb">, subgroups: { kept: boolean }[] = []): string {
  const on = subgroups.filter((subgroup) => subgroup.kept).length;
  const mixed = on > 0 && on < subgroups.length ? ` ${on} of ${subgroups.length} on.` : "";
  return `${row.effect} Frees about ${row.memoryMb} MB.${mixed}`;
}

/** The ids a group's switch writes: its subgroups, or the group's own id when it has none. */
export const groupSwitchIds = (group: Pick<ServiceRow, "id" | "subgroups">): string[] => (group.subgroups.length > 0 ? group.subgroups.map((subgroup) => subgroup.id) : [group.id]);

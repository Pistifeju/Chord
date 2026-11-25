import type { RunningSimulator, ServiceRow } from "../../../ipc/bindings";

export const simulatorsTitle = (count: number): string => `${count} ${count === 1 ? "Simulator" : "Simulators"} Running`;

export const simulatorLine = (simulator: RunningSimulator): string => [simulator.worktree, ...(simulator.preview ? ["Preview"] : []), simulator.model].join(" · ");

export const hasOtherSimulators = (running: RunningSimulator[]): boolean => running.some((simulator) => !simulator.open);

export function sizeText(bytes: number): string {
  if (bytes >= 1e9) return `${(bytes / 1e9).toFixed(1)} GB`;
  if (bytes >= 1e6) return `${Math.round(bytes / 1e6)} MB`;
  return `${Math.round(bytes / 1e3)} KB`;
}

export function serviceDescription(row: Pick<ServiceRow, "effect" | "memoryMb">, subgroups: { kept: boolean }[] = []): string {
  const on = subgroups.filter((subgroup) => subgroup.kept).length;
  const mixed = on > 0 && on < subgroups.length ? ` ${on} of ${subgroups.length} on.` : "";
  return `${row.effect} Frees about ${row.memoryMb} MB.${mixed}`;
}

export const groupSwitchIds = (group: Pick<ServiceRow, "id" | "subgroups">): string[] => (group.subgroups.length > 0 ? group.subgroups.map((subgroup) => subgroup.id) : [group.id]);

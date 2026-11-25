import type { Destinations, MacSupport, RunDestination } from "../../../ipc/bindings";

export const myMac: RunDestination = { kind: "mac", name: "My Mac" };

export const fallbackDestination = (mac: MacSupport | undefined, simulator: RunDestination | null): RunDestination | null => (mac === "only" ? myMac : simulator);

const destinationLabel = (destination: RunDestination): string =>
  destination.kind === "simulator" ? `${destination.name}, ${destination.os}` : destination.name;

export function sameDestination(a: RunDestination, b: RunDestination): boolean {
  if (a.kind === "device") return b.kind === "device" && a.id === b.id;
  if (a.kind === "simulator") return b.kind === "simulator" && a.model === b.model && a.runtime === b.runtime;
  return b.kind === "mac";
}

const listed = (all: Destinations): RunDestination[] => [...all.devices, ...all.simulators.flatMap((group) => group.models)];

const isAvailable = (pick: RunDestination, all: Destinations | null, mac: MacSupport | undefined): boolean =>
  pick.kind === "mac" ? mac !== "none" : mac !== "only" && (!all || listed(all).some((destination) => sameDestination(destination, pick)));

export function pickAfterListing(pick: RunDestination, all: Destinations | null, mac?: MacSupport): { pick: RunDestination | null; notice?: string } {
  if (isAvailable(pick, all, mac)) return { pick };
  const fallback = mac === "only" ? myMac : (all?.simulators[0]?.models[0] ?? null);
  const gone = `${destinationLabel(pick)} is not available.`;
  return { pick: fallback, notice: fallback ? `${gone} Using ${destinationLabel(fallback)}.` : gone };
}

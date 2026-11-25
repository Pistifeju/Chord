import type { Destinations, RunDestination } from "../../../ipc/bindings";

const destinationLabel = (destination: RunDestination): string =>
  destination.kind === "device" ? destination.name : `${destination.name}, ${destination.os}`;

export const sameDestination = (a: RunDestination, b: RunDestination): boolean =>
  a.kind === "device" ? b.kind === "device" && a.id === b.id : b.kind === "simulator" && a.model === b.model && a.runtime === b.runtime;

const listed = (all: Destinations): RunDestination[] => [...all.devices, ...all.simulators.flatMap((group) => group.models)];

export function pickAfterListing(pick: RunDestination, all: Destinations): { pick: RunDestination | null; notice?: string } {
  if (listed(all).some((destination) => sameDestination(destination, pick))) return { pick };
  const fallback = all.simulators[0]?.models[0] ?? null;
  const gone = `${destinationLabel(pick)} is not available.`;
  return { pick: fallback, notice: fallback ? `${gone} Using ${destinationLabel(fallback)}.` : gone };
}

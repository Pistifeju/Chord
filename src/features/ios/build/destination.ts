// Rules for the run destination: how one reads, which one the listing still has, and where a gone pick falls back to.
import type { Destinations, RunDestination } from "../../../ipc/bindings";

const destinationLabel = (destination: RunDestination): string =>
  destination.kind === "device" ? destination.name : `${destination.name}, ${destination.os}`;

export const sameDestination = (a: RunDestination, b: RunDestination): boolean =>
  a.kind === "device" ? b.kind === "device" && a.id === b.id : b.kind === "simulator" && a.model === b.model && a.runtime === b.runtime;

const listed = (all: Destinations): RunDestination[] => [...all.devices, ...all.simulators.flatMap((group) => group.models)];

/** The pick the project keeps once the listing is known: the pick while it exists, else the first simulator, else none; with the notice that names both when it changed. */
export function pickAfterListing(pick: RunDestination, all: Destinations): { pick: RunDestination | null; notice?: string } {
  if (listed(all).some((destination) => sameDestination(destination, pick))) return { pick };
  const fallback = all.simulators[0]?.models[0] ?? null;
  const gone = `${destinationLabel(pick)} is not available.`;
  return { pick: fallback, notice: fallback ? `${gone} Using ${destinationLabel(fallback)}.` : gone };
}

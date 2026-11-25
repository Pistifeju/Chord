import { useEffect } from "react";
import { commands, type Destinations, type RunDestination } from "../../../ipc/bindings";
import { externalStore } from "../../../ui/externalStore";
import { notify } from "../../../ui/alert";
import { projectPicks } from "../projectPicks";
import { fallbackDestination, pickAfterListing } from "./destination";
import { schemeListings } from "./schemeStore";

const runDestinations = projectPicks<RunDestination | null>((project, destination) => commands.changeState({ runDestination: { project, destination } }));
export const destinations = externalStore<Destinations | null>(null);

export const loadRunDestinations = (saved: Record<string, RunDestination>) => runDestinations.load(saved);

export const runDestinationOf = (worktree: string): RunDestination | null => runDestinations.of(worktree) ?? null;

export const useRunDestination = (worktree: string): RunDestination | null => runDestinations.use(worktree) ?? null;

export const chooseRunDestination = (worktree: string, destination: RunDestination) => runDestinations.choose(worktree, destination);

export function useFallbackSimulator(worktree: string): RunDestination | null {
  useEffect(() => {
    if (!destinations.get()) refreshDestinations(worktree).catch(console.error);
  }, [worktree]);
  return destinations.usePart((listing) => listing?.fallback ?? null);
}

export function useFallbackDestination(worktree: string): RunDestination | null {
  const simulator = useFallbackSimulator(worktree);
  return fallbackDestination(schemeListings.usePart((all) => all[worktree]?.mac), simulator);
}

function keepPickAvailable(worktree: string) {
  const current = runDestinations.of(worktree);
  if (!current) return;
  const { pick, notice } = pickAfterListing(current, destinations.get(), schemeListings.get()[worktree]?.mac);
  if (!notice) return;
  runDestinations.choose(worktree, pick);
  notify(notice);
}

schemeListings.subscribe(() => Object.keys(schemeListings.get()).forEach(keepPickAvailable));

export async function refreshDestinations(worktree: string) {
  destinations.set(await commands.listDestinations());
  keepPickAvailable(worktree);
}

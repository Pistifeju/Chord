// Connects the run destination rules to IPC: each project's pick, saved through Rust, and the listing of devices and simulators.
import { useEffect } from "react";
import { commands, type Destinations, type RunDestination } from "../../../ipc/bindings";
import { externalStore } from "../../../ui/externalStore";
import { notify } from "../../../ui/alert";
import { projectPicks } from "../projectPicks";
import { pickAfterListing } from "./destination";

/** Each project's pick; a project without one runs on the listing's fallback simulator. */
const runDestinations = projectPicks<RunDestination | null>((project, destination) => commands.changeState({ runDestination: { project, destination } }));
export const destinations = externalStore<Destinations | null>(null);

export const loadRunDestinations = (saved: Record<string, RunDestination>) => runDestinations.load(saved);

export const runDestinationOf = (worktree: string): RunDestination | null => runDestinations.of(worktree) ?? null;

export const useRunDestination = (worktree: string): RunDestination | null => runDestinations.use(worktree) ?? null;

export const chooseRunDestination = (worktree: string, destination: RunDestination) => runDestinations.choose(worktree, destination);

/** The simulator a run or preview without a pick uses; null until the listing comes. */
export function useFallbackSimulator(worktree: string): RunDestination | null {
  useEffect(() => {
    if (!destinations.get()) refreshDestinations(worktree).catch(console.error);
  }, [worktree]);
  return destinations.usePart((listing) => listing?.fallback ?? null);
}

/** Lists the devices and simulators; the worktree's project falls back when its pick is gone. */
export async function refreshDestinations(worktree: string) {
  const listing = await commands.listDestinations();
  destinations.set(listing);
  const current = runDestinations.of(worktree);
  if (!current) return;
  const { pick: kept, notice } = pickAfterListing(current, listing);
  if (!notice) return;
  runDestinations.choose(worktree, kept);
  notify(notice);
}

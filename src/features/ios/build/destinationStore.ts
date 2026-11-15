// Connects the run destination rules to IPC: each project's pick, saved through Rust, and the listing of devices and simulators.
import { commands, type Destinations, type RunDestination } from "../../../ipc/bindings";
import { externalStore } from "../../../ui/externalStore";
import { ownerOf } from "../../worktrees/projects";
import { fail, notify, projects } from "../../worktrees/store";
import { pickAfterListing } from "./destination";

/** Each project's pick; a project without one runs on the newest iPhone simulator. */
const runDestinations = externalStore<Record<string, RunDestination>>({});
export const destinations = externalStore<Destinations | null>(null);

export const loadRunDestinations = (saved: Record<string, RunDestination>) => runDestinations.set(saved);

export const runDestinationOf = (worktree: string): RunDestination | null => runDestinations.get()[ownerOf(projects.get(), worktree) ?? ""] ?? null;

export const useRunDestination = (worktree: string): RunDestination | null => runDestinations.usePart(() => runDestinationOf(worktree));

function pick(project: string, destination: RunDestination | null) {
  const { [project]: before, ...others } = runDestinations.get();
  runDestinations.set(destination ? { ...others, [project]: destination } : others);
  if (destination || before) commands.changeState({ runDestination: { project, destination } }).catch(fail);
}

export function chooseRunDestination(worktree: string, destination: RunDestination) {
  const project = ownerOf(projects.get(), worktree);
  if (project) pick(project, destination);
}

/** Lists the devices and simulators; the worktree's project falls back when its pick is gone. */
export async function refreshDestinations(worktree: string) {
  const listing = await commands.listDestinations();
  destinations.set(listing);
  const project = ownerOf(projects.get(), worktree);
  const current = project && runDestinations.get()[project];
  if (!project || !current) return;
  const { pick: kept, notice } = pickAfterListing(current, listing);
  if (!notice) return;
  pick(project, kept);
  notify(notice);
}

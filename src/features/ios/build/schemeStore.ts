import { commands, type MacSupport, type SchemePick_Serialize as SchemePick, type Schemes } from "../../../ipc/bindings";
import { externalStore } from "../../../ui/externalStore";
import { xcodeWorktree } from "../../worktrees/store";
import { projectPicks } from "../projectPicks";
import { shownBeforeListing, shownPick } from "./scheme";

const noPick: SchemePick = { scheme: null, configuration: null };

const schemePicks = projectPicks<SchemePick>((project, pick) => commands.changeState({ schemePick: { project, pick } }));
const schemeDefaults = projectPicks<SchemePick>((project, pick) => commands.changeState({ schemeDefault: { project, pick } }));
export const schemeListings = externalStore<Record<string, Schemes>>({});
export const macSupports = externalStore<Record<string, MacSupport>>({});

export function loadSchemePicks(saved: Record<string, SchemePick>, defaults: Record<string, SchemePick>) {
  schemePicks.load(saved);
  schemeDefaults.load(defaults);
}

export const useSavedShownPick = (worktree: string): SchemePick => shownBeforeListing(useSchemePick(worktree), schemeDefaults.use(worktree));

export const schemePickOf = (worktree: string): SchemePick => schemePicks.of(worktree) ?? noPick;

export const useSchemePick = (worktree: string): SchemePick => schemePicks.use(worktree) ?? noPick;

export const chooseSchemePick = (worktree: string, change: Partial<SchemePick>) => schemePicks.choose(worktree, { ...schemePickOf(worktree), ...change });

const isMacOnly = (all: Record<string, MacSupport>, worktree: string | null): boolean => !!worktree && all[worktree] === "only";

export const useMacOnly = (worktree: string | null): boolean => macSupports.usePart((all) => isMacOnly(all, worktree));

export const useMacSupport = (worktree: string): MacSupport | undefined => macSupports.usePart((all) => all[worktree]);

export function simulatorWorktree(): string | null {
  const worktree = xcodeWorktree();
  return isMacOnly(macSupports.get(), worktree) ? null : worktree;
}

async function refreshMacSupport(worktree: string) {
  const mac = await commands.macSupport(worktree);
  if (macSupports.get()[worktree] !== mac) macSupports.set({ ...macSupports.get(), [worktree]: mac });
}

export async function refreshSchemes(worktree: string) {
  const [listing] = await Promise.all([commands.listSchemes(worktree), refreshMacSupport(worktree)]);
  schemeListings.set({ ...schemeListings.get(), [worktree]: listing });
  const defaults = shownPick(noPick, listing);
  const saved = schemeDefaults.of(worktree);
  if (saved?.scheme !== defaults.scheme || saved?.configuration !== defaults.configuration) schemeDefaults.choose(worktree, defaults);
}

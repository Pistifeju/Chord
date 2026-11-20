// Connects the scheme and build configuration picks to IPC: each project's pick and the defaults its last listing gave, saved through Rust, and each worktree's listing of schemes and configurations.
import { commands, type SchemePick_Serialize as SchemePick, type Schemes } from "../../../ipc/bindings";
import { externalStore } from "../../../ui/externalStore";
import { projectPicks } from "../projectPicks";
import { shownBeforeListing, shownPick } from "./scheme";

const noPick: SchemePick = { scheme: null, configuration: null };

/** Each project's pick; a project without one builds the default scheme in its default configuration. */
const schemePicks = projectPicks<SchemePick>((project, pick) => commands.changeState({ schemePick: { project, pick } }));
/** What each project's last listing built without a pick, shown while a new listing loads. */
const schemeDefaults = projectPicks<SchemePick>((project, pick) => commands.changeState({ schemeDefault: { project, pick } }));
/** Per worktree: a branch can add or remove a scheme. */
export const schemeListings = externalStore<Record<string, Schemes>>({});

export function loadSchemePicks(saved: Record<string, SchemePick>, defaults: Record<string, SchemePick>) {
  schemePicks.load(saved);
  schemeDefaults.load(defaults);
}

/** The pick over the saved defaults, for the toolbar before the worktree's listing comes. */
export const useSavedShownPick = (worktree: string): SchemePick => shownBeforeListing(useSchemePick(worktree), schemeDefaults.use(worktree));

export const schemePickOf = (worktree: string): SchemePick => schemePicks.of(worktree) ?? noPick;

export const useSchemePick = (worktree: string): SchemePick => schemePicks.use(worktree) ?? noPick;

export const chooseSchemePick = (worktree: string, change: Partial<SchemePick>) => schemePicks.choose(worktree, { ...schemePickOf(worktree), ...change });

export async function refreshSchemes(worktree: string) {
  const listing = await commands.listSchemes(worktree);
  schemeListings.set({ ...schemeListings.get(), [worktree]: listing });
  const defaults = shownPick(noPick, listing);
  const saved = schemeDefaults.of(worktree);
  if (saved?.scheme !== defaults.scheme || saved?.configuration !== defaults.configuration) schemeDefaults.choose(worktree, defaults);
}

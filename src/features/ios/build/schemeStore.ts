// Connects the scheme and build configuration picks to IPC: each project's pick, saved through Rust, and each worktree's listing of schemes and configurations.
import { commands, type SchemePick_Serialize as SchemePick, type Schemes } from "../../../ipc/bindings";
import { externalStore } from "../../../ui/externalStore";
import { projectPicks } from "../projectPicks";

const noPick: SchemePick = { scheme: null, configuration: null };

/** Each project's pick; a project without one builds the default scheme in its default configuration. */
const schemePicks = projectPicks<SchemePick>((project, pick) => commands.changeState({ schemePick: { project, pick } }));
/** Per worktree: a branch can add or remove a scheme. */
export const schemeListings = externalStore<Record<string, Schemes>>({});

export const loadSchemePicks = (saved: Record<string, SchemePick>) => schemePicks.load(saved);

export const schemePickOf = (worktree: string): SchemePick => schemePicks.of(worktree) ?? noPick;

export const useSchemePick = (worktree: string): SchemePick => schemePicks.use(worktree) ?? noPick;

export const chooseSchemePick = (worktree: string, change: Partial<SchemePick>) => schemePicks.choose(worktree, { ...schemePickOf(worktree), ...change });

export async function refreshSchemes(worktree: string) {
  const listing = await commands.listSchemes(worktree);
  schemeListings.set({ ...schemeListings.get(), [worktree]: listing });
}

// Connects the scheme and build configuration picks to IPC: each project's pick, saved through Rust, and each worktree's listing of schemes and configurations.
import { commands, type SchemePick_Serialize as SchemePick, type Schemes } from "../../../ipc/bindings";
import { externalStore } from "../../../ui/externalStore";
import { ownerOf } from "../../worktrees/projects";
import { projects } from "../../worktrees/store";
import { fail } from "../../../ui/alert";

const noPick: SchemePick = { scheme: null, configuration: null };

/** Each project's pick; a project without one builds the default scheme in its default configuration. */
const schemePicks = externalStore<Record<string, SchemePick>>({});
/** Per worktree: a branch can add or remove a scheme. */
export const schemeListings = externalStore<Record<string, Schemes>>({});

export const loadSchemePicks = (saved: Record<string, SchemePick>) => schemePicks.set(saved);

export const schemePickOf = (worktree: string): SchemePick => schemePicks.get()[ownerOf(projects.get(), worktree) ?? ""] ?? noPick;

export function chooseSchemePick(worktree: string, change: Partial<SchemePick>) {
  const project = ownerOf(projects.get(), worktree);
  if (!project) return;
  const pick = { ...schemePickOf(worktree), ...change };
  schemePicks.set({ ...schemePicks.get(), [project]: pick });
  commands.changeState({ schemePick: { project, pick } }).catch(fail);
}

export async function refreshSchemes(worktree: string) {
  const listing = await commands.listSchemes(worktree);
  schemeListings.set({ ...schemeListings.get(), [worktree]: listing });
}

// A pick each project keeps for all its worktrees, such as the run destination or the scheme, read by worktree and saved through Rust.
import { externalStore } from "../../ui/externalStore";
import { ownerOf } from "../worktrees/projects";
import { projects } from "../worktrees/store";
import { fail } from "../../ui/alert";

/** A null pick removes the project's entry, and saves only when the project had one. */
export function projectPicks<T>(save: (project: string, pick: T) => Promise<unknown>) {
  const picks = externalStore<Record<string, T>>({});
  const of = (worktree: string): T | undefined => picks.get()[ownerOf(projects.get(), worktree) ?? ""];
  return {
    load: (saved: Record<string, T>) => picks.set(saved),
    of,
    use: (worktree: string) => picks.usePart(() => of(worktree)),
    choose(worktree: string, pick: T) {
      const project = ownerOf(projects.get(), worktree);
      if (!project) return;
      const { [project]: before, ...others } = picks.get();
      picks.set(pick ? { ...others, [project]: pick } : others);
      if (pick || before) save(project, pick).catch(fail);
    },
  };
}

// A resizable panel's open state and size per worktree, saved through Rust.
import { externalStore } from "./externalStore";

type Panel = { open: boolean };

export function panelStore<K extends string, P extends Panel & Record<K, number>>(config: {
  fallback: P;
  size: K;
  min: number;
  max: number;
  save: (worktree: string, panel: P) => Promise<unknown>;
}) {
  const panels = externalStore<Record<string, P>>({});
  const panelOf = (all: Record<string, P>, worktree: string): P => all[worktree] ?? config.fallback;
  const set = (worktree: string, panel: P) => panels.set({ ...panels.get(), [worktree]: panel });
  const save = (worktree: string) => void config.save(worktree, panelOf(panels.get(), worktree)).catch(console.error);
  return {
    panels,
    panelOf,
    load: (saved: Record<string, P>) => panels.set(saved),
    show(worktree: string, open: boolean) {
      const current = panelOf(panels.get(), worktree);
      if (current.open === open) return;
      set(worktree, { ...current, open });
      save(worktree);
    },
    // A drag changes the size many times and saves once, when it ends.
    resize: (worktree: string, size: number) =>
      set(worktree, { ...panelOf(panels.get(), worktree), [config.size]: Math.round(Math.min(config.max, Math.max(config.min, size))) }),
    endResize: save,
  };
}

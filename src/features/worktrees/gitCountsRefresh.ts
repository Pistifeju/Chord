// Keeps the cards' git counts fresh: on window focus, when a card appears, after a commit or a push, then every 10 seconds while the window is visible. A plain folder whose counts arrive became a repository, so its project's list reads again.
import { commands, type GitCounts_Serialize as GitCounts } from "../../ipc/bindings";
import { externalStore } from "../../ui/externalStore";
import { gainsPath, nextCounts } from "./gitCounts";
import { projectsTurnedGit, worktreeOrder } from "./projects";
import { projects, refreshWorktrees } from "./store";
import { fail } from "../../ui/alert";

const REFRESH_MS = 10_000;

export const gitCounts = externalStore<Record<string, GitCounts>>({});

let refreshNow = () => {};

/** Reads the counts again now, after a commit or a push that changed them. */
export const refreshGitCounts = () => refreshNow();

// The first counts, read once at launch so the cards show with them; a failure is left to the watch to report.
export async function loadGitCounts() {
  const paths = worktreeOrder(projects.get());
  if (paths.length === 0) return;
  const fresh = await commands.readGitCounts(paths).catch(() => null);
  if (fresh) gitCounts.set(nextCounts(gitCounts.get(), worktreeOrder(projects.get()), paths, fresh));
}

export function watchGitCounts(): () => void {
  let timer: number | undefined;
  let running = false;
  let pending = false;
  let stopped = false;
  let queried: string[] = [];
  const visible = () => worktreeOrder(projects.get());

  async function refresh() {
    if (running) return;
    const paths = visible();
    if (paths.length === 0) return;
    running = true;
    pending = false;
    queried = paths;
    try {
      const fresh = await commands.readGitCounts(paths);
      gitCounts.set(nextCounts(gitCounts.get(), worktreeOrder(projects.get()), paths, fresh));
      for (const project of projectsTurnedGit(projects.get(), fresh.map((c) => c.path))) refreshWorktrees(project).catch(fail);
    } catch (error) {
      stopped = true;
      stop();
      fail(error);
    } finally {
      running = false;
    }
    if (pending && !stopped) void refresh();
  }

  const stop = () => {
    window.clearInterval(timer);
    timer = undefined;
  };
  const start = () => {
    stop();
    stopped = false;
    void refresh();
    timer = window.setInterval(() => void refresh(), REFRESH_MS);
  };
  const onVisibility = () => (document.visibilityState === "visible" ? start() : stop());
  const refreshSoon = () => {
    if (stopped) return;
    if (running) pending = true;
    else void refresh();
  };
  const onChange = () => {
    if (document.visibilityState === "visible" && gainsPath(queried, visible())) refreshSoon();
  };
  refreshNow = refreshSoon;

  if (document.visibilityState === "visible") start();
  const unsubscribe = projects.subscribe(onChange);
  window.addEventListener("focus", start);
  document.addEventListener("visibilitychange", onVisibility);
  return () => {
    stop();
    refreshNow = () => {};
    unsubscribe();
    window.removeEventListener("focus", start);
    document.removeEventListener("visibilitychange", onVisibility);
  };
}

// Keeps the cards' git counts fresh: on window focus, when a card appears, after a commit or a push, when an agent's tool call or turn ends, those reads 3 seconds apart, then every minute while the window is visible. A plain folder whose counts arrive became a repository, and a git card whose counts miss may be gone, so its project's list reads again.
import { commands, type GitCounts_Serialize as GitCounts } from "../../ipc/bindings";
import { externalStore } from "../../ui/externalStore";
import { gainsPath, nextCounts } from "./gitCounts";
import { projectsLosingWorktrees, projectsTurnedGit, worktreeOrder } from "./projects";
import { projects, refreshWorktrees } from "./store";
import { fail } from "../../ui/alert";

const REFRESH_MS = 60_000;
// Agents end tool calls back to back, and each read runs git in every worktree.
const AGENT_GAP_MS = 3000;

export const gitCounts = externalStore<Record<string, GitCounts>>({});

let refreshNow = () => {};

/** Reads the counts again now, after a commit or a push that changed them. */
export const refreshGitCounts = () => refreshNow();

let refreshSpaced = () => {};

/** Reads the counts again after an agent's tool call or turn: now, or once `AGENT_GAP_MS` after the last read when one just ran. */
export const refreshGitCountsAfterAgent = () => refreshSpaced();

// The first counts, read once at launch so the cards show with them; a failure is left to the watch to report.
export async function loadGitCounts() {
  const paths = worktreeOrder(projects.get());
  if (paths.length === 0) return;
  const fresh = await commands.readGitCounts(paths).catch(() => null);
  if (fresh) gitCounts.set(nextCounts(gitCounts.get(), worktreeOrder(projects.get()), paths, fresh));
}

export function watchGitCounts(): () => void {
  let timer: number | undefined;
  let gapTimer: number | undefined;
  let ended = -AGENT_GAP_MS;
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
      const read = fresh.map((c) => c.path);
      for (const project of projectsTurnedGit(projects.get(), read)) refreshWorktrees(project).catch(fail);
      // An unmounted project fails this every tick, so it logs instead of alerting.
      for (const project of projectsLosingWorktrees(projects.get(), paths, read)) refreshWorktrees(project).catch(console.error);
    } catch (error) {
      stopped = true;
      stop();
      fail(error);
    } finally {
      running = false;
      ended = performance.now();
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
    // A hidden window reads nothing: its next focus reads.
    if (stopped || document.visibilityState !== "visible") return;
    if (running) pending = true;
    else void refresh();
  };
  const onChange = () => {
    if (gainsPath(queried, visible())) refreshSoon();
  };
  refreshNow = refreshSoon;
  refreshSpaced = () => {
    if (gapTimer !== undefined) return;
    const wait = running ? AGENT_GAP_MS : ended + AGENT_GAP_MS - performance.now();
    if (wait <= 0) return refreshSoon();
    gapTimer = window.setTimeout(() => {
      gapTimer = undefined;
      refreshSoon();
    }, wait);
  };

  if (document.visibilityState === "visible") start();
  const unsubscribe = projects.subscribe(onChange);
  window.addEventListener("focus", start);
  document.addEventListener("visibilitychange", onVisibility);
  return () => {
    stop();
    window.clearTimeout(gapTimer);
    refreshNow = () => {};
    refreshSpaced = () => {};
    unsubscribe();
    window.removeEventListener("focus", start);
    document.removeEventListener("visibilitychange", onVisibility);
  };
}

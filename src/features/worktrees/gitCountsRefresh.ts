// Keeps the cards' git counts fresh: on window focus, when a card appears, then every 10 seconds while the window is visible.
import { commands, type GitCounts_Serialize as GitCounts } from "../../ipc/bindings";
import { externalStore } from "../../ui/externalStore";
import { sidebar } from "../sidebar/store";
import { gainsPath, nextCounts, visibleWorktrees } from "./gitCounts";
import { worktreeOrder } from "./projects";
import { fail, projects } from "./store";

const REFRESH_MS = 10_000;

export const gitCounts = externalStore<Record<string, GitCounts>>({});

export function watchGitCounts(): () => void {
  let timer: number | undefined;
  let running = false;
  let pending = false;
  let stopped = false;
  let queried: string[] = [];
  const visible = () => visibleWorktrees(projects.get(), sidebar.get().folded);

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
  const onChange = () => {
    if (stopped || document.visibilityState !== "visible" || !gainsPath(queried, visible())) return;
    if (running) pending = true;
    else void refresh();
  };

  if (document.visibilityState === "visible") start();
  const unsubscribe = [projects.subscribe(onChange), sidebar.subscribe(onChange)];
  window.addEventListener("focus", start);
  document.addEventListener("visibilitychange", onVisibility);
  return () => {
    stop();
    unsubscribe.forEach((off) => off());
    window.removeEventListener("focus", start);
    document.removeEventListener("visibilitychange", onVisibility);
  };
}

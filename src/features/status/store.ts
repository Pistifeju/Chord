import { listen } from "@tauri-apps/api/event";
import { commands } from "../../ipc/bindings";
import { externalStore } from "../../ui/externalStore";
import { chats } from "../chat/store";
import { shownTabIds } from "../panes/layout";
import { focusPane, panes } from "../panes/store";
import { worktreeOrder } from "../worktrees/projects";
import { projects, selectWorktree, shownPanes, shownPanesStores } from "../worktrees/store";
import { notify } from "../../ui/alert";
import { agentPanes, attentionQueue, nextAttention, type AgentPane } from "./attention";
import { keptMarks, turnedFinished, type PaneKinds } from "./finished";

export const finished = externalStore<string[]>([]);

const kindsNow = (): PaneKinds =>
  Object.fromEntries(Object.entries(chats.get()).flatMap(([pane, chat]) => (chat.report ? [[pane, chat.report.status.kind]] : [])));

function openPanes(): string[] {
  const shown = shownPanes();
  return shown ? shownTabIds(shown.layout) : [];
}

export function watchFinished(): () => void {
  let before = kindsNow();
  const sync = () => {
    const now = kindsNow();
    const marks = keptMarks([...finished.get(), ...turnedFinished(before, now)], now, openPanes());
    before = now;
    if (marks.join() !== finished.get().join()) finished.set(marks);
  };
  let seen = chats.get();
  const syncReports = () => {
    const all = chats.get();
    const panes = Object.keys(all);
    const same = panes.length === Object.keys(seen).length && panes.every((pane) => all[pane].report === seen[pane]?.report);
    seen = all;
    if (!same) sync();
  };
  const unsubscribe = [chats.subscribe(syncReports), ...shownPanesStores().map((store) => store.subscribe(sync))];
  return () => unsubscribe.forEach((off) => off());
}

export function openAgent({ worktree, pane }: AgentPane) {
  selectWorktree(worktree);
  focusPane(worktree, pane);
}

export function openNextAttention() {
  const queue = attentionQueue(agentPanes(worktreeOrder(projects.get()), panes.get()), chats.get(), finished.get());
  const next = nextAttention(queue, shownPanes()?.focused ?? null);
  if (next) openAgent(next);
  else notify(queue.length ? "No other agent needs you" : "No agent needs you");
}

export function watchShownPanes(): () => void {
  let sent = "";
  const sync = () => {
    const shown = openPanes();
    if (shown.join() === sent) return;
    sent = shown.join();
    commands.showPanes(shown).catch(console.error);
  };
  sync();
  const unsubscribe = shownPanesStores().map((store) => store.subscribe(sync));
  return () => unsubscribe.forEach((off) => off());
}

export function watchNotificationClicks(): () => void {
  const unlisten = listen<[string, string]>("notification-opened", ({ payload: [worktree, pane] }) => {
    openAgent({ worktree, pane });
  });
  return () => void unlisten.then((off) => off());
}

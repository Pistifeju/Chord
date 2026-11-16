// The panes that finished unseen, and the pane a notification click opens.
import { listen } from "@tauri-apps/api/event";
import { commands } from "../../ipc/bindings";
import { externalStore } from "../../ui/externalStore";
import { chats } from "../chat/store";
import { shownTabIds } from "../panes/layout";
import { focusPane } from "../panes/store";
import { selectWorktree, shownPanes, shownPanesStores } from "../worktrees/store";
import { keptMarks, turnedDone, type PaneKinds } from "./finished";

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
    const marks = keptMarks([...finished.get(), ...turnedDone(before, now)], now, openPanes());
    before = now;
    if (marks.join() !== finished.get().join()) finished.set(marks);
  };
  // A reply's chunks leave every report as it was, so they skip the check.
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

// Tells Rust which panes the window shows, so it skips notifications for them.
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

// A click on a notification, sent by Rust with the worktree and the pane.
export function watchNotificationClicks(): () => void {
  const unlisten = listen<[string, string]>("notification-opened", ({ payload: [worktree, pane] }) => {
    selectWorktree(worktree);
    focusPane(worktree, pane);
  });
  return () => void unlisten.then((off) => off());
}

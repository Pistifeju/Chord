// The panes that finished unseen, and the pane a notification click opens.
import { listen } from "@tauri-apps/api/event";
import { commands } from "../../ipc/bindings";
import { externalStore } from "../../ui/externalStore";
import { chats } from "../chat/store";
import { paneIds } from "../panes/layout";
import { shownSession } from "../panes/sessions";
import { focusPane, panes } from "../panes/store";
import { settingsOpen } from "../settings/store";
import { projects, selectWorktree } from "../worktrees/store";
import { keptMarks, turnedDone, type PaneKinds } from "./finished";

export const finished = externalStore<string[]>([]);

const kindsNow = (): PaneKinds =>
  Object.fromEntries(Object.entries(chats.get()).flatMap(([pane, chat]) => (chat.report ? [[pane, chat.report.status.kind]] : [])));

function openPanes(): string[] {
  const shown = shownSession(panes.get(), projects.get().selected, settingsOpen.get());
  return shown ? paneIds(shown.layout) : [];
}

export function watchFinished(): () => void {
  let before = kindsNow();
  const sync = () => {
    const now = kindsNow();
    const marks = keptMarks([...finished.get(), ...turnedDone(before, now)], now, openPanes());
    before = now;
    if (marks.join() !== finished.get().join()) finished.set(marks);
  };
  const unsubscribe = [chats, panes, projects, settingsOpen].map((store) => store.subscribe(sync));
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
  const unsubscribe = [panes, projects, settingsOpen].map((store) => store.subscribe(sync));
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

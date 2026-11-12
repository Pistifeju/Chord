// Connects the session and layout rules to IPC: each worktree's tabs and panes, their agents, saved through Rust.
import { commands, type Direction, type Layout, type WorktreePanes } from "../../ipc/bindings";
import { externalStore } from "../../ui/externalStore";
import { forgetChat, startChat } from "../chat/store";
import { withEvenSizes, paneIds, withSizes, type Edge } from "./layout";
import { activeSession, firstPanes, withActiveSession, withFocus, withLayout, withNewSession, withoutPaneClosed, withSessionClosed, withSessionDropped, withSplitFocused } from "./sessions";

export const panes = externalStore<Record<string, WorktreePanes>>({});

export const loadPanes = (saved: Record<string, WorktreePanes>) => panes.set(saved);

const newId = () => crypto.randomUUID();

const save = (worktree: string) => commands.changeState({ panes: { worktree, panes: panes.get()[worktree] ?? null } }).catch(console.error);

const set = (worktree: string, next: WorktreePanes) => panes.set({ ...panes.get(), [worktree]: next });

// Opening a worktree for the first time gives it one session with one pane.
export function ensurePanes(worktree: string) {
  if (panes.get()[worktree]) return;
  set(worktree, firstPanes(newId(), newId()));
  save(worktree);
}

const panesOf = (worktree: string): WorktreePanes => {
  ensurePanes(worktree);
  return panes.get()[worktree];
};

// A new worktree's first pane starts its agent without a click on Start.
export function startFocused(worktree: string) {
  void startChat(worktree, activeSession(panesOf(worktree)).focused);
}

export function splitFocused(worktree: string, direction: Direction) {
  const pane = newId();
  set(worktree, withSplitFocused(panesOf(worktree), pane, direction));
  save(worktree);
  void startChat(worktree, pane);
}

export function newSession(worktree: string) {
  const pane = newId();
  set(worktree, withNewSession(panesOf(worktree), newId(), pane));
  save(worktree);
  void startChat(worktree, pane);
}

export function closeFocused(worktree: string) {
  const current = panesOf(worktree);
  const pane = activeSession(current).focused;
  const next = withoutPaneClosed(current, pane);
  if (next === current) return;
  set(worktree, next);
  save(worktree);
  forgetChat(pane);
}

export function closeSession(worktree: string, session: string) {
  const current = panesOf(worktree);
  const next = withSessionClosed(current, session);
  if (next === current) return;
  const gone = current.sessions.find((s) => s.id === session);
  set(worktree, next);
  save(worktree);
  if (gone) paneIds(gone.layout).forEach(forgetChat);
}

export function selectSession(worktree: string, session: string) {
  set(worktree, withActiveSession(panesOf(worktree), session));
  save(worktree);
}

export function dropSession(worktree: string, session: string, pane: string, edge: Edge) {
  const current = panesOf(worktree);
  const next = withSessionDropped(current, session, pane, edge);
  if (next === current) return;
  set(worktree, next);
  save(worktree);
}

// Focus stays in memory; the next layout save writes it.
export function focusPane(worktree: string, pane: string) {
  const current = panesOf(worktree);
  const next = withFocus(current, pane);
  if (next === current) return;
  set(worktree, next);
}

const changeLayout = (worktree: string, session: string, change: (layout: Layout) => Layout) => {
  const current = panesOf(worktree);
  const target = current.sessions.find((s) => s.id === session);
  if (target) set(worktree, withLayout(current, session, change(target.layout)));
};

// A drag changes the layout many times and saves once, when it ends.
export const resizeSplit = (worktree: string, session: string, path: number[], sizes: number[]) => changeLayout(worktree, session, (layout) => withSizes(layout, path, sizes));

export const endResize = save;

export function evenOutSplit(worktree: string, session: string, path: number[]) {
  changeLayout(worktree, session, (layout) => withEvenSizes(layout, path));
  save(worktree);
}

// The worktree is gone: its agents stop and its layouts go.
export function forgetPanes(worktree: string) {
  const gone = panes.get()[worktree];
  if (!gone) return;
  const { [worktree]: removed, ...rest } = panes.get();
  panes.set(rest);
  for (const session of gone.sessions) for (const pane of paneIds(session.layout)) forgetChat(pane);
  save(worktree);
}

// Layouts of worktrees that are not in the known paths go, as when one was removed while the app was closed.
export function forgetPanesExcept(known: Set<string>) {
  for (const worktree of Object.keys(panes.get())) if (!known.has(worktree)) forgetPanes(worktree);
}

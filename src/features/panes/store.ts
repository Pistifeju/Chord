// Connects the session and layout rules to IPC: each worktree's tabs and panes, their agents, saved through Rust.
import { commands, type Direction, type Layout, type WorktreePanes } from "../../ipc/bindings";
import { externalStore } from "../../ui/externalStore";
import { forgetChat, startChat } from "../chat/store";
import { paneChanges, resumedPanes, vanishedWorktrees } from "./agents";
import { withEvenSizes, withSizes, type Edge } from "./layout";
import { activeSession, firstPanes, withActiveSession, withFocus, withLayout, withNewSession, withoutPaneClosed, withSessionClosed, withSessionDropped, withSplitFocused } from "./sessions";

export const panes = externalStore<Record<string, WorktreePanes>>({});

export const loadPanes = (saved: Record<string, WorktreePanes>) => panes.set(saved);

export function resumeAgents(running: string[], existing: string[]) {
  for (const [worktree, pane] of resumedPanes(panes.get(), running, existing)) void startChat(worktree, pane);
}

const newId = () => crypto.randomUUID();

const save = (worktree: string) => commands.changeState({ panes: { worktree, panes: panes.get()[worktree] ?? null } }).catch(console.error);

const set = (worktree: string, next: WorktreePanes) => panes.set({ ...panes.get(), [worktree]: next });

// Every pane change goes through here: a pane that appears starts its agent, a pane that goes forgets its chat.
function commit(worktree: string, next: WorktreePanes | undefined, persist: boolean) {
  const { [worktree]: before, ...rest } = panes.get();
  panes.set(next ? { ...panes.get(), [worktree]: next } : rest);
  if (persist) save(worktree);
  const { started, gone } = paneChanges(before, next);
  for (const pane of started) void startChat(worktree, pane);
  for (const pane of gone) forgetChat(pane);
}

// Opening a worktree for the first time gives it one session with one pane, whose agent waits for Start.
export function ensurePanes(worktree: string) {
  if (panes.get()[worktree]) return;
  set(worktree, firstPanes(newId(), newId()));
  save(worktree);
}

const panesOf = (worktree: string): WorktreePanes => {
  ensurePanes(worktree);
  return panes.get()[worktree];
};

const change = (worktree: string, rule: (current: WorktreePanes) => WorktreePanes, persist = true) => {
  const current = panesOf(worktree);
  const next = rule(current);
  if (next !== current) commit(worktree, next, persist);
};

// A new worktree's first pane starts its agent without a click on Start.
export function startFocused(worktree: string) {
  void startChat(worktree, activeSession(panesOf(worktree)).focused);
}

export const splitFocused = (worktree: string, direction: Direction) => change(worktree, (p) => withSplitFocused(p, newId(), direction));

export const newSession = (worktree: string) => change(worktree, (p) => withNewSession(p, newId(), newId()));

export const closeFocused = (worktree: string) => closePane(worktree, activeSession(panesOf(worktree)).focused);

export const closePane = (worktree: string, pane: string) => change(worktree, (p) => withoutPaneClosed(p, pane));

export const closeSession = (worktree: string, session: string) => change(worktree, (p) => withSessionClosed(p, session));

export const selectSession = (worktree: string, session: string) => change(worktree, (p) => withActiveSession(p, session));

export const dropSession = (worktree: string, session: string, pane: string, edge: Edge) => change(worktree, (p) => withSessionDropped(p, session, pane, edge));

// Focus stays in memory; the next layout save writes it.
export const focusPane = (worktree: string, pane: string) => change(worktree, (p) => withFocus(p, pane), false);

const withSessionLayout = (session: string, layoutChange: (layout: Layout) => Layout) => (p: WorktreePanes) => {
  const target = p.sessions.find((s) => s.id === session);
  return target ? withLayout(p, session, layoutChange(target.layout)) : p;
};

// A drag changes the layout many times and saves once, when it ends.
export const resizeSplit = (worktree: string, session: string, path: number[], sizes: number[]) =>
  change(worktree, withSessionLayout(session, (layout) => withSizes(layout, path, sizes)), false);

export const endResize = save;

export const evenOutSplit = (worktree: string, session: string, path: number[]) => change(worktree, withSessionLayout(session, (layout) => withEvenSizes(layout, path)));

// The worktree is gone: its agents stop and its layouts go.
export function forgetPanes(worktree: string) {
  if (panes.get()[worktree]) commit(worktree, undefined, true);
}

// Layouts of worktrees that are not in the known paths go, as when one was removed while the app was closed.
export function forgetPanesExcept(known: string[]) {
  for (const worktree of vanishedWorktrees(panes.get(), known)) forgetPanes(worktree);
}

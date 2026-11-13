// Connects the session and layout rules to IPC: each worktree's tabs and panes, their agents, saved through Rust.
import { commands, type Direction, type Layout, type WorktreePanes } from "../../ipc/bindings";
import { externalStore } from "../../ui/externalStore";
import { closeSheet, openSheet, type Sheet } from "../../app/sheet";
import { chats, forgetChat, startChat } from "../chat/store";
import { paneChanges, resumedPanes, vanishedWorktrees } from "./agents";
import { closeFile, showSessions } from "../editor/store";
import { isAtOrInside } from "../editor/openFiles";
import { filePanes, withEvenSizes, withFilePanesMoved, withSizes, type Edge, type FilePane } from "./layout";
import {
  activeSession,
  firstPanes,
  paneCount,
  withActiveSession,
  withFileDropped,
  withFileSession,
  withFocus,
  withLayout,
  withNewSession,
  withoutPaneClosed,
  withSessionClosed,
  withSessionDropped,
  withSplitFocused,
} from "./sessions";

export const panes = externalStore<Record<string, WorktreePanes>>({});

export const loadPanes = (saved: Record<string, WorktreePanes>) => panes.set(saved);

export function resumeAgents(running: string[], existing: string[]) {
  return Promise.all(resumedPanes(panes.get(), running, existing).map(([worktree, pane]) => startChat(worktree, pane)));
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

type CloseAgents = Extract<Sheet, { kind: "closeAgents" }>;

const closeRule = ({ target, id }: Pick<CloseAgents, "target" | "id">) =>
  target === "pane" ? (p: WorktreePanes) => withoutPaneClosed(p, id) : (p: WorktreePanes) => withSessionClosed(p, id);

// A close that would stop a working agent asks first.
function closeAsking(worktree: string, target: CloseAgents["target"], id: string) {
  const current = panesOf(worktree);
  const { gone } = paneChanges(current, closeRule({ target, id })(current));
  if (gone.some((pane) => chats.get()[pane]?.running)) openSheet({ kind: "closeAgents", worktree, target, id });
  else change(worktree, closeRule({ target, id }));
}

export const closePane = (worktree: string, pane: string) => closeAsking(worktree, "pane", pane);

export const closeSession = (worktree: string, session: string) => closeAsking(worktree, "session", session);

export const cancelClose = () => closeSheet("closeAgents");

export function confirmClose(sheet: CloseAgents) {
  closeSheet("closeAgents");
  change(sheet.worktree, closeRule(sheet));
}

export const selectSession = (worktree: string, session: string) => change(worktree, (p) => withActiveSession(p, session));

export const dropSession = (worktree: string, session: string, pane: string, edge: Edge) => change(worktree, (p) => withSessionDropped(p, session, pane, edge));

const filePane = (path: string): FilePane => ({ kind: "file", id: newId(), path });

// The file leaves its tab for a pane beside `pane`.
export function dropFile(worktree: string, path: string, pane: string, edge: Edge) {
  change(worktree, (p) => withFileDropped(p, filePane(path), pane, edge));
  closeFile(worktree, path);
  showSessions(worktree);
}

// The file leaves its tab for a new session tab, with a new agent beside it.
export function splitFile(worktree: string, path: string, direction: Direction) {
  change(worktree, (p) => withFileSession(p, newId(), filePane(path), newId(), direction));
  // Hidden first, so closing the tab does not select a neighbour that is not shown.
  showSessions(worktree);
  closeFile(worktree, path);
}

export function moveFilePanes(worktree: string, from: string, to: string) {
  if (!panes.get()[worktree]) return;
  change(worktree, (p) => ({ ...p, sessions: p.sessions.map((s) => ({ ...s, layout: withFilePanesMoved(s.layout, from, to) })) }));
}

// A file pane that is the last pane of all gives its place to a new agent pane, since the last pane stays.
export function closeFilePane(worktree: string, pane: string) {
  const ensureAnother = (p: WorktreePanes) => (paneCount(p) === 1 ? withSplitFocused(p, newId(), "row") : p);
  change(worktree, (p) => withoutPaneClosed(ensureAnother(p), pane));
}

// Closes the file panes of `path` and of everything inside it.
export function closeFilePanesAt(worktree: string, path: string) {
  for (const session of panes.get()[worktree]?.sessions ?? [])
    for (const file of filePanes(session.layout)) if (isAtOrInside(file.path, path)) closeFilePane(worktree, file.id);
}

// The pane that shows `path`, if any, so the file opens there instead of in a second editor that would save over it.
export const filePaneOf = (worktree: string, path: string): FilePane | undefined =>
  panes.get()[worktree]?.sessions.flatMap((s) => filePanes(s.layout)).find((f) => f.path === path);

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

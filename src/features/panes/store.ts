// Connects the tab and layout rules to IPC: each worktree's tab groups, their agents, saved through Rust.
import { commands, type Direction, type WorktreePanes } from "../../ipc/bindings";
import { externalStore } from "../../ui/externalStore";
import { closeSheet, openSheet, type Sheet } from "../../app/sheet";
import { chats, forgetChat, startChat } from "../chat/store";
import { paneChanges, resumedPanes } from "./agents";
import { isAtOrInside } from "../files/paths";
import { filePanes, groupOf, withEvenSizes, withFilePanesMoved, withSizes, type FilePane } from "./layout";
import { firstPanes, paneCount, withFileOpened, withFocus, withNewSession, withSplitFocused, withTabClosed, withTabMoved, type TabDrop } from "./sessions";

export const panes = externalStore<Record<string, WorktreePanes>>({});

export const loadPanes = (saved: Record<string, WorktreePanes>) => panes.set(saved);

export function resumeAgents(running: string[]) {
  return Promise.all(resumedPanes(panes.get(), running).map(([worktree, pane]) => startChat(worktree, pane)));
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

// Opening a worktree for the first time gives it one group with one chat, whose agent waits for Start.
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
  void startChat(worktree, panesOf(worktree).focused);
}

export const splitFocused = (worktree: string, direction: Direction) => change(worktree, (p) => withSplitFocused(p, newId(), newId(), direction));

// A new chat tab in `group`, by default the focused one.
export const newSession = (worktree: string, group?: string) => change(worktree, (p) => withNewSession(p, newId(), group));

export const closeFocused = (worktree: string) => closePane(worktree, panesOf(worktree).focused);

type CloseAgents = Extract<Sheet, { kind: "closeAgents" }>;

// A close that would stop a working agent asks first.
export function closePane(worktree: string, tab: string) {
  const current = panesOf(worktree);
  const { gone } = paneChanges(current, withTabClosed(current, tab));
  if (gone.some((pane) => chats.get()[pane]?.running)) openSheet({ kind: "closeAgents", worktree, id: tab });
  else change(worktree, (p) => withTabClosed(p, tab));
}

export const cancelClose = () => closeSheet("closeAgents");

export function confirmClose(sheet: CloseAgents) {
  closeSheet("closeAgents");
  change(sheet.worktree, (p) => withTabClosed(p, sheet.id));
}

export const moveTab = (worktree: string, tab: string, drop: TabDrop) => change(worktree, (p) => withTabMoved(p, tab, drop, newId()));

// A file a tab already shows opens that tab, so a second editor never saves over it; any other opens in a new tab of the focused group.
export function openFileTab(worktree: string, path: string) {
  const shown = filePaneOf(worktree, path);
  const file: FilePane = { kind: "file", id: newId(), path };
  change(worktree, (p) => (shown ? withFocus(p, shown.id) : withFileOpened(p, file)));
}

export function moveFilePanes(worktree: string, from: string, to: string) {
  if (!panes.get()[worktree]) return;
  change(worktree, (p) => ({ ...p, layout: withFilePanesMoved(p.layout, from, to) }));
}

// A file tab that is the last tab of all gives its place to a new chat, since the last tab stays.
export function closeFilePane(worktree: string, tab: string) {
  const ensureAnother = (p: WorktreePanes) => (paneCount(p) === 1 ? withNewSession(p, newId(), groupOf(p.layout, tab)?.id) : p);
  change(worktree, (p) => withTabClosed(ensureAnother(p), tab));
}

// Closes the file tabs of `path` and of everything inside it.
export function closeFilePanesAt(worktree: string, path: string) {
  const saved = panes.get()[worktree];
  for (const file of saved ? filePanes(saved.layout) : []) if (isAtOrInside(file.path, path)) closeFilePane(worktree, file.id);
}

export const filePaneOf = (worktree: string, path: string): FilePane | undefined => {
  const saved = panes.get()[worktree];
  return saved && filePanes(saved.layout).find((f) => f.path === path);
};

// Focus stays in memory; the next layout save writes it.
export const focusPane = (worktree: string, tab: string) => change(worktree, (p) => withFocus(p, tab), false);

// A drag changes the layout many times and saves once, when it ends.
export const resizeSplit = (worktree: string, path: number[], sizes: number[]) =>
  change(worktree, (p) => ({ ...p, layout: withSizes(p.layout, path, sizes) }), false);

export const endResize = save;

export const evenOutSplit = (worktree: string, path: number[]) => change(worktree, (p) => ({ ...p, layout: withEvenSizes(p.layout, path) }));

// The worktree is gone: its agents stop and its layouts go.
export function forgetPanes(worktree: string) {
  if (panes.get()[worktree]) commit(worktree, undefined, true);
}

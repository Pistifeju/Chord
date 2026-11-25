import { commands, type DiffSide, type Direction, type ModelPick, type WorktreePanes } from "../../ipc/bindings";
import { externalStore } from "../../ui/externalStore";
import { closeSheet, openSheet, type Sheet } from "../../app/sheet";
import { chats, forgetChat, startChat } from "../chat/store";
import { anyRunning } from "../chat/transcript";
import { allPaneIds, paneChanges, resumedPanes } from "./agents";
import { isAtOrInside } from "../files/paths";
import { diffKey } from "../changes/changes";
import { browserPaneIds, filePanes, groupOf, withBrowserPage, withEvenSizes, withFilePanesMoved, withSizes, type BrowserPane, type FilePane } from "./layout";
import { firstPanes, withBrowserOpened, withFileKept, withFileOpened, withFocus, withNewSession, withSplitFocused, withTabClosed, withTabMoved, type TabDrop } from "./sessions";

export const panes = externalStore<Record<string, WorktreePanes>>({});

export const loadPanes = (saved: Record<string, WorktreePanes>) => panes.set(saved);

export const agentsWorkIn = (worktrees: string[]) => anyRunning(chats.get(), worktrees.flatMap((worktree) => allPaneIds(panes.get()[worktree])));

export function resumeAgents(running: string[]) {
  return Promise.all(resumedPanes(panes.get(), running).map(([worktree, pane]) => startChat(worktree, pane)));
}

const newId = () => crypto.randomUUID();

const save = (worktree: string) => commands.changeState({ panes: { worktree, panes: panes.get()[worktree] ?? null } }).catch(console.error);

const set = (worktree: string, next: WorktreePanes) => panes.set({ ...panes.get(), [worktree]: next });

const browsersIn = (saved: WorktreePanes | undefined): string[] => (saved ? browserPaneIds(saved.layout) : []);

const startsOn = new Map<string, { harness: string; pick: ModelPick | null }>();

function commit(worktree: string, next: WorktreePanes | undefined, persist: boolean) {
  const { [worktree]: before, ...rest } = panes.get();
  const kept = browsersIn(next);
  for (const tab of browsersIn(before)) if (!kept.includes(tab)) commands.closeBrowser(tab).catch(console.error);
  panes.set(next ? { ...panes.get(), [worktree]: next } : rest);
  if (persist) save(worktree);
  const { started, gone } = paneChanges(before, next);
  for (const pane of started) {
    const on = startsOn.get(pane);
    startsOn.delete(pane);
    void startChat(worktree, pane, on?.harness ?? null, on?.pick ?? null);
  }
  for (const pane of gone) forgetChat(pane);
}

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

export function startFocused(worktree: string) {
  void startChat(worktree, panesOf(worktree).focused);
}

export const splitFocused = (worktree: string, direction: Direction) => change(worktree, (p) => withSplitFocused(p, newId(), newId(), direction));

export const newSession = (worktree: string, group?: string) => change(worktree, (p) => withNewSession(p, newId(), group));

export function newSessionOn(worktree: string, harness: string, pick: ModelPick | null) {
  const id = newId();
  startsOn.set(id, { harness, pick });
  change(worktree, (p) => withNewSession(p, id));
}

export const closeFocused = (worktree: string) => closePane(worktree, panesOf(worktree).focused);

type CloseAgents = Extract<Sheet, { kind: "closeAgents" }>;

export function closePane(worktree: string, tab: string) {
  const current = panesOf(worktree);
  const { gone } = paneChanges(current, withTabClosed(current, tab));
  if (anyRunning(chats.get(), gone)) openSheet({ kind: "closeAgents", worktree, id: tab });
  else change(worktree, (p) => withTabClosed(p, tab));
}

export const cancelClose = () => closeSheet("closeAgents");

export function confirmClose(sheet: CloseAgents) {
  closeSheet("closeAgents");
  change(sheet.worktree, (p) => withTabClosed(p, sheet.id));
}

export const moveTab = (worktree: string, tab: string, drop: TabDrop) => change(worktree, (p) => withTabMoved(p, tab, drop, newId()));

export function openFileTab(worktree: string, path: string, preview: boolean, diff?: DiffSide) {
  const shown = filePaneOf(worktree, path, diff);
  const file: FilePane = { kind: "file", id: newId(), path, preview, ...(diff && { diff }) };
  change(worktree, (p) => (shown ? (preview ? withFocus(p, shown.id) : withFileKept(withFocus(p, shown.id), shown.id)) : withFileOpened(p, file)));
}

export function openBrowserTab(worktree: string, url: string) {
  const browser: BrowserPane = { kind: "browser", id: newId(), url, title: "" };
  change(worktree, (p) => withBrowserOpened(p, browser));
}

export function setBrowserPage(tab: string, page: Partial<Pick<BrowserPane, "url" | "title">>) {
  const worktree = Object.keys(panes.get()).find((w) => browsersIn(panes.get()[w]).includes(tab));
  const shown = worktree ? groupOf(panes.get()[worktree].layout, tab)?.tabs.find((t) => t.id === tab) : undefined;
  if (shown?.kind !== "browser" || Object.entries(page).every(([key, value]) => shown[key as keyof typeof page] === value)) return;
  if (worktree) change(worktree, (p) => ({ ...p, layout: withBrowserPage(p.layout, tab, page) }));
}

export const keepFilePane = (worktree: string, tab: string) => change(worktree, (p) => withFileKept(p, tab));

export function moveFilePanes(worktree: string, from: string, to: string) {
  if (!panes.get()[worktree]) return;
  change(worktree, (p) => ({ ...p, layout: withFilePanesMoved(p.layout, from, to) }));
}

export function closeFilePanesAt(worktree: string, path: string) {
  const saved = panes.get()[worktree];
  for (const file of saved ? filePanes(saved.layout) : []) if (isAtOrInside(file.path, path)) closePane(worktree, file.id);
}

const filePaneOf = (worktree: string, path: string, diff: DiffSide | undefined): FilePane | undefined => {
  const saved = panes.get()[worktree];
  const key = diff && diffKey(diff, path);
  return saved && filePanes(saved.layout).find((f) => f.path === path && (f.diff ? diffKey(f.diff, path) : undefined) === key);
};

export const focusPane = (worktree: string, tab: string) => change(worktree, (p) => withFocus(p, tab), false);

export const resizeSplit = (worktree: string, path: number[], sizes: number[]) =>
  change(worktree, (p) => ({ ...p, layout: withSizes(p.layout, path, sizes) }), false);

export const endResize = save;

export const evenOutSplit = (worktree: string, path: number[]) => change(worktree, (p) => ({ ...p, layout: withEvenSizes(p.layout, path) }));

export function forgetPanes(worktree: string) {
  if (panes.get()[worktree]) commit(worktree, undefined, true);
}

// Rules for a worktree's tabs and tab groups: new, open, split, move, close, and which tab has the focus.
import type { Direction, Tab, WorktreePanes } from "../../ipc/bindings";
import type { Item } from "../chat/transcript.ts";
import { edgeDirection, groupOf, groups, paneIds, withBeside, withGroup, withoutGroup, type Edge, type FilePane, type Group } from "./layout.ts";
import { fileName } from "../files/paths.ts";

const chatTab = (id: string, name: string): Tab => ({ kind: "chat", id, name });

const lone = (group: string, tab: Tab): Group => ({ kind: "group", id: group, tabs: [tab], active: tab.id });

export const firstPanes = (group: string, chat: string): WorktreePanes => ({ layout: lone(group, chatTab(chat, "Session 1")), focused: chat });

// The panes the window shows: the selected worktree's, none while Settings covers the window.
export function shownSession(all: Record<string, WorktreePanes>, selected: string | null, settingsOpen: boolean): WorktreePanes | null {
  return (selected && !settingsOpen && all[selected]) || null;
}

const nextName = (panes: WorktreePanes): string => {
  const numbers = groups(panes.layout).flatMap((g) => g.tabs).map((t) => Number(t.kind === "chat" ? /^Session (\d+)$/.exec(t.name)?.[1] ?? 0 : 0));
  return `Session ${Math.max(0, ...numbers) + 1}`;
};

const focusedGroup = (panes: WorktreePanes): Group => groupOf(panes.layout, panes.focused) ?? groups(panes.layout)[0];

// The tab joins the end of the group, opens there and takes the focus.
function withTabAdded(panes: WorktreePanes, group: string, tab: Tab): WorktreePanes {
  return { layout: withGroup(panes.layout, group, (g) => ({ ...g, tabs: [...g.tabs, tab], active: tab.id })), focused: tab.id };
}

// A new chat opens in `group`, by default the focused one.
export const withNewSession = (panes: WorktreePanes, chat: string, group = focusedGroup(panes).id): WorktreePanes =>
  withTabAdded(panes, group, chatTab(chat, nextName(panes)));

// A file opens in a tab of the focused group; a preview takes the place of the group's preview tab, keeping its id.
export function withFileOpened(panes: WorktreePanes, file: FilePane): WorktreePanes {
  const group = focusedGroup(panes);
  const preview = file.preview ? group.tabs.find((t) => t.kind === "file" && t.preview) : undefined;
  if (!preview) return withTabAdded(panes, group.id, file);
  const tabs = group.tabs.map((t) => (t.id === preview.id ? { ...file, id: preview.id } : t));
  return { layout: withGroup(panes.layout, group.id, (g) => ({ ...g, tabs, active: preview.id })), focused: preview.id };
}

// A preview tab becomes a kept one.
export function withFileKept(panes: WorktreePanes, tab: string): WorktreePanes {
  const owner = groupOf(panes.layout, tab);
  if (!owner?.tabs.some((t) => t.id === tab && t.kind === "file" && t.preview)) return panes;
  return { ...panes, layout: withGroup(panes.layout, owner.id, (g) => ({ ...g, tabs: g.tabs.map((t) => (t.id === tab ? { ...t, preview: false } : t)) })) };
}

// Opens the tab in its group and gives it the focus.
export function withFocus(panes: WorktreePanes, tab: string): WorktreePanes {
  const owner = groupOf(panes.layout, tab);
  if (!owner || (owner.active === tab && panes.focused === tab)) return panes;
  return { layout: withGroup(panes.layout, owner.id, (g) => ({ ...g, active: tab })), focused: tab };
}

// A new group with a new chat goes beside the focused group and takes the focus.
export function withSplitFocused(panes: WorktreePanes, group: string, chat: string, direction: Direction): WorktreePanes {
  return { layout: withBeside(panes.layout, focusedGroup(panes).id, lone(group, chatTab(chat, nextName(panes))), direction, false), focused: chat };
}

export const paneCount = (panes: WorktreePanes): number => paneIds(panes.layout).length;

export const canClosePane = (panes: WorktreePanes | undefined): boolean => !!panes && paneCount(panes) > 1;

/** The name of each pane's agent session, by pane id; a pane whose session has no user message yet is left out. */
export type PaneNames = Record<string, string | undefined>;

// A session is named after its first user message, on one line and cut to 30 characters.
function sessionName(items: Item[]): string | undefined {
  const first = items.find((item) => item.role === "user");
  if (first?.role !== "user") return undefined;
  // Every chunk asks again, so only the words that reach 30 characters are read.
  let name = "";
  for (const [word] of first.text.matchAll(/\S+/g)) {
    name += name ? ` ${word}` : word;
    if (name.length >= 30) break;
  }
  return name.slice(0, 30) || undefined;
}

export const paneNames = (chats: Record<string, { items: Item[] } | undefined>, panes: string[]): PaneNames =>
  Object.fromEntries(panes.flatMap((pane) => {
    const name = sessionName(chats[pane]?.items ?? []);
    return name ? [[pane, name]] : [];
  }));

// A chat tab shows the name of its agent session once that has one; a file tab its file name.
export const tabName = (tab: Tab, names: PaneNames): string => (tab.kind === "chat" ? names[tab.id] ?? tab.name : fileName(tab.path));

// The tab leaves its group, whose tab beside it opens; a group left empty goes.
function withoutTab(panes: WorktreePanes, tab: string): WorktreePanes {
  const owner = groupOf(panes.layout, tab);
  if (!owner) return panes;
  const at = owner.tabs.findIndex((t) => t.id === tab);
  const rest = owner.tabs.filter((t) => t.id !== tab);
  if (rest.length > 0) {
    const active = owner.active === tab ? (rest[at] ?? rest[at - 1]).id : owner.active;
    const layout = withGroup(panes.layout, owner.id, (g) => ({ ...g, tabs: rest, active }));
    return { layout, focused: panes.focused === tab ? active : panes.focused };
  }
  const layout = withoutGroup(panes.layout, owner.id);
  if (!layout) return panes;
  const left = groups(layout);
  const beside = left[Math.min(groups(panes.layout).indexOf(owner), left.length - 1)];
  return { layout, focused: panes.focused === tab ? beside.active : panes.focused };
}

// Closes a tab; the last tab of all stays.
export const withTabClosed = (panes: WorktreePanes, tab: string): WorktreePanes => (paneCount(panes) > 1 ? withoutTab(panes, tab) : panes);

/** Where a dragged tab lands: a group's tab bar (`edge` null), or a group's edge, where it opens a new group. */
export type TabDrop = { group: string; edge: Edge | null };

// Whether dropping the tab there changes anything: not into its own group, and not beside its own group while it is that group's only tab.
export function movesTab(panes: WorktreePanes, tab: string, { group, edge }: TabDrop): boolean {
  const owner = groupOf(panes.layout, tab);
  if (!owner) return false;
  return owner.id !== group || (edge !== null && owner.tabs.length > 1);
}

export function withTabMoved(panes: WorktreePanes, tab: string, drop: TabDrop, created: string): WorktreePanes {
  const moved = groupOf(panes.layout, tab)?.tabs.find((t) => t.id === tab);
  if (!moved || !movesTab(panes, tab, drop)) return panes;
  const { layout } = withoutTab(panes, tab);
  if (drop.edge === null) return withTabAdded({ layout, focused: tab }, drop.group, moved);
  const before = drop.edge === "left" || drop.edge === "top";
  return { layout: withBeside(layout, drop.group, lone(created, moved), edgeDirection(drop.edge), before), focused: tab };
}

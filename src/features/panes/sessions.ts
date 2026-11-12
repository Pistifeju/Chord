// Rules for a worktree's session tabs: new, open, split, drop, close, and which pane has the focus.
import type { Direction, Layout, PaneSession, WorktreePanes } from "../../ipc/bindings";
import type { Item } from "../chat/transcript.ts";
import { edgeDirection, paneIds, withBeside, withoutPane, withSplit, type Edge } from "./layout.ts";

export const firstPanes = (session: string, pane: string): WorktreePanes => ({
  sessions: [{ id: session, name: "Session 1", layout: { kind: "pane", id: pane }, focused: pane }],
  active: session,
});

export const activeSession = (panes: WorktreePanes): PaneSession => panes.sessions.find((s) => s.id === panes.active) ?? panes.sessions[0];

// The session the window shows: the selected worktree's open tab, none while Settings covers the window.
export function shownSession(all: Record<string, WorktreePanes>, selected: string | null, settingsOpen: boolean): PaneSession | null {
  const open = selected && !settingsOpen ? all[selected] : undefined;
  return open ? activeSession(open) : null;
}

const nextName = (sessions: PaneSession[]): string => {
  const numbers = sessions.map((s) => Number(/^Session (\d+)$/.exec(s.name)?.[1] ?? 0));
  return `Session ${Math.max(0, ...numbers) + 1}`;
};

export const withNewSession = (panes: WorktreePanes, session: string, pane: string): WorktreePanes => ({
  sessions: [...panes.sessions, { id: session, name: nextName(panes.sessions), layout: { kind: "pane", id: pane }, focused: pane }],
  active: session,
});

export const withActiveSession = (panes: WorktreePanes, session: string): WorktreePanes =>
  panes.sessions.some((s) => s.id === session) ? { ...panes, active: session } : panes;

const withSessionChanged = (panes: WorktreePanes, session: string, change: (s: PaneSession) => PaneSession): WorktreePanes => ({
  ...panes,
  sessions: panes.sessions.map((s) => (s.id === session ? change(s) : s)),
});

export const withLayout = (panes: WorktreePanes, session: string, layout: Layout): WorktreePanes =>
  withSessionChanged(panes, session, (s) => ({ ...s, layout }));

// Opens the pane's session and gives the pane the focus.
export function withFocus(panes: WorktreePanes, pane: string): WorktreePanes {
  const owner = panes.sessions.find((s) => paneIds(s.layout).includes(pane));
  if (!owner || (owner.focused === pane && panes.active === owner.id)) return panes;
  return { ...withSessionChanged(panes, owner.id, (s) => ({ ...s, focused: pane })), active: owner.id };
}

// The new pane goes beside the active session's focused pane and takes the focus.
export function withSplitFocused(panes: WorktreePanes, created: string, direction: Direction): WorktreePanes {
  const { id, layout, focused } = activeSession(panes);
  return withSessionChanged(panes, id, (s) => ({ ...s, layout: withSplit(layout, focused, created, direction), focused: created }));
}

export const paneCount = (panes: WorktreePanes): number => panes.sessions.reduce((sum, s) => sum + paneIds(s.layout).length, 0);

export const canClosePane = (panes: WorktreePanes | undefined): boolean => !!panes && paneCount(panes) > 1;

/** The name of each pane's agent session, by pane id; a pane whose session has no user message yet is left out. */
export type PaneNames = Record<string, string | undefined>;

// A session is named after its first user message, on one line and cut to 30 characters.
export function sessionName(items: Item[]): string | undefined {
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

// A tab shows the name of its focused pane's agent session once that has one.
export const tabName = (session: PaneSession, names: PaneNames): string => names[session.focused] ?? session.name;

// Closes a session tab with all its panes; the tab beside it opens. The last session stays.
export function withSessionClosed(panes: WorktreePanes, session: string): WorktreePanes {
  const at = panes.sessions.findIndex((s) => s.id === session);
  if (at < 0 || panes.sessions.length === 1) return panes;
  const sessions = panes.sessions.filter((s) => s.id !== session);
  const active = panes.active === session ? (sessions[at] ?? sessions[at - 1]).id : panes.active;
  return { sessions, active };
}

// Closes a pane. Its session goes with its last pane, and the tab beside it opens; the last pane of all stays.
export function withoutPaneClosed(panes: WorktreePanes, pane: string): WorktreePanes {
  const owner = panes.sessions.find((s) => paneIds(s.layout).includes(pane));
  if (!owner || paneCount(panes) === 1) return panes;
  const layout = withoutPane(owner.layout, pane);
  if (layout) {
    const order = paneIds(owner.layout);
    const rest = paneIds(layout);
    const beside = order[order.indexOf(pane) + 1] ?? order[order.indexOf(pane) - 1];
    const focused = owner.focused === pane ? (rest.includes(beside) ? beside : rest[0]) : owner.focused;
    return withSessionChanged(panes, owner.id, (s) => ({ ...s, layout, focused }));
  }
  const at = panes.sessions.indexOf(owner);
  const sessions = panes.sessions.filter((s) => s !== owner);
  const active = panes.active === owner.id ? (sessions[at] ?? sessions[at - 1]).id : panes.active;
  return { sessions, active };
}

// A session tab dropped on a pane edge: its whole layout joins that pane's session beside the pane, and its tab goes.
export function withSessionDropped(panes: WorktreePanes, source: string, pane: string, edge: Edge): WorktreePanes {
  const moved = panes.sessions.find((s) => s.id === source);
  const target = panes.sessions.find((s) => paneIds(s.layout).includes(pane));
  if (!moved || !target || moved === target) return panes;
  const layout = withBeside(target.layout, pane, moved.layout, edgeDirection(edge), edge === "left" || edge === "top");
  const joined = { ...target, layout, focused: moved.focused };
  return { sessions: panes.sessions.filter((s) => s !== moved).map((s) => (s === target ? joined : s)), active: target.id };
}

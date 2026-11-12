// Rules for a worktree's session tabs: new, open, split, close, and which pane has the focus.
import type { Direction, Layout, PaneSession, WorktreePanes } from "../../ipc/bindings";
import { paneIds, withoutPane, withSplit } from "./layout.ts";

export const firstPanes = (session: string, pane: string): WorktreePanes => ({
  sessions: [{ id: session, name: "Session 1", layout: { kind: "pane", id: pane }, focused: pane }],
  active: session,
});

export const activeSession = (panes: WorktreePanes): PaneSession => panes.sessions.find((s) => s.id === panes.active) ?? panes.sessions[0];

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

export type PaneRow = { pane: string; label: string; focused: boolean };

// One row per pane once the worktree has more than one; a session with several panes numbers them.
export function paneRows(panes: WorktreePanes | undefined, worktreeSelected: boolean): PaneRow[] {
  if (!panes || paneCount(panes) < 2) return [];
  const open = activeSession(panes).id;
  return panes.sessions.flatMap((session) => {
    const ids = paneIds(session.layout);
    return ids.map((pane, i) => ({
      pane,
      label: ids.length > 1 ? `${session.name} · ${i + 1}` : session.name,
      focused: worktreeSelected && pane === session.focused && session.id === open,
    }));
  });
}

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

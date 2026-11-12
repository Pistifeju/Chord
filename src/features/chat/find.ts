// Find in the transcript: which panes have the find bar open and what they search for.
import { externalStore } from "../../ui/externalStore";

// `focus` counts the requests to put the caret in the field, so a second ⌘F selects the query again.
export type Find = { query: string; focus: number };

export const finds = externalStore<Record<string, Find>>({});

export function openFind(pane: string) {
  const open = finds.get()[pane];
  finds.set({ ...finds.get(), [pane]: { query: open?.query ?? "", focus: (open?.focus ?? 0) + 1 } });
}

export function setFindQuery(pane: string, query: string) {
  finds.set({ ...finds.get(), [pane]: { query, focus: finds.get()[pane]?.focus ?? 0 } });
}

export function closeFind(pane: string) {
  const { [pane]: _closed, ...rest } = finds.get();
  finds.set(rest);
}

// Find bars' state: which views have one open, keyed by view, and what each searches for.
import { externalStore } from "./externalStore";

// `focus` counts the requests to put the caret in the field, so a second ⌘F selects the query again.
export type Find = { query: string; focus: number };

export function findStore() {
  const finds = externalStore<Record<string, Find>>({});
  return {
    finds,
    openFind(key: string) {
      const open = finds.get()[key];
      finds.set({ ...finds.get(), [key]: { query: open?.query ?? "", focus: (open?.focus ?? 0) + 1 } });
    },
    setFindQuery(key: string, query: string) {
      finds.set({ ...finds.get(), [key]: { query, focus: finds.get()[key]?.focus ?? 0 } });
    },
    closeFind(key: string) {
      const { [key]: _closed, ...rest } = finds.get();
      finds.set(rest);
    },
  };
}

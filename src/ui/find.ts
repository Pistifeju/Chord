import { externalStore } from "./externalStore";

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

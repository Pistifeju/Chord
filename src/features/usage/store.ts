import { commands, type Spend, type UsageWindow } from "../../ipc/bindings";
import { externalStore } from "../../ui/externalStore";
import { since, type Range } from "./spend";

export const usage = externalStore<Record<string, UsageWindow[]>>({});

export const usagePanelOpen = externalStore(false);

export const spend = externalStore<{ read: Spend; at: number } | null>(null);
export const spendReading = externalStore(false);

let reads = 0;

export function readSpend(range: Range) {
  const id = ++reads;
  const at = Date.now();
  spendReading.set(true);
  commands
    .readSpend(since(range, at))
    .then((read) => id === reads && spend.set({ read, at }))
    .catch(console.error)
    .finally(() => id === reads && spendReading.set(false));
}

const PREFETCH_DELAY_MS = 2000;

export function prefetchSpend() {
  setTimeout(() => {
    commands
      .readSpend(since("90d", Date.now()))
      .then(() => {
        if (spend.get() === null) readSpend("30d");
      })
      .catch(console.error);
  }, PREFETCH_DELAY_MS);
}

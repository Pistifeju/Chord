// The subscription usage windows of each harness, by the name a pane saves: the account's, so the latest any chat of that harness reported. And the Usage panel's open flag, the spend read last, and its read ahead of the first open.
import { commands, type Spend, type UsageWindow } from "../../ipc/bindings";
import { externalStore } from "../../ui/externalStore";
import { since, type Range } from "./spend";

export const usage = externalStore<Record<string, UsageWindow[]>>({});

export const usagePanelOpen = externalStore(false);

/** The last read and the time it ran at; none until the first one ends. */
export const spend = externalStore<{ read: Spend; at: number } | null>(null);
export const spendReading = externalStore(false);

let reads = 0;

/** Reads a range's spend. A read that a newer one overtook is dropped. */
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

/** The first read parses every session file for about 3 s; doing it after launch lets the panel open with its numbers. 90 days fills the cache for every range; the panel's own range follows from the cache. */
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

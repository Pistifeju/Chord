// The subscription usage windows of each harness, by the name a pane saves: the account's, so the latest any chat of that harness reported.
import type { UsageWindow } from "../../ipc/bindings";
import { externalStore } from "../../ui/externalStore";

export const usage = externalStore<Record<string, UsageWindow[]>>({});

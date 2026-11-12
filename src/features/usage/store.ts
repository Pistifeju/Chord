// The subscription usage windows: the account's, so the latest any chat reported.
import type { UsageWindow } from "../../ipc/bindings";
import { externalStore } from "../../ui/externalStore";

export const usage = externalStore<UsageWindow[]>([]);

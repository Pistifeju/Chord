import type { AppCommand } from "../../app/commands";
import { openNextAttention } from "./store";

export const statusCommands: AppCommand[] = [{ id: "agent.next", title: "Next Agent Needing You", shortcut: { key: "j", cmd: true }, run: openNextAttention }];

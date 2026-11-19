// The agent status commands for the commands table: open the next agent that needs you.
import type { AppCommand } from "../../app/commands";
import { openNextAttention } from "./store";

export const statusCommands: AppCommand[] = [{ id: "agent.next", title: "Next Agent Needing You", shortcut: { key: "j", cmd: true }, run: openNextAttention }];

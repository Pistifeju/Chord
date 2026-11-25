import type { AppCommand } from "../../../app/commands";
import { onTarget } from "../../../app/commandTarget";
import type { Button } from "../../../ipc/bindings";
import { showsSideView, sidePanels, toggleSideView } from "../../sidepanel/store";
import { simulatorWorktree } from "../build/schemeStore";
import { pressButton, screen, takeScreenshot } from "./store";

const shownWorktree = (): string | null => {
  const worktree = simulatorWorktree();
  return worktree && showsSideView(sidePanels.get(), worktree, "simulator") ? worktree : null;
};

const liveWorktree = (): string | null => {
  const worktree = shownWorktree();
  const shown = screen.get();
  return worktree !== null && shown?.worktree === worktree && shown.state.state === "live" ? worktree : null;
};

const press = (button: Button) => onTarget(shownWorktree, (worktree) => pressButton(worktree, button));

export const simulatorCommands: AppCommand[] = [
  { id: "simulator.toggle", title: "Show or Hide Simulator", shortcut: { key: "s", cmd: true, shift: true }, ...onTarget(simulatorWorktree, (worktree) => toggleSideView(worktree, "simulator")) },
  { id: "simulator.home", title: "Home", shortcut: { key: "h", cmd: true, shift: true }, ...press("home") },
  { id: "simulator.lock", title: "Lock or Unlock", shortcut: { key: "l", cmd: true }, ...press("lock") },
  { id: "simulator.shake", title: "Shake", shortcut: { key: "z", cmd: true, ctrl: true }, ...press("shake") },
  { id: "simulator.screenshot", title: "Take Screenshot", shortcut: { key: "s", cmd: true }, ...onTarget(liveWorktree, (worktree) => takeScreenshot(worktree)) },
];

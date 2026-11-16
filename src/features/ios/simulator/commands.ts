// The Simulator view's commands for the commands table: show or hide the view, take a screenshot, and press Home, Lock or Unlock, or Shake on its simulator.
import type { AppCommand } from "../../../app/commands";
import type { Button } from "../../../ipc/bindings";
import { showsSideView, sidePanels, toggleSideView } from "../../sidepanel/store";
import { xcodeWorktree } from "../build/commands";
import { pressButton, screen, takeScreenshot } from "./store";

// The selected worktree while its side panel shows the Simulator view.
const shownWorktree = (): string | null => {
  const worktree = xcodeWorktree();
  return worktree && showsSideView(sidePanels.get(), worktree, "simulator") ? worktree : null;
};

const screenIsLive = (worktree: string | null) => {
  const shown = screen.get();
  return worktree !== null && shown?.worktree === worktree && shown.state.state === "live";
};

const press = (button: Button) => () => pressButton(shownWorktree()!, button);

export const simulatorCommands: AppCommand[] = [
  {
    id: "simulator.toggle",
    title: "Show or Hide Simulator",
    shortcut: { key: "s", cmd: true, shift: true },
    when: () => xcodeWorktree() !== null,
    run: () => toggleSideView(xcodeWorktree()!, "simulator"),
  },
  { id: "simulator.home", title: "Home", shortcut: { key: "h", cmd: true, shift: true }, when: () => shownWorktree() !== null, run: press("home") },
  { id: "simulator.lock", title: "Lock or Unlock", shortcut: { key: "l", cmd: true }, when: () => shownWorktree() !== null, run: press("lock") },
  { id: "simulator.shake", title: "Shake", shortcut: { key: "z", cmd: true, ctrl: true }, when: () => shownWorktree() !== null, run: press("shake") },
  { id: "simulator.screenshot", title: "Take Screenshot", shortcut: { key: "k", cmd: true, shift: true }, when: () => screenIsLive(shownWorktree()), run: () => takeScreenshot(shownWorktree()!) },
];

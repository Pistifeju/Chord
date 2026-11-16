// Connects the Simulator view to IPC: the shown worktree's screen state from its channel, the screen shown, moved and hidden, and the view's touches, keys and buttons sent in order.
import { Channel } from "@tauri-apps/api/core";
import { commands, type Bezel, type Button, type ScreenButtons_Deserialize as ScreenButtons, type ScreenRect, type ScreenState_Deserialize, type TouchPhase } from "../../../ipc/bindings";
import { externalStore } from "../../../ui/externalStore";
import { fail } from "../../worktrees/store";
import type { Point } from "./screen";

export type Screen = { worktree: string; state: ScreenState_Deserialize };

/** The shown worktree's screen; null before its first state. */
export const screen = externalStore<Screen | null>(null);

/** Bumped by every show and hide: a hidden screen's late states are ignored. */
let turn = 0;
/** Grows with every touch; Rust drops a move older than the newest touch. */
let touchSeq = 0;

export function showScreen(worktree: string, rect: ScreenRect, bezel: Bezel) {
  const mine = ++turn;
  screen.set(null);
  const channel = new Channel<ScreenState_Deserialize>();
  channel.onmessage = (state) => {
    if (turn === mine) screen.set({ worktree, state });
  };
  commands.showSimulatorScreen(worktree, rect, bezel, channel).catch(fail);
}

export const moveScreen = (rect: ScreenRect, bezel: Bezel) => void commands.moveSimulatorScreen(rect, bezel).catch(console.error);

export const setButtons = (buttons: ScreenButtons) => void commands.setSimulatorButtons(buttons).catch(console.error);

export function hideScreen() {
  turn++;
  screen.set(null);
  commands.hideSimulatorScreen().catch(console.error);
}

export const sendTouch = (worktree: string, phase: TouchPhase, first: Point, second: Point | null, fromBottom: boolean) =>
  void commands.sendSimulatorInput(worktree, { kind: "touch", seq: ++touchSeq, phase, first, second, fromBottom }).catch(console.error);

export const sendKey = (worktree: string, usage: number, down: boolean) =>
  void commands.sendSimulatorInput(worktree, { kind: "key", usage, down }).catch(console.error);

/** Lock on a locked simulator unlocks it: Home does that, Lock only wakes the lock screen. */
export function pressButton(worktree: string, button: Button) {
  const shown = screen.get();
  const state = shown?.worktree === worktree ? shown.state : null;
  const unlocks = button === "lock" && state?.state === "live" && state.locked;
  commands.pressSimulatorButton(worktree, unlocks ? "home" : button).catch(fail);
}

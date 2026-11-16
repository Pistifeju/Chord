// Connects the Simulator view to IPC: the shown worktree's screen state from its channel, the screen shown, moved and hidden, the view's touches, keys and buttons sent in order, and its screenshots: taken, dragged out, pasted into the focused agent, and the folder they are saved in.
import { Channel } from "@tauri-apps/api/core";
import { commands, type Bezel, type Button, type ScreenButtons, type ScreenRect, type Screenshot, type ScreenState_Deserialize, type TouchPhase } from "../../../ipc/bindings";
import { externalStore } from "../../../ui/externalStore";
import { attachFiles, chatOf, chats, focusComposer, takesAttachments } from "../../chat/store";
import { chatPaneIds } from "../../panes/layout";
import { fail, shownPanes } from "../../worktrees/store";
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

export const openSimulatorApp = () => void commands.openSimulatorApp().catch(fail);

/** The screenshot taken last, with its preview as a data URL. */
export type Shot = { worktree: string; screenshot: Screenshot; preview: string };

export const shot = externalStore<Shot | null>(null);

/** The folder Settings saves screenshots in; null keeps them in a temporary one. */
export const screenshotFolder = externalStore<string | null>(null);

export async function takeScreenshot(worktree: string) {
  try {
    const screenshot = await commands.takeSimulatorScreenshot(worktree);
    shot.set({ worktree, screenshot, preview: `data:image/png;base64,${await commands.readImageFile(screenshot.preview)}` });
  } catch (error) {
    fail(error);
  }
}

export const dismissScreenshot = () => shot.set(null);

export const dragScreenshot = ({ screenshot }: Shot) => void commands.dragFileOut(screenshot.file, screenshot.preview).catch(fail);

/** The focused chat pane, unless its agent takes no attachments: the rule a drop into a pane follows. */
export const attachingPane = (): string | null => {
  const shown = shownPanes();
  const chat = shown?.focused && chatPaneIds(shown.layout).includes(shown.focused) ? chatOf(chats.get(), shown.focused) : null;
  return chat && takesAttachments(chat) ? shown!.focused : null;
};

/** Attaches the screenshot to the focused agent's next message and puts the focus in its composer. */
export function pasteScreenshot({ screenshot }: Shot) {
  const pane = attachingPane();
  if (!pane) return;
  void attachFiles(pane, [screenshot.file]);
  dismissScreenshot();
  focusComposer(pane);
}

export async function chooseScreenshotFolder() {
  try {
    screenshotFolder.set((await commands.pickScreenshotFolder()) ?? screenshotFolder.get());
  } catch (error) {
    fail(error);
  }
}

export function stopSavingScreenshots() {
  screenshotFolder.set(null);
  commands.changeState({ screenshotFolder: null }).catch(console.error);
}

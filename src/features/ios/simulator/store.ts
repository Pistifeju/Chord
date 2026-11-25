import { Channel } from "@tauri-apps/api/core";
import { commands, type Bezel, type Button, type ScreenButtons, type ScreenRect, type Screenshot, type ScreenState_Deserialize, type TouchPhase } from "../../../ipc/bindings";
import { externalStore } from "../../../ui/externalStore";
import { imageMime } from "../../chat/attachments";
import { attachFiles, chatOf, chats, focusComposer, takesAttachments } from "../../chat/store";
import { chatPaneIds } from "../../panes/layout";
import { shownPanes } from "../../worktrees/store";
import { fail } from "../../../ui/alert";
import type { Point } from "./screen";

export type Screen = { worktree: string; state: ScreenState_Deserialize };

export const screen = externalStore<Screen | null>(null);

let turn = 0;
let touchSeq = 0;

export function showScreen(worktree: string, preview: boolean, rect: ScreenRect, bezel: Bezel) {
  const mine = ++turn;
  screen.set(null);
  const channel = new Channel<ScreenState_Deserialize>();
  channel.onmessage = (state) => {
    if (turn === mine) screen.set({ worktree, state });
  };
  commands.showSimulatorScreen(worktree, preview, rect, bezel, channel).catch(fail);
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

export const sendScroll = (worktree: string, at: Point, delta: Point) =>
  void commands.sendSimulatorInput(worktree, { kind: "scroll", at, delta }).catch(console.error);

export const sendKey = (worktree: string, usage: number, down: boolean) =>
  void commands.sendSimulatorInput(worktree, { kind: "key", usage, down }).catch(console.error);

export function pressButton(worktree: string, button: Button) {
  const shown = screen.get();
  const state = shown?.worktree === worktree ? shown.state : null;
  const unlocks = button === "lock" && state?.state === "live" && state.locked;
  commands.pressSimulatorButton(worktree, unlocks ? "home" : button).catch(fail);
}

export const openSimulatorApp = () => void commands.openSimulatorApp().catch(fail);

export type Shot = { worktree: string; screenshot: Screenshot; preview: string };

export const shot = externalStore<Shot | null>(null);

export const screenshotFolder = externalStore<string | null>(null);

const showShot = async (worktree: string, screenshot: Screenshot) =>
  shot.set({ worktree, screenshot, preview: `data:image/png;base64,${await commands.readImageFile(screenshot.preview)}` });

export async function takeScreenshot(worktree: string, preview = false) {
  try {
    await showShot(worktree, await commands.takeSimulatorScreenshot(worktree, preview));
  } catch (error) {
    fail(error);
  }
}

export type Recording = { phase: "starting" | "recording" | "stopping"; since: number | null };

export const recordings = externalStore<Record<string, Recording>>({});

export const recordingKey = (worktree: string, preview: boolean) => `${preview ? "preview" : "run"}:${worktree}`;

function setRecording(key: string, state: Recording | null) {
  const { [key]: _, ...rest } = recordings.get();
  recordings.set(state ? { ...rest, [key]: state } : rest);
}

export async function toggleRecording(worktree: string, preview = false) {
  const key = recordingKey(worktree, preview);
  const state = recordings.get()[key];
  if (state && state.phase !== "recording") return;
  if (state) {
    setRecording(key, { ...state, phase: "stopping" });
    return void commands.stopSimulatorRecording(worktree, preview).catch(fail);
  }
  setRecording(key, { phase: "starting", since: null });
  const started = new Channel<null>();
  started.onmessage = () => recordings.get()[key]?.phase === "starting" && setRecording(key, { phase: "recording", since: Date.now() });
  try {
    const screenshot = await commands.recordSimulator(worktree, preview, started);
    setRecording(key, null);
    await showShot(worktree, screenshot);
  } catch (error) {
    setRecording(key, null);
    fail(error);
  }
}

export const dismissScreenshot = () => shot.set(null);

export const dragScreenshot = ({ screenshot }: Shot) => void commands.dragFileOut(screenshot.file, screenshot.preview).then(dismissScreenshot, fail);

export const attachable = ({ screenshot }: Shot) => imageMime(screenshot.file) !== null;

export const attachingPane = (): string | null => {
  const shown = shownPanes();
  const chat = shown?.focused && chatPaneIds(shown.layout).includes(shown.focused) ? chatOf(chats.get(), shown.focused) : null;
  return chat && takesAttachments(chat) ? shown!.focused : null;
};

export function pasteScreenshot(current: Shot) {
  const { screenshot } = current;
  const pane = attachingPane();
  if (!pane || !attachable(current)) return;
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

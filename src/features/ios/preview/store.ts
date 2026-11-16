// Connects the Preview view to IPC: each worktree's preview opened on its preview simulator, and the states its channel brings.
import { Channel } from "@tauri-apps/api/core";
import { commands, type PreviewState } from "../../../ipc/bindings";
import { externalStore } from "../../../ui/externalStore";
import { fail } from "../../worktrees/store";
import { runDestinationOf } from "../build/destinationStore";
import { appliedState, type Preview } from "./preview";

export const previews = externalStore<Record<string, Preview>>({});

/** The newest open per worktree: an older channel's late states are ignored. */
const latest = new Map<string, number>();

/** Returns the open's number, which `closePreview` takes. */
export function openPreview(worktree: string): number {
  const number = (latest.get(worktree) ?? 0) + 1;
  latest.set(worktree, number);
  const channel = new Channel<PreviewState>();
  channel.onmessage = (state) => {
    if (latest.get(worktree) !== number) return;
    const before = previews.get()[worktree];
    const after = appliedState(before, state);
    previews.set({ ...previews.get(), [worktree]: after });
    // A failure while the last preview stays on screen has no line to show it.
    if (state.state === "failed" && after.shown) fail(state.message);
  };
  commands.openPreview(worktree, runDestinationOf(worktree), channel).catch(fail);
  return number;
}

/** The view closed: saves stop rebuilding its preview, unless a newer view of the worktree opened since. */
export function closePreview(worktree: string, number: number) {
  if (latest.get(worktree) === number) commands.closePreview(worktree).catch(fail);
}

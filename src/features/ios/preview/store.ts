// Connects the Preview view to IPC: each worktree's preview opened on its preview simulator with its pick, the states its channel brings, the picks per worktree and the models per project saved through Rust, and each worktree's `#Preview` list.
import { Channel } from "@tauri-apps/api/core";
import { commands, type PreviewPick, type PreviewState, type RunDestination } from "../../../ipc/bindings";
import { externalStore } from "../../../ui/externalStore";
import { fail } from "../../../ui/alert";
import { projectPicks } from "../projectPicks";
import { appliedState, type Preview } from "./preview";

export const previews = externalStore<Record<string, Preview>>({});

/** Each worktree's pick; it never follows the editor. */
export const previewPicks = externalStore<Record<string, PreviewPick>>({});

/** Each project's preview model; a project without one previews on the newest iPhone. */
const previewModels = projectPicks<RunDestination>((project, model) => commands.changeState({ previewModel: { project, model } }));

/** Each worktree's previews; a worktree is missing while its first list loads. */
export const previewLists = externalStore<Record<string, PreviewPick[]>>({});

export function loadPreviewChoices(picks: Record<string, PreviewPick>, models: Record<string, RunDestination>) {
  previewPicks.set(picks);
  previewModels.load(models);
}

const previewModelOf = (worktree: string): RunDestination | null => previewModels.of(worktree) ?? null;

export const usePreviewModel = (worktree: string): RunDestination | null => previewModels.use(worktree) ?? null;

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
  commands.openPreview(worktree, previewModelOf(worktree), previewPicks.get()[worktree] ?? null, channel).catch(fail);
  return number;
}

/** The view closed: saves stop rebuilding its preview, unless a newer view of the worktree opened since. */
export function closePreview(worktree: string, number: number) {
  if (latest.get(worktree) === number) commands.closePreview(worktree).catch(fail);
}

export function loadPreviewList(worktree: string) {
  commands
    .listPreviews(worktree)
    .then((list) => previewLists.set({ ...previewLists.get(), [worktree]: list }))
    .catch(fail);
}

/** The list column's width; it lasts until the app quits. */
export const previewListWidth = externalStore(200);

export const resizePreviewList = (width: number) => previewListWidth.set(Math.round(Math.min(320, Math.max(140, width))));

const pendingPicks = new Map<string, ReturnType<typeof setTimeout>>();

/** The list shows the pick at once; the preview switches once ↑ or ↓ rest for 150 ms. */
export function choosePreview(worktree: string, pick: PreviewPick) {
  // The view opens again on the first pick, which boots and builds it.
  const first = !previewPicks.get()[worktree];
  previewPicks.set({ ...previewPicks.get(), [worktree]: pick });
  clearTimeout(pendingPicks.get(worktree));
  pendingPicks.set(
    worktree,
    setTimeout(() => {
      pendingPicks.delete(worktree);
      commands.changeState({ previewPick: { worktree, pick } }).catch(fail);
      if (!first) commands.pickPreview(worktree, pick).catch(fail);
    }, 150),
  );
}

/** The view reopens the preview on the new model's simulator. */
export const choosePreviewModel = (worktree: string, model: RunDestination) => previewModels.choose(worktree, model);

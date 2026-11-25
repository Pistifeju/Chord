import { Channel } from "@tauri-apps/api/core";
import { commands, type PreviewPick, type PreviewState, type RunDestination } from "../../../ipc/bindings";
import { externalStore } from "../../../ui/externalStore";
import { fail } from "../../../ui/alert";
import { projectPicks } from "../projectPicks";
import { appliedState, type Preview } from "./preview";

export const previews = externalStore<Record<string, Preview>>({});

export const previewPicks = externalStore<Record<string, PreviewPick>>({});

const previewModels = projectPicks<RunDestination>((project, model) => commands.changeState({ previewModel: { project, model } }));

export const previewLists = externalStore<Record<string, PreviewPick[]>>({});

export function loadPreviewChoices(picks: Record<string, PreviewPick>, models: Record<string, RunDestination>) {
  previewPicks.set(picks);
  previewModels.load(models);
}

const previewModelOf = (worktree: string): RunDestination | null => previewModels.of(worktree) ?? null;

export const usePreviewModel = (worktree: string): RunDestination | null => previewModels.use(worktree) ?? null;

const latest = new Map<string, number>();

export function openPreview(worktree: string): number {
  const number = (latest.get(worktree) ?? 0) + 1;
  latest.set(worktree, number);
  const channel = new Channel<PreviewState>();
  channel.onmessage = (state) => {
    if (latest.get(worktree) !== number) return;
    const before = previews.get()[worktree];
    const after = appliedState(before, state);
    previews.set({ ...previews.get(), [worktree]: after });
    if (state.state === "failed" && after.shown) fail(state.message);
  };
  commands.openPreview(worktree, previewModelOf(worktree), previewPicks.get()[worktree] ?? null, channel).catch(fail);
  return number;
}

export function closePreview(worktree: string, number: number) {
  if (latest.get(worktree) === number) commands.closePreview(worktree).catch(fail);
}

export function loadPreviewList(worktree: string) {
  commands
    .listPreviews(worktree)
    .then((list) => previewLists.set({ ...previewLists.get(), [worktree]: list }))
    .catch(fail);
}

export const previewListWidth = externalStore(200);

export const resizePreviewList = (width: number) => previewListWidth.set(Math.round(Math.min(320, Math.max(140, width))));

const pendingPicks = new Map<string, ReturnType<typeof setTimeout>>();

export function choosePreview(worktree: string, pick: PreviewPick) {
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

export const choosePreviewModel = (worktree: string, model: RunDestination) => previewModels.choose(worktree, model);

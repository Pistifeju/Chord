// Connects the Preview view to IPC: each worktree's preview opened on its preview simulator with its pick, the states its channel brings, the picks per worktree and the models per project saved through Rust, and the `#Preview` list the picker shows.
import { Channel } from "@tauri-apps/api/core";
import { commands, type PreviewPick, type PreviewState, type RunDestination } from "../../../ipc/bindings";
import { closeSheet, openSheet } from "../../../app/sheet";
import { externalStore } from "../../../ui/externalStore";
import { ownerOf } from "../../worktrees/projects";
import { projects } from "../../worktrees/store";
import { fail } from "../../../ui/alert";
import { appliedState, type Preview } from "./preview";

export const previews = externalStore<Record<string, Preview>>({});

/** Each worktree's pick; it never follows the editor. */
export const previewPicks = externalStore<Record<string, PreviewPick>>({});

/** Each project's preview model; a project without one previews on the newest iPhone. */
const previewModels = externalStore<Record<string, RunDestination>>({});

/** The previews the open picker lists; null while they load. */
export const previewList = externalStore<PreviewPick[] | null>(null);

export function loadPreviewChoices(picks: Record<string, PreviewPick>, models: Record<string, RunDestination>) {
  previewPicks.set(picks);
  previewModels.set(models);
}

const previewModelOf = (worktree: string): RunDestination | null => previewModels.get()[ownerOf(projects.get(), worktree) ?? ""] ?? null;

export const usePreviewModel = (worktree: string): RunDestination | null => previewModels.usePart(() => previewModelOf(worktree));

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

export function openPreviewPicker(worktree: string) {
  previewList.set(null);
  openSheet({ kind: "previewPicker", worktree });
  commands
    .listPreviews(worktree)
    .then((list) => previewList.set(list))
    .catch(fail);
}

export function choosePreview(worktree: string, pick: PreviewPick) {
  closeSheet("previewPicker");
  // The view opens again on the first pick, which boots and builds it.
  const first = !previewPicks.get()[worktree];
  previewPicks.set({ ...previewPicks.get(), [worktree]: pick });
  commands.changeState({ previewPick: { worktree, pick } }).catch(fail);
  if (!first) commands.pickPreview(worktree, pick).catch(fail);
}

/** The view reopens the preview on the new model's simulator. */
export function choosePreviewModel(worktree: string, model: RunDestination) {
  const project = ownerOf(projects.get(), worktree);
  if (!project) return;
  previewModels.set({ ...previewModels.get(), [project]: model });
  commands.changeState({ previewModel: { project, model } }).catch(fail);
}

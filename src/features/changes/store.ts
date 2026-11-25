import { commands, type BranchChanges_Serialize as BranchChanges, type Change, type CommitTarget_Serialize as CommitTarget, type CommittedFile_Serialize as CommittedFile, type Hunk, type HunkAction, type Stash, type StashAction } from "../../ipc/bindings";
import { closeSheet, openSheet, sheet, sheetOf } from "../../app/sheet";
import { externalStore } from "../../ui/externalStore";
import { pushedNotice } from "./branch";
import { canCommit } from "./changes";
import { fail, notify } from "../../ui/alert";
import { flushAllSaves } from "../editor/store";
import { reloadFiles, trees } from "../files/store";
import { closeFilePanesAt, openFileTab } from "../panes/store";
import { refreshGitCounts } from "../worktrees/gitCountsRefresh";

export const changes = externalStore<Record<string, Change[]>>({});

export const commitTargets = externalStore<Record<string, CommitTarget>>({});

export const branches = externalStore<Record<string, BranchChanges | null>>({});

export type Scope = "uncommitted" | "branch";

export const scopes = externalStore<Record<string, Scope>>({});

export const setScope = (worktree: string, scope: Scope) => scopes.set({ ...scopes.get(), [worktree]: scope });

const pickedBases = new Map<string, string>();

export function pickBase(worktree: string, base: string) {
  if (base === branches.get()[worktree]?.defaultBase) pickedBases.delete(worktree);
  else pickedBases.set(worktree, base);
  void readChanges(worktree);
}

export const commitFiles = externalStore<Record<string, CommittedFile[]>>({});

export const readCommitFiles = (worktree: string, hash: string) => readFilesOf(hash, () => commands.listCommitFiles(worktree, hash));

export const readStashFiles = (worktree: string, hash: string) => readFilesOf(hash, () => commands.listStashFiles(worktree, hash));

async function readFilesOf(hash: string, list: () => Promise<CommittedFile[]>) {
  if (commitFiles.get()[hash]) return;
  try {
    const files = await list();
    commitFiles.set({ ...commitFiles.get(), [hash]: files });
  } catch (error) {
    fail(error);
  }
}

export const stashes = externalStore<Record<string, Stash[]>>({});

export const openStashGroups = externalStore<Record<string, boolean>>({});

export const toggleStashGroup = (worktree: string) => openStashGroups.set({ ...openStashGroups.get(), [worktree]: !openStashGroups.get()[worktree] });

export const commitMessages = externalStore<Record<string, string>>({});

export type CommitOptions = { pushAfter: boolean; amend: boolean; amendedMessage?: string };
export const commitOptions = externalStore<Record<string, CommitOptions>>({});

export const pushAfterCommit = externalStore(false);

const defaults: Record<"push" | "stay", CommitOptions> = { push: { pushAfter: true, amend: false }, stay: { pushAfter: false, amend: false } };
export const optionsOf = (all: Record<string, CommitOptions>, worktree: string, push: boolean): CommitOptions => all[worktree] ?? defaults[push ? "push" : "stay"];
const currentOptions = (worktree: string) => optionsOf(commitOptions.get(), worktree, pushAfterCommit.get());
const setOptions = (worktree: string, options: CommitOptions) => commitOptions.set({ ...commitOptions.get(), [worktree]: options });

export type Running = "commit" | "push" | "fetch" | "pull" | "merge";

export const running = externalStore<Record<string, Running>>({});

const reading = new Map<string, boolean>();

const acted = new Set<string>();
const actedLists = new WeakSet<Change[]>();

export const followsAct = (list: Change[]) => actedLists.has(list);

export async function readChanges(worktree: string) {
  if (reading.has(worktree)) return void reading.set(worktree, true);
  reading.set(worktree, false);
  const afterAct = acted.delete(worktree);
  try {
    const [list, target, branch, stashList] = await Promise.all([
      commands.listChanges(worktree),
      commands.readCommitTarget(worktree),
      commands.readBranch(worktree, pickedBases.get(worktree) ?? null),
      commands.listStashes(worktree),
    ]);
    if (afterAct) actedLists.add(list);
    changes.set({ ...changes.get(), [worktree]: list });
    commitTargets.set({ ...commitTargets.get(), [worktree]: target });
    branches.set({ ...branches.get(), [worktree]: branch });
    stashes.set({ ...stashes.get(), [worktree]: stashList });
  } catch (error) {
    fail(error);
  } finally {
    const again = reading.get(worktree);
    reading.delete(worktree);
    if (again) void readChanges(worktree);
  }
}

export const showDiff = (worktree: string, change: Change, preview = true) => openFileTab(worktree, change.path, preview, change.staged ? "staged" : "unstaged");

const pathsOf = (list: Change[]) => list.map((c) => c.path);

async function act(worktree: string, run: () => Promise<unknown>) {
  try {
    await flushAllSaves();
    await run();
  } catch (error) {
    fail(error);
  }
  acted.add(worktree);
  await readChanges(worktree);
}

export const stageChanges = (worktree: string, list: Change[]) => act(worktree, () => commands.stageFiles(worktree, pathsOf(list)));

export const unstageChanges = (worktree: string, list: Change[]) => act(worktree, () => commands.unstageFiles(worktree, pathsOf(list)));

export const applyHunk = (worktree: string, path: string, hunk: Hunk, action: Exclude<HunkAction, "discard" | "discardStaged">) => act(worktree, () => commands.applyHunk(worktree, path, hunk, action));

export const requestDiscard = (worktree: string, list: Change[], hunk?: Hunk) => openSheet({ kind: "discardChanges", worktree, changes: list, ...(hunk && { hunk }) });

export const goesToTrash = (c: Change) => c.status === "untracked" || (c.staged && c.status === "added");

export async function confirmDiscard() {
  const pending = sheetOf(sheet.get(), "discardChanges");
  closeSheet("discardChanges");
  if (!pending) return;
  const { worktree, changes: list, hunk } = pending;
  await act(worktree, async () => {
    if (hunk && !goesToTrash(list[0])) await commands.applyHunk(worktree, list[0].path, hunk, list[0].staged ? "discardStaged" : "discard");
    else await commands.discardFiles(worktree, pathsOf(list), list[0].staged);
    for (const change of list) if (goesToTrash(change)) closeFilePanesAt(worktree, change.path);
  });
}

export const typeCommitMessage = (worktree: string, message: string) => commitMessages.set({ ...commitMessages.get(), [worktree]: message });

async function runGit(worktree: string, kind: Running, run: () => Promise<unknown>) {
  if (running.get()[worktree]) return;
  running.set({ ...running.get(), [worktree]: kind });
  try {
    await run();
  } catch (error) {
    fail(error);
  }
  const { [worktree]: _, ...rest } = running.get();
  running.set(rest);
  refreshGitCounts();
  if ((kind === "pull" || kind === "merge") && trees.get()[worktree]) void reloadFiles(worktree);
  await readChanges(worktree);
}

export function commitStaged(worktree: string) {
  const message = commitMessages.get()[worktree] ?? "";
  const options = currentOptions(worktree);
  if (!canCommit(message, options.amend, !!changes.get()[worktree]?.some((c) => c.staged))) return;
  return runGit(worktree, "commit", async () => {
    await commands.commitStaged(worktree, message, options.amend);
    if (commitMessages.get()[worktree] === message) typeCommitMessage(worktree, "");
    if (!options.amend && options.pushAfter !== pushAfterCommit.get()) {
      pushAfterCommit.set(options.pushAfter);
      commands.changeState({ pushAfterCommit: options.pushAfter }).catch(console.error);
    }
    const { [worktree]: _, ...rest } = commitOptions.get();
    commitOptions.set(rest);
    if (!options.pushAfter || !commitTargets.get()[worktree]?.pushTo) return;
    running.set({ ...running.get(), [worktree]: "push" });
    await commands.pushBranch(worktree);
  });
}

export function pushBranch(worktree: string) {
  const target = commitTargets.get()[worktree];
  return runGit(worktree, "push", async () => {
    await commands.pushBranch(worktree);
    if (target?.pushTo) notify(pushedNotice(target.ahead, target.pushTo));
  });
}

export const fetchRemote = (worktree: string) => runGit(worktree, "fetch", async () => notify(`Fetched ${await commands.fetchRemote(worktree)}`));

export function pullBranch(worktree: string) {
  const from = commitTargets.get()[worktree]?.pushTo;
  return runGit(worktree, "pull", async () => {
    await flushAllSaves();
    notify((await commands.pullBranch(worktree)) && from ? `Pulled from ${from}` : "Already up to date");
  });
}

export const mergeBranch = (worktree: string, branch: string, into: string) =>
  runGit(worktree, "merge", async () => {
    await flushAllSaves();
    notify((await commands.mergeBranch(worktree, branch)) ? `Merged ${branch} into ${into}` : `Already up to date with ${branch}`);
  });

export async function pickCommitMode(worktree: string, amend: boolean, pushAfter: boolean) {
  const options = currentOptions(worktree);
  const message = commitMessages.get()[worktree] ?? "";
  if (!amend) {
    if (options.amend && message === options.amendedMessage) typeCommitMessage(worktree, "");
    return setOptions(worktree, { pushAfter, amend: false });
  }
  setOptions(worktree, { ...options, amend: true, pushAfter });
  if (options.amend || message.trim()) return;
  try {
    const last = await commands.lastCommitMessage(worktree);
    if (!currentOptions(worktree).amend || (commitMessages.get()[worktree] ?? "") !== message) return;
    typeCommitMessage(worktree, last);
    setOptions(worktree, { ...currentOptions(worktree), amendedMessage: last });
  } catch (error) {
    fail(error);
  }
}

async function actOnStashes(worktree: string, run: () => Promise<unknown>) {
  await act(worktree, run);
  refreshGitCounts();
}

export function stashChanges(worktree: string) {
  const count = new Set(changes.get()[worktree]?.map((c) => c.path)).size;
  return actOnStashes(worktree, async () => {
    await commands.stashChanges(worktree);
    notify(count === 1 ? "Stashed 1 file" : `Stashed ${count} files`);
  });
}

export const actOnStash = (worktree: string, hash: string, action: Exclude<StashAction, "drop">) => actOnStashes(worktree, () => commands.actOnStash(worktree, hash, action));

export const requestDropStash = (worktree: string, stash: Stash) => openSheet({ kind: "dropStash", worktree, stash });

export async function confirmDropStash() {
  const pending = sheetOf(sheet.get(), "dropStash");
  closeSheet("dropStash");
  if (pending) await actOnStashes(pending.worktree, () => commands.actOnStash(pending.worktree, pending.stash.hash, "drop"));
}

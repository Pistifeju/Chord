// Connects the Changes view to IPC: each worktree's changed files read from git with its branch against its base, one read at a time, a failed read leaving the last list, the scope and base each worktree shows, the files of an opened commit, a change's diff opened in a tab, the stage, unstage and discard of files and hunks, each followed by a read, and the commit message with its options, commit, amend and push, each followed by a read and the git counts.
import { commands, type BranchChanges_Serialize as BranchChanges, type Change, type CommitTarget_Serialize as CommitTarget, type CommittedFile_Serialize as CommittedFile, type Hunk, type HunkAction } from "../../ipc/bindings";
import { closeSheet, openSheet, sheet, sheetOf } from "../../app/sheet";
import { externalStore } from "../../ui/externalStore";
import { canCommit } from "./changes";
import { fail } from "../../ui/alert";
import { flushAllSaves } from "../editor/store";
import { closeFilePanesAt, openFileTab } from "../panes/store";
import { refreshGitCounts } from "../worktrees/gitCountsRefresh";

/** Each worktree's changes; a new list per read, which an open diff follows. */
export const changes = externalStore<Record<string, Change[]>>({});

/** Who each worktree commits as and where its push goes, read with its changes. */
export const commitTargets = externalStore<Record<string, CommitTarget>>({});

/** Each worktree's branch against its base; null without a base, which leaves Uncommitted the only scope. */
export const branches = externalStore<Record<string, BranchChanges | null>>({});

export type Scope = "uncommitted" | "branch";

/** Each worktree's scope as picked; Branch shows only while the worktree has a base. */
export const scopes = externalStore<Record<string, Scope>>({});

export const setScope = (worktree: string, scope: Scope) => scopes.set({ ...scopes.get(), [worktree]: scope });

// The base each worktree compares with in place of its default, until the app quits.
const pickedBases = new Map<string, string>();

export function pickBase(worktree: string, base: string) {
  if (base === branches.get()[worktree]?.defaultBase) pickedBases.delete(worktree);
  else pickedBases.set(worktree, base);
  void readChanges(worktree);
}

/** The files of each commit opened, by hash; a commit never changes, so one read serves for good. */
export const commitFiles = externalStore<Record<string, CommittedFile[]>>({});

export async function readCommitFiles(worktree: string, hash: string) {
  if (commitFiles.get()[hash]) return;
  try {
    const files = await commands.listCommitFiles(worktree, hash);
    commitFiles.set({ ...commitFiles.get(), [hash]: files });
  } catch (error) {
    fail(error);
  }
}

/** Each worktree's commit message as typed. */
export const commitMessages = externalStore<Record<string, string>>({});

/** Each worktree's commit options: push right after the commit, amend the last commit with the last message it filled in. */
export type CommitOptions = { pushAfter: boolean; amend: boolean; amendedMessage?: string };
export const commitOptions = externalStore<Record<string, CommitOptions>>({});
// One object for every worktree without options, since a store hook re-renders on each new object it reads.
const noOptions: CommitOptions = { pushAfter: false, amend: false };
export const optionsOf = (all: Record<string, CommitOptions>, worktree: string): CommitOptions => all[worktree] ?? noOptions;
const setOptions = (worktree: string, options: CommitOptions) => commitOptions.set({ ...commitOptions.get(), [worktree]: options });

/** The commit or push that runs in each worktree, which takes no second click. */
export const running = externalStore<Record<string, "commit" | "push">>({});

// Each worktree with a read in flight, and whether another read was asked for meanwhile.
const reading = new Map<string, boolean>();

// The worktrees Chord staged, unstaged or discarded in since their last read began, and the lists read after that.
const acted = new Set<string>();
const actedLists = new WeakSet<Change[]>();

/** Whether the list came from a read after Chord's own stage, unstage or discard, so the selection may move on. */
export const followsAct = (list: Change[]) => actedLists.has(list);

export async function readChanges(worktree: string) {
  if (reading.has(worktree)) return void reading.set(worktree, true);
  reading.set(worktree, false);
  const afterAct = acted.delete(worktree);
  try {
    const [list, target, branch] = await Promise.all([commands.listChanges(worktree), commands.readCommitTarget(worktree), commands.readBranch(worktree, pickedBases.get(worktree) ?? null)]);
    if (afterAct) actedLists.add(list);
    changes.set({ ...changes.get(), [worktree]: list });
    commitTargets.set({ ...commitTargets.get(), [worktree]: target });
    branches.set({ ...branches.get(), [worktree]: branch });
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
    // A pending edit would land after git acts: staged without it, or brought back after a discard.
    await flushAllSaves();
    await run();
  } catch (error) {
    fail(error);
  }
  acted.add(worktree);
  await readChanges(worktree);
}

export const stageChanges = (worktree: string, list: Change[]) => act(worktree, () => commands.stageFiles(worktree, pathsOf(list)));

// A rename's old path goes back into the index with its new one.
const indexPaths = (list: Change[]) => list.flatMap((c) => (c.oldPath ? [c.path, c.oldPath] : [c.path]));

export const unstageChanges = (worktree: string, list: Change[]) => act(worktree, () => commands.unstageFiles(worktree, indexPaths(list)));

export const applyHunk = (worktree: string, path: string, hunk: Hunk, action: Exclude<HunkAction, "discard" | "discardStaged">) => act(worktree, () => commands.applyHunk(worktree, path, hunk, action));

/** Asks before the changes, or with `hunk` that hunk of the one change, are discarded. */
export const requestDiscard = (worktree: string, list: Change[], hunk?: Hunk) => openSheet({ kind: "discardChanges", worktree, changes: list, ...(hunk && { hunk }) });

/** Whether discarding the change moves its file to the Trash: git holds no copy of it to restore. */
export const goesToTrash = (c: Change) => c.status === "untracked" || (c.staged && c.status === "added");

export async function confirmDiscard() {
  const pending = sheetOf(sheet.get(), "discardChanges");
  closeSheet("discardChanges");
  if (!pending) return;
  const { worktree, changes: list, hunk } = pending;
  await act(worktree, async () => {
    // git holds no copy of a new file, so its hunk goes the file's way, to the Trash.
    if (hunk && !goesToTrash(list[0])) await commands.applyHunk(worktree, list[0].path, hunk, list[0].staged ? "discardStaged" : "discard");
    else if (list[0].staged) {
      // Unstaged first, the files go back to HEAD with the restore from the index.
      await commands.unstageFiles(worktree, indexPaths(list));
      await commands.discardFiles(worktree, indexPaths(list));
    } else await commands.discardFiles(worktree, pathsOf(list));
    // A file that went to the Trash loses its tabs, as in the Files tree.
    for (const change of list) if (goesToTrash(change)) closeFilePanesAt(worktree, change.path);
  });
}

export const typeCommitMessage = (worktree: string, message: string) => commitMessages.set({ ...commitMessages.get(), [worktree]: message });

async function runGit(worktree: string, kind: "commit" | "push", run: () => Promise<unknown>) {
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
  await readChanges(worktree);
}

export function commitStaged(worktree: string) {
  const message = commitMessages.get()[worktree] ?? "";
  const options = optionsOf(commitOptions.get(), worktree);
  if (!canCommit(message, options.amend, !!changes.get()[worktree]?.some((c) => c.staged))) return;
  return runGit(worktree, "commit", async () => {
    await commands.commitStaged(worktree, message, options.amend);
    // Text typed while the commit ran stays.
    if (commitMessages.get()[worktree] === message) typeCommitMessage(worktree, "");
    setOptions(worktree, { pushAfter: options.pushAfter, amend: false });
    if (!options.pushAfter || !commitTargets.get()[worktree]?.pushTo) return;
    running.set({ ...running.get(), [worktree]: "push" });
    await commands.pushBranch(worktree);
  });
}

export const pushBranch = (worktree: string) => runGit(worktree, "push", () => commands.pushBranch(worktree));

export const togglePushAfter = (worktree: string) => {
  const options = optionsOf(commitOptions.get(), worktree);
  setOptions(worktree, { ...options, pushAfter: !options.pushAfter });
};

/** Amend fills an empty field with the last commit's message, and taking it off empties the field while that message stands unchanged. */
export async function toggleAmend(worktree: string) {
  const options = optionsOf(commitOptions.get(), worktree);
  const message = commitMessages.get()[worktree] ?? "";
  if (options.amend) {
    if (message === options.amendedMessage) typeCommitMessage(worktree, "");
    return setOptions(worktree, { pushAfter: options.pushAfter, amend: false });
  }
  setOptions(worktree, { ...options, amend: true });
  if (message.trim()) return;
  try {
    const last = await commands.lastCommitMessage(worktree);
    if (!optionsOf(commitOptions.get(), worktree).amend || (commitMessages.get()[worktree] ?? "") !== message) return;
    typeCommitMessage(worktree, last);
    setOptions(worktree, { ...optionsOf(commitOptions.get(), worktree), amendedMessage: last });
  } catch (error) {
    fail(error);
  }
}

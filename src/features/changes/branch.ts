// The Changes view's branch line and menu: whether the menu shows, the button's label, the Fetch, Pull and Push tooltips and when each takes a click, the push notice, each branch row's last commit or holder, and what a branch's actions allow.
import type { CommitTarget_Serialize as CommitTarget, LocalBranch } from "../../ipc/bindings";
import { externalStore } from "../../ui/externalStore";
import { fileName } from "../files/paths";
import { ageLabel } from "../status/status";

export const branchMenuOpen = externalStore(false);

export const openBranchMenu = () => branchMenuOpen.set(true);

export const closeBranchMenu = () => branchMenuOpen.set(false);

export const branchLabel = (branch: string | null | undefined): string => branch ?? "Detached HEAD";

/** "3h ago", or "just now" in the first minute. */
function ago(time: number, now: number): string {
  const age = ageLabel(time, now);
  return age === "now" ? "just now" : `${age} ago`;
}

/** "In harbor-flaky", for a branch another worktree holds. */
const holderLabel = (heldBy: string) => `In ${fileName(heldBy)}`;

/** "1 commit", "2 Commits": `word` sets the case. */
const commits = (count: number, word: "commit" | "Commit") => `${count} ${word}${count === 1 ? "" : "s"}`;

type SyncButton = { title: string; count: number | null; disabled: boolean };

/** The Fetch tooltip, and Pull and Push with their counts; null while a push has nowhere to go: a detached HEAD or no remote. */
export function syncButtons(target: CommitTarget | undefined, now: number): { fetch: string; pull: SyncButton; push: SyncButton } | null {
  if (!target?.pushTo) return null;
  const { ahead, behind, fetched, pushTo, pushable } = target;
  return {
    fetch: fetched == null ? "Fetch" : `Fetch (fetched ${ago(fetched, now)})`,
    pull: { title: behind ? `Pull ${commits(behind, "Commit")}` : "Pull", count: behind, disabled: behind == null },
    push: { title: ahead ? `Push ${commits(ahead, "Commit")}` : pushable ? `Push to ${pushTo}` : "Push", count: ahead, disabled: !pushable },
  };
}

/** "Pushed 2 commits to origin/main", or "Pushed to origin/feat" the first time. */
export const pushedNotice = (ahead: number | null, pushTo: string) => (ahead ? `Pushed ${commits(ahead, "commit")} to ${pushTo}` : `Pushed to ${pushTo}`);

/** No switch to the current branch or to one another worktree holds, which git also keeps from being deleted; no merge of the current branch into itself. */
export function branchActionRules({ name, heldBy }: LocalBranch, current: string | null) {
  const taken = name === current || !!heldBy;
  return { switchHint: heldBy ? holderLabel(heldBy) : undefined, switchable: !taken, mergeable: name !== current, deletable: !taken };
}

/** "Last commit 3h ago", or "In harbor-flaky · 5h" for a branch another worktree holds. */
export function branchHint(branch: LocalBranch, now: number): string {
  if (branch.heldBy) return `${holderLabel(branch.heldBy)} · ${ageLabel(branch.time ?? now, now)}`;
  return `Last commit ${ago(branch.time ?? now, now)}`;
}

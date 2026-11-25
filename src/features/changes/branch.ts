import type { CommitTarget_Serialize as CommitTarget, LocalBranch } from "../../ipc/bindings";
import { externalStore } from "../../ui/externalStore";
import { fileName } from "../files/paths";
import { ageLabel } from "../status/status";

export const branchMenuOpen = externalStore(false);

export const openBranchMenu = () => branchMenuOpen.set(true);

export const closeBranchMenu = () => branchMenuOpen.set(false);

export const branchLabel = (branch: string | null | undefined): string => branch ?? "Detached HEAD";

function ago(time: number, now: number): string {
  const age = ageLabel(time, now);
  return age === "now" ? "just now" : `${age} ago`;
}

const holderLabel = (heldBy: string) => `In ${fileName(heldBy)}`;

const commits = (count: number, word: "commit" | "Commit") => `${count} ${word}${count === 1 ? "" : "s"}`;

type SyncButton = { title: string; count: number | null; disabled: boolean };

export function syncButtons(target: CommitTarget | undefined, now: number): { fetch: string; pull: SyncButton; push: SyncButton } | null {
  if (!target?.pushTo) return null;
  const { ahead, behind, fetched, pushTo, pushable } = target;
  return {
    fetch: fetched == null ? "Fetch" : `Fetch (fetched ${ago(fetched, now)})`,
    pull: { title: behind ? `Pull ${commits(behind, "Commit")}` : "Pull", count: behind, disabled: behind == null },
    push: { title: ahead ? `Push ${commits(ahead, "Commit")}` : pushable ? `Push to ${pushTo}` : "Push", count: ahead, disabled: !pushable },
  };
}

export const pushedNotice = (ahead: number | null, pushTo: string) => (ahead ? `Pushed ${commits(ahead, "commit")} to ${pushTo}` : `Pushed to ${pushTo}`);

export function branchActionRules({ name, heldBy }: LocalBranch, current: string | null) {
  const taken = name === current || !!heldBy;
  return { switchHint: heldBy ? holderLabel(heldBy) : undefined, switchable: !taken, mergeable: name !== current, deletable: !taken };
}

export function branchHint(branch: LocalBranch, now: number): string {
  if (branch.heldBy) return `${holderLabel(branch.heldBy)} · ${ageLabel(branch.time ?? now, now)}`;
  return `Last commit ${ago(branch.time ?? now, now)}`;
}

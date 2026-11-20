// The Changes view's branch menu: whether it shows, the button's label, when the menu takes a Filter field, and each branch row's last commit or holder.
import type { LocalBranch } from "../../ipc/bindings";
import { externalStore } from "../../ui/externalStore";
import { fileName } from "../files/paths";
import { ageLabel } from "../status/status";

export const branchMenuOpen = externalStore(false);

export const openBranchMenu = () => branchMenuOpen.set(true);

export const closeBranchMenu = () => branchMenuOpen.set(false);

export const branchLabel = (branch: string | null | undefined): string => branch ?? "Detached HEAD";

/** "Last commit 3h ago", or "In harbor-flaky · 5h" for a branch another worktree holds. */
export function branchHint(branch: LocalBranch, now: number): string {
  const age = ageLabel(branch.time ?? now, now);
  if (branch.heldBy) return `In ${fileName(branch.heldBy)} · ${age}`;
  return age === "now" ? "Last commit just now" : `Last commit ${age} ago`;
}

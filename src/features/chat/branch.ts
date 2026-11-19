// The composer's branch menu: the pane that shows it, the button's label, when the menu takes a Filter field, and each branch row's age and holder.
import type { LocalBranch } from "../../ipc/bindings";
import { externalStore } from "../../ui/externalStore";
import { fileName } from "../files/paths";
import { ageLabel } from "../status/status";

/** The pane whose branch menu shows; one at a time. */
export const branchMenuPane = externalStore<string | null>(null);

export const openBranchMenu = (pane: string) => branchMenuPane.set(pane);

export const closeBranchMenu = () => branchMenuPane.set(null);

export const branchLabel = (branch: string | null | undefined): string => branch ?? "Detached HEAD";

/** A short list reads at a glance; from five branches the menu adds a Filter field. */
export const filtersBranches = (count: number): boolean => count > 4;

/** "2h", or "In harbor-flaky · 5h" for a branch another worktree holds. */
export function branchMeta(branch: LocalBranch, now: number): string {
  const age = ageLabel(branch.time ?? now, now);
  return branch.heldBy ? `In ${fileName(branch.heldBy)} · ${age}` : age;
}

// The composer's branch menu, floating above the composer card as a picker menu: the worktree's local branches newest first, read when it opens, a pick switching the worktree to one.
import { useEffect, useState } from "react";
import { commands, type LocalBranch } from "../../ipc/bindings";
import { fail } from "../../ui/alert";
import { switchBranch } from "../worktrees/branchSwitch";
import { branchMeta, closeBranchMenu, filtersBranches } from "./branch";
import { MenuCard, Select } from "./DialogPanel";

export function BranchMenu({ worktree, current }: { worktree: string; current: string | null }) {
  const [branches, setBranches] = useState<LocalBranch[] | null>(null);

  useEffect(() => {
    let live = true;
    commands.listBranches(worktree).then(
      (list) => live && setBranches(list),
      (error) => {
        if (!live) return;
        fail(error);
        closeBranchMenu();
      },
    );
    return () => {
      live = false;
    };
  }, [worktree]);

  // The card shows once the list is read, so it opens at its height.
  if (!branches) return null;
  const now = Date.now();
  return (
    <MenuCard label="Switch Branch" onDismiss={closeBranchMenu}>
      <Select
        options={branches.map((b) => b.name)}
        current={current}
        filter={filtersBranches(branches.length) ? "field" : "typing"}
        meta={(at) => branchMeta(branches[at], now)}
        unavailable={(at) => !!branches[at].heldBy}
        answer={(answer) => {
          closeBranchMenu();
          if (answer.kind === "value" && answer.value !== current) void switchBranch(worktree, answer.value);
        }}
      />
    </MenuCard>
  );
}

// The Changes view's branch line: the worktree's branch as a button over the scope control, locked while an agent in the worktree runs, and the branch menu it drops under itself: the local branches newest first, read when it opens, a pick switching the worktree to one, and a row's right-click menu with Rename Branch and, for a branch no worktree holds, Delete Branch.
import { useEffect, useState } from "react";
import { showMenu } from "../../app/commands";
import { commands, type LocalBranch } from "../../ipc/bindings";
import { fail } from "../../ui/alert";
import { BranchIcon, ChevronDownIcon } from "../../ui/icons";
import { MenuCard, Select } from "../chat/DialogPanel";
import { filtersOptions } from "../chat/dialog";
import { chats } from "../chat/store";
import { anyRunning } from "../chat/transcript";
import { allPaneIds } from "../panes/agents";
import { panes } from "../panes/store";
import { switchBranch } from "../worktrees/branchSwitch";
import { worktreeAt } from "../worktrees/projects";
import { projects } from "../worktrees/store";
import { branchHint, branchLabel, branchMenuOpen, closeBranchMenu, openBranchMenu } from "./branch";

export function BranchLine({ worktree }: { worktree: string }) {
  const branch = projects.usePart((p) => worktreeAt(p, worktree)?.branch ?? null);
  // A switch changes the folder every pane of the worktree works in, so any running agent locks it.
  const busy = chats.usePart((all) => anyRunning(all, allPaneIds(panes.get()[worktree])));
  const open = branchMenuOpen.use() && !busy;
  useEffect(() => {
    if (busy) closeBranchMenu();
  }, [busy]);
  useEffect(() => closeBranchMenu, [worktree]);
  return (
    <div className="changes-branch-line">
      <button
        type="button"
        className="changes-branch-button"
        title={busy ? "Switch branches after the agent stops" : "Switch Branch"}
        disabled={busy}
        aria-expanded={open}
        data-menu-toggle
        onClick={open ? closeBranchMenu : openBranchMenu}
      >
        <BranchIcon />
        <span className="changes-branch-name">{branchLabel(branch)}</span>
        <ChevronDownIcon />
      </button>
      {open && <BranchMenu worktree={worktree} current={branch} />}
    </div>
  );
}

function BranchMenu({ worktree, current }: { worktree: string; current: string | null }) {
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
        stacked
        options={branches.map((b) => b.name)}
        current={current}
        filter={filtersOptions(branches.length) ? "field" : "typing"}
        descriptions={branches.map((b) => branchHint(b, now))}
        unavailable={(at) => !!branches[at].heldBy}
        rowMenu={(at, e) => {
          const { name, heldBy } = branches[at];
          void showMenu(e, heldBy || name === current ? [["branch.rename", name]] : [["branch.rename", name], ["branch.delete", name]]);
        }}
        answer={(answer) => {
          closeBranchMenu();
          if (answer.kind === "value" && answer.value !== current) void switchBranch(worktree, answer.value);
        }}
      />
    </MenuCard>
  );
}

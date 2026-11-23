// The Changes view's branch line: the worktree's branch as a button over the scope control, locked while an agent in the worktree runs, with Fetch, Pull and Push at its right end while the repository has a remote, and the branch menu it drops under itself: the local branches newest first, read when it opens, a pick switching the worktree to one, a row's More button turning the card into that branch's Switch, Merge, Rename Branch and Delete Branch, a row's right-click menu with Rename Branch and, for a branch no worktree holds, Delete Branch, and New Branch under the rows.
import { useEffect, useRef, useState, type KeyboardEvent, type ReactNode } from "react";
import { showMenu } from "../../app/commands";
import { commands, type LocalBranch } from "../../ipc/bindings";
import { fail } from "../../ui/alert";
import { ArrowDownIcon, ArrowLeftIcon, ArrowUpIcon, BranchIcon, ChevronDownIcon, PlusIcon, ReloadIcon } from "../../ui/icons";
import { MenuCard, Select } from "../chat/DialogPanel";
import { filtersOptions } from "../chat/dialog";
import { chats } from "../chat/store";
import { anyRunning } from "../chat/transcript";
import { allPaneIds } from "../panes/agents";
import { panes } from "../panes/store";
import { switchBranch } from "../worktrees/branchSwitch";
import { requestDeleteBranch } from "../worktrees/localBranches";
import { openNewBranch } from "../worktrees/newBranch";
import { worktreeAt } from "../worktrees/projects";
import { startBranchRename } from "../worktrees/rename";
import { projects } from "../worktrees/store";
import { branchActionRules, branchHint, branchLabel, branchMenuOpen, closeBranchMenu, openBranchMenu, syncButtons } from "./branch";
import { commitTargets, fetchRemote, mergeBranch, pullBranch, pushBranch, running } from "./store";

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
      <SyncButtons worktree={worktree} />
      {open && <BranchMenu worktree={worktree} current={branch} />}
    </div>
  );
}

function SyncButtons({ worktree }: { worktree: string }) {
  const target = commitTargets.usePart((all) => all[worktree]);
  const busy = running.usePart((all) => all[worktree]);
  const sync = syncButtons(target, Date.now());
  if (!sync) return null;
  return (
    <div className="changes-sync">
      <button type="button" className="icon-button changes-fetch" title={sync.fetch} aria-label="Fetch" aria-busy={busy === "fetch"} onClick={() => void fetchRemote(worktree)}>
        <ReloadIcon />
      </button>
      <SyncButton {...sync.pull} busy={busy === "pull"} onClick={() => void pullBranch(worktree)}>
        <ArrowDownIcon />
      </SyncButton>
      <SyncButton {...sync.push} busy={busy === "push"} onClick={() => void pushBranch(worktree)}>
        <ArrowUpIcon size={16} />
      </SyncButton>
    </div>
  );
}

function SyncButton({ title, count, busy, disabled, onClick, children }: { title: string; count: number | null; busy: boolean; disabled: boolean; onClick: () => void; children: ReactNode }) {
  return (
    <button type="button" className="icon-button changes-sync-count" title={title} aria-label={title} aria-busy={busy} disabled={disabled && !busy} onClick={onClick}>
      {busy ? <span className="spinner" /> : children}
      {!!count && <span>{count}</span>}
    </button>
  );
}

function BranchMenu({ worktree, current }: { worktree: string; current: string | null }) {
  const [branches, setBranches] = useState<LocalBranch[] | null>(null);
  // The branch whose actions the card shows in place of the list.
  const [picked, setPicked] = useState<LocalBranch | null>(null);

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
    <MenuCard label={picked ? picked.name : "Switch Branch"} onDismiss={closeBranchMenu}>
      {picked ? (
        <BranchActions worktree={worktree} branch={picked} current={current} back={() => setPicked(null)} />
      ) : (
        <>
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
            rowMore={(at) => setPicked(branches[at])}
            answer={(answer) => {
              closeBranchMenu();
              if (answer.kind === "value" && answer.value !== current) void switchBranch(worktree, answer.value);
            }}
          />
          <div className="changes-branch-foot">
            <button
              type="button"
              className="changes-branch-item"
              onClick={() => {
                closeBranchMenu();
                openNewBranch(worktree);
              }}
            >
              <PlusIcon />
              New Branch
            </button>
          </div>
        </>
      )}
    </MenuCard>
  );
}

type Action = { label: string; hint?: string; danger?: boolean; disabled?: boolean; run: () => void };

function BranchActions({ worktree, branch, current, back }: { worktree: string; branch: LocalBranch; current: string | null; back: () => void }) {
  const first = useRef<HTMLButtonElement>(null);
  useEffect(() => first.current?.focus(), []);
  const { name } = branch;
  const rules = branchActionRules(branch, current);
  const actions: Action[] = [
    { label: "Switch", hint: rules.switchHint, disabled: !rules.switchable, run: () => void switchBranch(worktree, name) },
    { label: `Merge into ${branchLabel(current)}`, disabled: !rules.mergeable, run: () => void mergeBranch(worktree, name, branchLabel(current)) },
    { label: "Rename Branch", run: () => startBranchRename(worktree, name) },
    { label: "Delete Branch", danger: true, disabled: !rules.deletable, run: () => requestDeleteBranch(worktree, name) },
  ];
  const onKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    if (e.key === "ArrowLeft") return back();
    if (e.key !== "ArrowDown" && e.key !== "ArrowUp") return;
    e.preventDefault();
    const items = [...e.currentTarget.querySelectorAll<HTMLElement>("button:not(:disabled)")];
    const at = items.indexOf(document.activeElement as HTMLElement);
    items[(at + (e.key === "ArrowDown" ? 1 : items.length - 1)) % items.length]?.focus();
  };
  return (
    <div className="changes-branch-actions" onKeyDown={onKeyDown}>
      <button ref={first} type="button" className="changes-branch-item changes-branch-back" title="Back (←)" onClick={back}>
        <ArrowLeftIcon />
        <span className="changes-branch-name">{name}</span>
      </button>
      {actions.map((action) => (
        <button
          key={action.label}
          type="button"
          className="changes-commit-mode changes-branch-action"
          data-danger={action.danger || undefined}
          disabled={action.disabled}
          onClick={() => {
            closeBranchMenu();
            action.run();
          }}
        >
          <span className="changes-commit-mode-text">
            <span className="changes-branch-name">{action.label}</span>
            {action.hint && <span className="changes-commit-hint">{action.hint}</span>}
          </span>
        </button>
      ))}
    </div>
  );
}

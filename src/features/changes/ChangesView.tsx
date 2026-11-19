// The Changes view: its scope control, Uncommitted or Branch while the worktree has a base (see BranchList), then under Uncommitted the worktree's changed files under Staged and Changes, with the commit area under them: the author, the message field, then Push Immediately and Amend beside Push and Commit; each row with its name, folder, line counts and status; a click opens its diff. A checkbox on each row and group heading stages or unstages, and a row's More button and right-click menu hold its Stage or Unstage, and Discard. Also its tab bar button.
import { memo, useEffect, useRef, useState, type MouseEvent, type ReactNode } from "react";
import type { Change } from "../../ipc/bindings";
import { showMenu, tooltip } from "../../app/commands";
import { BranchIcon, CheckIcon, MoreIcon } from "../../ui/icons";
import { shortcutChanges } from "../settings/store";
import { showsSideView, sidePanels, toggleSideView } from "../sidepanel/store";
import { panes } from "../panes/store";
import { gitCounts } from "../worktrees/gitCountsRefresh";
import { Segmented } from "../../ui/Segmented";
import { arrivals, canCommit, changeGroups, diffKey, focusedDiff, nextSelection, pickRange, statusWord } from "./changes";
import { BaseControl, BranchList } from "./BranchList";
import { ChangeParts } from "./ChangeParts";
import { changesTarget } from "./commands";
import { branches, changes, commitMessages, commitOptions, commitStaged, commitTargets, followsAct, optionsOf, pushBranch, readChanges, running, scopes, setScope, showDiff, stageChanges, toggleAmend, togglePushAfter, typeCommitMessage, unstageChanges, type Scope } from "./store";
import "./changes.css";

export function ChangesToggle({ worktree }: { worktree: string }) {
  shortcutChanges.use();
  const shown = sidePanels.usePart((all) => showsSideView(all, worktree, "changes"));
  return (
    <button
      type="button"
      className="icon-button"
      title={tooltip("changes.toggle")}
      aria-label="Show or Hide Changes"
      aria-pressed={shown}
      onClick={() => toggleSideView(worktree, "changes")}
    >
      <BranchIcon />
    </button>
  );
}

const scopeOptions: { value: Scope; label: string }[] = [
  { value: "uncommitted", label: "Uncommitted" },
  { value: "branch", label: "Branch" },
];

// A side panel drag re-renders the panel each frame; the view reads its own stores.
export const ChangesView = memo(function ChangesView({ worktree }: { worktree: string }) {
  const branch = branches.usePart((all) => all[worktree]);
  const picked = scopes.usePart((all) => all[worktree] ?? "uncommitted");
  // Reads when it shows, then with each git counts refresh, which also runs on window focus.
  useEffect(() => {
    void readChanges(worktree);
    const unsubscribe = gitCounts.subscribe(() => void readChanges(worktree));
    return () => void unsubscribe();
  }, [worktree]);
  // Without a base, on a detached HEAD or a main checkout without an upstream, Uncommitted is the only scope, so no control shows.
  const scope = branch ? picked : "uncommitted";
  return (
    <>
      {branch && (
        <div className="changes-header">
          <Segmented label="Scope" options={scopeOptions} value={scope} onChange={(value) => setScope(worktree, value)} />
          {scope === "branch" && <BaseControl worktree={worktree} branch={branch} />}
        </div>
      )}
      {branch && scope === "branch" ? <BranchList worktree={worktree} branch={branch} /> : <UncommittedList worktree={worktree} />}
    </>
  );
});

function UncommittedList({ worktree }: { worktree: string }) {
  const list = changes.usePart((all) => all[worktree]);
  const selected = panes.usePart((all) => focusedDiff(all[worktree]));
  const tree = useRef<HTMLDivElement>(null);
  useEffect(() => {
    tree.current?.querySelector('[aria-selected="true"]')?.scrollIntoView({ block: "nearest" });
  }, [selected]);
  const groups = list ? changeGroups(list) : [];
  // Kept across renders, so a selection change mid-animation does not cut a row's arrival short.
  const arrived = useRef({ worktree, list, keys: new Set<string>() });
  if (arrived.current.list !== list) {
    const last = arrived.current;
    arrived.current = { worktree, list, keys: last.worktree === worktree ? arrivals(last.list, list, last.keys) : new Set() };
  }
  // The rows a shift-click or shift-arrow picked, from the anchor of the last plain click or arrow; a read drops the ones that moved or went.
  const [pick, setPick] = useState<{ anchor: string | null; keys: string[] }>({ anchor: null, keys: [] });
  const rows = groups.flatMap((group) => group.changes.map((change) => ({ group, change, key: diffKey(group.side, change.path) })));
  const picked = rows.filter((row) => pick.keys.includes(row.key));
  const many = picked.length > 1;
  const pickedChanges = picked.map((row) => row.change);
  const pickTo = (key: string, extend: boolean) => {
    const row = rows.find((r) => r.key === key);
    if (!row) return;
    const anchor = pick.anchor ?? selected;
    setPick(extend ? { anchor, keys: pickRange(row.group, anchor, key) } : { anchor: key, keys: [key] });
  };
  // Stage, unstage, discard or a hunk that takes the selected row out of its group selects the next row.
  const before = useRef({ worktree, list });
  useEffect(() => {
    const last = before.current;
    before.current = { worktree, list };
    const key = last.worktree === worktree && last.list && list && followsAct(list) && selected && nextSelection(last.list, list, selected);
    const change = key && list?.find((c) => diffKey(c.staged ? "staged" : "unstaged", c.path) === key);
    if (!key || !change) return;
    setPick({ anchor: key, keys: [key] });
    showDiff(worktree, change);
    // The focus moves along only from the tree, or from the row that just left it.
    const active = document.activeElement;
    if (active === document.body || tree.current?.contains(active)) tree.current?.querySelector<HTMLElement>(`[data-key="${CSS.escape(key)}"]`)?.focus();
  }, [worktree, list]);
  // Space stages or unstages the focused row, or the picked rows with it.
  const toggleFrom = (key: string) => {
    const row = rows.find((r) => r.key === key);
    if (row) void toggleStaged(worktree, many && pick.keys.includes(key) ? pickedChanges : [row.change], row.change.staged);
  };
  return (
    <>
      <div
        ref={tree}
        className="files-tree"
        role="tree"
        aria-label="Changes"
        onKeyDown={(e) => {
          const step = e.key === "ArrowDown" ? 1 : e.key === "ArrowUp" ? -1 : 0;
          if (e.key === "Escape" && many) return setPick({ ...pick, keys: [] });
          const key = (e.target as HTMLElement).dataset.key;
          if (e.key === " " && key) {
            // Space ticks the row's checkbox instead of opening the diff.
            e.preventDefault();
            if (!e.repeat) toggleFrom(key);
            return;
          }
          if (!step || e.metaKey || e.altKey || e.ctrlKey) return;
          const items = [...e.currentTarget.querySelectorAll<HTMLElement>('[role="treeitem"]')];
          const at = items.indexOf(e.target as HTMLElement);
          if (at < 0) return;
          e.preventDefault();
          e.currentTarget.dataset.keys = "";
          const next = items[at + step];
          if (!next?.dataset.key) return;
          next.focus();
          // A plain arrow opens the next file's diff, as a click does; shift only picks.
          if (e.shiftKey) pickTo(next.dataset.key, true);
          else next.click();
        }}
        onMouseMove={(e) => delete e.currentTarget.dataset.keys}
      >
        {groups.map((group) => (
          <div key={group.side} className="changes-section" role="group" aria-label={group.title}>
            <div className="changes-group">
              <StageBox worktree={worktree} list={group.changes} staged={group.side === "staged" && group.changes.length > 0} label={group.side === "staged" ? "Unstage All" : "Stage All"} focusable />
              <span>{group.title}</span>
              <span className="changes-count">{group.changes.length}</span>
            </div>
            {group.changes.length === 0 && <div className="files-empty">{group.side === "staged" ? "No staged files" : "No changes"}</div>}
            {group.changes.map((change) => (
              <ChangeRow
                key={change.path}
                worktree={worktree}
                change={change}
                rowKey={diffKey(group.side, change.path)}
                arrived={arrived.current.keys.has(diffKey(group.side, change.path))}
                selected={many ? pick.keys.includes(diffKey(group.side, change.path)) : selected === diffKey(group.side, change.path)}
                actsOn={many && pick.keys.includes(diffKey(group.side, change.path)) ? pickedChanges : [change]}
                onPick={pickTo}
              />
            ))}
          </div>
        ))}
      </div>
      <CommitBox worktree={worktree} staged={groups[0]?.changes.length > 0} />
    </>
  );
}

function CommitBox({ worktree, staged }: { worktree: string; staged: boolean }) {
  const message = commitMessages.usePart((all) => all[worktree] ?? "");
  const target = commitTargets.usePart((all) => all[worktree]);
  const options = commitOptions.usePart((all) => optionsOf(all, worktree));
  const busy = running.usePart((all) => all[worktree]);
  const ready = canCommit(message, options.amend, staged);
  const [, name, email] = /^(.*?)\s*(<[^>]*>)?$/.exec(target?.author ?? "") ?? [];
  return (
    <div className="changes-commit">
      {name && (
        <div className="changes-author" title={target?.author ?? undefined}>
          <span className="changes-author-name">{name}</span>
          {email && <span className="changes-author-email">{email}</span>}
        </div>
      )}
      <textarea
        className="input changes-message"
        placeholder="Message"
        aria-label="Commit message"
        value={message}
        onChange={(e) => typeCommitMessage(worktree, e.target.value)}
        onKeyDown={(e) => {
          if (e.key !== "Enter" || !e.metaKey) return;
          e.preventDefault();
          if (!e.repeat) void commitStaged(worktree);
        }}
      />
      <div className="changes-commit-bar">
        <div className="changes-options">
          {target?.pushTo && (
            <OptionBox checked={options.pushAfter} onToggle={() => togglePushAfter(worktree)}>
              Push changes immediately to {target.pushTo}
            </OptionBox>
          )}
          {target?.amendable && (
            <OptionBox checked={options.amend} onToggle={() => void toggleAmend(worktree)}>
              Amend last commit
            </OptionBox>
          )}
        </div>
        {target?.pushable && (
          <button type="button" className="button-secondary" aria-busy={busy === "push"} onClick={() => void pushBranch(worktree)}>
            <span className="button-label">Push</span>
            {busy === "push" && <span className="spinner" aria-label="Pushing" />}
          </button>
        )}
        <button type="button" className="button-primary" title="Commit (⌘Return)" disabled={!ready && busy !== "commit"} aria-busy={busy === "commit"} onClick={() => void commitStaged(worktree)}>
          <span className="button-label">Commit</span>
          {busy === "commit" && <span className="spinner" aria-label="Committing" />}
        </button>
      </div>
    </div>
  );
}

function OptionBox({ checked, onToggle, children }: { checked: boolean; onToggle: () => void; children: ReactNode }) {
  return (
    <button type="button" role="checkbox" className="changes-option" aria-checked={checked} onClick={onToggle}>
      <span className="changes-check">
        <CheckIcon size={12} />
      </span>
      <span className="changes-option-label">{children}</span>
    </button>
  );
}

// Checked while the files sit in the index: a click stages them or takes them out.
function StageBox({ worktree, list, staged, label, focusable = false }: { worktree: string; list: Change[]; staged: boolean; label: string; focusable?: boolean }) {
  // A row's checkbox takes no focus of its own, so a click hands it to its row, where Space and the arrows carry on.
  const focusRow = (box: HTMLElement) => !focusable && box.nextElementSibling instanceof HTMLElement && box.nextElementSibling.focus();
  return (
    <button
      type="button"
      role="checkbox"
      className="changes-check"
      aria-checked={staged}
      aria-label={label}
      title={label}
      tabIndex={focusable ? 0 : -1}
      disabled={list.length === 0}
      onClick={(e) => {
        focusRow(e.currentTarget);
        void toggleStaged(worktree, list, staged);
      }}
    >
      <CheckIcon size={12} />
    </button>
  );
}

const toggleStaged = (worktree: string, list: Change[], staged: boolean) => (staged ? unstageChanges(worktree, list) : stageChanges(worktree, list));

// The picked rows all sit in one group, so the first one says which. git restores no conflicted file, so a pick with one offers no Discard.
const showRowMenu = (e: MouseEvent, list: Change[]) => {
  const target = changesTarget(list);
  const discards = !list.some((c) => c.status === "conflicted");
  showMenu(
    e,
    list[0].staged
      ? [["changes.unstage", target], ...(discards ? [["changes.discardStaged", target] as [string, string]] : [])]
      : [["changes.stage", target], ...(discards ? [["changes.discard", target] as [string, string]] : [])],
  );
};

type RowProps = { worktree: string; change: Change; rowKey: string; arrived: boolean; selected: boolean; actsOn: Change[]; onPick: (key: string, extend: boolean) => void };

// A row's checkbox, Space, More and right-click act on all the picked rows while it is one of them.
function ChangeRow({ worktree, change, rowKey, arrived, selected, actsOn, onPick }: RowProps) {
  const word = statusWord(change.status);
  return (
    <div className="changes-item" data-arrived={arrived || undefined}>
      <StageBox worktree={worktree} list={actsOn} staged={change.staged} label={`${change.staged ? "Unstage" : "Stage"} ${actsOn.length > 1 ? `${actsOn.length} files` : change.path}`} />
      <button
        type="button"
        role="treeitem"
        className="files-row changes-row"
        aria-selected={selected}
        aria-checked={change.staged}
        data-key={rowKey}
        title={change.oldPath ? `${change.oldPath} → ${change.path}` : change.path}
        aria-label={[change.path, word, change.added && `${change.added} added`, change.removed && `${change.removed} removed`].filter(Boolean).join(", ")}
        onClick={(e) => {
          // WebKit drops the focus when the mouse presses a button, so the click puts it back: ↑, ↓ and Space act on the focused row.
          e.currentTarget.focus();
          onPick(rowKey, e.shiftKey);
          if (!e.shiftKey) showDiff(worktree, change);
        }}
        onDoubleClick={(e) => !e.shiftKey && showDiff(worktree, change, false)}
        // The tree handles Space; the key's release must not click the row.
        onKeyUp={(e) => e.key === " " && e.preventDefault()}
        onContextMenu={(e) => showRowMenu(e, actsOn)}
      >
        <ChangeParts path={change.path} status={change.status} added={change.added} removed={change.removed} />
      </button>
      <button type="button" className="icon-button changes-more" title="More" aria-label={`More for ${change.path}`} onClick={(e) => showRowMenu(e, actsOn)}>
        <MoreIcon />
      </button>
    </div>
  );
}

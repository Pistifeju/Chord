// The Changes view: the worktree's changed files under Staged and Changes, each row with its name, folder, line counts and status; a click opens its diff. A checkbox on each row and group heading stages or unstages, and a row's More button and right-click menu hold its Stage or Unstage, and Discard. Also its tab bar button.
import { memo, useEffect, useRef, useState, type MouseEvent } from "react";
import type { Change } from "../../ipc/bindings";
import { showMenu, tooltip } from "../../app/commands";
import { BranchIcon, CheckIcon, MoreIcon } from "../../ui/icons";
import { shortcutChanges } from "../settings/store";
import { showsSideView, sidePanels, toggleSideView } from "../sidepanel/store";
import { panes } from "../panes/store";
import { gitCounts } from "../worktrees/gitCountsRefresh";
import { fileName, parentFolder } from "../files/paths";
import { changeGroups, diffKey, focusedDiff, nextSelection, pickRange, statusLetters, statusWord } from "./changes";
import { changesTarget } from "./commands";
import { changes, followsAct, readChanges, showDiff, stageChanges, unstageChanges } from "./store";
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

// A side panel drag re-renders the panel each frame; the view reads its own stores.
export const ChangesView = memo(function ChangesView({ worktree }: { worktree: string }) {
  const list = changes.usePart((all) => all[worktree]);
  const selected = panes.usePart((all) => focusedDiff(all[worktree]));
  // Reads when it shows, then with each git counts refresh, which also runs on window focus.
  useEffect(() => {
    void readChanges(worktree);
    const unsubscribe = gitCounts.subscribe(() => void readChanges(worktree));
    return () => void unsubscribe();
  }, [worktree]);
  const tree = useRef<HTMLDivElement>(null);
  useEffect(() => {
    tree.current?.querySelector('[aria-selected="true"]')?.scrollIntoView({ block: "nearest" });
  }, [selected]);
  const groups = list ? changeGroups(list) : [];
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
              selected={many ? pick.keys.includes(diffKey(group.side, change.path)) : selected === diffKey(group.side, change.path)}
              actsOn={many && pick.keys.includes(diffKey(group.side, change.path)) ? pickedChanges : [change]}
              onPick={pickTo}
            />
          ))}
        </div>
      ))}
    </div>
  );
});

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
      {staged && <CheckIcon size={12} />}
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

type RowProps = { worktree: string; change: Change; rowKey: string; selected: boolean; actsOn: Change[]; onPick: (key: string, extend: boolean) => void };

// A row's checkbox, Space, More and right-click act on all the picked rows while it is one of them.
function ChangeRow({ worktree, change, rowKey, selected, actsOn, onPick }: RowProps) {
  const folder = parentFolder(change.path);
  const word = statusWord(change.status);
  return (
    <div className="changes-item">
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
        <span className="files-name changes-name" data-status={change.status}>
          {fileName(change.path)}
        </span>
        {folder && <span className="changes-folder">{folder}</span>}
        <span className="changes-lines">
          {!!change.added && <span className="changes-added">+{change.added}</span>}
          {!!change.removed && <span className="changes-removed">−{change.removed}</span>}
        </span>
        <span className="changes-status" data-status={change.status} title={word}>
          {statusLetters[change.status]}
        </span>
      </button>
      <button type="button" className="icon-button changes-more" title="More" aria-label={`More for ${change.path}`} onClick={(e) => showRowMenu(e, actsOn)}>
        <MoreIcon />
      </button>
    </div>
  );
}

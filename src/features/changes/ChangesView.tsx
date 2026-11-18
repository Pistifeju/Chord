// The Changes view: the worktree's changed files under Staged and Changes, each row with its name, folder, line counts and status; a click opens its diff. Also its tab bar button.
import { memo, useEffect, useRef } from "react";
import type { Change } from "../../ipc/bindings";
import { tooltip } from "../../app/commands";
import { BranchIcon } from "../../ui/icons";
import { shortcutChanges } from "../settings/store";
import { showsSideView, sidePanels, toggleSideView } from "../sidepanel/store";
import { panes } from "../panes/store";
import { gitCounts } from "../worktrees/gitCountsRefresh";
import { fileName, parentFolder } from "../files/paths";
import { changeGroups, diffKey, focusedDiff, statusLetters, statusWord } from "./changes";
import { changes, readChanges, showDiff } from "./store";
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
  const groups = changeGroups(list ?? []);
  return (
    <div
      ref={tree}
      className="files-tree"
      role="tree"
      aria-label="Changes"
      onKeyDown={(e) => {
        const step = e.key === "ArrowDown" ? 1 : e.key === "ArrowUp" ? -1 : 0;
        if (!step || e.metaKey || e.altKey || e.ctrlKey || e.shiftKey) return;
        const rows = [...e.currentTarget.querySelectorAll<HTMLElement>('[role="treeitem"]')];
        const at = rows.indexOf(e.target as HTMLElement);
        if (at < 0) return;
        e.preventDefault();
        e.currentTarget.dataset.keys = "";
        rows[at + step]?.focus();
      }}
      onMouseMove={(e) => delete e.currentTarget.dataset.keys}
    >
      {groups.map((group) => (
        <div key={group.side} className="changes-section" role="group" aria-label={group.title}>
          <div className="changes-group">
            <span>{group.title}</span>
            <span className="changes-count">{group.changes.length}</span>
          </div>
          {group.changes.map((change) => (
            <ChangeRow key={change.path} worktree={worktree} change={change} selected={selected === diffKey(group.side, change.path)} />
          ))}
        </div>
      ))}
      {list && groups.length === 0 && <div className="files-empty">No changes</div>}
    </div>
  );
});

function ChangeRow({ worktree, change, selected }: { worktree: string; change: Change; selected: boolean }) {
  const folder = parentFolder(change.path);
  const word = statusWord(change.status);
  return (
    <button
      type="button"
      role="treeitem"
      className="files-row changes-row"
      aria-selected={selected}
      title={change.oldPath ? `${change.oldPath} → ${change.path}` : change.path}
      aria-label={[change.path, word, change.added && `${change.added} added`, change.removed && `${change.removed} removed`].filter(Boolean).join(", ")}
      // WebKit does not focus a clicked button, and ↑ and ↓ move on from the focused row.
      onMouseDown={(e) => e.currentTarget.focus()}
      onClick={() => showDiff(worktree, change)}
      onDoubleClick={() => showDiff(worktree, change, false)}
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
  );
}

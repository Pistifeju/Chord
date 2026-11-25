import type { KeyboardEvent } from "react";
import { showMenu } from "../../app/commands";
import { ChevronIcon } from "../../ui/icons";
import { CommitRow, CommittedRow, useOpenCommits } from "./BranchList";
import { diffKey } from "./changes";
import { commitFiles, openStashGroups, readStashFiles, stashes, toggleStashGroup } from "./store";

export function StashList({ worktree, selected }: { worktree: string; selected: string | null }) {
  const list = stashes.usePart((all) => all[worktree]);
  const groupOpen = openStashGroups.usePart((all) => !!all[worktree]);
  const files = commitFiles.use();
  const [open, toggle] = useOpenCommits((hash) => void readStashFiles(worktree, hash));
  if (!list?.length) return null;
  const onKeyDown = (e: KeyboardEvent) => {
    if (e.key !== "ArrowRight" && e.key !== "ArrowLeft") return;
    const item = e.target as HTMLElement;
    const opens = e.key === "ArrowRight";
    if (item.dataset.stashes !== undefined) {
      e.preventDefault();
      if (opens !== groupOpen) toggleStashGroup(worktree);
    } else if (item.dataset.commit) {
      e.preventDefault();
      toggle(item.dataset.commit, opens);
    }
  };
  return (
    <div className="changes-section" role="group" aria-label="Stashes" onKeyDown={onKeyDown}>
      <button
        type="button"
        role="treeitem"
        className="changes-group changes-group-toggle"
        aria-expanded={groupOpen}
        data-stashes=""
        title={groupOpen ? "Hide Stashes" : "Show Stashes"}
        onClick={(e) => {
          e.currentTarget.focus();
          toggleStashGroup(worktree);
        }}
      >
        <span className="turn-chevron files-chevron" data-open={groupOpen}>
          <ChevronIcon />
        </span>
        <span>Stashes</span>
        <span className="changes-count">{list.length}</span>
      </button>
      {groupOpen &&
        list.map((stash) => (
          <CommitRow
            key={stash.hash}
            hash={stash.hash}
            subject={stash.message}
            time={stash.time}
            opened={open.includes(stash.hash)}
            toggle={toggle}
            detail={stash.branch && <span className="changes-stash-branch">{stash.branch}</span>}
            onContextMenu={(e) => void showMenu(e, [["changes.applyStash", stash.hash], ["changes.popStash", stash.hash], "separator", ["changes.dropStash", stash.hash]])}
          >
            {files[stash.hash]?.map((file) => {
              const key = diffKey({ stash: stash.hash }, file.path);
              return <CommittedRow key={key} worktree={worktree} file={file} side={{ stash: stash.hash }} rowKey={key} depth={1} selected={selected === key} />;
            })}
          </CommitRow>
        ))}
    </div>
  );
}

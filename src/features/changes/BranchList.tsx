// The Changes view's Branch scope: the base it compares with and its menu, the branch's commits since the merge base, each opening to the files it changed, then the files the branch changed; a click on a file opens its diff for the branch or for that commit.
import { CheckMenuItem, Menu, MenuItem, PredefinedMenuItem } from "@tauri-apps/api/menu";
import { LogicalPosition } from "@tauri-apps/api/dpi";
import { useEffect, useRef, useState, type CSSProperties } from "react";
import { commands, type BranchChanges_Serialize as BranchChanges, type CommittedFile_Serialize as CommittedFile, type DiffSide } from "../../ipc/bindings";
import { openContextMenu } from "../../app/contextMenu";
import { fail } from "../../ui/alert";
import { ChevronDownIcon, ChevronIcon } from "../../ui/icons";
import { ageLabel } from "../status/status";
import { openFileTab, panes } from "../panes/store";
import { diffKey, focusedDiff } from "./changes";
import { ChangeParts } from "./ChangeParts";
import { commitFiles, pickBase, readCommitFiles } from "./store";

async function showBaseMenu(worktree: string, button: HTMLElement, branch: BranchChanges) {
  // Listed outside the menu lock, so a slow git cannot block every other menu.
  const listed = await commands.listBaseBranches(worktree);
  const choice = (name: string) => CheckMenuItem.new({ text: name, checked: name === branch.base, action: () => pickBase(worktree, name) });
  const items: (MenuItem | CheckMenuItem | PredefinedMenuItem)[] = [await choice(branch.defaultBase)];
  for (const [title, names] of [
    ["Local", listed.local],
    ["Remote", listed.remote],
  ] as const) {
    const rest = names.filter((name) => name !== branch.defaultBase);
    if (rest.length === 0) continue;
    items.push(await PredefinedMenuItem.new({ item: "Separator" }), await MenuItem.new({ text: title, enabled: false }));
    for (const name of rest) items.push(await choice(name));
  }
  const box = button.getBoundingClientRect();
  await openContextMenu(async () => (await Menu.new({ items })).popup(new LogicalPosition(box.left, box.bottom)));
}

export function BaseControl({ worktree, branch }: { worktree: string; branch: BranchChanges }) {
  return (
    <div className="changes-base">
      <span>Compared to</span>
      <button type="button" className="changes-base-button" title="Base Branch" aria-label={`Compared to ${branch.base}`} onClick={(e) => showBaseMenu(worktree, e.currentTarget, branch).catch(fail)}>
        <span className="changes-base-name">{branch.base}</span>
        <ChevronDownIcon />
      </button>
    </div>
  );
}

export function BranchList({ worktree, branch }: { worktree: string; branch: BranchChanges }) {
  const selected = panes.usePart((all) => focusedDiff(all[worktree]));
  const files = commitFiles.use();
  const [open, setOpen] = useState<string[]>([]);
  const tree = useRef<HTMLDivElement>(null);
  useEffect(() => {
    tree.current?.querySelector('[aria-selected="true"]')?.scrollIntoView({ block: "nearest" });
  }, [selected]);
  const toggle = (hash: string, opens: boolean) => {
    setOpen((was) => (opens ? [...new Set([...was, hash])] : was.filter((h) => h !== hash)));
    if (opens) void readCommitFiles(worktree, hash);
  };
  const now = Date.now();
  const fileRow = (file: CommittedFile, side: DiffSide, depth: number) => {
    const key = diffKey(side, file.path);
    return <CommittedRow key={key} worktree={worktree} file={file} side={side} rowKey={key} depth={depth} selected={selected === key} />;
  };
  return (
    <div
      ref={tree}
      className="files-tree changes-branch"
      role="tree"
      aria-label="Branch"
      onKeyDown={(e) => {
        const item = e.target as HTMLElement;
        const hash = item.dataset.commit;
        if (hash && (e.key === "ArrowRight" || e.key === "ArrowLeft")) {
          e.preventDefault();
          return toggle(hash, e.key === "ArrowRight");
        }
        const step = e.key === "ArrowDown" ? 1 : e.key === "ArrowUp" ? -1 : 0;
        if (!step || e.metaKey || e.altKey || e.ctrlKey || e.shiftKey) return;
        const items = [...e.currentTarget.querySelectorAll<HTMLElement>('[role="treeitem"]')];
        const next = items[items.indexOf(item) + step];
        if (!next || !items.includes(item)) return;
        e.preventDefault();
        e.currentTarget.dataset.keys = "";
        next.focus();
        // A file opens its diff as a click does; a commit only takes the focus.
        if (!next.dataset.commit) next.click();
      }}
      onMouseMove={(e) => delete e.currentTarget.dataset.keys}
    >
      <div className="changes-section" role="group" aria-label="Commits">
        <div className="changes-group">
          <span>Commits</span>
          <span className="changes-count">{branch.commits.length}</span>
        </div>
        {branch.commits.length === 0 && <div className="files-empty">No commits</div>}
        {branch.commits.map((commit) => {
          const opened = open.includes(commit.hash);
          return (
            <div key={commit.hash} role="none">
              <button
                type="button"
                role="treeitem"
                className="files-row changes-log-row"
                aria-expanded={opened}
                data-commit={commit.hash}
                title={commit.subject}
                onClick={(e) => {
                  e.currentTarget.focus();
                  toggle(commit.hash, !opened);
                }}
              >
                <span className="turn-chevron files-chevron" data-open={opened}>
                  <ChevronIcon />
                </span>
                <span className="changes-subject">{commit.subject}</span>
                <span className="changes-hash">{commit.shortHash}</span>
                <span className="changes-age">{ageLabel(commit.time ?? now, now)}</span>
              </button>
              {opened && <div role="group">{files[commit.hash]?.map((file) => fileRow(file, { commit: commit.hash }, 1))}</div>}
            </div>
          );
        })}
      </div>
      <div className="changes-section" role="group" aria-label="Files">
        <div className="changes-group">
          <span>Files</span>
          <span className="changes-count">{branch.files.length}</span>
        </div>
        {branch.files.length === 0 && <div className="files-empty">No changes</div>}
        {branch.files.map((file) => fileRow(file, { branch: branch.base }, 0))}
      </div>
    </div>
  );
}

type RowProps = { worktree: string; file: CommittedFile; side: DiffSide; rowKey: string; depth: number; selected: boolean };

function CommittedRow({ worktree, file, side, rowKey, depth, selected }: RowProps) {
  return (
    <button
      type="button"
      role="treeitem"
      className="files-row changes-row changes-committed"
      style={{ "--depth": depth } as CSSProperties}
      aria-selected={selected}
      data-key={rowKey}
      title={file.oldPath ? `${file.oldPath} → ${file.path}` : file.path}
      onClick={(e) => {
        // WebKit drops the focus when the mouse presses a button, so the click puts it back for ↑ and ↓.
        e.currentTarget.focus();
        openFileTab(worktree, file.path, true, side);
      }}
      onDoubleClick={() => openFileTab(worktree, file.path, false, side)}
    >
      <ChangeParts path={file.path} status={file.status} added={file.added} removed={file.removed} />
    </button>
  );
}

// The Files view: the worktree folder's name with Collapse All and Reload, its filter field and its tree.
import { useEffect, useRef, type CSSProperties } from "react";
import { ChevronIcon, CollapseAllIcon, FileIcon, FilterIcon, FolderIcon, ReloadIcon } from "../../ui/icons";
import { openFile } from "../editor/store";
import { collapseAll, loadRoot, reloadFiles, setFilter, toggleFolder, trees } from "./store";
import { emptyTree, visibleRows } from "./tree";
import "./files.css";

export function FilesView({ worktree }: { worktree: string }) {
  const tree = trees.use()[worktree] ?? emptyTree;
  const rows = visibleRows(tree);
  const list = useRef<HTMLDivElement>(null);
  useEffect(() => {
    loadRoot(worktree);
    const onFocus = () => void reloadFiles(worktree);
    window.addEventListener("focus", onFocus);
    return () => window.removeEventListener("focus", onFocus);
  }, [worktree]);
  useEffect(() => {
    list.current?.querySelector('[aria-selected="true"]')?.scrollIntoView({ block: "nearest" });
  }, [tree.selected, rows.length]);
  return (
    <div className="files">
      <div className="files-header">
        <span className="files-title" title={worktree}>
          {worktree.split("/").pop()}
        </span>
        <button type="button" className="icon-button" title="Collapse All" aria-label="Collapse All" disabled={tree.open.length === 0} onClick={() => collapseAll(worktree)}>
          <CollapseAllIcon />
        </button>
        <button type="button" className="icon-button" title="Reload" aria-label="Reload" onClick={() => void reloadFiles(worktree)}>
          <ReloadIcon />
        </button>
      </div>
      <div className="files-filter">
        <FilterIcon />
        <input className="input" placeholder="Filter" aria-label="Filter Files" value={tree.filter} onChange={(e) => setFilter(worktree, e.target.value)} />
      </div>
      <div ref={list} className="files-tree" role="tree" aria-label="Files">
        {rows.map(({ entry, depth, open }) => (
          <button
            key={entry.path}
            type="button"
            role="treeitem"
            className="files-row"
            style={{ "--depth": depth } as CSSProperties}
            title={entry.path}
            aria-level={depth + 1}
            aria-expanded={entry.isDir ? open : undefined}
            aria-selected={entry.path === tree.selected}
            onClick={() => (entry.isDir ? toggleFolder(worktree, entry.path) : openFile(worktree, entry.path))}
          >
            <span className="turn-chevron files-chevron" data-open={open} data-hidden={!entry.isDir}>
              <ChevronIcon />
            </span>
            <span className="files-icon">{entry.isDir ? <FolderIcon size={16} /> : <FileIcon />}</span>
            <span className="files-name">{entry.name}</span>
          </button>
        ))}
        {rows.length === 0 && tree.filter && <div className="files-empty">No matches</div>}
      </div>
    </div>
  );
}

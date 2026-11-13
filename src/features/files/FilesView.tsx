// The Files view: the worktree folder's name with Collapse All and Reload, its filter field and its tree with its right-click menu and name fields.
import { Fragment, useEffect, useRef, useState, type CSSProperties } from "react";
import { addPathToDraft, pathDropPane } from "../chat/store";
import { showMenu } from "../../app/commands";
import { usePointerDrag } from "../../ui/pointerDrag";
import { ChevronIcon, CollapseAllIcon, FileIcon, FilterIcon, FolderIcon, ReloadIcon } from "../../ui/icons";
import { showFile } from "../editor/showing";
import { finishNameEdit } from "./entries";
import { parentFolder } from "./paths";
import { collapseAll, loadRoot, nameEdit, reloadFiles, setFilter, toggleFolder, trees } from "./store";
import { emptyTree, stemLength, treeKeyAction, visibleRows, type Row } from "./tree";
import type { FileEntry } from "../../ipc/bindings";
import "./files.css";

export function FilesView({ worktree }: { worktree: string }) {
  const tree = trees.use()[worktree] ?? emptyTree;
  const rows = visibleRows(tree);
  const edit = nameEdit.use();
  const editing = edit?.worktree === worktree ? edit : null;
  const creating = editing?.kind === "rename" ? null : editing;
  // A new entry's field sits first inside its folder; the root's ("") sits at the top.
  const folderAt = creating ? rows.findIndex((row) => row.entry.path === creating.folder) : -1;
  const newField = creating && <NewEntryField isDir={creating.kind === "newFolder"} depth={folderAt < 0 ? 0 : rows[folderAt].depth + 1} />;
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
      <div
        ref={list}
        className="files-tree"
        role="tree"
        aria-label="Files"
        onContextMenu={(e) => showMenu(e, [["files.newFile", ""], ["files.newFolder", ""]])}
        onKeyDown={(e) => {
          // Only a row's keys move through the tree; a name field keeps its arrows.
          const path = (e.target as HTMLElement).dataset.path;
          const action = path === undefined || e.metaKey || e.altKey || e.ctrlKey || e.shiftKey ? null : treeKeyAction(rows, path, e.key);
          if (!action) return;
          e.preventDefault();
          if ("toggle" in action) toggleFolder(worktree, action.toggle);
          else list.current?.querySelector<HTMLElement>(`[data-path="${CSS.escape(action.focus)}"]`)?.focus();
        }}
      >
        {folderAt < 0 && newField}
        {rows.map((row, i) => (
          <Fragment key={row.entry.path}>
            {editing?.kind === "rename" && editing.path === row.entry.path ? (
              <NameField row={row} />
            ) : (
              <EntryRow worktree={worktree} row={row} selected={row.entry.path === tree.selected} />
            )}
            {i === folderAt && newField}
          </Fragment>
        ))}
        {rows.length === 0 && tree.filter && <div className="files-empty">No matches</div>}
      </div>
    </div>
  );
}

// The pane of the enabled composer under the pointer.
function composerPaneAt(x: number, y: number): string | null {
  const composer = document.elementFromPoint(x, y)?.closest(".chat-composer");
  if (!composer || composer.querySelector("textarea")?.disabled !== false) return null;
  return composer.closest<HTMLElement>("[data-pane]")?.dataset.pane ?? null;
}

// A file row dragged onto a composer puts its relative path in the draft.
function usePathDrag() {
  const [dragging, setDragging] = useState(false);
  const drag = usePointerDrag<string>({
    onStart: () => setDragging(true),
    onMove: (e) => {
      const pane = composerPaneAt(e.clientX, e.clientY);
      if (pane !== pathDropPane.get()) pathDropPane.set(pane);
    },
    onEnd: (path, dropped) => {
      setDragging(false);
      const pane = pathDropPane.get();
      pathDropPane.set(null);
      if (!dropped || !pane) return;
      addPathToDraft(pane, path);
      document.querySelector<HTMLElement>(`[data-pane="${CSS.escape(pane)}"] .chat-input`)?.focus();
    },
  });
  return { dragging, ...drag };
}

function EntryRow({ worktree, row: { entry, depth, open }, selected }: { worktree: string; row: Row; selected: boolean }) {
  const folder = entry.isDir ? entry.path : parentFolder(entry.path);
  const pathDrag = usePathDrag();
  return (
    <button
      type="button"
      role="treeitem"
      className="files-row"
      style={{ "--depth": depth } as CSSProperties}
      title={entry.path}
      data-path={entry.path}
      data-folder={folder}
      aria-level={depth + 1}
      aria-expanded={entry.isDir ? open : undefined}
      aria-selected={selected}
      data-dragging={pathDrag.dragging || undefined}
      {...(entry.isDir ? {} : pathDrag.handlers(entry.path))}
      // WebKit does not focus a clicked button, and ⌘⌫ acts on the focused row.
      onMouseDown={(event) => event.currentTarget.focus()}
      onClick={() => (entry.isDir ? toggleFolder(worktree, entry.path) : pathDrag.clicked() && showFile(worktree, entry.path))}
      onContextMenu={(e) =>
        showMenu(e, [
          ["files.newFile", folder],
          ["files.newFolder", folder],
          ["files.rename", entry.path],
          ["files.copyPath", entry.path],
          ["files.copyRelativePath", entry.path],
          ["files.reveal", entry.path],
          ["files.trash", entry.path],
        ])
      }
    >
      <RowStart entry={entry} open={open} />
      <span className="files-name">{entry.name}</span>
    </button>
  );
}

function RowStart({ entry, open }: { entry: Pick<FileEntry, "isDir">; open: boolean }) {
  return (
    <>
      <span className="turn-chevron files-chevron" data-open={open} data-hidden={!entry.isDir}>
        <ChevronIcon />
      </span>
      <span className="files-icon">{entry.isDir ? <FolderIcon size={16} /> : <FileIcon />}</span>
    </>
  );
}

const NewEntryField = ({ isDir, depth }: { isDir: boolean; depth: number }) => <NameField row={{ entry: { name: "", path: "", isDir }, depth, open: false }} />;

// The row's name as a field, its name without the extension selected: Return or leaving it keeps the name, Esc drops it.
function NameField({ row: { entry, depth, open } }: { row: Row }) {
  const name = entry.path.split("/").pop() ?? "";
  const field = useRef<HTMLInputElement>(null);
  useEffect(() => {
    field.current?.focus();
    field.current?.setSelectionRange(0, entry.isDir ? name.length : stemLength(name));
  }, [entry.isDir, name]);
  return (
    <div className="files-row" style={{ "--depth": depth } as CSSProperties}>
      <RowStart entry={entry} open={open} />
      <input
        ref={field}
        className="files-name-field"
        aria-label={name ? `Rename ${name}` : entry.isDir ? "New Folder Name" : "New File Name"}
        defaultValue={name}
        spellCheck={false}
        onContextMenu={(e) => e.stopPropagation()}
        onBlur={(e) => void finishNameEdit(e.currentTarget.value)}
        onKeyDown={(e) => {
          if (e.key !== "Enter" && e.key !== "Escape") return;
          e.preventDefault();
          e.stopPropagation();
          if (e.key === "Escape") nameEdit.set(null);
          else void finishNameEdit(e.currentTarget.value);
        }}
      />
    </div>
  );
}

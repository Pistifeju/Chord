// The Open File by Name sheet: a field for the name, the worktree's matching files under it; Return or a click opens one.
import { useEffect, useMemo, useRef, useState, type KeyboardEvent } from "react";
import { closeSheet, sheet, sheetOf } from "../../app/sheet";
import { FileIcon } from "../../ui/icons";
import { steppedIndex } from "../chat/dialog";
import { matchingFiles } from "./openByName";
import { pickFileByName } from "../editor/showing";
import { namedFiles } from "./store";
import "./files.css";

export function OpenFileByNamePopup() {
  const open = sheetOf(sheet.use(), "openFileByName");
  if (!open) return null;
  return <OpenFileByName key={open.worktree} worktree={open.worktree} />;
}

function OpenFileByName({ worktree }: { worktree: string }) {
  const files = namedFiles.use();
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const list = useRef<HTMLDivElement>(null);
  const shown = useMemo(() => matchingFiles(files ?? [], query), [files, query]);
  const close = () => closeSheet("openFileByName");

  useEffect(() => {
    list.current?.querySelector("[data-active]")?.scrollIntoView({ block: "nearest" });
  }, [active, query]);

  const onKey = (e: KeyboardEvent) => {
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      setActive(steppedIndex(active, e.key === "ArrowDown" ? 1 : -1, shown.length));
    } else if (e.key === "Enter" && !e.nativeEvent.isComposing && shown[active] !== undefined) {
      e.preventDefault();
      pickFileByName(worktree, shown[active]);
    }
  };

  return (
    <div className="sheet-backdrop" onMouseDown={close}>
      <div className="sheet open-by-name" role="dialog" aria-modal="true" aria-label="Open File by Name" onMouseDown={(e) => e.stopPropagation()}>
        <input
          autoFocus
          className="input"
          value={query}
          placeholder="File name"
          aria-label="File Name"
          spellCheck={false}
          autoCapitalize="off"
          autoCorrect="off"
          onChange={(e) => {
            setQuery(e.target.value);
            setActive(0);
          }}
          onKeyDown={onKey}
        />
        <div className="open-by-name-list" role="listbox" ref={list}>
          {shown.map((path, i) => {
            const slash = path.lastIndexOf("/");
            return (
              <div
                key={path}
                role="option"
                aria-selected={i === active}
                data-active={i === active || undefined}
                className="open-by-name-row"
                title={path}
                onMouseMove={() => i !== active && setActive(i)}
                onClick={() => pickFileByName(worktree, path)}
              >
                <FileIcon />
                <span className="open-by-name-name">{path.slice(slash + 1)}</span>
                {slash !== -1 && <span className="open-by-name-folder">{path.slice(0, slash)}</span>}
              </div>
            );
          })}
          {files !== null && shown.length === 0 && <p className="open-by-name-empty">No matches</p>}
        </div>
      </div>
    </div>
  );
}

// The preview picker sheet, in the Open File by Name sheet's layout: a search field over the worktree's `#Preview`s by file; Return or a click shows one.
import { useEffect, useMemo, useRef, useState, type KeyboardEvent } from "react";
import { closeSheet, sheet, sheetOf } from "../../../app/sheet";
import { steppedIndex } from "../../chat/dialog";
import { matchingPreviews, previewName } from "./preview";
import { choosePreview, previewList } from "./store";
import "../../files/files.css";

export function PreviewPickerPopup() {
  const open = sheetOf(sheet.use(), "previewPicker");
  if (!open) return null;
  return <PreviewPicker key={open.worktree} worktree={open.worktree} />;
}

function PreviewPicker({ worktree }: { worktree: string }) {
  const all = previewList.use();
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const list = useRef<HTMLDivElement>(null);
  const shown = useMemo(() => matchingPreviews(all ?? [], query), [all, query]);
  const close = () => closeSheet("previewPicker");

  useEffect(() => {
    list.current?.querySelector("[data-active]")?.scrollIntoView({ block: "nearest" });
  }, [active, query]);

  const onKey = (e: KeyboardEvent) => {
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      setActive(steppedIndex(active, e.key === "ArrowDown" ? 1 : -1, shown.length));
    } else if (e.key === "Enter" && !e.nativeEvent.isComposing && shown[active] !== undefined) {
      e.preventDefault();
      choosePreview(worktree, shown[active]);
    }
  };

  return (
    <div className="sheet-backdrop" onMouseDown={close}>
      <div className="sheet open-by-name" role="dialog" aria-modal="true" aria-label="Choose Preview" onMouseDown={(e) => e.stopPropagation()}>
        <input
          autoFocus
          className="input"
          value={query}
          placeholder="Preview name"
          aria-label="Preview Name"
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
          {shown.map((pick, i) => (
            <div
              key={`${pick.file}#${pick.index}`}
              role="option"
              aria-selected={i === active}
              data-active={i === active || undefined}
              className="open-by-name-row"
              title={`${previewName(pick)} · ${pick.file}`}
              onMouseMove={() => i !== active && setActive(i)}
              onClick={() => choosePreview(worktree, pick)}
            >
              <span className="open-by-name-name">{previewName(pick)}</span>
              <span className="open-by-name-folder">{pick.file}</span>
            </div>
          ))}
          {all?.length === 0 && <p className="open-by-name-empty">No previews in this worktree</p>}
          {!!all?.length && shown.length === 0 && <p className="open-by-name-empty">No matches</p>}
        </div>
      </div>
    </div>
  );
}

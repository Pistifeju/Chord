import { useEffect, useMemo, useRef, useState, type KeyboardEvent, type ReactNode } from "react";
import { steppedIndex } from "./listStep";
import "./pickerSheet.css";

type Props<T> = {
  label: string;
  placeholder: string;
  fieldLabel: string;
  items: T[] | null;
  matching: (items: T[], query: string) => T[];
  keyOf: (item: T) => string;
  titleOf: (item: T) => string;
  row: (item: T) => ReactNode;
  noItems?: string;
  onPick: (item: T) => void;
  onClose: () => void;
};

export function PickerSheet<T>({ label, placeholder, fieldLabel, items, matching, keyOf, titleOf, row, noItems = "No matches", onPick, onClose }: Props<T>) {
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const list = useRef<HTMLDivElement>(null);
  const shown = useMemo(() => matching(items ?? [], query), [items, query, matching]);

  useEffect(() => {
    list.current?.querySelector("[data-active]")?.scrollIntoView({ block: "nearest" });
  }, [active, query]);

  const onKey = (e: KeyboardEvent) => {
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      setActive(steppedIndex(active, e.key === "ArrowDown" ? 1 : -1, shown.length));
    } else if (e.key === "Enter" && !e.nativeEvent.isComposing && shown[active] !== undefined) {
      e.preventDefault();
      onPick(shown[active]);
    }
  };

  return (
    <div className="sheet-backdrop" onMouseDown={onClose}>
      <div className="sheet picker-sheet" role="dialog" aria-modal="true" aria-label={label} onMouseDown={(e) => e.stopPropagation()}>
        <input
          autoFocus
          className="input"
          value={query}
          placeholder={placeholder}
          aria-label={fieldLabel}
          spellCheck={false}
          autoCapitalize="off"
          autoCorrect="off"
          onChange={(e) => {
            setQuery(e.target.value);
            setActive(0);
          }}
          onKeyDown={onKey}
        />
        <div className="picker-sheet-list" role="listbox" ref={list}>
          {shown.map((item, i) => (
            <div
              key={keyOf(item)}
              role="option"
              aria-selected={i === active}
              data-active={i === active || undefined}
              className="picker-sheet-row"
              title={titleOf(item)}
              onMouseMove={() => i !== active && setActive(i)}
              onClick={() => onPick(item)}
            >
              {row(item)}
            </div>
          ))}
          {items?.length === 0 && <p className="picker-sheet-empty">{noItems}</p>}
          {!!items?.length && shown.length === 0 && <p className="picker-sheet-empty">No matches</p>}
        </div>
      </div>
    </div>
  );
}

// The dialog an agent's extension opens, shown in place of the composer: a select with its filter row, a confirm or an input; the menu a composer bar picker floats above it; and the select itself, which the branch menu takes too.
import { useEffect, useRef, useState, type KeyboardEvent, type ReactNode } from "react";
import type { DialogAnswer } from "../../ipc/bindings";
import { CheckIcon } from "../../ui/icons";
import { useOutsidePointer } from "../../ui/outsidePointer";
import { steppedPast } from "../../ui/listStep";
import { matchingOptions, startIndex, typedIndex } from "./dialog";
import { answerDialog } from "./store";
import type { OpenDialog } from "./transcript";

// "field" filters through a Filter field, "typing" moves the highlight to what the user types (as a Mac menu does), "none" does neither.
type Filter = "field" | "typing" | "none";

// A pause this long starts a new typed name, as in a Mac menu.
const TYPING_PAUSE_MS = 1000;

type SelectProps = {
  options: string[];
  current: string | null;
  answer: (answer: DialogAnswer) => void;
  filter?: Filter;
  /** A meta line at the right end of the row for the option at this index, before the current option's check. */
  meta?: (at: number) => string;
  /** Whether the option at this index shows but cannot be picked. */
  unavailable?: (at: number) => boolean;
};

const noneUnavailable = () => false;

const openFrom = (at: number, shown: number[], unavailable: (at: number) => boolean) =>
  shown[at] !== undefined && unavailable(shown[at]) ? steppedPast(at, 1, shown.length, (i) => unavailable(shown[i])) : at;

export function Select({ options, current, answer, filter = "field", meta, unavailable = noneUnavailable }: SelectProps) {
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(() => openFrom(startIndex(options, current), matchingOptions(options, ""), unavailable));
  const root = useRef<HTMLDivElement>(null);
  const typed = useRef({ text: "", at: 0 });
  const list = useRef<HTMLDivElement>(null);
  const shown = matchingOptions(options, query);
  const choose = (at: number) => !unavailable(at) && answer({ kind: "value", value: options[at] });
  const narrow = (next: string) => {
    setQuery(next);
    setActive(openFrom(0, matchingOptions(options, next), unavailable));
  };

  useEffect(() => {
    if (filter !== "field") root.current?.focus();
  }, [filter]);

  useEffect(() => {
    list.current?.querySelector("[data-active]")?.scrollIntoView({ block: "nearest" });
  }, [active, query]);

  const onKey = (e: KeyboardEvent) => {
    if (filter === "typing" && e.key.length === 1 && !e.metaKey && !e.ctrlKey && !e.nativeEvent.isComposing) {
      e.preventDefault();
      const text = (e.timeStamp - typed.current.at < TYPING_PAUSE_MS ? typed.current.text : "") + e.key;
      typed.current = { text, at: e.timeStamp };
      const at = typedIndex(options, text);
      if (at !== -1 && !unavailable(at)) setActive(at);
    } else if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      setActive(steppedPast(active, e.key === "ArrowDown" ? 1 : -1, shown.length, (i) => unavailable(shown[i])));
    } else if (e.key === "Enter" && !e.nativeEvent.isComposing && shown[active] !== undefined) {
      e.preventDefault();
      choose(shown[active]);
    }
  };

  return (
    // Without a filter row the list itself takes the focus, so the arrows and Return still reach it.
    <div className="chat-dialog-select" onKeyDown={onKey} tabIndex={filter === "field" ? undefined : -1} ref={root}>
      {filter === "field" && <input autoFocus className="input" value={query} placeholder="Filter" aria-label="Filter Options" onChange={(e) => narrow(e.target.value)} />}
      <div className="chat-dialog-options" role="listbox" ref={list}>
        {shown.map((at, i) => {
          const option = options[at];
          const isCurrent = option === current;
          const check = (
            <span className="chat-dialog-current" role={isCurrent ? "img" : undefined} aria-label={isCurrent ? "Current" : undefined}>
              {isCurrent && <CheckIcon />}
            </span>
          );
          return (
            <div
              key={at}
              role="option"
              aria-selected={i === active}
              aria-disabled={unavailable(at) || undefined}
              data-active={i === active || undefined}
              className="chat-dialog-option"
              onMouseMove={() => i !== active && !unavailable(at) && setActive(i)}
              onClick={() => choose(at)}
            >
              {meta ? (
                <>
                  <span className="chat-dialog-name">{option}</span>
                  <span className="chat-dialog-meta">
                    {meta(at)}
                    {check}
                  </span>
                </>
              ) : (
                <>
                  {option}
                  {isCurrent && check}
                </>
              )}
            </div>
          );
        })}
        {shown.length === 0 && <p className="chat-dialog-empty">No matches</p>}
      </div>
    </div>
  );
}

function Input({ placeholder, answer }: { placeholder: string; answer: (answer: DialogAnswer) => void }) {
  const [value, setValue] = useState("");
  const submit = () => answer({ kind: "value", value });
  return (
    <>
      <input
        autoFocus
        className="input"
        value={value}
        placeholder={placeholder}
        onChange={(e) => setValue(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter" && !e.nativeEvent.isComposing) submit();
        }}
      />
      <div className="chat-dialog-buttons">
        <button type="button" className="button-secondary" onClick={() => answer({ kind: "cancelled" })}>
          Cancel
        </button>
        <button type="button" className="button-primary" onClick={submit}>
          Submit
        </button>
      </div>
    </>
  );
}

/** The card a composer bar menu floats in above the composer; a press outside it dismisses it. */
export function MenuCard({ label, onDismiss, children }: { label: string; onDismiss: () => void; children: ReactNode }) {
  const card = useRef<HTMLDivElement>(null);
  useOutsidePointer(card, onDismiss);
  return (
    <div className="chat-slash-menu chat-dialog-menu" role="dialog" aria-label={label} ref={card}>
      {children}
    </div>
  );
}

export function DialogMenu({ pane, dialog, options, current, filter }: { pane: string; dialog: OpenDialog; options: string[]; current: string | null; filter: boolean }) {
  const answer = (given: DialogAnswer) => answerDialog(pane, dialog.id, given);
  return (
    <MenuCard label={dialog.title} onDismiss={() => answer({ kind: "cancelled" })}>
      <Select options={options} current={current} answer={answer} filter={filter ? "typing" : "none"} />
    </MenuCard>
  );
}

export function DialogPanel({ pane, dialog }: { pane: string; dialog: OpenDialog }) {
  const answer = (given: DialogAnswer) => answerDialog(pane, dialog.id, given);
  const { ask } = dialog;
  return (
    <div className="chat-composer chat-dialog" role="dialog" aria-label={dialog.title} tabIndex={-1}>
      <h2 className="chat-dialog-title">{dialog.title}</h2>
      {ask.kind === "select" && <Select options={ask.options} current={ask.current ?? null} answer={answer} />}
      {ask.kind === "input" && <Input placeholder={ask.placeholder} answer={answer} />}
      {ask.kind === "confirm" && (
        <>
          {ask.message && <p className="chat-dialog-message">{ask.message}</p>}
          <div className="chat-dialog-buttons">
            <button type="button" className="button-secondary" onClick={() => answer({ kind: "confirmed", confirmed: false })}>
              No
            </button>
            <button type="button" className="button-primary" autoFocus onClick={() => answer({ kind: "confirmed", confirmed: true })}>
              Yes
            </button>
          </div>
        </>
      )}
    </div>
  );
}

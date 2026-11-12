// The dialog an agent's extension opens, shown in place of the composer: a select (filtered when long), a confirm or an input.
import { useEffect, useRef, useState, type KeyboardEvent } from "react";
import type { DialogAnswer } from "../../ipc/bindings";
import { filters, matchingOptions, steppedIndex } from "./dialog";
import { answerDialog } from "./store";
import type { OpenDialog } from "./transcript";

function Select({ options, answer }: { options: string[]; answer: (answer: DialogAnswer) => void }) {
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const list = useRef<HTMLDivElement>(null);
  const root = useRef<HTMLDivElement>(null);
  const shown = matchingOptions(options, query);
  const choose = (value: string) => answer({ kind: "value", value });

  // Without a filter field the list takes the keys.
  useEffect(() => {
    if (!filters(options)) root.current?.focus();
  }, [options]);

  useEffect(() => {
    list.current?.querySelector("[data-active]")?.scrollIntoView({ block: "nearest" });
  }, [active, query]);

  const onKey = (e: KeyboardEvent) => {
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      setActive(steppedIndex(active, e.key === "ArrowDown" ? 1 : -1, shown.length));
    } else if (e.key === "Enter" && !e.nativeEvent.isComposing && shown[active] !== undefined) {
      e.preventDefault();
      choose(shown[active]);
    }
  };

  return (
    <div className="chat-dialog-select" ref={root} tabIndex={-1} onKeyDown={onKey}>
      {filters(options) && (
        <input
          autoFocus
          className="input"
          value={query}
          placeholder="Filter"
          aria-label="Filter Options"
          onChange={(e) => {
            setQuery(e.target.value);
            setActive(0);
          }}
        />
      )}
      <div className="chat-dialog-options" role="listbox" ref={list}>
        {shown.map((option, i) => (
          <div
            key={`${i}:${option}`}
            role="option"
            aria-selected={i === active}
            data-active={i === active || undefined}
            className="chat-dialog-option"
            onMouseMove={() => i !== active && setActive(i)}
            onClick={() => choose(option)}
          >
            {option}
          </div>
        ))}
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

export function DialogPanel({ pane, dialog }: { pane: string; dialog: OpenDialog }) {
  const answer = (given: DialogAnswer) => answerDialog(pane, dialog.id, given);
  const { ask } = dialog;
  return (
    <div className="chat-composer chat-dialog" role="dialog" aria-label={dialog.title} tabIndex={-1}>
      <h2 className="chat-dialog-title">{dialog.title}</h2>
      {ask.kind === "select" && <Select options={ask.options} answer={answer} />}
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

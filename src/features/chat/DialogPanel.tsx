// The dialog an agent opens, shown in place of the composer: a select with its filter row, a confirm or an input; the menu a composer bar picker floats above it; and the select itself, which the branch menu takes too.
import { useEffect, useRef, useState, type KeyboardEvent, type MouseEvent, type ReactNode } from "react";
import type { DialogAnswer } from "../../ipc/bindings";
import { CheckIcon } from "../../ui/icons";
import { useOutsidePointer } from "../../ui/outsidePointer";
import { steppedPast } from "../../ui/listStep";
import { openInDefaultBrowser } from "../browser/store";
import { filtersOptions, matchingOptions, startIndex, typedIndex, TYPING_PAUSE_MS } from "./dialog";
import { answerDialog } from "./store";
import type { OpenDialog } from "./transcript";

// "field" filters through a Filter field, "typing" moves the highlight to what the user types (as a Mac menu does), "none" does neither.
type Filter = "field" | "typing" | "none";


type SelectProps = {
  options: string[];
  current: string | null;
  answer: (answer: DialogAnswer) => void;
  filter?: Filter;
  /** A meta line at the right end of the row for the option at this index, before the current option's check. */
  meta?: (at: number) => string;
  /** Whether the option at this index shows but cannot be picked. */
  unavailable?: (at: number) => boolean;
  /** The right-click menu of the option at this index. */
  rowMenu?: (at: number, event: MouseEvent) => void;
  /** One line per option, shown under its label. */
  descriptions?: string[];
  /** Each option takes a checkbox, and Submit answers with the checked ones. */
  multi?: boolean;
  /** The rows lead with the check column and stack the description under the name, as the branch menu does. */
  stacked?: boolean;
  /** The icon each row leads with. */
  icon?: (option: string) => ReactNode;
};

const noneUnavailable = () => false;

const openFrom = (at: number, shown: number[], unavailable: (at: number) => boolean) =>
  shown[at] !== undefined && unavailable(shown[at]) ? steppedPast(at, 1, shown.length, (i) => unavailable(shown[i])) : at;

export function Select({ options, current, answer, filter = "field", meta, unavailable = noneUnavailable, rowMenu, descriptions = [], multi = false, stacked = false, icon }: SelectProps) {
  const [query, setQuery] = useState("");
  const [checked, setChecked] = useState<ReadonlySet<number>>(new Set());
  const [active, setActive] = useState(() => openFrom(startIndex(options, current), matchingOptions(options, ""), unavailable));
  const root = useRef<HTMLDivElement>(null);
  const typed = useRef({ text: "", at: 0 });
  const list = useRef<HTMLDivElement>(null);
  const shown = matchingOptions(options, query);
  const toggle = (at: number) => setChecked((was) => (was.has(at) ? new Set([...was].filter((i) => i !== at)) : new Set([...was, at])));
  const choose = (at: number) => !unavailable(at) && (multi ? toggle(at) : answer({ kind: "value", value: options[at] }));
  const submit = () => answer({ kind: "values", values: options.filter((_, at) => checked.has(at)) });
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

  const field = <input autoFocus className="input" value={query} placeholder="Filter" aria-label="Filter Options" onChange={(e) => narrow(e.target.value)} />;
  return (
    // Without a filter row the list itself takes the focus, so the arrows and Return still reach it.
    <div className="chat-dialog-select" onKeyDown={onKey} tabIndex={filter === "field" ? undefined : -1} ref={root}>
      {filter === "field" && field}
      <div className="chat-dialog-options" role="listbox" ref={list}>
        {shown.map((at, i) => {
          const option = options[at];
          const isCurrent = option === current;
          const description = descriptions[at];
          const label = description ? (
            <span className="chat-dialog-label">
              {option}
              <span className="chat-dialog-description">{description}</span>
            </span>
          ) : (
            option
          );
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
              aria-checked={multi ? checked.has(at) : undefined}
              aria-disabled={unavailable(at) || undefined}
              data-active={i === active || undefined}
              data-stacked={stacked || undefined}
              className="chat-dialog-option"
              onMouseMove={() => i !== active && !unavailable(at) && setActive(i)}
              onClick={() => choose(at)}
              onContextMenu={rowMenu && ((e) => rowMenu(at, e))}
            >
              {multi && (
                <span className="checkbox" aria-hidden="true" aria-checked={checked.has(at)}>
                  <CheckIcon size={12} />
                </span>
              )}
              {stacked ? (
                <>
                  <span className="chat-dialog-current" role={isCurrent ? "img" : undefined} aria-label={isCurrent ? "Current" : undefined}>
                    {isCurrent && <CheckIcon size={12} />}
                  </span>
                  <span className="chat-dialog-label">
                    <span className="chat-dialog-name">{option}</span>
                    {description && <span className="chat-dialog-description">{description}</span>}
                  </span>
                </>
              ) : meta ? (
                <>
                  <span className="chat-dialog-name">{option}</span>
                  <span className="chat-dialog-meta">
                    {meta(at)}
                    {check}
                  </span>
                </>
              ) : icon ? (
                <>
                  <span className="chat-dialog-icon">{icon(option)}</span>
                  <span className="chat-dialog-name">{option}</span>
                  {isCurrent && check}
                </>
              ) : (
                <>
                  {label}
                  {isCurrent && check}
                </>
              )}
            </div>
          );
        })}
        {shown.length === 0 && <p className="chat-dialog-empty">No matches</p>}
      </div>
      {multi && (
        <div className="chat-dialog-buttons">
          <button type="button" className="button-primary" onClick={submit}>
            Submit
          </button>
        </div>
      )}
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

// With a link, Open sends it to the default browser before it answers, whatever the link setting says.
function Confirm({ message, url, answer }: { message: string; url: string | null; answer: (answer: DialogAnswer) => void }) {
  const confirm = () => {
    if (url) openInDefaultBrowser(url);
    answer({ kind: "confirmed", confirmed: true });
  };
  return (
    <>
      {message && <p className="chat-dialog-message">{message}</p>}
      {url && (
        <p className="chat-dialog-message chat-dialog-link" data-selectable>
          {url}
        </p>
      )}
      <div className="chat-dialog-buttons">
        <button type="button" className="button-secondary" onClick={() => answer({ kind: "confirmed", confirmed: false })}>
          {url ? "Decline" : "No"}
        </button>
        <button type="button" className="button-primary" autoFocus onClick={confirm}>
          {url ? "Open" : "Yes"}
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

export function DialogMenu({ pane, dialog, options, current, filter, icon }: { pane: string; dialog: OpenDialog; options: string[]; current: string | null; filter: boolean; icon?: (option: string) => ReactNode }) {
  const answer = (given: DialogAnswer) => answerDialog(pane, dialog.id, given);
  return (
    <MenuCard label={dialog.title} onDismiss={() => answer({ kind: "cancelled" })}>
      <Select options={options} current={current} answer={answer} filter={filter ? "typing" : "none"} icon={icon} />
    </MenuCard>
  );
}

export function DialogPanel({ pane, dialog }: { pane: string; dialog: OpenDialog }) {
  const answer = (given: DialogAnswer) => answerDialog(pane, dialog.id, given);
  const { ask } = dialog;
  return (
    <div className="chat-composer chat-dialog" role="dialog" aria-label={dialog.title} tabIndex={-1}>
      <h2 className="chat-dialog-title">{dialog.title}</h2>
      {ask.kind === "select" && ask.detail && (
        <pre className="chat-dialog-detail" data-selectable>
          {ask.detail}
        </pre>
      )}
      {ask.kind === "select" && <Select options={ask.options} current={ask.current ?? null} answer={answer} filter={!ask.detail && filtersOptions(ask.options.length) ? "field" : "typing"} descriptions={ask.descriptions} multi={ask.multi} />}
      {ask.kind === "input" && <Input placeholder={ask.placeholder} answer={answer} />}
      {ask.kind === "confirm" && <Confirm message={ask.message} url={ask.url ?? null} answer={answer} />}
    </div>
  );
}

import { memo, useEffect, useLayoutEffect, useRef, useState, type MouseEvent } from "react";
import type { Change } from "../../ipc/bindings";
import { showMenu } from "../../app/commands";
import { CheckIcon, ChevronDownIcon, MoreIcon, StashIcon } from "../../ui/icons";
import { useOutsidePointer } from "../../ui/outsidePointer";
import { panes } from "../panes/store";
import { gitCounts } from "../worktrees/gitCountsRefresh";
import { Segmented } from "../../ui/Segmented";
import { arrivals, canCommit, changeGroups, diffKey, focusedDiff, nextSelection, pickRange, statusWord } from "./changes";
import { BaseControl, BranchList } from "./BranchList";
import { BranchLine } from "./BranchMenu";
import { ChangeParts } from "./ChangeParts";
import { StashList } from "./StashList";
import { changesTarget } from "./commands";
import { branches, changes, commitMessages, commitOptions, commitStaged, commitTargets, followsAct, optionsOf, pickCommitMode, pushAfterCommit, readChanges, running, scopes, setScope, showDiff, stageChanges, stashChanges, typeCommitMessage, unstageChanges, type Running, type Scope } from "./store";
import "./changes.css";

const scopeOptions: { value: Scope; label: string }[] = [
  { value: "uncommitted", label: "Uncommitted" },
  { value: "branch", label: "Branch" },
];

export const ChangesView = memo(function ChangesView({ worktree }: { worktree: string }) {
  const branch = branches.usePart((all) => all[worktree]);
  const picked = scopes.usePart((all) => all[worktree] ?? "uncommitted");
  useEffect(() => {
    void readChanges(worktree);
    const unsubscribe = gitCounts.subscribe(() => void readChanges(worktree));
    return () => void unsubscribe();
  }, [worktree]);
  const scope = branch ? picked : "uncommitted";
  return (
    <>
      <BranchLine worktree={worktree} />
      {branch && (
        <div className="changes-header">
          <Segmented label="Scope" options={scopeOptions} value={scope} onChange={(value) => setScope(worktree, value)} />
          {scope === "branch" && <BaseControl worktree={worktree} branch={branch} />}
        </div>
      )}
      {branch && scope === "branch" ? <BranchList worktree={worktree} branch={branch} /> : <UncommittedList worktree={worktree} />}
    </>
  );
});

function UncommittedList({ worktree }: { worktree: string }) {
  const list = changes.usePart((all) => all[worktree]);
  const selected = panes.usePart((all) => focusedDiff(all[worktree]));
  const tree = useRef<HTMLDivElement>(null);
  useEffect(() => {
    tree.current?.querySelector('[aria-selected="true"]')?.scrollIntoView({ block: "nearest" });
  }, [selected]);
  const groups = list ? changeGroups(list) : [];
  const arrived = useRef({ worktree, list, keys: new Set<string>() });
  if (arrived.current.list !== list) {
    const last = arrived.current;
    arrived.current = { worktree, list, keys: last.worktree === worktree ? arrivals(last.list, list, last.keys) : new Set() };
  }
  const [pick, setPick] = useState<{ anchor: string | null; keys: string[] }>({ anchor: null, keys: [] });
  const rows = groups.flatMap((group) => group.changes.map((change) => ({ group, change, key: diffKey(group.side, change.path) })));
  const picked = rows.filter((row) => pick.keys.includes(row.key));
  const many = picked.length > 1;
  const pickedChanges = picked.map((row) => row.change);
  const pickTo = (key: string, extend: boolean) => {
    const row = rows.find((r) => r.key === key);
    if (!row) return;
    const anchor = pick.anchor ?? selected;
    setPick(extend ? { anchor, keys: pickRange(row.group, anchor, key) } : { anchor: key, keys: [key] });
  };
  const before = useRef({ worktree, list });
  useEffect(() => {
    const last = before.current;
    before.current = { worktree, list };
    const key = last.worktree === worktree && last.list && list && followsAct(list) && selected && nextSelection(last.list, list, selected);
    const change = key && list?.find((c) => diffKey(c.staged ? "staged" : "unstaged", c.path) === key);
    if (!key || !change) return;
    setPick({ anchor: key, keys: [key] });
    showDiff(worktree, change);
    const active = document.activeElement;
    if (active === document.body || tree.current?.contains(active)) tree.current?.querySelector<HTMLElement>(`[data-key="${CSS.escape(key)}"]`)?.focus();
  }, [worktree, list]);
  const toggleFrom = (key: string) => {
    const row = rows.find((r) => r.key === key);
    if (row) void toggleStaged(worktree, many && pick.keys.includes(key) ? pickedChanges : [row.change], row.change.staged);
  };
  return (
    <>
      <div
        ref={tree}
        className="files-tree"
        role="tree"
        aria-label="Changes"
        onKeyDown={(e) => {
          const step = e.key === "ArrowDown" ? 1 : e.key === "ArrowUp" ? -1 : 0;
          if (e.key === "Escape" && many) return setPick({ ...pick, keys: [] });
          const key = (e.target as HTMLElement).dataset.key;
          if (e.key === " " && key && rows.some((r) => r.key === key)) {
            e.preventDefault();
            if (!e.repeat) toggleFrom(key);
            return;
          }
          if (!step || e.metaKey || e.altKey || e.ctrlKey) return;
          const items = [...e.currentTarget.querySelectorAll<HTMLElement>('[role="treeitem"]')];
          const at = items.indexOf(e.target as HTMLElement);
          if (at < 0) return;
          e.preventDefault();
          e.currentTarget.dataset.keys = "";
          const next = items[at + step];
          if (!next) return;
          next.focus();
          if (!next.dataset.key) return;
          if (e.shiftKey) pickTo(next.dataset.key, true);
          else next.click();
        }}
        onMouseMove={(e) => delete e.currentTarget.dataset.keys}
      >
        {groups.map((group) => (
          <div key={group.side} className="changes-section" role="group" aria-label={group.title}>
            <div className="changes-group">
              <StageBox worktree={worktree} list={group.changes} staged={group.side === "staged" && group.changes.length > 0} label={group.side === "staged" ? "Unstage All" : "Stage All"} focusable />
              <span>{group.title}</span>
              <span className="changes-count">{group.changes.length}</span>
              {group.side === "unstaged" && (
                <button type="button" className="icon-button changes-stash" title="Stash Changes" aria-label="Stash Changes" disabled={!list?.length} onClick={() => void stashChanges(worktree)}>
                  <StashIcon />
                </button>
              )}
            </div>
            {group.changes.length === 0 && <div className="files-empty">{group.side === "staged" ? "No staged files" : "No changes"}</div>}
            {group.changes.map((change) => (
              <ChangeRow
                key={change.path}
                worktree={worktree}
                change={change}
                rowKey={diffKey(group.side, change.path)}
                arrived={arrived.current.keys.has(diffKey(group.side, change.path))}
                selected={many ? pick.keys.includes(diffKey(group.side, change.path)) : selected === diffKey(group.side, change.path)}
                actsOn={many && pick.keys.includes(diffKey(group.side, change.path)) ? pickedChanges : [change]}
                onPick={pickTo}
              />
            ))}
          </div>
        ))}
        <StashList worktree={worktree} selected={selected} />
      </div>
      <CommitBox worktree={worktree} staged={groups[0]?.changes.length > 0} />
    </>
  );
}

function CommitBox({ worktree, staged }: { worktree: string; staged: boolean }) {
  const message = commitMessages.usePart((all) => all[worktree] ?? "");
  const target = commitTargets.usePart((all) => all[worktree]);
  const savedPush = pushAfterCommit.use();
  const options = commitOptions.usePart((all) => optionsOf(all, worktree, savedPush));
  const busy = running.usePart((all) => all[worktree]);
  const ready = canCommit(message, options.amend, staged);
  const subject = message.split("\n")[0].length;
  const field = useRef<HTMLTextAreaElement>(null);
  useLayoutEffect(() => {
    const el = field.current;
    if (!el) return;
    el.style.height = "auto";
    el.style.height = `${el.scrollHeight}px`;
  }, [message]);
  return (
    <div className="changes-commit">
      <div className="changes-card">
        <textarea
          ref={field}
          className="changes-message"
          placeholder="Message"
          aria-label="Commit message"
          title={target?.author ?? undefined}
          value={message}
          onChange={(e) => typeCommitMessage(worktree, e.target.value)}
          onKeyDown={(e) => {
            if (e.key !== "Enter" || !e.metaKey) return;
            e.preventDefault();
            if (!e.repeat) void commitStaged(worktree);
          }}
        />
        <div className="changes-card-foot">
          <div className={subject > 50 ? "changes-subject-length changes-subject-long" : "changes-subject-length"} title="Subject length">
            {subject > 0 && `${subject}/50`}
          </div>
          <CommitButton worktree={worktree} amend={options.amend} push={options.pushAfter && !!target?.pushTo} ready={ready} busy={busy} />
        </div>
      </div>
    </div>
  );
}

const commitModes = [
  { amend: false, push: false, label: "Commit" },
  { amend: false, push: true, label: "Commit & Push" },
  { amend: true, push: false, label: "Amend" },
  { amend: true, push: true, label: "Amend & Push" },
];

function CommitButton({ worktree, amend, push, ready, busy }: { worktree: string; amend: boolean; push: boolean; ready: boolean; busy: Running | undefined }) {
  const target = commitTargets.usePart((all) => all[worktree]);
  const [open, setOpen] = useState(false);
  const menu = useRef<HTMLDivElement>(null);
  useOutsidePointer(menu, () => setOpen(false));
  useEffect(() => {
    if (open) menu.current?.querySelector<HTMLElement>("[aria-checked=true]")?.focus();
  }, [open]);
  const pushTo = target?.pushTo;
  const hints = { Commit: "Record the staged files", "Commit & Push": `Then push to ${pushTo}`, Amend: "Rewrite the last commit", "Amend & Push": `Rewrite it, then push to ${pushTo}` };
  const modes = commitModes.filter((mode) => (!mode.amend || target?.amendable) && (!mode.push || pushTo));
  const label = commitModes.find((mode) => mode.amend === amend && mode.push === push)!.label;
  const disabled = !ready && busy !== "commit";
  return (
    <div className="changes-commit-split">
      <button type="button" className="button-primary changes-commit-button" title="Commit (⌘Return)" disabled={disabled} aria-busy={busy === "commit"} onClick={() => void commitStaged(worktree)}>
        <span className="button-label">{label}</span>
        {busy === "commit" && <span className="spinner" aria-label="Committing" />}
      </button>
      {modes.length > 1 && (
        <button type="button" className="button-primary changes-commit-more" data-menu-toggle data-quiet={disabled || undefined} title="Commit Mode" aria-haspopup="menu" aria-expanded={open} disabled={!!busy} onClick={() => setOpen(!open)}>
          <ChevronDownIcon />
        </button>
      )}
      {open && (
        <div
          ref={menu}
          className="changes-commit-menu"
          role="menu"
          onKeyDown={(e) => {
            if (e.key === "Escape") setOpen(false);
            if (e.key !== "ArrowDown" && e.key !== "ArrowUp") return;
            e.preventDefault();
            const items = [...e.currentTarget.querySelectorAll<HTMLElement>("[role=menuitemradio]")];
            const at = items.indexOf(document.activeElement as HTMLElement);
            items[(at + (e.key === "ArrowDown" ? 1 : items.length - 1)) % items.length]?.focus();
          }}
        >
          {modes.map((mode) => {
            const on = mode.amend === amend && mode.push === push;
            return (
              <button
                key={mode.label}
                type="button"
                role="menuitemradio"
                aria-checked={on}
                className="changes-commit-mode"
                onClick={() => {
                  setOpen(false);
                  void pickCommitMode(worktree, mode.amend, mode.push);
                }}
              >
                <span className="changes-commit-check">{on && <CheckIcon size={12} />}</span>
                <span className="changes-commit-mode-text">
                  {mode.label}
                  <span className="changes-commit-hint">{hints[mode.label as keyof typeof hints]}</span>
                </span>
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
}

function StageBox({ worktree, list, staged, label, focusable = false }: { worktree: string; list: Change[]; staged: boolean; label: string; focusable?: boolean }) {
  const focusRow = (box: HTMLElement) => !focusable && box.nextElementSibling instanceof HTMLElement && box.nextElementSibling.focus();
  return (
    <button
      type="button"
      role="checkbox"
      className="checkbox"
      aria-checked={staged}
      aria-label={label}
      title={label}
      tabIndex={focusable ? 0 : -1}
      disabled={list.length === 0}
      onClick={(e) => {
        focusRow(e.currentTarget);
        void toggleStaged(worktree, list, staged);
      }}
    >
      <CheckIcon size={12} />
    </button>
  );
}

const toggleStaged = (worktree: string, list: Change[], staged: boolean) => (staged ? unstageChanges(worktree, list) : stageChanges(worktree, list));

const showRowMenu = (e: MouseEvent, list: Change[]) => {
  const target = changesTarget(list);
  const discards = !list.some((c) => c.status === "conflicted");
  showMenu(
    e,
    list[0].staged
      ? [["changes.unstage", target], ...(discards ? [["changes.discardStaged", target] as [string, string]] : [])]
      : [["changes.stage", target], ...(discards ? [["changes.discard", target] as [string, string]] : [])],
  );
};

type RowProps = { worktree: string; change: Change; rowKey: string; arrived: boolean; selected: boolean; actsOn: Change[]; onPick: (key: string, extend: boolean) => void };

function ChangeRow({ worktree, change, rowKey, arrived, selected, actsOn, onPick }: RowProps) {
  const word = statusWord(change.status);
  return (
    <div className="changes-item" data-arrived={arrived || undefined}>
      <StageBox worktree={worktree} list={actsOn} staged={change.staged} label={`${change.staged ? "Unstage" : "Stage"} ${actsOn.length > 1 ? `${actsOn.length} files` : change.path}`} />
      <button
        type="button"
        role="treeitem"
        className="files-row changes-row"
        aria-selected={selected}
        aria-checked={change.staged}
        data-key={rowKey}
        title={change.oldPath ? `${change.oldPath} → ${change.path}` : change.path}
        aria-label={[change.path, word, change.added && `${change.added} added`, change.removed && `${change.removed} removed`].filter(Boolean).join(", ")}
        onClick={(e) => {
          e.currentTarget.focus();
          onPick(rowKey, e.shiftKey);
          if (!e.shiftKey) showDiff(worktree, change);
        }}
        onDoubleClick={(e) => !e.shiftKey && showDiff(worktree, change, false)}
        onKeyUp={(e) => e.key === " " && e.preventDefault()}
        onContextMenu={(e) => showRowMenu(e, actsOn)}
      >
        <ChangeParts path={change.path} status={change.status} added={change.added} removed={change.removed} />
      </button>
      <button type="button" className="icon-button changes-more" title="More" aria-label={`More for ${change.path}`} onClick={(e) => showRowMenu(e, actsOn)}>
        <MoreIcon />
      </button>
    </div>
  );
}

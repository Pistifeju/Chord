// The Preview view in the side panel: its header with the preview model's menu and the line while a rebuild runs, over the worktree's `#Preview` list, as a Files tree with its filter, beside the worktree's preview simulator screen and the compile error's bar, with the Simulator view's screen, bezel, input and Screenshot button, or the line that stands in for it. Also its tab bar button.
import { Menu } from "@tauri-apps/api/menu";
import { useEffect, useRef, useState, type CSSProperties, type MouseEvent } from "react";
import { tooltip } from "../../../app/commands";
import type { BuildIssue_Deserialize } from "../../../ipc/bindings";
import { ChevronIcon, EyeIcon, FileIcon, FilterIcon, PhoneIcon, WarningIcon } from "../../../ui/icons";
import { showFileAt } from "../../editor/showing";
import { fail } from "../../../ui/alert";
import { popUpUnder, simulatorItems } from "../build/RunControl";
import { issuePlace, pathInWorktree } from "../build/run";
import { shortcutChanges } from "../../settings/store";
import { showsSideView, sidePanels, toggleSideView } from "../../sidepanel/store";
import { SimulatorView } from "../simulator/SimulatorView";
import { treeKeyAction } from "../../files/tree";
import "../../files/files.css";
import { issueLine, previewKey, previewLine, previewRows, rebuilding, shownError, type PreviewRow } from "./preview";
import "./preview.css";
import { PanelEdge } from "../../../ui/PanelEdge";
import { choosePreview, choosePreviewModel, closePreview, loadPreviewList, openPreview, previewListWidth, previewLists, previewPicks, previews, resizePreviewList, usePreviewModel } from "./store";

export function PreviewToggle({ worktree }: { worktree: string }) {
  shortcutChanges.use();
  const shown = sidePanels.usePart((all) => showsSideView(all, worktree, "preview"));
  return (
    <button
      type="button"
      className="icon-button"
      title={tooltip("preview.toggle")}
      aria-label="Show or Hide Preview"
      aria-pressed={shown}
      onClick={() => toggleSideView(worktree, "preview")}
    >
      <EyeIcon />
    </button>
  );
}

function ErrorBar({ worktree, issue }: { worktree: string; issue: BuildIssue_Deserialize }) {
  const place = issuePlace(issue);
  const path = issue.file ? pathInWorktree(worktree, issue.file) : null;
  const content = (
    <>
      <span className="preview-error-icon">
        <WarningIcon />
      </span>
      {place && <span className="preview-error-place">{place}</span>}
      <span className="preview-error-message">{issue.message}</span>
    </>
  );
  if (path === null)
    return (
      <div className="preview-error" aria-live="polite" title={issueLine(issue)}>
        {content}
      </div>
    );
  return (
    <button type="button" className="preview-error" aria-live="polite" title={issueLine(issue)} onClick={() => showFileAt(worktree, path, issue.line)}>
      {content}
    </button>
  );
}

export function PreviewView({ worktree }: { worktree: string }) {
  const line = previews.usePart((all) => previewLine(all[worktree]));
  const building = previews.usePart((all) => rebuilding(all[worktree]));
  const error = previews.usePart((all) => shownError(all[worktree]));
  const pick = previewPicks.usePart((all) => all[worktree] ?? null);
  const model = usePreviewModel(worktree);
  const picked = pick !== null;
  useEffect(() => {
    const number = openPreview(worktree);
    return () => closePreview(worktree, number);
  }, [worktree, model, picked]);
  const modelMenu = async () => Menu.new({ items: await simulatorItems(model, (chosen) => choosePreviewModel(worktree, chosen)) });
  return (
    <div className="preview">
      <div className="preview-header">
        {building && (
          <span className="preview-rebuilding" role="status">
            <span className="spinner" aria-hidden="true" />
            Rebuilding
          </span>
        )}
        <button
          type="button"
          className="preview-button preview-model"
          title="Preview Device"
          onClick={(e: MouseEvent<HTMLButtonElement>) => popUpUnder(worktree, e.currentTarget, modelMenu).catch(fail)}
        >
          <PhoneIcon />
          <span className="preview-file">{model?.name ?? "Newest iPhone"}</span>
        </button>
      </div>
      <div className="preview-body">
        <PreviewList worktree={worktree} />
        <SimulatorView worktree={worktree} preview={line} />
      </div>
      {error && <ErrorBar worktree={worktree} issue={error} />}
    </div>
  );
}

// Focusing a preview's row shows it, so ↑ and ↓ step through the previews.
function PreviewList({ worktree }: { worktree: string }) {
  const all = previewLists.usePart((lists) => lists[worktree] ?? null);
  const pick = previewPicks.usePart((picks) => picks[worktree] ?? null);
  const [query, setQuery] = useState("");
  const [closed, setClosed] = useState<string[]>([]);
  const rows = previewRows(all ?? [], query, closed);
  const list = useRef<HTMLDivElement>(null);
  const width = previewListWidth.use();
  const toggle = (file: string) => setClosed((files) => (files.includes(file) ? files.filter((f) => f !== file) : [...files, file]));
  useEffect(() => {
    loadPreviewList(worktree);
    const onFocus = () => loadPreviewList(worktree);
    window.addEventListener("focus", onFocus);
    return () => window.removeEventListener("focus", onFocus);
  }, [worktree]);
  return (
    <div className="preview-list" style={{ width }}>
      <PanelEdge side="right" onResize={resizePreviewList} onEnd={() => {}} />
      <div className="files-filter">
        <FilterIcon />
        <input className="input" placeholder="Filter" aria-label="Filter Previews" value={query} onChange={(e) => setQuery(e.target.value)} />
      </div>
      <div
        ref={list}
        className="files-tree"
        role="tree"
        aria-label="Previews"
        onKeyDown={(e) => {
          const path = (e.target as HTMLElement).dataset.path;
          const action = path === undefined || e.metaKey || e.altKey || e.ctrlKey || e.shiftKey ? null : treeKeyAction(rows, path, e.key);
          if (!action) return;
          e.preventDefault();
          if ("toggle" in action) toggle(action.toggle);
          else list.current?.querySelector<HTMLElement>(`[data-path="${CSS.escape(action.focus)}"]`)?.focus();
        }}
      >
        {rows.map((row) => (
          <ListRow key={row.entry.path} row={row} selected={pick !== null && row.pick !== null && previewKey(pick) === row.entry.path} onToggle={toggle} onPick={(chosen) => choosePreview(worktree, chosen)} />
        ))}
        {all?.length === 0 && <div className="files-empty">No previews in this worktree</div>}
        {!!all?.length && rows.length === 0 && <div className="files-empty">No matches</div>}
      </div>
    </div>
  );
}

function ListRow({ row, selected, onToggle, onPick }: { row: PreviewRow; selected: boolean; onToggle: (file: string) => void; onPick: (pick: NonNullable<PreviewRow["pick"]>) => void }) {
  const { entry, depth, open, pick } = row;
  return (
    <button
      type="button"
      role="treeitem"
      className="files-row"
      style={{ "--depth": depth } as CSSProperties}
      title={pick ? `${entry.name} · ${pick.file}` : entry.path}
      data-path={entry.path}
      aria-level={depth + 1}
      aria-expanded={pick ? undefined : open}
      aria-selected={selected}
      // WebKit does not focus a clicked button, and the focus is what shows a preview.
      onMouseDown={(event) => event.currentTarget.focus()}
      onFocus={() => pick && !selected && onPick(pick)}
      onClick={() => !pick && onToggle(entry.path)}
    >
      <span className="turn-chevron files-chevron" data-open={open} data-hidden={pick !== null}>
        <ChevronIcon />
      </span>
      <span className="files-icon">{pick ? <EyeIcon /> : <FileIcon />}</span>
      <span className="files-name">{entry.name}</span>
    </button>
  );
}

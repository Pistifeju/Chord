// The Preview view in the side panel, or its centered line in a worktree without a `#Preview`: the worktree's preview simulator screen under its header, the preview model's menu and the line while a rebuild runs, beside the worktree's `#Preview` list on the panel's outer edge, as a Files tree that ↑ and ↓ step through, with a filter from five previews, and the compile error's bar, with the Simulator view's screen, bezel, input and Screenshot button, or the line that stands in for it. Also its toolbar button.
import { Menu } from "@tauri-apps/api/menu";
import { useEffect, useRef, useState, type CSSProperties, type MouseEvent } from "react";
import { tooltip } from "../../../app/commands";
import type { BuildIssue_Deserialize } from "../../../ipc/bindings";
import { ChevronDownIcon, ChevronIcon, EyeIcon, FileIcon, PhoneIcon, WarningIcon } from "../../../ui/icons";
import { showFileAt } from "../../editor/showing";
import { fail } from "../../../ui/alert";
import { popUpUnder, simulatorItems } from "../build/Toolbar";
import { useFallbackSimulator } from "../build/destinationStore";
import { issuePlace, pathInWorktree } from "../build/run";
import { shortcutChanges } from "../../settings/store";
import { showsSideView, sidePanels, toggleSideView } from "../../sidepanel/store";
import { SimulatorView } from "../simulator/SimulatorView";
import { treeKeyAction } from "../../files/tree";
import "../../files/files.css";
import { issueLine, previewKey, previewLine, previewName, previewRows, rebuilding, shownError, steppedPreview, type PreviewRow } from "./preview";
import "./preview.css";
import { PanelEdge } from "../../../ui/PanelEdge";
import { filtersOptions } from "../../chat/dialog";
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
  const fallback = useFallbackSimulator(worktree);
  const picked = pick !== null;
  const count = previewLists.usePart((lists) => lists[worktree]?.length ?? null);
  useEffect(() => {
    loadPreviewList(worktree);
    const onFocus = () => loadPreviewList(worktree);
    window.addEventListener("focus", onFocus);
    return () => window.removeEventListener("focus", onFocus);
  }, [worktree]);
  useEffect(() => {
    const number = openPreview(worktree);
    return () => closePreview(worktree, number);
  }, [worktree, model, picked]);
  const modelMenu = async () => Menu.new({ items: await simulatorItems(model, (chosen) => choosePreviewModel(worktree, chosen)) });
  // The first list read decides between the screen and the line, so neither flashes.
  if (count === null) return <div className="preview" />;
  if (count === 0)
    return (
      <div className="preview">
        <div className="preview-empty">
          <p>No previews in this worktree.</p>
          <p>
            Add a <code>#Preview</code> to a SwiftUI view to see it here.
          </p>
        </div>
      </div>
    );
  return (
    <div className="preview">
      <div className="preview-body">
        <div className="preview-main">
          <div className="preview-header">
            <button
              type="button"
              className="preview-button preview-model"
              title="Preview Device"
              onClick={(e: MouseEvent<HTMLButtonElement>) => popUpUnder(worktree, e.currentTarget, modelMenu).catch(fail)}
            >
              <PhoneIcon />
              <span className="preview-file">{(model ?? fallback)?.name}</span>
              <span className="preview-model-chevron">
                <ChevronDownIcon />
              </span>
            </button>
            {building && (
              <span className="preview-rebuilding" role="status">
                <span className="spinner" aria-hidden="true" />
                Rebuilding
              </span>
            )}
          </div>
          <SimulatorView worktree={worktree} preview={line} />
        </div>
        <PreviewList worktree={worktree} />
      </div>
      {error && <ErrorBar worktree={worktree} issue={error} />}
    </div>
  );
}

// Focusing a preview's row shows it. ↑ and ↓ show the next preview from a row or the filter field, past file rows, and ⌥ jumps a file.
function PreviewList({ worktree }: { worktree: string }) {
  const all = previewLists.usePart((lists) => lists[worktree] ?? null);
  const pick = previewPicks.usePart((picks) => picks[worktree] ?? null);
  const [query, setQuery] = useState("");
  const [closed, setClosed] = useState<string[]>([]);
  const filtering = filtersOptions(all?.length ?? 0);
  const rows = previewRows(all ?? [], filtering ? query : "", closed);
  const list = useRef<HTMLDivElement>(null);
  const width = previewListWidth.use();
  const toggle = (file: string) => setClosed((files) => (files.includes(file) ? files.filter((f) => f !== file) : [...files, file]));
  return (
    <div
      className="preview-list"
      style={{ width }}
      onKeyDown={(e) => {
        if ((e.key !== "ArrowDown" && e.key !== "ArrowUp") || e.metaKey || e.ctrlKey || e.shiftKey) return;
        e.preventDefault();
        const row = (e.target as HTMLElement).dataset.path;
        const next = steppedPreview(rows, row ?? (pick && previewKey(pick)), e.key === "ArrowDown", e.altKey);
        if (!next) return;
        if (list.current) list.current.dataset.keys = "";
        const element = list.current?.querySelector<HTMLElement>(`[data-path="${CSS.escape(previewKey(next))}"]`);
        // A row hands the focus on, which shows the preview; the field keeps it, so typing goes on.
        if (row !== undefined) element?.focus();
        else {
          element?.scrollIntoView({ block: "nearest" });
          choosePreview(worktree, next);
        }
      }}
    >
      <PanelEdge side="left" onResize={resizePreviewList} onEnd={() => {}} />
      {filtering && (
        <div className="files-filter">
          <input className="input" placeholder="Filter" aria-label="Filter Previews" value={query} onChange={(e) => setQuery(e.target.value)} />
        </div>
      )}
      <div
        ref={list}
        className="files-tree"
        role="tree"
        aria-label="Previews"
        onKeyDown={(e) => {
          const path = (e.target as HTMLElement).dataset.path;
          const action = path === undefined || (e.key !== "ArrowLeft" && e.key !== "ArrowRight") || e.metaKey || e.altKey || e.ctrlKey || e.shiftKey ? null : treeKeyAction(rows, path, e.key);
          if (!action) return;
          e.preventDefault();
          e.currentTarget.dataset.keys = "";
          if ("toggle" in action) toggle(action.toggle);
          else list.current?.querySelector<HTMLElement>(`[data-path="${CSS.escape(action.focus)}"]`)?.focus();
        }}
        onMouseMove={(e) => delete e.currentTarget.dataset.keys}
      >
        {rows.map((row) => (
          <ListRow key={row.entry.path} row={row} selected={pick !== null && row.pick !== null && previewKey(pick) === row.entry.path} onToggle={toggle} onPick={(chosen) => choosePreview(worktree, chosen)} />
        ))}
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
      title={pick ? `${previewName(pick)} · ${pick.file}` : entry.path}
      data-path={entry.path}
      aria-level={depth + 1}
      aria-expanded={pick ? undefined : open}
      aria-selected={selected}
      // WebKit does not focus a clicked button and blurs it after mousedown, and the focus is what shows a preview.
      onMouseDown={(event) => {
        event.preventDefault();
        event.currentTarget.focus();
      }}
      onFocus={() => pick && !selected && onPick(pick)}
      onClick={() => !pick && onToggle(entry.path)}
    >
      <span className="turn-chevron files-chevron" data-open={open} data-hidden={pick !== null}>
        <ChevronIcon />
      </span>
      {depth === 0 && (
        <span className="files-icon">
          <FileIcon />
        </span>
      )}
      <span className="files-name">{entry.name}</span>
    </button>
  );
}

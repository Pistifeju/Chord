// The Preview view in the side panel: its header with the preview picker's button, the preview model's menu and the line while a rebuild runs, over the worktree's preview simulator screen and the compile error's bar, with the Simulator view's screen, bezel and input and without its buttons, or the line that stands in for it. Also its tab bar button.
import { Menu } from "@tauri-apps/api/menu";
import { useEffect, type MouseEvent } from "react";
import { tooltip } from "../../../app/commands";
import type { BuildIssue_Deserialize } from "../../../ipc/bindings";
import { ChevronDownIcon, EyeIcon, PhoneIcon, WarningIcon } from "../../../ui/icons";
import { showFileAt } from "../../editor/showing";
import { fail } from "../../worktrees/store";
import { popUpUnder, simulatorItems } from "../build/RunControl";
import { issuePlace, pathInWorktree } from "../build/run";
import { shortcutChanges } from "../../settings/store";
import { showsSideView, sidePanels, toggleSideView } from "../../sidepanel/store";
import { SimulatorView } from "../simulator/SimulatorView";
import { issueLine, previewFileName, previewLine, previewName, rebuilding, shownError } from "./preview";
import "./preview.css";
import { choosePreviewModel, closePreview, openPreview, openPreviewPicker, previewPicks, previews, usePreviewModel } from "./store";

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
        <button type="button" className="preview-button" title="Choose Preview" onClick={() => openPreviewPicker(worktree)}>
          {pick ? (
            <>
              <span className="preview-file">{previewFileName(pick)}</span>
              <span className="preview-muted">{previewName(pick)}</span>
            </>
          ) : (
            <span className="preview-muted">Choose Preview</span>
          )}
          <ChevronDownIcon />
        </button>
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
      <SimulatorView worktree={worktree} preview={line} />
      {error && <ErrorBar worktree={worktree} issue={error} />}
    </div>
  );
}

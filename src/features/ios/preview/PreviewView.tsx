// The Preview view in the side panel: its header with the preview picker's button, the preview model's menu and the line while a rebuild runs, over the worktree's preview simulator screen, with the Simulator view's screen, bezel and input and without its buttons, or the line that stands in for it. Also its tab bar button.
import { Menu } from "@tauri-apps/api/menu";
import { useEffect, type MouseEvent } from "react";
import { tooltip } from "../../../app/commands";
import { ChevronDownIcon, EyeIcon, PhoneIcon } from "../../../ui/icons";
import { fail } from "../../worktrees/store";
import { popUpUnder, simulatorItems } from "../build/RunControl";
import { shortcutChanges } from "../../settings/store";
import { showsSideView, sidePanels, toggleSideView } from "../../sidepanel/store";
import { SimulatorView } from "../simulator/SimulatorView";
import { previewFileName, previewLine, previewName, rebuilding } from "./preview";
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

export function PreviewView({ worktree }: { worktree: string }) {
  const line = previews.usePart((all) => previewLine(all[worktree]));
  const building = previews.usePart((all) => rebuilding(all[worktree]));
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
    </div>
  );
}

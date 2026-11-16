// The Preview view in the side panel: the worktree's preview simulator screen, with the Simulator view's screen, bezel and input and without its buttons, or the line that stands in for it. Also its tab bar button.
import { useEffect } from "react";
import { tooltip } from "../../../app/commands";
import { EyeIcon } from "../../../ui/icons";
import { shortcutChanges } from "../../settings/store";
import { showsSideView, sidePanels, toggleSideView } from "../../sidepanel/store";
import { SimulatorView } from "../simulator/SimulatorView";
import { previewLine } from "./preview";
import { openPreview, previews } from "./store";

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
  useEffect(() => openPreview(worktree), [worktree]);
  return <SimulatorView worktree={worktree} preview={line} />;
}

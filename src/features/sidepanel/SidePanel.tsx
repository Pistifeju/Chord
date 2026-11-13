// The side panel right of the panes: the title bar row over it with the buttons the panes pass in, then one view at a time, today the Files view.
import type { ReactNode } from "react";
import { FilesView } from "../files/FilesView";
import { PanelEdge } from "../../ui/PanelEdge";
import { endSidePanelResize, resizeSidePanel, sidePanelOf, sidePanels } from "./store";
import "./sidepanel.css";

export function SidePanel({ worktree, bar }: { worktree: string; bar: ReactNode }) {
  const { open, width } = sidePanelOf(sidePanels.use(), worktree);
  if (!open) return null;
  return (
    <aside className="side-panel" style={{ flexBasis: width }} aria-label="Files">
      <PanelEdge side="left" onResize={(size) => resizeSidePanel(worktree, size)} onEnd={() => endSidePanelResize(worktree)} />
      <div className="side-panel-bar" data-tauri-drag-region>
        {bar}
      </div>
      <FilesView worktree={worktree} />
    </aside>
  );
}

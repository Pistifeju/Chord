// The side panel right of the panes: one view at a time, today the Files view.
import { FilesView } from "../files/FilesView";
import { PanelEdge } from "../../ui/PanelEdge";
import { endSidePanelResize, resizeSidePanel, sidePanelOf, sidePanels } from "./store";
import "./sidepanel.css";

export function SidePanel({ worktree }: { worktree: string }) {
  const { open, width } = sidePanelOf(sidePanels.use(), worktree);
  if (!open) return null;
  return (
    <aside className="side-panel" style={{ flexBasis: width }} aria-label="Files">
      <PanelEdge side="left" onResize={(size) => resizeSidePanel(worktree, size)} onEnd={() => endSidePanelResize(worktree)} />
      <FilesView worktree={worktree} />
    </aside>
  );
}

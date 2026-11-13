// The bottom panel under the panes: today the worktree's terminal.
import { PanelEdge } from "../../ui/PanelEdge";
import { TerminalView } from "../terminal/TerminalView";
import { bottomPanelOf, bottomPanels, endBottomPanelResize, resizeBottomPanel } from "./store";
import "./bottompanel.css";

export function BottomPanel({ worktree }: { worktree: string }) {
  const { open, height } = bottomPanelOf(bottomPanels.use(), worktree);
  if (!open) return null;
  return (
    <section className="bottom-panel" style={{ flexBasis: height }} aria-label="Terminal">
      <PanelEdge side="top" size={height} onResize={(size) => resizeBottomPanel(worktree, size)} onEnd={() => endBottomPanelResize(worktree)} />
      <TerminalView key={worktree} worktree={worktree} />
    </section>
  );
}

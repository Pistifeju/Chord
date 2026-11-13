// The bottom panel under the panes: today the worktree's terminal.
import { PanelEdge } from "../../ui/PanelEdge";
import { terminalView } from "../../app/deferredViews";
import "@xterm/xterm/css/xterm.css";
import "../terminal/terminal.css";
import { bottomPanelOf, bottomPanels, endBottomPanelResize, resizeBottomPanel } from "./store";
import "./bottompanel.css";

export function BottomPanel({ worktree }: { worktree: string }) {
  const { open, height } = bottomPanelOf(bottomPanels.use(), worktree);
  const terminal = terminalView.use();
  if (!open) return null;
  return (
    <section className="bottom-panel" style={{ flexBasis: height }} aria-label="Terminal">
      <PanelEdge side="top" onResize={(size) => resizeBottomPanel(worktree, size)} onEnd={() => endBottomPanelResize(worktree)} />
      {terminal && <terminal.TerminalView key={worktree} worktree={worktree} />}
    </section>
  );
}

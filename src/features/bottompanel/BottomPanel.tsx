// The bottom panel under the panes: the worktree's terminal and, right of it, the views the window passes in, one at a time in a column of its own with a header, tabs when there are several, and a Hide button; each column shows on its own.
import type { ReactNode } from "react";
import { tooltip } from "../../app/commands";
import type { BottomView } from "../../ipc/bindings";
import { PanelEdge } from "../../ui/PanelEdge";
import { CloseIcon } from "../../ui/icons";
import { terminalView } from "../../app/deferredViews";
import "@xterm/xterm/css/xterm.css";
import "../terminal/terminal.css";
import { bottomPanelOf, bottomPanels, changeBottomPanel, endBottomPanelResize, resizeBottomPanel, showBottomPanel } from "./store";
import "./bottompanel.css";

/** A view beside the terminal, with the command that shows or hides it. */
export type BottomViewSlot = { id: Exclude<BottomView, "terminal">; title: string; toggleCommand: string; body: ReactNode };

export function BottomPanel({ worktree, views }: { worktree: string; views: BottomViewSlot[] }) {
  const { open, height, view } = bottomPanelOf(bottomPanels.use(), worktree);
  const terminal = terminalView.use();
  // A saved view this worktree no longer has stays hidden.
  const shown = views.find((slot) => slot.id === view);
  if (!open && !shown) return null;
  return (
    <section className="bottom-panel" style={{ flexBasis: height }} aria-label={open ? "Terminal" : shown?.title}>
      <PanelEdge side="top" onResize={(size) => resizeBottomPanel(worktree, size)} onEnd={() => endBottomPanelResize(worktree)} />
      {open && (
      <div className="bottom-main">
        <div className="bottom-header">
          <span className="bottom-title">Terminal</span>
          <button type="button" className="icon-button bottom-hide" title={tooltip("terminal.toggle")} aria-label="Hide Terminal" onClick={() => showBottomPanel(worktree, false)}>
            <CloseIcon />
          </button>
        </div>
        {terminal && <terminal.TerminalView key={worktree} worktree={worktree} />}
      </div>
      )}
      {shown && (
        <div className="bottom-main bottom-side" aria-label={shown.title}>
          <div className="bottom-header">
            {views.length > 1 ? (
              <div className="bottom-tabs" role="tablist">
                {views.map((slot) => (
                  <button key={slot.id} type="button" role="tab" className="bottom-tab" aria-selected={slot.id === shown.id} onClick={() => changeBottomPanel(worktree, { view: slot.id })}>
                    {slot.title}
                  </button>
                ))}
              </div>
            ) : (
              <span className="bottom-title">{shown.title}</span>
            )}
            <button type="button" className="icon-button bottom-hide" title={tooltip(shown.toggleCommand)} aria-label={`Hide ${shown.title}`} onClick={() => changeBottomPanel(worktree, { view: "terminal" })}>
              <CloseIcon />
            </button>
          </div>
          {shown.body}
        </div>
      )}
    </section>
  );
}

import { useState, type CSSProperties, type ReactNode } from "react";
import { tooltip } from "../../app/commands";
import type { BottomView } from "../../ipc/bindings";
import { PanelEdge } from "../../ui/PanelEdge";
import { CloseIcon } from "../../ui/icons";
import { terminalView } from "../../app/deferredViews";
import "@xterm/xterm/css/xterm.css";
import "../terminal/terminal.css";
import {
  bottomPanelMin,
  bottomPanelOf,
  bottomPanels,
  changeBottomPanel,
  endBottomPanelResize,
  hideBottomPanel,
  resizeBottomPanel,
  resizeBottomSide,
  showBottomPanel,
} from "./store";
import "./bottompanel.css";

export type BottomViewSlot = { id: Exclude<BottomView, "terminal">; title: string; toggleCommand: string; body: ReactNode };

export function BottomPanel({ worktree, views }: { worktree: string; views: BottomViewSlot[] }) {
  const { open, height, view, sideWidth } = bottomPanelOf(bottomPanels.use(), worktree);
  const terminal = terminalView.use();
  const [snapped, setSnapped] = useState(false);
  const snapsBelow = bottomPanelMin / 2;
  const shown = views.find((slot) => slot.id === view);
  if (!open && !shown) return null;
  return (
    <section
      className="bottom-panel"
      data-snapped={snapped || undefined}
      style={{ flexBasis: snapped ? 0 : height, "--panel-height": `${height}px` } as CSSProperties}
      aria-label={open ? "Terminal" : shown?.title}
    >
      <PanelEdge
        side="top"
        shrinksBelow={bottomPanelMin}
        onResize={(size) => {
          setSnapped(size < snapsBelow);
          if (size >= snapsBelow) resizeBottomPanel(worktree, size);
        }}
        onEnd={(size) => {
          setSnapped(false);
          if (size < snapsBelow) hideBottomPanel(worktree);
          else endBottomPanelResize(worktree);
        }}
      />
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
        <div className="bottom-main bottom-side" style={open ? { flex: `0 1 ${sideWidth}px` } : undefined} aria-label={shown.title}>
          {open && <PanelEdge side="left" onResize={(size) => resizeBottomSide(worktree, size)} onEnd={() => endBottomPanelResize(worktree)} />}
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

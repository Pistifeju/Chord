// The side panel right of the panes: one view at a time, today the Files view.
import { useRef, type PointerEvent } from "react";
import { FilesView } from "../files/FilesView";
import { endSidePanelResize, resizeSidePanel, sidePanelOf, sidePanels } from "./store";
import "./sidepanel.css";

type Drag = { startX: number; startWidth: number; moved: boolean };

export function SidePanel({ worktree }: { worktree: string }) {
  const { open, width } = sidePanelOf(sidePanels.use(), worktree);
  const drag = useRef<Drag | null>(null);
  if (!open) return null;

  const onPointerDown = (e: PointerEvent<HTMLDivElement>) => {
    e.currentTarget.setPointerCapture(e.pointerId);
    drag.current = { startX: e.clientX, startWidth: width, moved: false };
  };

  // The panel grows as its left edge moves left.
  const onPointerMove = (e: PointerEvent) => {
    const d = drag.current;
    if (!d) return;
    d.moved = true;
    resizeSidePanel(worktree, d.startWidth + d.startX - e.clientX);
  };

  const onPointerEnd = () => {
    const d = drag.current;
    drag.current = null;
    if (d?.moved) endSidePanelResize(worktree);
  };

  return (
    <aside className="side-panel" style={{ flexBasis: width }} aria-label="Files">
      <div
        className="side-panel-edge"
        role="separator"
        aria-orientation="vertical"
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerEnd}
        onPointerCancel={onPointerEnd}
      />
      <FilesView worktree={worktree} />
    </aside>
  );
}

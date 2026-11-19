// The side panel right of the panes, under the toolbar: one view at a time of the views the window passes in, the Simulator or the Preview view.
import { useEffect, type ReactNode } from "react";
import type { SideView } from "../../ipc/bindings";
import { PanelEdge } from "../../ui/PanelEdge";
import { endSidePanelResize, keepSideView, leaveSidebarView, resizeSidePanel, sidePanelOf, sidePanels } from "./store";
import "./sidepanel.css";

/** A view of the side panel; its button sits in the toolbar. */
export type SideViewSlot = { id: Exclude<SideView, "files" | "changes">; title: string; body: ReactNode };

export function SidePanel({ worktree, views }: { worktree: string; views: SideViewSlot[] }) {
  const { open, width, view } = sidePanelOf(sidePanels.use(), worktree);
  const shown = views.find((slot) => slot.id === view);
  // A saved view this worktree no longer has turns to the first view, so its button and shortcut read the panel right; Files and Changes, now in the sidebar, close the panel too.
  const first = views[0]?.id;
  useEffect(() => {
    if (shown || !first) return;
    if (view === "files" || view === "changes") leaveSidebarView(worktree, first);
    else keepSideView(worktree, first);
  }, [shown, first, view, worktree]);
  if (!open || !shown) return null;
  return (
    <aside className="side-panel" style={{ flexBasis: width }} aria-label={shown.title}>
      <PanelEdge side="left" onResize={(size) => resizeSidePanel(worktree, size)} onEnd={() => endSidePanelResize(worktree)} />
      {shown.body}
    </aside>
  );
}

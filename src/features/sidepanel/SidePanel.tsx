// The side panel right of the panes: the title bar row over it with the buttons the panes pass in, then one view at a time: the Files view or a view the window passes in.
import { useEffect, type ReactNode } from "react";
import type { SideView } from "../../ipc/bindings";
import { FilesView } from "../files/FilesView";
import { PanelEdge } from "../../ui/PanelEdge";
import { endSidePanelResize, keepSideView, resizeSidePanel, sidePanelOf, sidePanels } from "./store";
import "./sidepanel.css";

/** A view beside Files, with its button for the title bar row. */
export type SideViewSlot = { id: Exclude<SideView, "files">; title: string; button: ReactNode; body: ReactNode };

export function SidePanel({ worktree, bar, views }: { worktree: string; bar: ReactNode; views: SideViewSlot[] }) {
  const { open, width, view } = sidePanelOf(sidePanels.use(), worktree);
  const shown = views.find((slot) => slot.id === view);
  // A saved view this worktree no longer has turns back to Files, so the Files button and ⌘⇧E read the panel right.
  const gone = view !== "files" && !shown;
  useEffect(() => {
    if (gone) keepSideView(worktree, "files");
  }, [gone, worktree]);
  if (!open) return null;
  return (
    <aside className="side-panel" style={{ flexBasis: width }} aria-label={shown?.title ?? "Files"}>
      <PanelEdge side="left" onResize={(size) => resizeSidePanel(worktree, size)} onEnd={() => endSidePanelResize(worktree)} />
      <div className="side-panel-bar" data-tauri-drag-region>
        {bar}
      </div>
      {shown ? shown.body : <FilesView worktree={worktree} />}
    </aside>
  );
}

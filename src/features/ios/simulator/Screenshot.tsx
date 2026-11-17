// The thumbnail a Simulator screenshot or recording leaves at the view's bottom left: dragged out, it is a file for Finder or any app, and a screenshot dragged into a pane an attachment; a click pastes a screenshot into the focused agent.
import { useEffect, useState } from "react";
import { usePointerDrag } from "../../../ui/pointerDrag";
import { attachable, attachingPane, dismissScreenshot, dragScreenshot, pasteScreenshot, shot, type Shot } from "./store";

/** How long the thumbnail stays while the pointer is away. */
const THUMBNAIL_MS = 4000;

function Thumbnail({ current, bottom, leaving, onGone }: { current: Shot; bottom: number; leaving: boolean; onGone: () => void }) {
  const [hovered, setHovered] = useState(false);
  const drag = usePointerDrag<Shot>({ onStart: dragScreenshot, onMove: () => {}, onEnd: () => {} });

  useEffect(() => {
    if (hovered || leaving) return;
    const timer = window.setTimeout(dismissScreenshot, THUMBNAIL_MS);
    return () => window.clearTimeout(timer);
  }, [current, hovered, leaving]);

  return (
    <button
      type="button"
      className="simulator-thumbnail"
      style={{ bottom }}
      data-leaving={leaving || undefined}
      inert={leaving}
      onAnimationEnd={() => leaving && onGone()}
      title={attachable(current) && attachingPane() ? "Drag it out, or click to attach it to the focused agent" : "Drag it out"}
      aria-label={attachable(current) ? "Screenshot" : "Recording"}
      onPointerEnter={() => setHovered(true)}
      onPointerLeave={() => setHovered(false)}
      onClick={() => drag.clicked() && pasteScreenshot(current)}
      {...drag.handlers(current)}
    >
      <img src={current.preview} alt="" draggable={false} />
    </button>
  );
}

/** `bottom`: from the view's bottom edge, level with the button row's bottom. */
export function ScreenshotThumbnail({ worktree, bottom }: { worktree: string; bottom: number }) {
  const current = shot.usePart((last) => (last?.worktree === worktree ? last : null));
  // The dismissed shot stays mounted until its exit animation ends.
  const [shown, setShown] = useState(current);
  if (current && current !== shown) setShown(current);
  if (shown?.worktree !== worktree) return null;
  return <Thumbnail key={shown.screenshot.file} current={shown} bottom={bottom} leaving={!current} onGone={() => setShown(null)} />;
}

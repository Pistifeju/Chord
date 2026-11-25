import { useEffect, useState } from "react";
import { usePointerDrag } from "../../../ui/pointerDrag";
import { attachable, attachingPane, dismissScreenshot, dragScreenshot, pasteScreenshot, shot, type Shot } from "./store";

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

export function ScreenshotThumbnail({ worktree, bottom }: { worktree: string; bottom: number }) {
  const current = shot.usePart((last) => (last?.worktree === worktree ? last : null));
  const [shown, setShown] = useState(current);
  if (current && current !== shown) setShown(current);
  if (shown?.worktree !== worktree) return null;
  return <Thumbnail key={shown.screenshot.file} current={shown} bottom={bottom} leaving={!current} onGone={() => setShown(null)} />;
}

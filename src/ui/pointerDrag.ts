// A press that moves past a few pixels becomes a drag of one item; the click that ends it is swallowed.
import { useEffect, useRef, type PointerEvent } from "react";

/** Pixels a press moves before it becomes a drag, so a click still clicks. */
const DRAG_THRESHOLD = 5;

type Drag<T> = { item: T; startX: number; startY: number; started: boolean };

type PointerDragCallbacks<T> = {
  onStart: (item: T) => void;
  onMove: (e: PointerEvent, item: T) => void;
  /** `dropped` is false for a cancelled drag, and for one whose element went away before the pointer came up. */
  onEnd: (item: T, dropped: boolean) => void;
};

export function usePointerDrag<T>(callbacks: PointerDragCallbacks<T>) {
  const drag = useRef<Drag<T> | null>(null);
  const suppressClick = useRef(false);
  const latest = useRef(callbacks);
  latest.current = callbacks;

  const end = (dropped: boolean) => {
    const d = drag.current;
    drag.current = null;
    if (!d?.started) return;
    suppressClick.current = true;
    latest.current.onEnd(d.item, dropped);
  };

  // An element removed mid-drag never gets its pointerup.
  useEffect(() => () => end(false), []);

  return {
    handlers: (item: T) => ({
      onPointerDown: (e: PointerEvent<HTMLElement>) => {
        suppressClick.current = false;
        if (e.button !== 0) return;
        e.currentTarget.setPointerCapture(e.pointerId);
        drag.current = { item, startX: e.clientX, startY: e.clientY, started: false };
      },
      onPointerMove: (e: PointerEvent) => {
        const d = drag.current;
        if (!d) return;
        if (!d.started) {
          if (Math.hypot(e.clientX - d.startX, e.clientY - d.startY) < DRAG_THRESHOLD) return;
          d.started = true;
          latest.current.onStart(d.item);
        }
        latest.current.onMove(e, d.item);
      },
      onPointerUp: () => end(true),
      onPointerCancel: () => end(false),
    }),
    // A press that ended a drag is no click.
    clicked: () => {
      const clicked = !suppressClick.current;
      suppressClick.current = false;
      return clicked;
    },
  };
}

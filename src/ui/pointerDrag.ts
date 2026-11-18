// A press that moves past a few pixels becomes a drag of one item; the click that ends it is swallowed.
import { useCallback, useEffect, useRef, type MouseEvent, type PointerEvent } from "react";

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
  const pointer = useRef({ x: 0, y: 0 });
  const ghostElement = useRef<HTMLElement | null>(null);
  const place = (el: HTMLElement) => {
    el.style.left = `${pointer.current.x}px`;
    el.style.top = `${pointer.current.y}px`;
  };
  // The chip under the pointer moves without a React render, which lags a frame or more behind.
  const ghost = useCallback((el: HTMLElement | null) => {
    ghostElement.current = el;
    if (el) place(el);
  }, []);

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
      // A press left to WebKit starts a text selection that the drag then stretches over the editor; the blur keeps WebKit's own focus drop.
      onMouseDown: (e: MouseEvent<HTMLElement>) => {
        if (e.button !== 0) return;
        e.preventDefault();
        (document.activeElement as HTMLElement | null)?.blur();
      },
      onPointerMove: (e: PointerEvent) => {
        const d = drag.current;
        if (!d) return;
        pointer.current = { x: e.clientX, y: e.clientY };
        if (ghostElement.current) place(ghostElement.current);
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
    /** The ref for the chip that follows the pointer. */
    ghost,
    // A press that ended a drag is no click.
    clicked: () => {
      const clicked = !suppressClick.current;
      suppressClick.current = false;
      return clicked;
    },
  };
}

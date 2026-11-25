import { useCallback, useEffect, useRef, type MouseEvent, type PointerEvent } from "react";

const DRAG_THRESHOLD = 5;

type Drag<T> = { item: T; startX: number; startY: number; started: boolean };

type PointerDragCallbacks<T> = {
  onStart: (item: T) => void;
  onMove: (e: PointerEvent, item: T) => void;
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

  useEffect(() => () => end(false), []);

  return {
    handlers: (item: T) => ({
      onPointerDown: (e: PointerEvent<HTMLElement>) => {
        suppressClick.current = false;
        if (e.button !== 0) return;
        e.currentTarget.setPointerCapture(e.pointerId);
        drag.current = { item, startX: e.clientX, startY: e.clientY, started: false };
      },
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
    ghost,
    clicked: () => {
      const clicked = !suppressClick.current;
      suppressClick.current = false;
      return clicked;
    },
  };
}

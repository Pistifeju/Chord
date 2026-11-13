// The draggable edge of a panel that sits against the window's right or bottom side: dragging it toward the middle grows the panel.
import { useRef, type PointerEvent } from "react";
import "./panelEdge.css";

type Drag = { start: number; startSize: number; size: number | null; frame: number };

type Props = {
  side: "left" | "top";
  size: number;
  onResize: (size: number) => void;
  /** Runs once when a drag ends, so the size saves once. */
  onEnd: () => void;
};

export function PanelEdge({ side, size, onResize, onEnd }: Props) {
  const drag = useRef<Drag | null>(null);
  const along = (e: PointerEvent) => (side === "left" ? e.clientX : e.clientY);

  const onPointerDown = (e: PointerEvent<HTMLDivElement>) => {
    e.currentTarget.setPointerCapture(e.pointerId);
    drag.current = { start: along(e), startSize: size, size: null, frame: 0 };
  };

  const onPointerMove = (e: PointerEvent) => {
    const d = drag.current;
    if (!d) return;
    d.size = d.startSize + d.start - along(e);
    // At most one layout per frame; the panel below may be a terminal that refits on each.
    d.frame ||= requestAnimationFrame(() => {
      d.frame = 0;
      if (d.size !== null) onResize(d.size);
    });
  };

  const onPointerEnd = () => {
    const d = drag.current;
    drag.current = null;
    if (!d || d.size === null) return;
    cancelAnimationFrame(d.frame);
    onResize(d.size);
    onEnd();
  };

  return (
    <div
      className="panel-edge"
      data-side={side}
      role="separator"
      aria-orientation={side === "left" ? "vertical" : "horizontal"}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerEnd}
      onPointerCancel={onPointerEnd}
    />
  );
}

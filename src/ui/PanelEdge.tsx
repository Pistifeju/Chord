import { useRef, type PointerEvent } from "react";
import "./panelEdge.css";

type Drag = { panel: HTMLElement; start: number; startSize: number; size: number | null; frame: number };

type Props = {
  side: "left" | "right" | "top";
  onResize: (size: number) => void;
  onEnd: (size: number) => void;
  shrinksBelow?: number;
};

export function PanelEdge({ side, onResize, onEnd, shrinksBelow = 0 }: Props) {
  const drag = useRef<Drag | null>(null);
  const across = side !== "top";
  const along = (e: PointerEvent) => (side === "right" ? -e.clientX : across ? e.clientX : e.clientY);
  const shownSize = (panel: HTMLElement) => (across ? panel.offsetWidth : panel.offsetHeight);

  const onPointerDown = (e: PointerEvent<HTMLDivElement>) => {
    const panel = e.currentTarget.parentElement!;
    e.currentTarget.setPointerCapture(e.pointerId);
    drag.current = { panel, start: along(e), startSize: shownSize(panel), size: null, frame: 0 };
  };

  const onPointerMove = (e: PointerEvent) => {
    const d = drag.current;
    if (!d) return;
    const shown = shownSize(d.panel);
    const keepsOvershoot = d.size !== null && shrinksBelow > 0 && d.size < Math.max(shown, shrinksBelow);
    if (d.size !== null && !keepsOvershoot && !d.frame && Math.abs(d.size - shown) > 1) d.startSize += shown - d.size;
    d.size = d.startSize + d.start - along(e);
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
    onEnd(d.size);
  };

  return (
    <div
      className="panel-edge"
      data-side={side}
      role="separator"
      aria-orientation={across ? "vertical" : "horizontal"}
      onPointerDown={onPointerDown}
      onMouseDown={(e) => e.preventDefault()}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerEnd}
      onPointerCancel={onPointerEnd}
    />
  );
}

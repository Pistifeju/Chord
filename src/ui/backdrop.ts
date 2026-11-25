import type { MouseEvent } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";

export function dragWindowOrClose(e: MouseEvent, close: () => void) {
  if (e.button !== 0) return;
  const { screenX, screenY } = e;
  const stop = () => {
    window.removeEventListener("mousemove", move);
    window.removeEventListener("mouseup", up);
  };
  const move = (m: globalThis.MouseEvent) => {
    if (Math.hypot(m.screenX - screenX, m.screenY - screenY) < 4) return;
    stop();
    void getCurrentWindow().startDragging();
  };
  const up = () => {
    stop();
    close();
  };
  window.addEventListener("mousemove", move);
  window.addEventListener("mouseup", up);
}

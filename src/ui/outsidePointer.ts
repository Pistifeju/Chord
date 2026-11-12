// Closes a floating panel when the pointer goes down anywhere outside it.
import { useEffect, useRef, type RefObject } from "react";

// A control marked `data-menu-toggle` opens and closes the panel itself, so its press is not outside.
export function useOutsidePointer(panel: RefObject<HTMLElement | null>, onOutside: () => void) {
  const latest = useRef(onOutside);
  latest.current = onOutside;
  useEffect(() => {
    const press = (e: PointerEvent) => {
      const target = e.target as Element;
      if (!panel.current?.contains(target) && !target.closest?.("[data-menu-toggle]")) latest.current();
    };
    document.addEventListener("pointerdown", press, true);
    return () => document.removeEventListener("pointerdown", press, true);
  }, [panel]);
}

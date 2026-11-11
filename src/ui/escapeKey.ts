// Runs a handler on Escape while a sheet is open.
import { useEffect } from "react";

export function useEscape(active: boolean, onEscape: () => void) {
  useEffect(() => {
    if (!active) return;
    const close = (e: KeyboardEvent) => e.key === "Escape" && onEscape();
    window.addEventListener("keydown", close);
    return () => window.removeEventListener("keydown", close);
  }, [active, onEscape]);
}

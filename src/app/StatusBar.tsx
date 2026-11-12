// The strip at the window's bottom: the subscription usage while a harness reports it, and nothing otherwise.
import { useEffect, useState } from "react";
import { usage } from "../features/usage/store";
import { openWindows, usageText } from "../features/usage/usage";

const REFRESH_MS = 60000;

export function StatusBar() {
  const windows = usage.use();
  // A window goes stale when its reset passes, with no event to say so.
  const [now, setNow] = useState(Date.now);
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), REFRESH_MS);
    return () => clearInterval(timer);
  }, []);
  const open = openWindows(windows, now);
  if (open.length === 0) return null;
  return (
    <div className="statusbar">
      <span className="statusbar-usage">{open.map((window) => usageText(window, now)).join("  ·  ")}</span>
    </div>
  );
}

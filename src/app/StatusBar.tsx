// The strip under the main area: a slot for the simulators button, and the subscription usage with the shortest window's bar while a harness reports it.
import { useEffect, useState, type ReactNode } from "react";
import { usage } from "../features/usage/store";
import { openWindows, usageLevel, usageText, usageTitle, usedPercent } from "../features/usage/usage";

const REFRESH_MS = 60000;

export function StatusBar({ simulators }: { simulators: ReactNode }) {
  const windows = usage.use();
  // A window goes stale when its reset passes, with no event to say so.
  const [now, setNow] = useState(Date.now);
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), REFRESH_MS);
    return () => clearInterval(timer);
  }, []);
  const open = openWindows(windows, now);
  const percent = open[0] && usedPercent(open[0]);
  return (
    <div className="statusbar">
      {simulators}
      {open.length > 0 && (
        <span className="statusbar-usage" title={open.map((window) => usageTitle(window, now)).join("\n")}>
          {percent != null && (
            <span className="usage-bar" data-level={usageLevel(percent)} aria-hidden>
              <span style={{ width: `${Math.min(percent, 100)}%` }} />
            </span>
          )}
          {open.map((window) => usageText(window, now)).join("  ·  ")}
        </span>
      )}
    </div>
  );
}

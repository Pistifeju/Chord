// The strip under the main area: a slot for the simulators button, and the subscription usage of each harness that reports it, after its logo, with the shortest window's bar; a click on it opens the Usage panel.
import { useEffect, useState, type ReactNode } from "react";
import type { UsageWindow } from "../ipc/bindings";
import { harnessMarks } from "../features/harnesses/logos";
import { openUsagePanel } from "../features/usage/commands";
import { usage } from "../features/usage/store";
import { openWindows, usageLevel, usageText, usageTitle, usedPercent } from "../features/usage/usage";

const REFRESH_MS = 60000;

type Mark = (typeof harnessMarks)[string];

function HarnessUsage({ mark, windows, now }: { mark: Mark; windows: UsageWindow[]; now: number }) {
  const percent = usedPercent(windows[0]);
  const title = [mark.title, ...windows.map((window) => usageTitle(window, now))].join("\n");
  return (
    <button type="button" className="statusbar-harness-usage" title={title} onClick={openUsagePanel}>
      <mark.Logo />
      {percent != null && (
        <span className="usage-bar" data-level={usageLevel(percent)} aria-hidden>
          <span style={{ width: `${Math.min(percent, 100)}%` }} />
        </span>
      )}
      {windows.map((window) => usageText(window, now)).join("  ·  ")}
    </button>
  );
}

export function StatusBar({ simulators }: { simulators: ReactNode }) {
  const reported = usage.use();
  // A window goes stale when its reset passes, with no event to say so.
  const [now, setNow] = useState(Date.now);
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), REFRESH_MS);
    return () => clearInterval(timer);
  }, []);
  const shown = Object.entries(harnessMarks)
    .map(([harness, mark]) => ({ harness, mark, windows: openWindows(reported[harness] ?? [], now) }))
    .filter(({ windows }) => windows.length > 0);
  return (
    <div className="statusbar">
      {simulators}
      {shown.length > 0 && (
        <span className="statusbar-usage">
          {shown.map(({ harness, mark, windows }) => (
            <HarnessUsage key={harness} mark={mark} windows={windows} now={now} />
          ))}
        </span>
      )}
    </div>
  );
}

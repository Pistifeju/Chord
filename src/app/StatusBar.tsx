// The strip under the main area: the Settings button while the sidebar is hidden (the sidebar holds it otherwise), and the subscription usage with the shortest window's bar while a harness reports it.
import { useEffect, useState } from "react";
import { usage } from "../features/usage/store";
import { openWindows, usageText, usageTitle, usedPercent } from "../features/usage/usage";
import { openSettings, shortcutChanges } from "../features/settings/store";
import { sidebar } from "../features/sidebar/store";
import { GearIcon } from "../ui/icons";
import { tooltip } from "./commands";

const REFRESH_MS = 60000;

export function SettingsButton() {
  shortcutChanges.use();
  return (
    <button type="button" className="icon-button" title={tooltip("settings.open")} aria-label="Settings" onClick={openSettings}>
      <GearIcon />
    </button>
  );
}

export function StatusBar() {
  const windows = usage.use();
  const { hidden } = sidebar.use();
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
      {hidden && <SettingsButton />}
      {open.length > 0 && (
        <span className="statusbar-usage" title={open.map((window) => usageTitle(window, now)).join("\n")}>
          {percent != null && (
            <span className="usage-bar" aria-hidden>
              <span style={{ width: `${Math.min(percent, 100)}%` }} />
            </span>
          )}
          {open.map((window) => usageText(window, now)).join("  ·  ")}
        </span>
      )}
    </div>
  );
}

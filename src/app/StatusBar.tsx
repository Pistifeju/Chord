// The strip at the window's bottom: the Settings button, and the subscription usage while a harness reports it.
import { useEffect, useState } from "react";
import { usage } from "../features/usage/store";
import { openWindows, usageText } from "../features/usage/usage";
import { openSettings, shortcutChanges } from "../features/settings/store";
import { GearIcon } from "../ui/icons";
import { tooltip } from "./commands";

const REFRESH_MS = 60000;

export function StatusBar() {
  const windows = usage.use();
  shortcutChanges.use();
  // A window goes stale when its reset passes, with no event to say so.
  const [now, setNow] = useState(Date.now);
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), REFRESH_MS);
    return () => clearInterval(timer);
  }, []);
  const open = openWindows(windows, now);
  return (
    <div className="statusbar">
      <button type="button" className="icon-button" title={tooltip("settings.open")} aria-label="Settings" onClick={openSettings}>
        <GearIcon />
      </button>
      {open.length > 0 && <span className="statusbar-usage">{open.map((window) => usageText(window, now)).join("  ·  ")}</span>}
    </div>
  );
}

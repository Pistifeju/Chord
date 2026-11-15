// The run control at the tab bar's right end: a play button that builds, a stop square while the build runs.
import { tooltip } from "../../../app/commands";
import { shortcutChanges } from "../../settings/store";
import { PlayIcon, StopIcon } from "../../../ui/icons";
import { isBuilding } from "./run";
import { builds, startBuild, stopBuild } from "./store";

export function RunControl({ worktree }: { worktree: string }) {
  shortcutChanges.use();
  const building = builds.usePart((all) => isBuilding(all[worktree]));
  return (
    <button
      type="button"
      className="icon-button run-control"
      title={tooltip(building ? "build.stop" : "build.start")}
      aria-label={building ? "Stop" : "Build"}
      onClick={() => (building ? stopBuild(worktree) : startBuild(worktree))}
    >
      {building ? <StopIcon /> : <PlayIcon />}
    </button>
  );
}

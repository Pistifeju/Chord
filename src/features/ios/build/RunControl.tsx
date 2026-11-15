// The run control at the tab bar's right end: a play button that builds and runs, a stop square from the start of a build until the app launches.
import { tooltip } from "../../../app/commands";
import { shortcutChanges } from "../../settings/store";
import { PlayIcon, StopIcon } from "../../../ui/icons";
import { isStarting } from "./run";
import { builds, startBuild, stopBuild } from "./store";

export function RunControl({ worktree }: { worktree: string }) {
  shortcutChanges.use();
  const building = builds.usePart((all) => isStarting(all[worktree]));
  return (
    <button
      type="button"
      className="icon-button run-control"
      title={tooltip(building ? "build.stop" : "build.run")}
      aria-label={building ? "Stop" : "Run"}
      onClick={() => (building ? stopBuild(worktree) : startBuild(worktree, true))}
    >
      {building ? <StopIcon /> : <PlayIcon />}
    </button>
  );
}

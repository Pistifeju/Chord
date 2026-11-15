// Connects the running simulators and the build data size to IPC: the list kept fresh while the button shows, Shut Down Others, and the size read when Settings shows it.
import { commands, type RunningSimulator } from "../../../ipc/bindings";
import { externalStore } from "../../../ui/externalStore";
import { fail } from "../../worktrees/store";

const REFRESH_MS = 5000;

export const runningSimulators = externalStore<RunningSimulator[]>([]);
export const buildDataSize = externalStore<number | null>(null);

// A failed read shows no simulators and reports each distinct error once, so a missing xcrun shows its install step without a toast every few seconds.
let reported = "";
export function watchRunningSimulators(worktree: string): () => void {
  let live = true;
  const refresh = () => {
    if (document.visibilityState !== "visible") return;
    commands
      .runningSimulators(worktree)
      .catch((error) => {
        if (String(error) !== reported) fail(error);
        reported = String(error);
        return [];
      })
      .then((list) => live && runningSimulators.set(list));
  };
  refresh();
  const timer = window.setInterval(refresh, REFRESH_MS);
  window.addEventListener("focus", refresh);
  return () => {
    live = false;
    window.clearInterval(timer);
    window.removeEventListener("focus", refresh);
  };
}

export async function shutDownOthers(worktree: string) {
  await commands.shutDownOtherSimulators(worktree);
  runningSimulators.set(await commands.runningSimulators(worktree));
}

export async function loadBuildDataSize() {
  // The size is a readout: a failed read leaves the value blank.
  buildDataSize.set(await commands.buildDataSize().catch(() => null));
}

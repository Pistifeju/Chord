// Connects the running simulators, the build data size and the service groups to IPC: the list kept fresh while the button shows, Shut Down Others, the size and the groups read when Settings shows them, and the switches saved through Rust and the groups read back with each one's resolved state.
import { commands, events, type RunningSimulator, type ServiceRow } from "../../../ipc/bindings";
import { externalStore } from "../../../ui/externalStore";
import { fail } from "../../worktrees/store";

/** How often the list is read when the simulator frameworks did not load and no change event comes. */
const REFRESH_MS = 5000;

export const runningSimulators = externalStore<RunningSimulator[]>([]);
export const buildDataSize = externalStore<number | null>(null);
export const serviceGroups = externalStore<ServiceRow[]>([]);

// A failed read shows no simulators and reports each distinct error once, so a missing xcrun shows its install step without a toast every few seconds.
let reported = "";
export function watchRunningSimulators(worktree: string): () => void {
  let live = true;
  let reading = false;
  let again = false;
  let timer: number | undefined;
  // A boot sends several change events; they share one read, and one more after it.
  const refresh = () => {
    if (!live || document.visibilityState !== "visible") return;
    if (reading) return void (again = true);
    reading = true;
    commands
      .runningSimulators(worktree)
      .catch((error) => {
        if (String(error) !== reported) fail(error);
        reported = String(error);
        return [];
      })
      .then((list) => live && runningSimulators.set(list))
      .finally(() => {
        reading = false;
        if (again) {
          again = false;
          refresh();
        }
      });
  };
  refresh();
  const unlisten = events.simulatorsChanged.listen(refresh);
  commands
    .watchSimulators()
    .catch(() => false)
    .then((watching) => {
      if (!watching && live) timer = window.setInterval(refresh, REFRESH_MS);
    });
  // Changes while the window was hidden were not read.
  document.addEventListener("visibilitychange", refresh);
  window.addEventListener("focus", refresh);
  return () => {
    live = false;
    window.clearInterval(timer);
    void unlisten.then((off) => off());
    document.removeEventListener("visibilitychange", refresh);
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

export async function loadServiceGroups() {
  serviceGroups.set(await commands.simulatorServiceGroups());
}

export async function setServiceKept(ids: string[], kept: boolean) {
  await commands.changeState({ keptServices: { ids, kept } });
  await loadServiceGroups();
}

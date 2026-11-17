// The run control at the tab bar's right end: the run destination, which opens the run menu, and a play button that builds and runs, a stop square from the start of a build until the app launches.
import { CheckMenuItem, Menu, MenuItem, PredefinedMenuItem } from "@tauri-apps/api/menu";
import { LogicalPosition } from "@tauri-apps/api/dpi";
import type { MouseEvent } from "react";
import type { RunDestination } from "../../../ipc/bindings";
import { menuItem, tooltip } from "../../../app/commands";
import { openContextMenu } from "../../../app/contextMenu";
import { shortcutChanges } from "../../settings/store";
import { fail } from "../../../ui/alert";
import { ChevronDownIcon, PhoneIcon, PlayIcon, StopIcon } from "../../../ui/icons";
import { sameDestination } from "./destination";
import { chooseRunDestination, destinations, refreshDestinations, runDestinationOf, useRunDestination } from "./destinationStore";
import { isRunning, isStarting } from "./run";
import { builds, runApp, stopBuild } from "./store";

const disabledLine = (text: string) => MenuItem.new({ text, enabled: false });

const choiceItem = (pick: RunDestination | null, destination: RunDestination, choose: (destination: RunDestination) => void) =>
  CheckMenuItem.new({ text: destination.name, checked: !!pick && sameDestination(pick, destination), action: () => choose(destination) });

/** The simulator models under a line per iOS version, `pick` checked; the Preview view's model menu lists them too. */
export async function simulatorItems(pick: RunDestination | null, choose: (destination: RunDestination) => void) {
  const listing = destinations.get();
  if (!listing?.simulators.length) return [await disabledLine("No Simulators")];
  const items = [];
  for (const group of listing.simulators) {
    items.push(await disabledLine(group.os));
    for (const model of group.models) items.push(await choiceItem(pick, model, choose));
  }
  return items;
}

// The commands, then the devices under their line and the simulator models under a line per iOS version; the project's pick is checked.
async function runMenu(worktree: string) {
  const listing = destinations.get();
  const pick = runDestinationOf(worktree);
  const choose = (destination: RunDestination) => chooseRunDestination(worktree, destination);
  const build = builds.get()[worktree];
  const stop = await MenuItem.new({ ...menuItem("build.stop", worktree), enabled: isStarting(build) || isRunning(build) });
  const items = [await MenuItem.new(menuItem("build.run", worktree)), await MenuItem.new(menuItem("build.start", worktree)), stop, await PredefinedMenuItem.new({ item: "Separator" })];
  if (listing?.devices.length) {
    items.push(await disabledLine("Devices"));
    for (const device of listing.devices) items.push(await choiceItem(pick, device, choose));
  }
  items.push(...(await simulatorItems(pick, choose)));
  return Menu.new({ items });
}

/** Opens `menu` under `button` once a destination listing is known. */
export function popUpUnder(worktree: string, button: HTMLElement, menu: () => Promise<Menu>) {
  return openContextMenu(async () => {
    // The listing is the last one while the new one loads: the menu opens at once and a later open shows the new one.
    const refreshing = refreshDestinations(worktree).catch(fail);
    if (!destinations.get()) await refreshing;
    const box = button.getBoundingClientRect();
    await (await menu()).popup(new LogicalPosition(box.left, box.bottom));
  });
}

export function RunControl({ worktree }: { worktree: string }) {
  shortcutChanges.use();
  const building = builds.usePart((all) => isStarting(all[worktree]));
  const destination = useRunDestination(worktree);
  const name = destination?.name ?? "Newest iPhone";
  const os = destination?.kind === "simulator" ? destination.os : null;
  return (
    <div className="run-controls">
      <button
        type="button"
        className="run-destination"
        title="Run Destination"
        aria-label={`Run Destination: ${os ? `${name}, ${os}` : name}`}
        onClick={(e: MouseEvent<HTMLButtonElement>) => popUpUnder(worktree, e.currentTarget, () => runMenu(worktree)).catch(fail)}
      >
        <PhoneIcon />
        <span className="run-destination-name">{name}</span>
        {os && <span className="run-destination-os">{os}</span>}
        <ChevronDownIcon />
      </button>
      <button
        type="button"
        className="icon-button run-control"
        title={tooltip(building ? "build.stop" : "build.run")}
        aria-label={building ? "Stop" : "Run"}
        onClick={() => (building ? stopBuild(worktree) : runApp(worktree))}
      >
        {building ? <StopIcon /> : <PlayIcon />}
      </button>
    </div>
  );
}

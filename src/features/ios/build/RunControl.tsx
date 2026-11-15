// The run control at the tab bar's right end: the run destination, which opens the run menu, and a play button that builds and runs, a stop square from the start of a build until the app launches.
import { CheckMenuItem, Menu, MenuItem, PredefinedMenuItem } from "@tauri-apps/api/menu";
import { LogicalPosition } from "@tauri-apps/api/dpi";
import type { MouseEvent } from "react";
import { menuItem, tooltip } from "../../../app/commands";
import { shortcutChanges } from "../../settings/store";
import { fail } from "../../worktrees/store";
import { ChevronDownIcon, PhoneIcon, PlayIcon, StopIcon } from "../../../ui/icons";
import { sameDestination } from "./destination";
import { chooseRunDestination, destinations, refreshDestinations, runDestinationOf, useRunDestination } from "./destinationStore";
import { isRunning, isStarting } from "./run";
import { builds, startBuild, stopBuild } from "./store";

const disabledLine = (text: string) => MenuItem.new({ text, enabled: false });

// The commands, then the devices under their line and the simulator models under a line per iOS version; the project's pick is checked.
async function runMenu(worktree: string) {
  const listing = destinations.get();
  const pick = runDestinationOf(worktree);
  const choice = (destination: NonNullable<typeof pick>, text: string) =>
    CheckMenuItem.new({ text, checked: !!pick && sameDestination(pick, destination), action: () => chooseRunDestination(worktree, destination) });
  const build = builds.get()[worktree];
  const stop = await MenuItem.new({ ...menuItem("build.stop", worktree), enabled: isStarting(build) || isRunning(build) });
  const items = [await MenuItem.new(menuItem("build.run", worktree)), await MenuItem.new(menuItem("build.start", worktree)), stop, await PredefinedMenuItem.new({ item: "Separator" })];
  if (listing?.devices.length) {
    items.push(await disabledLine("Devices"));
    for (const device of listing.devices) items.push(await choice(device, device.name));
  }
  if (!listing?.simulators.length) items.push(await disabledLine("No Simulators"));
  for (const group of listing?.simulators ?? []) {
    items.push(await disabledLine(group.os));
    for (const model of group.models) items.push(await choice(model, model.name));
  }
  return Menu.new({ items });
}

async function openRunMenu(worktree: string, button: HTMLElement) {
  // The listing is the last one while the new one loads: the menu opens at once and a later open shows the new one.
  const refreshing = refreshDestinations(worktree).catch(fail);
  if (!destinations.get()) await refreshing;
  const box = button.getBoundingClientRect();
  await (await runMenu(worktree)).popup(new LogicalPosition(box.left, box.bottom));
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
        onClick={(e: MouseEvent<HTMLButtonElement>) => openRunMenu(worktree, e.currentTarget).catch(fail)}
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
        onClick={() => (building ? stopBuild(worktree) : startBuild(worktree, true))}
      >
        {building ? <StopIcon /> : <PlayIcon />}
      </button>
    </div>
  );
}

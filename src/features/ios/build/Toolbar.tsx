// The window toolbar over the panes in a worktree with an Xcode project: Run, Stop and Test, the scheme and run destination button that opens the run menu with the build configuration, the activity field with the build's status and error count, and the side panel's view buttons.
import { CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu } from "@tauri-apps/api/menu";
import { LogicalPosition } from "@tauri-apps/api/dpi";
import { useEffect, type MouseEvent, type ReactNode } from "react";
import type { RunDestination } from "../../../ipc/bindings";
import { menuItem, tooltip } from "../../../app/commands";
import { openContextMenu } from "../../../app/contextMenu";
import { shortcutChanges } from "../../settings/store";
import { fail } from "../../../ui/alert";
import { ChevronIcon, PhoneIcon, PlayIcon, SchemeIcon, StopIcon, TestIcon, WarningIcon } from "../../../ui/icons";
import { sidebar } from "../../sidebar/store";
import { sameDestination } from "./destination";
import { chooseRunDestination, destinations, refreshDestinations, runDestinationOf, useRunDestination } from "./destinationStore";
import { activityOf, isRunning, isStarting } from "./run";
import { shownPick } from "./scheme";
import { chooseSchemePick, refreshSchemes, schemeListings, schemePickOf, useSchemePick } from "./schemeStore";
import { builds, runApp, showBuildLog, stopBuild } from "./store";
import { messageTime } from "../../chat/messageTime";
import "./toolbar.css";

const disabledLine = (text: string) => MenuItem.new({ text, enabled: false });

const choiceItem = (pick: RunDestination | null, destination: RunDestination, choose: (destination: RunDestination) => void) =>
  CheckMenuItem.new({ text: destination.name, checked: !!pick && sameDestination(pick, destination), action: () => choose(destination) });

/** The simulator models under a line per iOS version, a separator between versions, `pick` checked; the Preview view's model menu lists them too. */
export async function simulatorItems(pick: RunDestination | null, choose: (destination: RunDestination) => void) {
  const listing = destinations.get();
  if (!listing?.simulators.length) return [await disabledLine("No Simulators")];
  const items: (MenuItem | CheckMenuItem | PredefinedMenuItem)[] = [];
  for (const group of listing.simulators) {
    if (items.length) items.push(await PredefinedMenuItem.new({ item: "Separator" }));
    items.push(await disabledLine(group.os));
    for (const model of group.models) items.push(await choiceItem(pick, model, choose));
  }
  return items;
}

// The Scheme and Build Configuration submenus, the project's picks checked; none before the first listing.
async function schemeItems(worktree: string) {
  const listing = schemeListings.get()[worktree];
  if (!listing) return [];
  const shown = shownPick(schemePickOf(worktree), listing);
  const submenu = async (text: string, names: string[], checked: string | null, choose: (name: string) => void) =>
    Submenu.new({ text, items: await Promise.all(names.map((name) => CheckMenuItem.new({ text: name, checked: name === checked, action: () => choose(name) }))) });
  const items = [await PredefinedMenuItem.new({ item: "Separator" }), await submenu("Scheme", listing.schemes.map((listed) => listed.name), shown.scheme, (scheme) => chooseSchemePick(worktree, { scheme }))];
  if (listing.configurations.length)
    items.push(await submenu("Build Configuration", listing.configurations, shown.configuration, (configuration) => chooseSchemePick(worktree, { configuration })));
  return items;
}

// The commands, then the devices under their line and the simulator models under a line per iOS version, then the scheme submenus; the project's picks are checked.
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
    items.push(await PredefinedMenuItem.new({ item: "Separator" }));
  }
  items.push(...(await simulatorItems(pick, choose)), ...(await schemeItems(worktree)));
  return Menu.new({ items });
}

/** Opens `menu` under `button` once a destination listing is known and `loading` ends. */
export async function popUpUnder(worktree: string, button: HTMLElement, menu: () => Promise<Menu>, loading?: Promise<unknown>) {
  // The listing is the last one while the new one loads: the menu opens at once and a later open shows the new one.
  const refreshing = refreshDestinations(worktree).catch(fail);
  // Loads wait outside the menu lock, so a slow one cannot block every other menu.
  await Promise.all([destinations.get() ? null : refreshing, loading]);
  const box = button.getBoundingClientRect();
  await openContextMenu(async () => (await menu()).popup(new LogicalPosition(box.left, box.bottom)));
}

// The scheme's name is known once the listing came; the toolbar reads it when it shows, as the menu does when it opens.
function useSchemeName(worktree: string): string | null {
  const listing = schemeListings.usePart((all) => all[worktree]);
  const pick = useSchemePick(worktree);
  useEffect(() => {
    if (!schemeListings.get()[worktree]) refreshSchemes(worktree).catch(console.error);
  }, [worktree]);
  return listing ? shownPick(pick, listing).scheme : null;
}

// Today's time reads "today at 10:32", the way a message's older times read.
function endTime(at: number): string {
  const time = messageTime(at, Date.now());
  return time.includes(" at ") ? time : `today at ${time}`;
}

function Activity({ worktree, scheme, destination }: { worktree: string; scheme: string | null; destination: string }) {
  const build = builds.usePart((all) => all[worktree]);
  const activity = activityOf(build, scheme, destination, endTime);
  const text = activity.detail ? `${activity.lead} ${activity.detail}` : activity.lead;
  return (
    <div className="toolbar-activity" data-tone={activity.tone} role="status">
      <span className="toolbar-activity-text" title={text}>
        <span className="toolbar-activity-lead">{activity.lead}</span>
        {activity.detail && ` ${activity.detail}`}
      </span>
      {activity.errors > 0 && (
        <button type="button" className="toolbar-errors" title="Show Build Log" aria-label={`${activity.errors} errors, Show Build Log`} onClick={() => showBuildLog(worktree)}>
          <WarningIcon />
          {activity.errors}
        </button>
      )}
      {activity.tone === "busy" && <span className="toolbar-progress" aria-hidden="true" />}
    </div>
  );
}

// `viewButtons` are the side panel's view buttons, passed in by the window.
export function Toolbar({ worktree, viewButtons }: { worktree: string; viewButtons: ReactNode }) {
  shortcutChanges.use();
  const { hidden } = sidebar.use();
  const busy = builds.usePart((all) => isStarting(all[worktree]) || isRunning(all[worktree]));
  const destination = useRunDestination(worktree);
  const scheme = useSchemeName(worktree);
  const name = destination?.name ?? "Newest iPhone";
  const os = destination?.kind === "simulator" ? destination.os : null;
  return (
    <div className="toolbar" data-tauri-drag-region data-sidebar-hidden={hidden || undefined}>
      <button type="button" className="icon-button toolbar-run" title={tooltip("build.run")} aria-label="Run" onClick={() => runApp(worktree)}>
        <PlayIcon />
      </button>
      <button type="button" className="icon-button toolbar-stop" title={tooltip("build.stop")} aria-label="Stop" disabled={!busy} onClick={() => stopBuild(worktree)}>
        <StopIcon />
      </button>
      <button type="button" className="icon-button" title="Test" aria-label="Test" disabled>
        <TestIcon />
      </button>
      <button
        type="button"
        className="toolbar-scheme"
        title="Scheme and Run Destination"
        aria-label={`Scheme and Run Destination: ${[scheme, name, os].filter(Boolean).join(", ")}`}
        onClick={(e: MouseEvent<HTMLButtonElement>) => {
          // Listed beside the destinations; like them, the last listing shows while the new one loads.
          const listing = refreshSchemes(worktree).catch(fail);
          popUpUnder(worktree, e.currentTarget, () => runMenu(worktree), schemeListings.get()[worktree] ? undefined : listing).catch(fail);
        }}
      >
        <SchemeIcon />
        {scheme && (
          <>
            <span className="toolbar-scheme-name">{scheme}</span>
            <span className="toolbar-scheme-separator">
              <ChevronIcon />
            </span>
          </>
        )}
        <PhoneIcon />
        <span className="toolbar-destination">{name}</span>
        {os && <span className="toolbar-destination-os">{os}</span>}
      </button>
      <span className="toolbar-spacer" data-tauri-drag-region />
      <Activity worktree={worktree} scheme={scheme} destination={name} />
      <span className="toolbar-spacer" data-tauri-drag-region />
      <div className="side-view-toggles">{viewButtons}</div>
    </div>
  );
}

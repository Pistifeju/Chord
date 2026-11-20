// The window toolbar over the panes in a worktree with an Xcode project: Run and Stop, the activity field with the run target (the scheme button with its scheme and build configuration menu, the run destination button with the run menu), the build's status and error count with its error list, and the side panel's view buttons.
import { CheckMenuItem, Menu, MenuItem, PredefinedMenuItem } from "@tauri-apps/api/menu";
import { LogicalPosition } from "@tauri-apps/api/dpi";
import { useEffect, useRef, type MouseEvent, type ReactNode } from "react";
import type { RunDestination, SchemePick_Serialize as SchemePick } from "../../../ipc/bindings";
import { menuItem, tooltip } from "../../../app/commands";
import { openContextMenu } from "../../../app/contextMenu";
import { shortcutChanges } from "../../settings/store";
import { fail } from "../../../ui/alert";
import { ChevronIcon, PhoneIcon, PlayIcon, SchemeIcon, StopIcon, WarningIcon } from "../../../ui/icons";
import { sidebar } from "../../sidebar/store";
import { sameDestination } from "./destination";
import { chooseRunDestination, destinations, refreshDestinations, runDestinationOf, useFallbackSimulator, useRunDestination } from "./destinationStore";
import { closeErrorList, errorList, toggleErrorList } from "./errorListStore";
import { ErrorList } from "./ErrorList";
import { activityOf, failedBuild, isRunning, isStarting } from "./run";
import { shownPick } from "./scheme";
import { chooseSchemePick, refreshSchemes, schemeListings, schemePickOf, useSchemePick } from "./schemeStore";
import { builds, runApp, stopBuild } from "./store";
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

// The worktree's schemes, then its project's configurations under their line; the shown picks are checked.
async function schemeMenu(worktree: string) {
  const listing = schemeListings.get()[worktree];
  if (!listing) return Menu.new({ items: [await disabledLine("No Schemes")] });
  const shown = shownPick(schemePickOf(worktree), listing);
  const choices = (names: string[], checked: string | null, choose: (name: string) => void) =>
    Promise.all(names.map((name) => CheckMenuItem.new({ text: name, checked: name === checked, action: () => choose(name) })));
  const items: (MenuItem | CheckMenuItem | PredefinedMenuItem)[] = await choices(
    listing.schemes.map((listed) => listed.name),
    shown.scheme,
    (scheme) => chooseSchemePick(worktree, { scheme }),
  );
  if (listing.configurations.length)
    items.push(
      await PredefinedMenuItem.new({ item: "Separator" }),
      await disabledLine("Build Configuration"),
      ...(await choices(listing.configurations, shown.configuration, (configuration) => chooseSchemePick(worktree, { configuration }))),
    );
  return Menu.new({ items });
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
    items.push(await PredefinedMenuItem.new({ item: "Separator" }));
  }
  items.push(...(await simulatorItems(pick, choose)));
  return Menu.new({ items });
}

/** Opens `menu` under `button` once `loading` ends. */
async function popUp(button: HTMLElement, menu: () => Promise<Menu>, loading?: Promise<unknown>) {
  // Loads wait outside the menu lock, so a slow one cannot block every other menu.
  await loading;
  const box = button.getBoundingClientRect();
  await openContextMenu(async () => (await menu()).popup(new LogicalPosition(box.left, box.bottom)));
}

/** Opens `menu` under `button` once a destination listing is known and `loading` ends. */
export function popUpUnder(worktree: string, button: HTMLElement, menu: () => Promise<Menu>, loading?: Promise<unknown>) {
  // The listing is the last one while the new one loads: the menu opens at once and a later open shows the new one.
  const refreshing = refreshDestinations(worktree).catch(fail);
  return popUp(button, menu, Promise.all([destinations.get() ? null : refreshing, loading]));
}

// The saved pick shows at once; without one, the scheme a build uses shows once the listing came.
function useShownPick(worktree: string): SchemePick {
  const listing = schemeListings.usePart((all) => all[worktree]);
  const pick = useSchemePick(worktree);
  useEffect(() => {
    if (!schemeListings.get()[worktree]) refreshSchemes(worktree).catch(console.error);
  }, [worktree]);
  return listing ? shownPick(pick, listing) : pick;
}

// Today's time reads "today at 10:32", the way a message's older times read.
function endTime(at: number): string {
  const time = messageTime(at, Date.now());
  return time.includes(" at ") ? time : `today at ${time}`;
}

// `target` is the scheme and destination buttons, at the field's left.
function Activity({ worktree, scheme, destination, target }: { worktree: string; scheme: string | null; destination: string; target: ReactNode }) {
  const build = builds.usePart((all) => all[worktree]);
  const activity = activityOf(build, scheme, destination, endTime);
  const text = activity.detail ? `${activity.lead} ${activity.detail}` : activity.lead;
  const count = useRef<HTMLButtonElement>(null);
  const failed = failedBuild(build);
  const open = errorList.usePart((shown) => shown === worktree);
  // A new build takes the errors away, so their list closes.
  useEffect(() => {
    if (open && !failed) closeErrorList();
  }, [open, failed]);
  return (
    <div className="toolbar-activity" data-tone={activity.tone}>
      {target}
      <span className="toolbar-activity-text" role="status" title={text}>
        <span className="toolbar-activity-lead">{activity.lead}</span>
        {activity.detail && ` ${activity.detail}`}
      </span>
      {activity.errors > 0 && (
        <button
          ref={count}
          type="button"
          className="toolbar-errors"
          data-menu-toggle
          aria-haspopup="dialog"
          aria-expanded={open}
          title={tooltip("build.showErrors")}
          aria-label={`${activity.errors} errors, Show Build Errors`}
          onClick={() => toggleErrorList(worktree)}
        >
          <WarningIcon />
          {activity.errors}
        </button>
      )}
      {open && failed && <ErrorList worktree={worktree} build={failed} anchor={count} />}
      {activity.tone === "busy" && <span className="toolbar-progress" aria-hidden="true" />}
    </div>
  );
}

// `viewButtons` are the side panel's view buttons, passed in by the window.
export function Toolbar({ worktree, viewButtons }: { worktree: string; viewButtons: ReactNode }) {
  shortcutChanges.use();
  const { hidden } = sidebar.use();
  const busy = builds.usePart((all) => isStarting(all[worktree]) || isRunning(all[worktree]));
  const fallback = useFallbackSimulator(worktree);
  const destination = useRunDestination(worktree) ?? fallback;
  const { scheme, configuration } = useShownPick(worktree);
  const name = destination?.name ?? "";
  const os = destination?.kind === "simulator" ? destination.os : null;
  return (
    <div className="toolbar" data-tauri-drag-region data-sidebar-hidden={hidden || undefined}>
      <div className="toolbar-lead" data-tauri-drag-region>
        <button type="button" className="icon-button toolbar-run" title={tooltip("build.run")} aria-label="Run" onClick={() => runApp(worktree)}>
          <PlayIcon />
        </button>
        <button type="button" className="icon-button toolbar-stop" title={tooltip("build.stop")} aria-label="Stop" disabled={!busy} onClick={() => stopBuild(worktree)}>
          <StopIcon />
        </button>
      </div>
      <Activity
        worktree={worktree}
        scheme={scheme}
        destination={name}
        target={
          <div className="toolbar-target">
            <button
              type="button"
              className="toolbar-scheme-part"
              title="Scheme and Build Configuration"
              aria-label={`Scheme and Build Configuration: ${[scheme, configuration].filter(Boolean).join(", ")}`}
              onClick={(e: MouseEvent<HTMLButtonElement>) => {
                // The last listing shows while the new one loads.
                const listing = refreshSchemes(worktree).catch(fail);
                popUp(e.currentTarget, () => schemeMenu(worktree), schemeListings.get()[worktree] ? undefined : listing).catch(fail);
              }}
            >
              <SchemeIcon />
              {scheme && <span className="toolbar-scheme-name">{scheme}</span>}
              {configuration && <span className="toolbar-configuration">{configuration}</span>}
            </button>
            <span className="toolbar-scheme-separator">
              <ChevronIcon />
            </span>
            <button
              type="button"
              className="toolbar-scheme-part"
              title="Run Destination"
              aria-label={`Run Destination: ${[name, os].filter(Boolean).join(", ")}`}
              onClick={(e: MouseEvent<HTMLButtonElement>) => popUpUnder(worktree, e.currentTarget, () => runMenu(worktree)).catch(fail)}
            >
              <PhoneIcon />
              <span className="toolbar-destination">{name}</span>
              {os && <span className="toolbar-destination-os">{os}</span>}
            </button>
          </div>
        }
      />
      <div className="side-view-toggles">{viewButtons}</div>
    </div>
  );
}

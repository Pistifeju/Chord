// The status bar's simulators button: how many simulators Chord created are booted, and the menu that lists them with Shut Down Others.
import { LogicalPosition } from "@tauri-apps/api/dpi";
import { Menu, MenuItem, PredefinedMenuItem } from "@tauri-apps/api/menu";
import { useEffect, type MouseEvent } from "react";
import { openContextMenu } from "../../../app/contextMenu";
import { fail } from "../../worktrees/store";
import { PhoneIcon } from "../../../ui/icons";
import { hasOtherSimulators, simulatorLine, simulatorsTitle } from "./simulators";
import { runningSimulators, shutDownOthers, watchRunningSimulators } from "./store";
import "./simulators.css";

async function simulatorsMenu(worktree: string) {
  const running = runningSimulators.get();
  const items = await Promise.all(running.map((simulator) => MenuItem.new({ text: simulatorLine(simulator), enabled: false })));
  return Menu.new({
    items: [
      ...items,
      await PredefinedMenuItem.new({ item: "Separator" }),
      await MenuItem.new({ text: "Shut Down Others", enabled: hasOtherSimulators(running), action: () => shutDownOthers(worktree).catch(fail) }),
    ],
  });
}

function openMenu(worktree: string, button: HTMLElement) {
  const box = button.getBoundingClientRect();
  return openContextMenu(async () => (await simulatorsMenu(worktree)).popup(new LogicalPosition(box.left, box.top)));
}

export function SimulatorsButton({ worktree }: { worktree: string }) {
  useEffect(() => watchRunningSimulators(worktree), [worktree]);
  const count = runningSimulators.usePart((list) => list.length);
  if (count === 0) return null;
  const title = simulatorsTitle(count);
  return (
    <button type="button" className="statusbar-simulators" title={title} aria-label={title} onClick={(e: MouseEvent<HTMLButtonElement>) => openMenu(worktree, e.currentTarget).catch(fail)}>
      <PhoneIcon />
      {count}
    </button>
  );
}

// The Appearance button in the Simulator view's header: a native menu of light and dark, the Dynamic Type size, the language and the region of the worktree's simulator. Disabled while the simulator is not booted.
import { LogicalPosition } from "@tauri-apps/api/dpi";
import { CheckMenuItem, Menu, Submenu } from "@tauri-apps/api/menu";
import type { MouseEvent } from "react";
import { commands, type SimulatorAppearance, type SimulatorAppearanceChange, type InterfaceStyle } from "../../../ipc/bindings";
import { AppearanceIcon } from "../../../ui/icons";
import { fail } from "../../worktrees/store";
import { namedCodes, sizeLabel } from "./appearance";

const change = (worktree: string, what: SimulatorAppearanceChange) => commands.setSimulatorAppearance(worktree, what).catch(fail);

function displayName(type: "language" | "region") {
  const names = new Intl.DisplayNames([navigator.language], { type });
  return (code: string) => {
    try {
      return names.of(code);
    } catch {
      return undefined;
    }
  };
}

/** One checked item per value, the current one checked; picking one sends its change. */
const choices = <T,>(worktree: string, values: { value: T; text: string }[], current: unknown, toChange: (value: T) => SimulatorAppearanceChange) =>
  Promise.all(values.map(({ value, text }) => CheckMenuItem.new({ text, checked: current === value, action: () => void change(worktree, toChange(value)) })));

const named = (codes: string[], type: "language" | "region") => namedCodes(codes, displayName(type)).map(({ code, name }) => ({ value: code, text: name }));

async function appearanceMenu(worktree: string, now: SimulatorAppearance) {
  const styles: { value: InterfaceStyle; text: string }[] = [
    { value: "light", text: "Light" },
    { value: "dark", text: "Dark" },
  ];
  const sizes = now.contentSizes.map((size) => ({ value: size, text: sizeLabel(size) }));
  return Menu.new({
    items: [
      ...(await choices(worktree, styles, now.style, (style) => ({ kind: "style", style }))),
      await Submenu.new({ text: "Text Size", items: await choices(worktree, sizes, now.contentSize, (size) => ({ kind: "content_size", size })) }),
      await Submenu.new({ text: "Language", items: await choices(worktree, named(now.languages, "language"), now.language, (language) => ({ kind: "language", language })) }),
      await Submenu.new({ text: "Region", items: await choices(worktree, named(now.regions, "region"), now.region, (region) => ({ kind: "region", region })) }),
    ],
  });
}

export function AppearanceButton({ worktree, enabled }: { worktree: string; enabled: boolean }) {
  const open = async (e: MouseEvent<HTMLButtonElement>) => {
    const box = e.currentTarget.getBoundingClientRect();
    const menu = await appearanceMenu(worktree, await commands.simulatorAppearance(worktree));
    await menu.popup(new LogicalPosition(box.left, box.bottom));
  };
  return (
    <button type="button" className="icon-button" title="Appearance" aria-label="Appearance" disabled={!enabled} onClick={(e) => open(e).catch(fail)}>
      <AppearanceIcon />
    </button>
  );
}

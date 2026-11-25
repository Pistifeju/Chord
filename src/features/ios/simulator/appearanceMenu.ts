import { LogicalPosition } from "@tauri-apps/api/dpi";
import { Menu, type CheckMenuItemOptions } from "@tauri-apps/api/menu";
import { commands, type SimulatorAppearance, type SimulatorAppearanceChange, type InterfaceStyle } from "../../../ipc/bindings";
import { openContextMenu } from "../../../app/contextMenu";
import { fail } from "../../../ui/alert";
import { namedCodes, sizeLabel, withChange } from "./appearance";

const known = new Map<string, SimulatorAppearance>();

const keyOf = (worktree: string, preview: boolean) => `${preview ? "preview" : "run"}:${worktree}`;

export async function loadAppearance(worktree: string, preview: boolean) {
  const now = await commands.simulatorAppearance(worktree, preview);
  known.set(keyOf(worktree, preview), now);
  return now;
}

function change(worktree: string, preview: boolean, what: SimulatorAppearanceChange) {
  const now = known.get(keyOf(worktree, preview));
  if (now) known.set(keyOf(worktree, preview), withChange(now, what));
  commands
    .setSimulatorAppearance(worktree, preview, what)
    .then(() => loadAppearance(worktree, preview))
    .catch(fail);
}

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

const choices = <T,>(worktree: string, preview: boolean, values: { value: T; text: string }[], current: unknown, toChange: (value: T) => SimulatorAppearanceChange): CheckMenuItemOptions[] =>
  values.map(({ value, text }) => ({ text, checked: current === value, action: () => change(worktree, preview, toChange(value)) }));

const named = (codes: string[], type: "language" | "region") => namedCodes(codes, displayName(type)).map(({ code, name }) => ({ value: code, text: name }));

function appearanceMenu(worktree: string, preview: boolean, now: SimulatorAppearance) {
  const styles: { value: InterfaceStyle; text: string }[] = [
    { value: "light", text: "Light" },
    { value: "dark", text: "Dark" },
  ];
  const sizes = now.contentSizes.map((size) => ({ value: size, text: sizeLabel(size) }));
  return Menu.new({
    items: [
      ...choices(worktree, preview, styles, now.style, (style) => ({ kind: "style", style })),
      { text: "Text Size", items: choices(worktree, preview, sizes, now.contentSize, (size) => ({ kind: "content_size", size })) },
      { text: "Language", items: choices(worktree, preview, named(now.languages, "language"), now.language, (language) => ({ kind: "language", language })) },
      { text: "Region", items: choices(worktree, preview, named(now.regions, "region"), now.region, (region) => ({ kind: "region", region })) },
    ],
  });
}

export async function openAppearanceMenu(worktree: string, preview: boolean, button: HTMLElement) {
  const box = button.getBoundingClientRect();
  const reading = loadAppearance(worktree, preview);
  const last = known.get(keyOf(worktree, preview));
  if (last) reading.catch(console.error);
  const now = last ?? (await reading);
  await openContextMenu(async () => (await appearanceMenu(worktree, preview, now)).popup(new LogicalPosition(box.left, box.bottom)));
}

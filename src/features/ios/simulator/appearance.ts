// Rules for the Simulator view's Appearance menu: how a Dynamic Type size reads, the language and region codes by name, and the settings after a change.
import type { SimulatorAppearance, SimulatorAppearanceChange } from "../../../ipc/bindings";

/** "extra-large" reads "Extra Large". */
export const sizeLabel = (category: string) =>
  category
    .split("-")
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
    .join(" ");

/** The codes with the name `nameOf` gives each, in name order; a code without a name keeps the code. */
export function namedCodes(codes: string[], nameOf: (code: string) => string | undefined): { code: string; name: string }[] {
  return codes.map((code) => ({ code, name: nameOf(code) ?? code })).sort((a, b) => a.name.localeCompare(b.name));
}

/** The settings after `change`, before the simulator reports them. */
export function withChange(now: SimulatorAppearance, change: SimulatorAppearanceChange): SimulatorAppearance {
  switch (change.kind) {
    case "style":
      return { ...now, style: change.style };
    case "content_size":
      return { ...now, contentSize: change.size };
    case "language":
      return { ...now, language: change.language };
    case "region":
      return { ...now, region: change.region };
  }
}
